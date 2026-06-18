//! Rendering kernel output: text streams, errors, and inline graphics.
//!
//! Frames come off the websocket as JSON; [`jet_core::events::parse_event`]
//! turns them into a typed [`Event`]. [`Renderer`] consumes each event:
//! rendering content events to stdout, and forwarding `Idle` parent_ids on
//! `idle_tx` so the REPL knows it's safe to prompt again.

mod kitty;
mod tmux;

pub use kitty::emit_png;
pub use tmux::warn_if_passthrough_off;

use std::io::Write;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use base64::Engine;
use jet_core::events::{Event, EventData, InputRequest};
use serde_json::Value;
use tokio::sync::mpsc;

pub type SharedWriter = Arc<Mutex<dyn Write + Send>>;

#[derive(Clone)]
pub struct Renderer {
    pub render_graphics: bool,
    pub idle_tx: mpsc::UnboundedSender<String>,
    pub input_tx: Option<mpsc::UnboundedSender<InputRequest>>,
    writer: SharedWriter,
    // True when the next byte we write will start a fresh line, so a
    // session prefix should be emitted before it. Tracked across writes
    // because kernel streams arrive in arbitrary chunks — a partial line
    // followed by more text must NOT get a second prefix.
    at_line_start: Arc<Mutex<bool>>,
}

impl Renderer {
    pub fn new(
        render_graphics: bool,
        idle_tx: mpsc::UnboundedSender<String>,
        writer: SharedWriter,
    ) -> Self {
        Self {
            render_graphics,
            idle_tx,
            input_tx: None,
            writer,
            at_line_start: Arc::new(Mutex::new(true)),
        }
    }

    pub fn with_input_tx(mut self, tx: mpsc::UnboundedSender<InputRequest>) -> Self {
        self.input_tx = Some(tx);
        self
    }

    pub fn handle_event(&self, event: Event) -> Result<()> {
        let (session_name, session_type) = if let Some(session_id) = event.parent_session.as_deref()
        {
            (
                session_id.split("---").nth(0),
                session_id.split("---").nth(1),
            )
        } else {
            (None, None)
        };

        // The default session name "jet" means the user didn't pass
        // --session-name, so we don't tag any output. Any other name is
        // shown as a `[name]` prefix on each line.
        let prefix = match session_name {
            Some("jet") | None => None,
            Some(name) => Some(name.to_string()),
        };

        match event.data {
            EventData::Stream { name: _, text } => self.write_prefixed(&text, prefix.as_deref())?,
            EventData::ExecuteInput { code } => match (session_name, session_type) {
                // Our own REPL already echoed the prompt locally — don't
                // duplicate it when the kernel rebroadcasts the input.
                (_, Some("repl")) => {}
                (Some(session), _) => self.write_line(&format!("[{session}]> {code}"))?,
                (_, _) => self.write_line(&format!("> {code}"))?,
            },
            EventData::DisplayData { data } => self.render_data(&data)?,
            EventData::Error { traceback } => {
                self.write_prefixed(&traceback, prefix.as_deref())?;
                self.ensure_newline()?;
            }
            EventData::Banner { text } => self.write_line(&text)?,
            EventData::Idle { parent_id } => {
                let _ = self.idle_tx.send(parent_id.unwrap_or_default());
            }
            EventData::InputRequest {
                prompt,
                password,
                parent_id,
            } => {
                if let Some(tx) = &self.input_tx {
                    let _ = tx.send(InputRequest {
                        prompt,
                        password,
                        parent_id: parent_id.unwrap_or_default(),
                    });
                }
            }
            EventData::KernelExited | EventData::Other => {}
        }
        Ok(())
    }

    /// Write a chunk to the terminal, inserting `[prefix] ` at the start
    /// of every new line. Honours streaming boundaries: if the previous
    /// write ended without a newline, the next call continues the same
    /// line (no extra prefix); a chunk that ends with `\n` leaves the
    /// next call expecting to start a new line.
    fn write_prefixed(&self, text: &str, prefix: Option<&str>) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        match prefix {
            None => {
                write!(w, "{text}")?;
            }
            Some(p) => {
                let tag = format!("[{p}] ");
                let mut first = true;
                // Use split_inclusive so we can tell whether the final
                // segment ended in a newline (full line) or not (partial).
                for segment in text.split_inclusive('\n') {
                    if first {
                        if *at_start {
                            write!(w, "{tag}")?;
                        }
                        first = false;
                    } else {
                        // We're past the first segment, so the previous
                        // segment ended with '\n' — start a fresh prefix.
                        write!(w, "{tag}")?;
                    }
                    write!(w, "{segment}")?;
                }
            }
        }
        *at_start = text.ends_with('\n');
        w.flush()?;
        Ok(())
    }

    /// Write a complete line (appends a newline if missing). Resets the
    /// streaming state to "at line start" so subsequent stream output is
    /// re-prefixed.
    fn write_line(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        if text.ends_with('\n') {
            write!(w, "{text}")?;
        } else {
            writeln!(w, "{text}")?;
        }
        *at_start = true;
        w.flush()?;
        Ok(())
    }

    fn ensure_newline(&self) -> Result<()> {
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        if !*at_start {
            writeln!(w)?;
            *at_start = true;
            w.flush()?;
        }
        Ok(())
    }

    fn render_data(&self, data: &Value) -> Result<()> {
        if !data.is_object() {
            return Ok(());
        }

        if let Some(image_data) = data.get("image/png").and_then(|s| s.as_str()) {
            if self.render_graphics {
                let mut w = self.writer.lock().unwrap();
                let mut at_start = self.at_line_start.lock().unwrap();
                match emit_png(&mut *w, image_data) {
                    Ok(()) => {
                        *at_start = true;
                        return Ok(());
                    }
                    Err(e) => {
                        log::warn!("kitty render failed: {e}");
                        eprintln!("\x1b[33m[jet] kitty render failed: {e}\x1b[0m");
                        return Ok(());
                    }
                }
            } else {
                let len = base64::engine::general_purpose::STANDARD
                    .decode(image_data)
                    .map(|b| b.len())
                    .unwrap_or(0);
                self.write_line(&format!("[image/png {len} bytes]"))?;
                return Ok(());
            }
        };

        if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
            self.write_line(t)?;
            return Ok(());
        }

        // Should we really return Ok here?
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_writes_stream_to_injected_writer() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        r.handle_event(Event {
            parent_session: None,
            data: EventData::Stream {
                name: "stdout".into(),
                text: "hello".into(),
            },
        })
        .unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(&*bytes, b"hello");
    }

    #[test]
    fn renderer_writes_stderr_uncolored() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        r.handle_event(Event {
            parent_session: None,
            data: EventData::Stream {
                name: "stderr".into(),
                text: "oops".into(),
            },
        })
        .unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "oops");
    }

    #[test]
    fn stream_event_prefixes_first_line_and_subsequent_lines() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        r.handle_event(Event {
            parent_session: Some("my-session---bg".into()),
            data: EventData::Stream {
                name: "stdout".into(),
                text: "Error\nSomething went wrong".into(),
            },
        })
        .unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(
            std::str::from_utf8(&*bytes).unwrap(),
            "[my-session] Error\n[my-session] Something went wrong"
        );
    }

    #[test]
    fn partial_lines_dont_get_double_prefixed() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        for chunk in ["hel", "lo ", "world\nbye"] {
            r.handle_event(Event {
                parent_session: Some("s---bg".into()),
                data: EventData::Stream {
                    name: "stdout".into(),
                    text: chunk.into(),
                },
            })
            .unwrap();
        }
        let bytes = captured.lock().unwrap();
        assert_eq!(
            std::str::from_utf8(&*bytes).unwrap(),
            "[s] hello world\n[s] bye"
        );
    }

    #[test]
    fn repl_session_is_not_prefixed() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        r.handle_event(Event {
            parent_session: Some("jet---repl".into()),
            data: EventData::Stream {
                name: "stdout".into(),
                text: "a\nb".into(),
            },
        })
        .unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(std::str::from_utf8(&*bytes).unwrap(), "a\nb");
    }

    #[test]
    fn renderer_signals_idle() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        r.handle_event(Event {
            parent_session: None,
            data: EventData::Idle {
                parent_id: Some("msg-1".into()),
            },
        })
        .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "msg-1");
    }
}

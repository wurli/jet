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
    // The session name passed via --session-name (None or "jet" if not
    // set). Output that originated from this same session is shown
    // un-prefixed; output from any *other* session sharing the kernel
    // is tagged so the user can tell who's typing.
    own_session_name: Option<String>,
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
            own_session_name: None,
            at_line_start: Arc::new(Mutex::new(true)),
        }
    }

    pub fn with_own_session_name(mut self, name: Option<String>) -> Self {
        self.own_session_name = name;
        self
    }

    pub fn with_input_tx(mut self, tx: mpsc::UnboundedSender<InputRequest>) -> Self {
        self.input_tx = Some(tx);
        self
    }

    pub fn handle_event(&self, event: Event) -> Result<()> {
        let session_name = event
            .parent_session
            .as_deref()
            .and_then(|id| id.split("---").next());

        // Tag every kernel-emitted line with the originating session
        // name when one was set, regardless of whether it came from
        // *this* REPL or another client sharing the kernel — the user
        // wants `[s2]` on every kernel line so it's clear which session
        // produced it. The default name "jet" (no --session-name) is
        // treated as "no tag".
        let own = self.own_session_name.as_deref().unwrap_or("jet");
        let is_own_session = session_name == Some(own);
        let prefix = match session_name {
            Some("jet") | None => None,
            Some(name) => Some(name.to_string()),
        };

        match event.data {
            EventData::Stream { name: _, text } => self.write_prefixed(&text, prefix.as_deref())?,
            EventData::ExecuteInput { code } => {
                // Skip the kernel's iopub echo of *our own* input —
                // rustyline already drew the prompt locally. For any
                // other session's input we render `[name]> code` so the
                // user can follow what other clients are doing.
                if !is_own_session {
                    if let Some(name) = session_name {
                        self.break_for_async_write()?;
                        self.write_line(&format!("[{name}]> {code}"))?;
                    } else {
                        self.break_for_async_write()?;
                        self.write_line(&format!("> {code}"))?;
                    }
                }
            }
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
                // segment ended in a line break (full line) or not (partial).
                // Both '\n' and '\r' count: '\r' is used by spinners /
                // progress bars to redraw a line in place, and we want each
                // redraw to start with a fresh prefix.
                for segment in text.split_inclusive(['\n', '\r']) {
                    if first {
                        if *at_start {
                            write!(w, "{tag}")?;
                        }
                        first = false;
                    } else {
                        write!(w, "{tag}")?;
                    }
                    write!(w, "{segment}")?;
                }
            }
        }
        *at_start = text.ends_with(['\n', '\r']);
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

    /// Make sure the next write starts on its own clean line, without
    /// destroying any content the renderer has already written.
    ///
    /// - If we last left the cursor at column 0 (`at_line_start`), the
    ///   only thing that could be on the current line is rustyline's
    ///   prompt — wipe it with `\r\x1b[2K` so async other-session output
    ///   doesn't land on top of it. Rustyline redraws on next keystroke.
    /// - If we last wrote partial content with no trailing newline (e.g.
    ///   a spinner frame), clearing would erase it. Emit a `\n` instead
    ///   to drop to a fresh line, preserving what's above.
    fn break_for_async_write(&self) -> Result<()> {
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        if *at_start {
            write!(w, "\r\x1b[2K")?;
        } else {
            writeln!(w)?;
            *at_start = true;
        }
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
    fn own_session_output_is_not_prefixed_other_sessions_are() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer)
            .with_own_session_name(Some("alice".into()));

        r.handle_event(Event {
            parent_session: Some("alice---repl---abc".into()),
            data: EventData::Stream {
                name: "stdout".into(),
                text: "mine\n".into(),
            },
        })
        .unwrap();
        r.handle_event(Event {
            parent_session: Some("bob---repl---xyz".into()),
            data: EventData::Stream {
                name: "stdout".into(),
                text: "theirs\n".into(),
            },
        })
        .unwrap();

        let bytes = captured.lock().unwrap();
        assert_eq!(
            std::str::from_utf8(&*bytes).unwrap(),
            "mine\n[bob] theirs\n"
        );
    }

    #[test]
    fn carriage_return_starts_a_fresh_prefix() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        r.handle_event(Event {
            parent_session: Some("s---bg".into()),
            data: EventData::Stream {
                name: "stdout".into(),
                text: "frame1\rframe2".into(),
            },
        })
        .unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(
            std::str::from_utf8(&*bytes).unwrap(),
            "[s] frame1\r[s] frame2"
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

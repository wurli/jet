//! Rendering kernel output: text streams, errors, and inline graphics.
//!
//! Frames come off the websocket as JSON; [`jet_core::events::parse_event`]
//! turns them into a typed [`Event`]. [`Renderer`] consumes each event:
//! rendering content events to stdout, and forwarding `Idle` parent_ids on
//! `idle_tx` so the REPL knows it's safe to prompt again.

pub mod ansi;
mod kitty;
mod tmux;

pub use kitty::emit_png;
pub use tmux::warn_if_passthrough_off;

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use base64::Engine;
use jet_core::events::{Event, EventData, InputRequest, IsCompleteReplyMsg};
use serde_json::Value;
use tokio::sync::{Notify, mpsc};

pub type SharedWriter = Arc<Mutex<dyn Write + Send>>;

/// Shared kernel-busy state visible to the REPL. The renderer flips
/// `busy` to true on a Busy status from a *different* session and back
/// to false on the matching Idle, notifying waiters on every transition
/// to false. The REPL parks on `notify` before drawing a new prompt so
/// it doesn't redraw while another client is mid-execute.
#[derive(Default, Clone)]
pub struct BusyState {
    pub busy: Arc<AtomicBool>,
    pub notify: Arc<Notify>,
    /// session name currently holding the kernel busy, for display.
    pub holder: Arc<Mutex<Option<String>>>,
}

#[derive(Clone)]
pub struct Renderer {
    pub render_graphics: bool,
    pub idle_tx: mpsc::UnboundedSender<String>,
    pub input_tx: Option<mpsc::UnboundedSender<InputRequest>>,
    pub is_complete_tx: Option<mpsc::UnboundedSender<IsCompleteReplyMsg>>,
    pub busy_state: BusyState,
    writer: SharedWriter,
    // The session name passed via --session-name (None or "jet" if not
    // set). Output that originated from this same session is shown
    // un-prefixed; output from any *other* session sharing the kernel
    // is tagged so the user can tell who's typing.
    own_session_name: Option<String>,
    // Our own client_id (the full `<name>---repl---<rand>` string).
    // Foreign-vs-own is determined by comparing the event's full
    // parent_session against this, not just the name portion — two
    // clients sharing the kernel without `--session-name` both produce
    // `jet---repl---<rand>` so a name-only compare wrongly merges them.
    own_client_id: Option<String>,
    // True when the next byte we write will start a fresh line, so a
    // session prefix should be emitted before it. Tracked across writes
    // because kernel streams arrive in arbitrary chunks — a partial line
    // followed by more text must NOT get a second prefix.
    at_line_start: Arc<Mutex<bool>>,
    // Reedline holds the tty in raw mode during `read_line`, so writing
    // foreign-session output via `writer` (a plain stdout) leaves the
    // cursor in the prompt's column on each `\n` (no ONLCR) — producing
    // a staircase. Routing those writes through reedline's external
    // printer instead lets reedline clear the prompt line, print at
    // column 0, and redraw the prompt below.
    //
    // The printer is line-oriented: a chunk that arrives without a
    // trailing newline gets buffered in `foreign_line_buf` and emitted
    // when a complete line finally arrives.
    external_printer: Option<reedline::ExternalPrinter<String>>,
    foreign_line_buf: Arc<Mutex<String>>,
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
            is_complete_tx: None,
            busy_state: BusyState::default(),
            writer,
            own_session_name: None,
            own_client_id: None,
            at_line_start: Arc::new(Mutex::new(true)),
            external_printer: None,
            foreign_line_buf: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn with_own_session_name(mut self, name: Option<String>) -> Self {
        self.own_session_name = name;
        self
    }

    pub fn with_own_client_id(mut self, id: String) -> Self {
        self.own_client_id = Some(id);
        self
    }

    pub fn with_external_printer(mut self, printer: reedline::ExternalPrinter<String>) -> Self {
        self.external_printer = Some(printer);
        self
    }

    pub fn with_input_tx(mut self, tx: mpsc::UnboundedSender<InputRequest>) -> Self {
        self.input_tx = Some(tx);
        self
    }

    pub fn with_is_complete_tx(mut self, tx: mpsc::UnboundedSender<IsCompleteReplyMsg>) -> Self {
        self.is_complete_tx = Some(tx);
        self
    }

    pub fn handle_event(&self, event: Event) -> Result<()> {
        let parent = event.parent_session.as_deref();
        let session_name = parent.and_then(|id| id.split("---").next());

        // Identity is the full client_id (`<name>---repl---<rand>`), not
        // just the name — two clients sharing the kernel without
        // `--session-name` both report `jet` as the name. When we know
        // our own client_id, compare against that. Without one (tests,
        // or callers that didn't set it), fall back to name-only.
        // Messages with no parent_session at all (banners, replies to
        // our own kernel_info_request) are always treated as own.
        let is_own_session = match (parent, self.own_client_id.as_deref()) {
            (None, _) => true,
            (Some(p), Some(own_id)) => p == own_id,
            (Some(_), None) => {
                let own_name = self.own_session_name.as_deref().unwrap_or("jet");
                session_name == Some(own_name)
            }
        };
        // Display tag: show `[name]` on every foreign line so the user
        // can tell another client is typing — even when the foreign
        // client used the default name (`jet`). Own-session output is
        // un-prefixed (reedline's own prompt makes ownership obvious).
        let prefix = if is_own_session {
            None
        } else {
            Some(session_name.unwrap_or("jet").to_string())
        };

        log::info!(
            "Handling event: {:?} (own_session={is_own_session})",
            event.data
        );

        let is_foreign = !is_own_session;
        match event.data {
            EventData::ExecuteInput { code } => {
                // Skip the kernel's iopub echo of *our own* input —
                // reedline already drew the prompt locally. For any
                // other session's input we render `[name]> code` so the
                // user can follow what other clients are doing.
                if is_foreign {
                    self.write_line(&format!("> {code}"), prefix.as_deref(), is_foreign)?;
                }
            }
            EventData::DisplayData { data } => {
                self.render_data(&data, prefix.as_deref(), is_foreign)?
            }
            EventData::Stream { name: _, text } => {
                self.write(&text, prefix.as_deref(), is_foreign)?;
            }
            EventData::Error { traceback } => {
                self.write(&traceback, prefix.as_deref(), is_foreign)?;
                self.ensure_newline()?;
            }
            EventData::Banner { text } => self.write_line(&text, None, false)?,
            EventData::Idle { parent_id } => {
                if !is_own_session {
                    // Flush any trailing partial line that never got a
                    // newline so it shows up before the prompt is redrawn.
                    self.flush_foreign_partial()?;
                    // Without an external printer wired (e.g. tests), we
                    // still emit a fresh `> ` directly so the next prompt
                    // doesn't collide with the foreign output. With a
                    // printer, reedline redraws the prompt itself.
                    if self.external_printer.is_none() {
                        self.ensure_newline()?;
                        let mut w = self.writer.lock().unwrap();
                        write!(w, "> ")?;
                        w.flush()?;
                    }
                    // Release the REPL prompt-gate that was set when this
                    // session went Busy.
                    self.busy_state.busy.store(false, Ordering::SeqCst);
                    *self.busy_state.holder.lock().unwrap() = None;
                    self.busy_state.notify.notify_waiters();
                }
                let _ = self.idle_tx.send(parent_id.unwrap_or_default());
            }
            EventData::Busy { .. } => {
                // Another session has started executing. Set the
                // prompt-gate so this REPL parks instead of drawing a new
                // prompt over the in-flight output.
                if !is_own_session {
                    self.busy_state.busy.store(true, Ordering::SeqCst);
                    *self.busy_state.holder.lock().unwrap() = session_name.map(|s| s.to_string());
                }
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
            EventData::IsComplete {
                parent_id,
                status,
                indent,
            } => {
                if let Some(tx) = &self.is_complete_tx {
                    let _ = tx.send(IsCompleteReplyMsg {
                        parent_id: parent_id.unwrap_or_default(),
                        status,
                        indent,
                    });
                }
            }
            EventData::KernelExited | EventData::Other => {}
        }
        Ok(())
    }

    /// Write a chunk to the terminal, optionally inserting `[prefix] ` at
    /// the start of every new line. Honours streaming boundaries: if the
    /// previous write ended without a newline, the next call continues
    /// the same line (no extra prefix); a chunk that ends with `\n`
    /// leaves the next call expecting to start a new line.
    fn write(&self, text: &str, prefix: Option<&str>, is_foreign: bool) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        // Foreign-session output goes through reedline's external printer
        // (when wired) so it doesn't fight the active prompt's raw mode.
        // The display `prefix` is independent — an unnamed foreign client
        // still routes through the printer, just without a `[...]` tag.
        if is_foreign && self.external_printer.is_some() {
            return self.write_foreign(text, prefix);
        }
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        match prefix {
            None => {
                write!(w, "{text}")?;
            }
            Some(p) => {
                // Dim the `[session]` tag so it stays visually
                // subordinate to kernel output.
                let tag = format!("{} ", ansi::dim(&format!("[{p}]")));
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

    /// Write a complete line (appends a newline if missing), optionally
    /// prefixed. Resets the streaming state to "at line start" so
    /// subsequent stream output is re-prefixed.
    fn write_line(&self, text: &str, prefix: Option<&str>, is_foreign: bool) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let needs_newline = !text.ends_with('\n');
        self.write(text, prefix, is_foreign)?;
        if needs_newline {
            if is_foreign && self.external_printer.is_some() {
                self.flush_foreign_partial()?;
            } else {
                let mut w = self.writer.lock().unwrap();
                let mut at_start = self.at_line_start.lock().unwrap();
                writeln!(w)?;
                *at_start = true;
                w.flush()?;
            }
        }
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

    /// Buffer foreign-session text by line and ship complete lines to
    /// reedline's external printer. A partial trailing line stays in the
    /// buffer until the next chunk (or the session's idle event) flushes
    /// it. `\r` is treated as a line terminator too so spinners redraw
    /// cleanly as separate lines under the printer.
    fn write_foreign(&self, text: &str, prefix_name: Option<&str>) -> Result<()> {
        let printer = self
            .external_printer
            .as_ref()
            .expect("write_foreign called without printer");
        let tag = prefix_name.map(|p| format!("{} ", ansi::dim(&format!("[{p}]"))));
        let mut buf = self.foreign_line_buf.lock().unwrap();
        for segment in text.split_inclusive(['\n', '\r']) {
            if buf.is_empty()
                && let Some(t) = &tag
            {
                buf.push_str(t);
            }
            let terminates = segment.ends_with(['\n', '\r']);
            if terminates {
                // Strip the trailing newline — the printer adds its own.
                buf.push_str(segment.trim_end_matches(['\n', '\r']));
                let line = std::mem::take(&mut *buf);
                let _ = printer.sender().send(line);
            } else {
                buf.push_str(segment);
            }
        }
        Ok(())
    }

    /// Flush any partially-buffered foreign line. Called when an Idle
    /// event arrives so the trailing text shows up before the prompt is
    /// redrawn.
    fn flush_foreign_partial(&self) -> Result<()> {
        let Some(printer) = self.external_printer.as_ref() else {
            return Ok(());
        };
        let mut buf = self.foreign_line_buf.lock().unwrap();
        if !buf.is_empty() {
            let line = std::mem::take(&mut *buf);
            let _ = printer.sender().send(line);
        }
        Ok(())
    }

    fn render_data(&self, data: &Value, prefix: Option<&str>, is_foreign: bool) -> Result<()> {
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
                        eprintln!("{}", ansi::yellow(&format!("Image render failed: {e}")));
                        return Ok(());
                    }
                }
            } else {
                let len = base64::engine::general_purpose::STANDARD
                    .decode(image_data)
                    .map(|b| b.len())
                    .unwrap_or(0);
                self.write_line(&format!("[image/png {len} bytes]"), None, is_foreign)?;
                return Ok(());
            }
        };

        if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
            // execute_result/display_data carry a complete value, not a
            // streaming chunk: ark's `Sys.getenv("X")` sends `[1] "123"`
            // with no trailing newline, and ipykernel does the same for
            // bare expressions. Use write_line so the next prompt lands
            // on a fresh line instead of clobbering the value via the
            // bracketed-paste sequence rustyline emits.
            self.write_line(t, prefix, is_foreign)?;
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
            std::str::from_utf8(&bytes).unwrap(),
            &format!(
                "{} Error\n{} Something went wrong",
                ansi::dim("[my-session]"),
                ansi::dim("[my-session]"),
            )
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
            std::str::from_utf8(&bytes).unwrap(),
            &format!("{} hello world\n{} bye", ansi::dim("[s]"), ansi::dim("[s]"))
        );
    }

    #[test]
    fn own_session_output_is_not_prefixed_other_sessions_are() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer).with_own_session_name(Some("alice".into()));

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
            std::str::from_utf8(&bytes).unwrap(),
            &format!("mine\n{} theirs\n", ansi::dim("[bob]"))
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
            std::str::from_utf8(&bytes).unwrap(),
            &format!("{} frame1\r{} frame2", ansi::dim("[s]"), ansi::dim("[s]"))
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
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "a\nb");
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

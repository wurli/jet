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
    /// reedline's `with_break_signal` flag. Flipped to `true` when a
    /// foreign session goes Busy so reedline's in-flight `read_line`
    /// returns `Signal::ExternalBreak(buffer)` instead of fighting the
    /// foreign output's writes with its own prompt repaint.
    pub break_signal: Arc<AtomicBool>,
    /// True while a reedline `read_line` is in flight on the blocking
    /// thread. The renderer uses this to decide whether to route foreign
    /// output through reedline's external printer (when reedline is the
    /// one drawing the screen) or directly to stdout (when reedline is
    /// suspended via `break_signal`, so plain stdout writes don't fight
    /// raw mode).
    pub read_line_active: Arc<AtomicBool>,
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

        let is_foreign = !is_own_session;
        match event.data {
            // --- content events ---
            EventData::ExecuteInput { code } => {
                if is_foreign {
                    self.render_foreign_execute_input(&code, prefix.as_deref())?;
                }
                // Own-session ExecuteInput is intentionally skipped:
                // reedline already drew the input on the prompt line.
            }
            EventData::Stream { name: _, text } => {
                if is_foreign {
                    self.render_foreign_chunk(&text, prefix.as_deref())?;
                } else {
                    self.write(&text, prefix.as_deref(), is_foreign)?;
                }
            }
            EventData::Error { traceback } => {
                if is_foreign {
                    self.render_foreign_chunk(&traceback, prefix.as_deref())?;
                    self.foreign_ensure_newline()?;
                } else {
                    self.write(&traceback, prefix.as_deref(), is_foreign)?;
                    self.ensure_newline()?;
                }
            }
            EventData::DisplayData { data } => {
                if is_foreign {
                    self.render_foreign_data(&data, prefix.as_deref())?;
                } else {
                    self.render_data(&data, prefix.as_deref(), is_foreign)?;
                }
            }
            EventData::Banner { text } => self.write_line(&text, None, false)?,

            // --- lifecycle events ---
            EventData::Busy { .. } => {
                // Foreign Busy parks our prompt and trips reedline's
                // break_signal so any in-flight `read_line` returns
                // `ExternalBreak(buffer)` and yields the terminal.
                if !is_own_session {
                    self.busy_state.busy.store(true, Ordering::SeqCst);
                    *self.busy_state.holder.lock().unwrap() =
                        session_name.map(|s| s.to_string());
                    self.busy_state.break_signal.store(true, Ordering::SeqCst);
                }
            }
            EventData::Idle { parent_id } => {
                if !is_own_session {
                    // If the last foreign write left the cursor mid-line
                    // (e.g. a trailing partial Stream chunk with no \n),
                    // emit one now so reedline's resume `read_line` anchors
                    // its prompt on a fresh row below the foreign content
                    // rather than overwriting it.
                    self.foreign_ensure_newline()?;
                    // Release the prompt-gate.
                    self.busy_state.busy.store(false, Ordering::SeqCst);
                    *self.busy_state.holder.lock().unwrap() = None;
                    self.busy_state.notify.notify_waiters();
                }
                let _ = self.idle_tx.send(parent_id.unwrap_or_default());
            }

            // --- side-channel events: forward to REPL via mpsc ---
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

    /// Format the session tag (e.g. `[jet] ` in dim grey) for foreign output.
    fn foreign_tag(prefix: Option<&str>) -> String {
        prefix
            .map(|p| format!("{} ", ansi::dim(&format!("[{p}]"))))
            .unwrap_or_default()
    }

    /// Foreign `ExecuteInput`: write `[name] > code` between two `\r\n`s so
    /// the line sits on its own row outside reedline's
    /// `previous_prompt_rows_range`. Without those bookends, reedline's
    /// resume from `ExternalBreak` reuses the prior prompt position and
    /// clears down over our foreign content.
    fn render_foreign_execute_input(&self, code: &str, prefix: Option<&str>) -> Result<()> {
        let tag = Self::foreign_tag(prefix);
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        // Multi-line cells: the first line takes a `> ` indicator; each
        // continuation line takes `+ `, matching the prompt style. Each
        // wrapped line gets its own session tag.
        // Erase the current line first: reedline's ExternalBreak leaves
        // its own `> ` prompt on screen, and we want the foreign line to
        // overwrite it rather than sit below an empty prompt row.
        write!(w, "\r\x1b[2K")?;
        for (i, line) in code.split('\n').enumerate() {
            let indicator = if i == 0 { "> " } else { "+ " };
            write!(w, "{tag}{indicator}{line}\r\n")?;
        }
        *at_start = true;
        w.flush()?;
        Ok(())
    }

    /// Foreign streaming text (Stream, Error, plain-text DisplayData).
    /// Prefixes every fresh line with the session tag, translates `\n` to
    /// `\r\n` so we don't staircase under reedline's raw mode, and tracks
    /// `at_line_start` so partial chunks don't get a mid-line prefix.
    fn render_foreign_chunk(&self, body: &str, prefix: Option<&str>) -> Result<()> {
        if body.is_empty() {
            return Ok(());
        }
        let tag = Self::foreign_tag(prefix);
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        // Both '\n' and '\r' end a "visual line" — '\r' is used by spinners
        // to redraw a line in place, and we want each redraw to start with
        // a fresh prefix.
        for segment in body.split_inclusive(['\n', '\r']) {
            if *at_start {
                write!(w, "{tag}")?;
            }
            if let Some(stripped) = segment.strip_suffix('\n') {
                write!(w, "{stripped}\r\n")?;
            } else {
                write!(w, "{segment}")?;
            }
            *at_start = segment.ends_with(['\n', '\r']);
        }
        w.flush()?;
        Ok(())
    }

    /// Foreign `DisplayData`. Inline kitty PNGs when graphics are enabled;
    /// otherwise tag a `[image/png N bytes]` placeholder. Falls back to
    /// `text/plain` like the own-session path.
    fn render_foreign_data(&self, data: &Value, prefix: Option<&str>) -> Result<()> {
        if !data.is_object() {
            return Ok(());
        }
        if let Some(image_data) = data.get("image/png").and_then(|s| s.as_str()) {
            if self.render_graphics {
                let mut w = self.writer.lock().unwrap();
                let mut at_start = self.at_line_start.lock().unwrap();
                match emit_png(&mut *w, image_data) {
                    Ok(()) => *at_start = true,
                    Err(e) => {
                        log::warn!("kitty render failed: {e}");
                        let tag = Self::foreign_tag(prefix);
                        write!(w, "{tag}Image render failed: {e}\r\n")?;
                        *at_start = true;
                    }
                }
                w.flush()?;
            } else {
                let len = base64::engine::general_purpose::STANDARD
                    .decode(image_data)
                    .map(|b| b.len())
                    .unwrap_or(0);
                let placeholder = format!("[image/png {len} bytes]");
                self.render_foreign_chunk(&placeholder, prefix)?;
                self.foreign_ensure_newline()?;
            }
            return Ok(());
        }
        if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
            self.render_foreign_chunk(t, prefix)?;
            self.foreign_ensure_newline()?;
        }
        Ok(())
    }

    /// Like `ensure_newline` but uses `\r\n` (safe in raw mode too). For
    /// the foreign path, where reedline may still be in raw mode when we
    /// write.
    fn foreign_ensure_newline(&self) -> Result<()> {
        let mut w = self.writer.lock().unwrap();
        let mut at_start = self.at_line_start.lock().unwrap();
        if !*at_start {
            write!(w, "\r\n")?;
            *at_start = true;
            w.flush()?;
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
        // ONLY while reedline is actively running `read_line` (and thus
        // owns the screen + raw mode). When reedline has been suspended via
        // the break signal, we write directly to stdout below — no raw
        // mode is held, so `\n` works normally and we avoid reedline's
        // buggy `print_external_message` accounting.
        if is_foreign
            && self.external_printer.is_some()
            && self.busy_state.read_line_active.load(Ordering::SeqCst)
        {
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
            if is_foreign
                && self.external_printer.is_some()
                && self.busy_state.read_line_active.load(Ordering::SeqCst)
            {
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
                "{} Error\r\n{} Something went wrong",
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
            &format!("{} hello world\r\n{} bye", ansi::dim("[s]"), ansi::dim("[s]"))
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
            &format!("mine\n{} theirs\r\n", ansi::dim("[bob]"))
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

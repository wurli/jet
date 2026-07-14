//! Rendering kernel output: text streams, errors, and inline graphics.
//!
//! Frames come off the websocket as JSON; [`jet_core::events::parse_event`]
//! turns them into a typed [`Event`]. [`Renderer`] consumes each event:
//! rendering content events to stdout, and forwarding `Idle` parent_ids on
//! `idle_tx` so the REPL knows it's safe to prompt again.

pub mod ansi;
mod kitty;
mod style;
mod tmux;

pub use kitty::emit_png;
use style::{OwnStyle, PromptStyle, SessionStyle, WrappedStyle};
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

/// How to render output from *other* clients sharing this kernel.
///
/// - `Wrap`: draw a `┌─name` header + `│ ` gutter around every foreign
///   line.
/// - `Prompt`: skip the header and gutter; foreign `execute_input`
///   renders as `name> code` (name colored) and foreign output prints
///   raw with no prefix.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ExternalClientStyle {
    #[default]
    Wrap,
    Prompt,
}

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
    /// Forwards each shell `execute_reply`'s parent_id to the REPL. Per the Jupyter
    /// spec, "execute done" means both this reply *and* the iopub `status: idle`
    /// with the same parent — they arrive on different sockets and may reorder.
    pub execute_reply_tx: Option<mpsc::UnboundedSender<String>>,
    pub busy_state: BusyState,
    writer: SharedWriter,
    // The session name passed via --session-name, or None when not set.
    // Output that originated from this same session is shown un-prefixed;
    // output from any *other named* session sharing the kernel is tagged
    // so the user can tell who's typing. Unnamed clients (None / "") never
    // show a prefix — there's nothing meaningful to display.
    own_session_name: Option<String>,
    // Our own client_id (the full `<name>---repl---<rand>` string).
    // Foreign-vs-own is determined by comparing the event's full
    // parent_session against this, not just the name portion — two
    // clients sharing the kernel without `--session-name` both produce
    // `---repl---<rand>` so a name-only compare wrongly merges them.
    own_client_id: Option<String>,
    // True when the next byte we write will start a fresh line, so a
    // session prefix should be emitted before it. Tracked across writes
    // because kernel streams arrive in arbitrary chunks — a partial line
    // followed by more text must NOT get a second prefix.
    at_line_start: Arc<Mutex<bool>>,
    // `block_key()` of the style whose "block" we're currently drawing
    // (typically the foreign session's name). Set when we emit the top
    // rule; cleared when an own-session ExecuteInput arrives (observer
    // took the terminal back). Back-to-back blocks with the same key
    // skip redrawing the header — the gutter alone carries attribution.
    active_block_key: Arc<Mutex<Option<String>>>,
    external_client_style: ExternalClientStyle,
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
            execute_reply_tx: None,
            busy_state: BusyState::default(),
            writer,
            own_session_name: None,
            own_client_id: None,
            at_line_start: Arc::new(Mutex::new(true)),
            active_block_key: Arc::new(Mutex::new(None)),
            external_client_style: ExternalClientStyle::Wrap,
        }
    }

    pub fn with_external_client_style(mut self, style: ExternalClientStyle) -> Self {
        self.external_client_style = style;
        self
    }

    pub fn with_own_session_name(mut self, name: Option<String>) -> Self {
        self.own_session_name = name;
        self
    }

    pub fn with_own_client_id(mut self, id: String) -> Self {
        self.own_client_id = Some(id);
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

    pub fn with_execute_reply_tx(mut self, tx: mpsc::UnboundedSender<String>) -> Self {
        self.execute_reply_tx = Some(tx);
        self
    }

    pub fn handle_event(&self, event: Event) -> Result<()> {
        let parent = event.parent_session.as_deref();
        let session_name = parent.and_then(|id| id.split("---").next());

        // Identity is the full client_id (`<name>---repl---<rand>`), not just the name — two
        // clients sharing the kernel without `--session-name` both produce `---repl---<rand>` (empty
        // name prefix). When we know our own client_id, compare against that. Without one (tests, or
        // callers that didn't set it), fall back to name-only matching against own_session_name (""
        // when unset). Messages with no parent_session at all (banners, replies to our own
        // kernel_info_request) are always treated as own.
        let is_own_session = match (parent, self.own_client_id.as_deref()) {
            (None, _) => true,
            (Some(p), Some(own_id)) => p == own_id,
            (Some(_), None) => {
                let own_name = self.own_session_name.as_deref().unwrap_or("");
                session_name == Some(own_name)
            }
        };
        // Display tag: show `[name]` on every foreign line so the user can tell another client is
        // typing. Unnamed foreign clients (empty session name) produce no prefix — nothing to show.
        // Own-session output is un-prefixed (reedline's own prompt makes ownership obvious).
        let prefix = if is_own_session {
            None
        } else {
            let name = session_name.unwrap_or("").to_string();
            if name.is_empty() { None } else { Some(name) }
        };

        let style = self.select_style(is_own_session, prefix.as_deref());
        match event.data {
            // --- content events ---
            EventData::ExecuteInput { code } => {
                if is_own_session {
                    // Observer just submitted a cell — any prior foreign
                    // visual block is now closed; the next foreign event
                    // should redraw its own header. Also, reedline already
                    // drew the code on the prompt line, so we emit nothing.
                    *self.active_block_key.lock().unwrap() = None;
                } else {
                    // Erase the current line first: reedline's ExternalBreak
                    // leaves its own `> ` prompt on screen, and we want the
                    // foreign block to overwrite it rather than sit below an
                    // empty prompt row.
                    {
                        let mut w = self.writer.lock().unwrap();
                        write!(w, "\r\x1b[2K")?;
                        w.flush()?;
                    }
                    self.ensure_block_header(&*style)?;
                    let bytes = style.execute_input(&code);
                    self.write_styled(&*style, &bytes)?;
                }
            }
            EventData::Stream { name: _, text } => {
                self.ensure_block_header(&*style)?;
                self.emit_stream(&*style, &text)?;
            }
            EventData::Error { traceback } => {
                self.ensure_block_header(&*style)?;
                self.emit_stream(&*style, &traceback)?;
                self.ensure_newline(&*style)?;
            }
            EventData::DisplayData { data } => {
                self.render_display_data(&*style, &data)?;
            }
            EventData::Banner { text } => {
                // Banner always renders as own-style regardless of who
                // sent it (kernel_info_reply after our own request).
                let own = OwnStyle;
                self.emit_stream(&own, &text)?;
                if !text.ends_with('\n') {
                    self.ensure_newline(&own)?;
                }
            }

            // --- lifecycle events ---
            EventData::Busy { .. } => {
                // Foreign Busy parks our prompt and trips reedline's
                // break_signal so any in-flight `read_line` returns
                // `ExternalBreak(buffer)` and yields the terminal.
                if !is_own_session {
                    self.busy_state.busy.store(true, Ordering::SeqCst);
                    *self.busy_state.holder.lock().unwrap() = session_name.map(|s| s.to_string());
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
                    self.ensure_newline(&*style)?;
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
            EventData::ExecuteReply { parent_id } => {
                if let Some(tx) = &self.execute_reply_tx {
                    let _ = tx.send(parent_id.unwrap_or_default());
                }
            }
            EventData::KernelExited | EventData::Other => {}
        }
        Ok(())
    }

    /// Pick the [`SessionStyle`] for an incoming event based on whether
    /// it came from us or another client, and — for foreign events —
    /// the user's `--external-client-style` choice.
    fn select_style(&self, is_own: bool, name: Option<&str>) -> Box<dyn SessionStyle> {
        if is_own {
            return Box::new(OwnStyle);
        }
        let name = name.map(str::to_string);
        match self.external_client_style {
            ExternalClientStyle::Wrap => Box::new(WrappedStyle::new(name)),
            ExternalClientStyle::Prompt => Box::new(PromptStyle::new(name)),
        }
    }

    /// If the style has a `block_key` that differs from the currently
    /// active one, close the old block (implicit) and emit the new
    /// style's header. Consecutive events sharing a key skip re-emission
    /// so gutters stay contiguous.
    fn ensure_block_header(&self, style: &dyn SessionStyle) -> Result<()> {
        let Some(key) = style.block_key() else {
            return Ok(());
        };
        let mut active = self.active_block_key.lock().unwrap();
        if active.as_deref() == Some(key) {
            return Ok(());
        }
        *active = Some(key.to_string());
        drop(active);
        let header = style.header();
        if header.is_empty() {
            return Ok(());
        }
        // Header is a full line — same write path as any other bytes.
        self.write_styled(style, &header)?;
        Ok(())
    }

    /// Ask the style to render a streaming chunk, write the result,
    /// and update `at_line_start`. Callers of stream/error/text-plain
    /// paths use this so they don't have to manage the bit themselves.
    fn emit_stream(&self, style: &dyn SessionStyle, body: &str) -> Result<()> {
        let at_start = *self.at_line_start.lock().unwrap();
        let bytes = style.stream_chunk(body, at_start);
        self.write_styled(style, &bytes)
    }

    /// Write styled bytes (already carrying any headers, gutters, and
    /// `\n` line breaks) to the shared writer. Updates `at_line_start`
    /// based on the emitted bytes' last char.
    ///
    /// Foreign styles get `\n`→`\r\n` translation, since reedline holds
    /// the tty in raw mode during `read_line` and a bare `\n` would
    /// leave the cursor in the prompt's column (staircase).
    fn write_styled(&self, style: &dyn SessionStyle, bytes: &str) -> Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let prev_at_line_start = *self.at_line_start.lock().unwrap();
        let new_at_line_start = bytes.ends_with(['\n', '\r']);
        let mut w = self.writer.lock().unwrap();
        if style.needs_crlf() {
            for segment in bytes.split_inclusive('\n') {
                if let Some(stripped) = segment.strip_suffix('\n') {
                    write!(w, "{stripped}\r\n")?;
                } else {
                    write!(w, "{segment}")?;
                }
            }
        } else {
            write!(w, "{bytes}")?;
        }
        w.flush()?;
        *self.at_line_start.lock().unwrap() = new_at_line_start;
        log::debug!(
            "renderer -> tty: write_styled bytes={bytes:?} needs_crlf={} at_line_start {prev_at_line_start}->{new_at_line_start}",
            style.needs_crlf(),
        );
        Ok(())
    }

    /// If the previous write left the cursor mid-line, emit a newline.
    fn ensure_newline(&self, style: &dyn SessionStyle) -> Result<()> {
        if *self.at_line_start.lock().unwrap() {
            log::debug!("renderer -> tty: ensure_newline noop (already at line start)");
            return Ok(());
        }
        let mut w = self.writer.lock().unwrap();
        let wrote = if style.needs_crlf() {
            write!(w, "\r\n")?;
            "\r\n"
        } else {
            writeln!(w)?;
            "\n"
        };
        w.flush()?;
        *self.at_line_start.lock().unwrap() = true;
        log::debug!(
            "renderer -> tty: ensure_newline wrote={wrote:?} needs_crlf={}",
            style.needs_crlf(),
        );
        Ok(())
    }

    /// Render `DisplayData`: inline kitty PNGs when graphics are on,
    /// otherwise a `[image/png N bytes]` placeholder. Falls back to
    /// `text/plain`.
    fn render_display_data(&self, style: &dyn SessionStyle, data: &Value) -> Result<()> {
        if !data.is_object() {
            return Ok(());
        }
        if let Some(image_data) = data.get("image/png").and_then(|s| s.as_str()) {
            self.ensure_block_header(style)?;
            if self.render_graphics {
                // PNGs bypass the styler — kitty escape sequences are
                // control bytes, not text to be gutter-prefixed. On failure
                // we render the error message through the style so it
                // still gets attributed.
                let result = {
                    let mut w = self.writer.lock().unwrap();
                    let r = emit_png(&mut *w, image_data);
                    w.flush()?;
                    r
                };
                match result {
                    Ok(()) => *self.at_line_start.lock().unwrap() = true,
                    Err(e) => {
                        log::warn!("kitty render failed: {e}");
                        self.emit_stream(style, &format!("Image render failed: {e}\n"))?;
                    }
                }
                return Ok(());
            }
            let len = base64::engine::general_purpose::STANDARD
                .decode(image_data)
                .map(|b| b.len())
                .unwrap_or(0);
            self.emit_stream(style, &format!("[image/png {len} bytes]"))?;
            self.ensure_newline(style)?;
            return Ok(());
        }
        if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
            // execute_result/display_data carry a complete value, not a
            // streaming chunk: ark's `Sys.getenv("X")` sends `[1] "123"`
            // with no trailing newline, and ipykernel does the same for
            // bare expressions. Force a trailing newline so the next
            // prompt doesn't clobber the value.
            self.ensure_block_header(style)?;
            self.emit_stream(style, t)?;
            self.ensure_newline(style)?;
        }
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

    /// Stable expectation for the block header: session-color + "┌ name "
    /// + dashes filling the terminal width. Tests only check that the
    fn header_prefix(name: &str) -> String {
        format!("{}┌─{name}{}\r\n", ansi::session_color(name), ansi::RESET)
    }
    fn gutter(name: &str) -> String {
        format!("{}│{} ", ansi::session_color(name), ansi::RESET)
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
        let s = std::str::from_utf8(&bytes).unwrap();
        let g = gutter("my-session");
        assert!(s.starts_with(&header_prefix("my-session")), "got: {s:?}");
        assert!(
            s.contains(&format!("\r\n{g}Error\r\n{g}Something went wrong")),
            "got: {s:?}"
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
        let s = std::str::from_utf8(&bytes).unwrap();
        let g = gutter("s");
        assert!(s.starts_with(&header_prefix("s")), "got: {s:?}");
        assert!(
            s.contains(&format!("{g}hello world\r\n{g}bye")),
            "got: {s:?}"
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
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(
            s.starts_with("mine\n"),
            "own output should be unprefixed: {s:?}"
        );
        assert!(s.contains(&header_prefix("bob")), "got: {s:?}");
        assert!(
            s.contains(&format!("{}theirs\r\n", gutter("bob"))),
            "got: {s:?}"
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
        let s = std::str::from_utf8(&bytes).unwrap();
        let g = gutter("s");
        assert!(s.contains(&format!("{g}frame1\r{g}frame2")), "got: {s:?}");
    }

    #[test]
    fn repl_session_is_not_prefixed() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        // No session name set → own_session_name fallback is "". A client_id
        // built with no name produces "---repl---<rand>"; its leading segment
        // (before "---") is "" which matches the fallback.
        let r = Renderer::new(false, tx, writer);
        r.handle_event(Event {
            parent_session: Some("---repl---abc123".into()),
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
    fn prompt_style_execute_input_uses_name_prompt_no_gutter() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer)
            .with_external_client_style(ExternalClientStyle::Prompt);
        r.handle_event(Event {
            parent_session: Some("beta---bg".into()),
            data: EventData::ExecuteInput {
                code: "print(\"y\")".into(),
            },
        })
        .unwrap();
        r.handle_event(Event {
            parent_session: Some("beta---bg".into()),
            data: EventData::Stream {
                name: "stdout".into(),
                text: "y\n".into(),
            },
        })
        .unwrap();
        let bytes = captured.lock().unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        let color = ansi::session_color("beta");
        let reset = ansi::RESET;
        // No `┌─` block header.
        assert!(!s.contains("┌"), "unexpected block header: {s:?}");
        // No `│` gutter.
        assert!(!s.contains("│"), "unexpected gutter: {s:?}");
        // Colored `beta` glued to `> ` before the code.
        assert!(
            s.contains(&format!("{color}beta{reset}> print(\"y\")\r\n")),
            "got: {s:?}"
        );
        // Stream output prints raw (no prefix).
        assert!(s.ends_with("y\r\n"), "got: {s:?}");
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

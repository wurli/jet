//! Rendering kernel output: text streams, errors, and inline graphics.
//!
//! Frames come off the websocket as JSON; [`parse_event`] turns them into
//! a typed [`Event`]. [`Renderer`] consumes each event: rendering content
//! events to stdout, and forwarding `Idle` parent_ids on `idle_tx` so the
//! REPL knows it's safe to prompt again.

mod kitty;
mod tmux;

pub use kitty::emit_png;
pub use tmux::warn_if_passthrough_off;

use std::io::Write;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use base64::Engine;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;

pub type SharedWriter = Arc<Mutex<dyn Write + Send>>;

#[derive(Debug)]
pub enum Event {
    Stream { name: String, text: String },
    DisplayData { data: Value },
    Error { traceback: String },
    Banner { text: String },
    Idle { parent_id: String },
    Other,
}

#[derive(Deserialize)]
struct IncomingMessage {
    #[serde(default)]
    channel: String,
    #[serde(default)]
    header: Header,
    #[serde(default)]
    parent_header: Option<ParentHeader>,
    #[serde(default)]
    content: Value,
}

#[derive(Deserialize, Default)]
struct Header {
    #[serde(default)]
    msg_type: String,
}

#[derive(Deserialize, Default)]
struct ParentHeader {
    #[serde(default)]
    msg_id: String,
}

pub fn parse_event(text: &str) -> Result<Event> {
    let m: IncomingMessage = serde_json::from_str(text)?;
    let event = match (m.channel.as_str(), m.header.msg_type.as_str()) {
        ("iopub", "stream") => {
            let name = m
                .content
                .get("name")
                .and_then(|s| s.as_str())
                .unwrap_or("stdout")
                .to_string();
            let text = m
                .content
                .get("text")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            Event::Stream { name, text }
        }
        ("iopub", "execute_result") | ("iopub", "display_data") => {
            let data = m.content.get("data").cloned().unwrap_or(Value::Null);
            Event::DisplayData { data }
        }
        ("iopub", "error") => {
            let traceback = m
                .content
                .get("traceback")
                .and_then(|t| t.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            Event::Error { traceback }
        }
        ("shell", "kernel_info_reply") => {
            let text = m
                .content
                .get("banner")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            Event::Banner { text }
        }
        ("iopub", "status") => {
            let state = m
                .content
                .get("execution_state")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            let parent_id = m.parent_header.map(|p| p.msg_id).unwrap_or_default();
            if state == "idle" && !parent_id.is_empty() {
                Event::Idle { parent_id }
            } else {
                Event::Other
            }
        }
        _ => Event::Other,
    };
    Ok(event)
}

pub struct Renderer {
    pub render_graphics: bool,
    pub idle_tx: mpsc::UnboundedSender<String>,
    writer: SharedWriter,
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
            writer,
        }
    }

    pub fn handle_text(&self, text: &str) -> Result<()> {
        match parse_event(text)? {
            Event::Stream { name, text } => self.write_stream(&name, &text)?,
            Event::DisplayData { data } => self.render_data(&data)?,
            Event::Error { traceback } => {
                let mut w = self.writer.lock().unwrap();
                writeln!(w, "\x1b[31m{traceback}\x1b[0m")?;
                w.flush()?;
            }
            Event::Banner { text } => self.write_banner(&text)?,
            Event::Idle { parent_id } => {
                let _ = self.idle_tx.send(parent_id);
            }
            Event::Other => {}
        }
        Ok(())
    }

    fn write_stream(&self, name: &str, text: &str) -> Result<()> {
        let mut w = self.writer.lock().unwrap();
        if name == "stderr" {
            write!(w, "\x1b[31m{text}\x1b[0m")?;
        } else {
            write!(w, "{text}")?;
        }
        w.flush()?;
        Ok(())
    }

    fn write_banner(&self, banner: &str) -> Result<()> {
        if banner.is_empty() {
            return Ok(());
        }
        let mut w = self.writer.lock().unwrap();
        if banner.ends_with('\n') {
            write!(w, "{banner}")?;
        } else {
            writeln!(w, "{banner}")?;
        }
        w.flush()?;
        Ok(())
    }

    fn render_data(&self, data: &Value) -> Result<()> {
        if !data.is_object() {
            return Ok(());
        }
        let png = data.get("image/png").and_then(|s| s.as_str());
        let handled = match (self.render_graphics, png) {
            (true, Some(b64)) => {
                let mut w = self.writer.lock().unwrap();
                match emit_png(&mut *w, b64) {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("\x1b[33m[jet] kitty render failed: {e}\x1b[0m");
                        false
                    }
                }
            }
            (false, Some(b64)) => {
                let len = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map(|b| b.len())
                    .unwrap_or(0);
                let mut w = self.writer.lock().unwrap();
                writeln!(w, "[image/png {len} bytes]")?;
                w.flush()?;
                true
            }
            (_, None) => false,
        };
        if handled {
            return Ok(());
        }
        if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
            let mut w = self.writer.lock().unwrap();
            writeln!(w, "{t}")?;
            w.flush()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frame(channel: &str, msg_type: &str, parent_id: &str, content: Value) -> String {
        json!({
            "channel": channel,
            "header": {"msg_type": msg_type},
            "parent_header": {"msg_id": parent_id},
            "content": content,
        })
        .to_string()
    }

    #[test]
    fn parse_stream_event() {
        let f = frame("iopub", "stream", "", json!({"name": "stdout", "text": "hi"}));
        match parse_event(&f).unwrap() {
            Event::Stream { name, text } => {
                assert_eq!(name, "stdout");
                assert_eq!(text, "hi");
            }
            other => panic!("expected Stream, got {other:?}"),
        }
    }

    #[test]
    fn parse_display_data_event() {
        let f = frame(
            "iopub",
            "display_data",
            "",
            json!({"data": {"text/plain": "x"}}),
        );
        match parse_event(&f).unwrap() {
            Event::DisplayData { data } => {
                assert_eq!(data["text/plain"], "x");
            }
            other => panic!("expected DisplayData, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_event() {
        let f = frame(
            "iopub",
            "error",
            "",
            json!({"traceback": ["line1", "line2"]}),
        );
        match parse_event(&f).unwrap() {
            Event::Error { traceback } => assert_eq!(traceback, "line1\nline2"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn parse_banner_event() {
        let f = frame("shell", "kernel_info_reply", "", json!({"banner": "hello"}));
        match parse_event(&f).unwrap() {
            Event::Banner { text } => assert_eq!(text, "hello"),
            other => panic!("expected Banner, got {other:?}"),
        }
    }

    #[test]
    fn parse_idle_event() {
        let f = frame(
            "iopub",
            "status",
            "abc",
            json!({"execution_state": "idle"}),
        );
        match parse_event(&f).unwrap() {
            Event::Idle { parent_id } => assert_eq!(parent_id, "abc"),
            other => panic!("expected Idle, got {other:?}"),
        }
    }

    #[test]
    fn parse_busy_status_is_other() {
        let f = frame(
            "iopub",
            "status",
            "abc",
            json!({"execution_state": "busy"}),
        );
        assert!(matches!(parse_event(&f).unwrap(), Event::Other));
    }

    #[test]
    fn parse_idle_without_parent_is_other() {
        let f = frame("iopub", "status", "", json!({"execution_state": "idle"}));
        assert!(matches!(parse_event(&f).unwrap(), Event::Other));
    }

    #[test]
    fn parse_unknown_msg_type_is_other() {
        let f = frame("iopub", "comm_msg", "", json!({}));
        assert!(matches!(parse_event(&f).unwrap(), Event::Other));
    }

    #[test]
    fn parse_handles_null_parent_header() {
        let raw = serde_json::json!({
            "channel": "iopub",
            "header": {"msg_type": "status"},
            "parent_header": null,
            "content": {"execution_state": "starting"},
        })
        .to_string();
        assert!(matches!(parse_event(&raw).unwrap(), Event::Other));
    }

    #[test]
    fn renderer_writes_stream_to_injected_writer() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        let f = frame("iopub", "stream", "", json!({"name": "stdout", "text": "hello"}));
        r.handle_text(&f).unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(&*bytes, b"hello");
    }

    #[test]
    fn renderer_writes_stderr_with_color() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured.clone();
        let (tx, _rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        let f = frame("iopub", "stream", "", json!({"name": "stderr", "text": "oops"}));
        r.handle_text(&f).unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "\x1b[31moops\x1b[0m");
    }

    #[test]
    fn renderer_signals_idle() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        let f = frame("iopub", "status", "msg-1", json!({"execution_state": "idle"}));
        r.handle_text(&f).unwrap();
        assert_eq!(rx.try_recv().unwrap(), "msg-1");
    }
}

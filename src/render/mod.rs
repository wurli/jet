//! Rendering kernel output: text streams, errors, and inline graphics.
//!
//! [`Renderer`] drives one websocket: it parses each frame, dispatches by
//! `(channel, msg_type)`, and emits text/escape sequences to stdout. When
//! the kernel reports its execution state as `idle`, the parent message id
//! is forwarded on `idle_tx` so the REPL knows it's safe to prompt again.

mod kitty;
mod tmux;

pub use kitty::emit_png;
pub use tmux::warn_if_passthrough_off;

use std::io::Write;

use anyhow::Result;
use base64::Engine;
use serde_json::Value;
use tokio::sync::mpsc;

pub struct Renderer {
    pub render_graphics: bool,
    pub idle_tx: mpsc::UnboundedSender<String>,
}

impl Renderer {
    pub fn new(render_graphics: bool, idle_tx: mpsc::UnboundedSender<String>) -> Self {
        Self {
            render_graphics,
            idle_tx,
        }
    }

    pub fn handle_text(&self, text: &str) -> Result<()> {
        let v: Value = serde_json::from_str(text)?;
        let channel = v.get("channel").and_then(|s| s.as_str()).unwrap_or("");
        let msg_type = v
            .pointer("/header/msg_type")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let parent_id = v
            .pointer("/parent_header/msg_id")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let content = v.get("content").cloned().unwrap_or(Value::Null);

        match (channel, msg_type) {
            ("iopub", "stream") => self.write_stream(&content),
            ("iopub", "execute_result") | ("iopub", "display_data") => {
                self.render_data(&content)
            }
            ("iopub", "error") => self.write_error(&content),
            ("iopub", "status") => {
                let state = content
                    .get("execution_state")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                if state == "idle" && !parent_id.is_empty() {
                    let _ = self.idle_tx.send(parent_id);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn write_stream(&self, content: &Value) {
        let name = content
            .get("name")
            .and_then(|s| s.as_str())
            .unwrap_or("stdout");
        let txt = content.get("text").and_then(|s| s.as_str()).unwrap_or("");
        let mut out = std::io::stdout();
        if name == "stderr" {
            let _ = write!(out, "\x1b[31m{txt}\x1b[0m");
        } else {
            let _ = write!(out, "{txt}");
        }
        let _ = out.flush();
    }

    fn write_error(&self, content: &Value) {
        let traceback = content
            .get("traceback")
            .and_then(|t| t.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        println!("\x1b[31m{traceback}\x1b[0m");
    }

    fn render_data(&self, content: &Value) {
        let Some(data) = content.get("data") else {
            return;
        };
        if !data.is_object() {
            return;
        }
        if self.render_graphics {
            if let Some(b64) = data.get("image/png").and_then(|s| s.as_str()) {
                match emit_png(b64) {
                    Ok(()) => return,
                    Err(e) => eprintln!("\x1b[33m[jet] kitty render failed: {e}\x1b[0m"),
                }
            }
        } else if let Some(b64) = data.get("image/png").and_then(|s| s.as_str()) {
            let len = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map(|b| b.len())
                .unwrap_or(0);
            println!("[image/png {len} bytes]");
            return;
        }
        if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
            println!("{t}");
        }
    }
}

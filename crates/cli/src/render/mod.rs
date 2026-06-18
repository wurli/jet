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
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use base64::Engine;
use jet_core::events::{EventData, InputRequest};
use serde_json::Value;
use tokio::sync::mpsc;

pub type SharedWriter = Arc<Mutex<dyn Write + Send>>;

#[derive(Clone)]
pub struct Renderer {
    pub render_graphics: bool,
    pub idle_tx: mpsc::UnboundedSender<String>,
    pub input_tx: Option<mpsc::UnboundedSender<InputRequest>>,
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
            input_tx: None,
            writer,
        }
    }

    pub fn with_input_tx(mut self, tx: mpsc::UnboundedSender<InputRequest>) -> Self {
        self.input_tx = Some(tx);
        self
    }

    pub fn handle_event(&self, event: EventData) -> Result<()> {
        match event {
            EventData::Stream { name: _, text } => self.write_stream(&text)?,
            EventData::DisplayData { data } => self.render_data(&data)?,
            EventData::Error { traceback } => self.write(traceback.deref())?,
            EventData::Banner { text } => self.write(&text)?,
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

    fn write(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let mut w = self.writer.lock().unwrap();
        if text.ends_with('\n') {
            write!(w, "{text}")?;
        } else {
            writeln!(w, "{text}")?;
        }
        w.flush()?;
        Ok(())
    }

    fn write_stream(&self, text: &str) -> Result<()> {
        let mut w = self.writer.lock().unwrap();
        write!(w, "{text}")?;
        w.flush()?;
        Ok(())
    }

    fn render_data(&self, data: &Value) -> Result<()> {
        if !data.is_object() {
            return Ok(());
        }

        if let Some(image_data) = data.get("image/png").and_then(|s| s.as_str()) {
            if self.render_graphics {
                let mut w = self.writer.lock().unwrap();
                match emit_png(&mut *w, image_data) {
                    Ok(()) => return Ok(()),
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
                let mut w = self.writer.lock().unwrap();
                writeln!(w, "[image/png {len} bytes]")?;
                w.flush()?;
                return Ok(());
            }
        };

        if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
            let mut w = self.writer.lock().unwrap();
            writeln!(w, "{t}")?;
            w.flush()?;
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
        r.handle_event(EventData::Stream {
            name: "stdout".into(),
            text: "hello".into(),
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
        r.handle_event(EventData::Stream {
            name: "stderr".into(),
            text: "oops".into(),
        })
        .unwrap();
        let bytes = captured.lock().unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "oops");
    }

    #[test]
    fn renderer_signals_idle() {
        let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = captured;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let r = Renderer::new(false, tx, writer);
        r.handle_event(EventData::Idle {
            parent_id: Some("msg-1".into()),
        })
        .unwrap();
        assert_eq!(rx.try_recv().unwrap(), "msg-1");
    }
}

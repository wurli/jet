//! Demuxes kernel websocket frames into per-msg_id channels so each Lua
//! poll closure only sees the frames for its own request.

use crossbeam_channel::{Receiver, Sender, unbounded};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// One kernel frame routed to a Lua-side poller.
///
/// `Idle` is the terminal item — once seen, the per-request channel is
/// torn down and the polling closure starts returning `nil`.
pub enum PollItem {
    Frame {
        /// Jupyter `header.msg_type` (`stream`, `execute_result`,
        /// `display_data`, `error`, `input_request`, `comm_msg`, …).
        msg_type: String,
        /// Frame `content` field, handed to Lua as a serde-converted table.
        content: Value,
    },
    Idle,
}

pub struct FrameRouter {
    /// Live per-request senders, keyed by the parent_msg_id we generated
    /// when sending the request.
    by_parent: Mutex<HashMap<String, Sender<PollItem>>>,
}

impl FrameRouter {
    pub fn new() -> Self {
        Self {
            by_parent: Mutex::new(HashMap::new()),
        }
    }

    /// Register a sender for a freshly-allocated parent_msg_id and return
    /// the receiver Lua will poll.
    pub fn register(&self, parent_msg_id: String) -> Receiver<PollItem> {
        let (tx, rx) = unbounded();
        self.by_parent.lock().unwrap().insert(parent_msg_id, tx);
        rx
    }

    /// Drop a registered sender (e.g. on poller close, or after Idle).
    pub fn forget(&self, parent_msg_id: &str) {
        self.by_parent.lock().unwrap().remove(parent_msg_id);
    }

    /// Route one parsed frame.
    ///
    /// `Idle` for a registered parent closes out the corresponding poller;
    /// for an unregistered parent (e.g. our internal kernel_info_request
    /// during connect/attach) it's a no-op. `Content` for an unregistered
    /// parent — kernel-initiated `comm_msg` / `comm_open` — is dropped, as
    /// the Lua surface has no consumer for it yet.
    pub fn dispatch(&self, parent_msg_id: Option<&str>, frame: Frame) {
        match frame {
            Frame::Idle { parent_msg_id } => {
                if let Some(tx) = self
                    .by_parent
                    .lock()
                    .unwrap()
                    .remove(&parent_msg_id.unwrap_or_default())
                {
                    let _ = tx.send(PollItem::Idle);
                }
            }
            Frame::Content { msg_type, content } => {
                if let Some(pid) = parent_msg_id {
                    if let Some(tx) = self.by_parent.lock().unwrap().get(pid) {
                        let _ = tx.send(PollItem::Frame { msg_type, content });
                    }
                }
            }
        }
    }
}

/// Parsed-enough form of a websocket frame to make a routing decision.
pub enum Frame {
    Idle { parent_msg_id: Option<String> },
    Content { msg_type: String, content: Value },
}

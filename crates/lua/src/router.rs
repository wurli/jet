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
    /// Catch-all for kernel-initiated frames whose parent_msg_id we never
    /// registered (e.g. async `comm_msg`s, kernel-driven `comm_open`).
    unsolicited_tx: Sender<PollItem>,
    #[allow(dead_code)]
    unsolicited_rx: Receiver<PollItem>,
}

impl FrameRouter {
    pub fn new() -> Self {
        let (unsolicited_tx, unsolicited_rx) = unbounded();
        Self {
            by_parent: Mutex::new(HashMap::new()),
            unsolicited_tx,
            unsolicited_rx,
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
    /// `idle_for` is `Some(parent_msg_id)` when this frame is an iopub
    /// `status` with `execution_state=idle` — the terminal signal that
    /// closes out a request. Otherwise the frame is forwarded to its
    /// matching per-request sender, falling back to `unsolicited`.
    pub fn dispatch(&self, parent_msg_id: Option<&str>, frame: Frame) {
        match frame {
            Frame::Idle { parent_msg_id } => {
                let mut map = self.by_parent.lock().unwrap();
                if let Some(tx) = map.remove(&parent_msg_id) {
                    let _ = tx.send(PollItem::Idle);
                }
                // Idle for an unregistered parent (e.g. our internal
                // kernel_info_request during start_kernel) is a no-op.
            }
            Frame::Content { msg_type, content } => {
                let item = PollItem::Frame { msg_type, content };
                if let Some(pid) = parent_msg_id {
                    if let Some(tx) = self.by_parent.lock().unwrap().get(pid) {
                        let _ = tx.send(item);
                        return;
                    }
                }
                let _ = self.unsolicited_tx.send(item);
            }
        }
    }
}

/// Parsed-enough form of a websocket frame to make a routing decision.
pub enum Frame {
    Idle { parent_msg_id: String },
    Content { msg_type: String, content: Value },
}

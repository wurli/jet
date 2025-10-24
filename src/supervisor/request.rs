use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use crate::msg::wire::jupyter_message::Message;
use crate::supervisor::iopub_broker::RequestChannels;

/// Helper struct to collect all messages for a single execution request
#[derive(Debug, Default)]
pub struct ExecutionResult {
    pub status_messages: Vec<Message>,
    pub execution_messages: Vec<Message>,
    pub stream_messages: Vec<Message>,
    pub display_messages: Vec<Message>,
    pub comm_messages: Vec<Message>,
}

impl ExecutionResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create channels and receivers for collecting execution results
    pub fn create_channels() -> (RequestChannels, ExecutionCollector) {
        let (status_tx, status_rx) = channel();
        let (execution_tx, execution_rx) = channel();
        let (stream_tx, stream_rx) = channel();
        let (display_tx, display_rx) = channel();
        let (comm_tx, comm_rx) = channel();

        let channels = RequestChannels {
            status_tx,
            execution_tx,
            tx: stream_tx,
            display_tx,
            comm_tx,
        };

        let collector = ExecutionCollector {
            status_rx,
            execution_rx,
            stream_rx,
            display_rx,
            comm_rx,
        };

        (channels, collector)
    }
}

/// Receiver ends of the execution result channels
pub struct ExecutionCollector {
    pub status_rx: Receiver<Message>,
    pub execution_rx: Receiver<Message>,
    pub stream_rx: Receiver<Message>,
    pub display_rx: Receiver<Message>,
    pub comm_rx: Receiver<Message>,
}

impl ExecutionCollector {
    /// Collect all available messages with a timeout
    pub fn collect_all(&self, timeout: Duration) -> ExecutionResult {
        let mut result = ExecutionResult::new();
        let deadline = Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }

            let timeout_per_channel = Duration::from_millis(10);

            // Try to receive from each channel with a short timeout
            if let Ok(msg) = self.status_rx.recv_timeout(timeout_per_channel) {
                result.status_messages.push(msg);
                continue;
            }

            if let Ok(msg) = self.execution_rx.recv_timeout(timeout_per_channel) {
                result.execution_messages.push(msg);
                continue;
            }

            if let Ok(msg) = self.stream_rx.recv_timeout(timeout_per_channel) {
                result.stream_messages.push(msg);
                continue;
            }

            if let Ok(msg) = self.display_rx.recv_timeout(timeout_per_channel) {
                result.display_messages.push(msg);
                continue;
            }

            if let Ok(msg) = self.comm_rx.recv_timeout(timeout_per_channel) {
                result.comm_messages.push(msg);
                continue;
            }

            // No messages available on any channel - check if we should continue
            // If we haven't seen an Idle status yet, keep waiting
            if result.has_completed() {
                break;
            }
        }

        result
    }

    /// Try to receive the next message from any channel with a timeout
    pub fn recv_any_timeout(&self, timeout: Duration) -> Result<Message, RecvTimeoutError> {
        let deadline = Instant::now() + timeout;
        let timeout_per_channel = Duration::from_millis(10);

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(RecvTimeoutError::Timeout);
            }

            if let Ok(msg) = self.status_rx.recv_timeout(timeout_per_channel) {
                return Ok(msg);
            }

            if let Ok(msg) = self.execution_rx.recv_timeout(timeout_per_channel) {
                return Ok(msg);
            }

            if let Ok(msg) = self.stream_rx.recv_timeout(timeout_per_channel) {
                return Ok(msg);
            }

            if let Ok(msg) = self.display_rx.recv_timeout(timeout_per_channel) {
                return Ok(msg);
            }

            if let Ok(msg) = self.comm_rx.recv_timeout(timeout_per_channel) {
                return Ok(msg);
            }
        }
    }
}

impl ExecutionResult {
    /// Check if execution has completed (received Idle status after Busy)
    pub fn has_completed(&self) -> bool {
        use crate::msg::wire::status::ExecutionState;

        let mut saw_busy = false;
        for msg in &self.status_messages {
            if let Message::Status(status) = msg {
                match status.content.execution_state {
                    ExecutionState::Busy => saw_busy = true,
                    ExecutionState::Idle if saw_busy => return true,
                    _ => {}
                }
            }
        }
        false
    }
}

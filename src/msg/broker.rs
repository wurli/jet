/*
 * broker.rs
 *
 * Message broker for routing IOPub messages to appropriate handlers
 * based on parent message correlation.
 *
 */

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{channel, Sender, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::msg::wire::jupyter_message::Message;

/// Uniquely identifies a request-response cycle using the request's msg_id
pub type RequestId = String;

/// Channels for different categories of IOPub messages related to a request
#[derive(Debug)]
pub struct RequestChannels {
    pub status_tx: Sender<Message>,
    pub execution_tx: Sender<Message>,
    pub stream_tx: Sender<Message>,
    pub display_tx: Sender<Message>,
    pub comm_tx: Sender<Message>,
}

/// Context for tracking an active request that expects IOPub messages
struct RequestContext {
    #[allow(dead_code)]
    request_id: RequestId,
    started_at: Instant,
    channels: RequestChannels,
}

/// Configuration for the IOPub broker
#[derive(Debug, Clone)]
pub struct BrokerConfig {
    /// Maximum number of orphan messages to buffer
    pub orphan_buffer_max: usize,
    /// Maximum age of orphan messages before cleanup
    pub orphan_max_age: Duration,
    /// Maximum age of stale requests before cleanup
    pub request_timeout: Duration,
    /// Interval between cleanup operations
    pub cleanup_interval: Duration,
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            orphan_buffer_max: 1000,
            orphan_max_age: Duration::from_secs(60),
            request_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(30),
        }
    }
}

/// Central message broker that routes IOPub messages based on parent headers
pub struct IopubBroker {
    /// Active requests waiting for messages
    active_requests: Arc<RwLock<HashMap<RequestId, RequestContext>>>,

    /// Global subscribers that receive all messages regardless of correlation
    global_subscribers: Arc<RwLock<Vec<Sender<Message>>>>,

    /// Buffer for "orphan" messages (no matching request)
    orphan_buffer: Arc<Mutex<VecDeque<(Message, Instant)>>>,

    /// Configuration
    pub config: BrokerConfig,
}

impl IopubBroker {
    /// Create a new IOPub broker with default configuration
    pub fn new() -> Self {
        Self::with_config(BrokerConfig::default())
    }

    /// Create a new IOPub broker with custom configuration
    pub fn with_config(config: BrokerConfig) -> Self {
        Self {
            active_requests: Arc::new(RwLock::new(HashMap::new())),
            global_subscribers: Arc::new(RwLock::new(Vec::new())),
            orphan_buffer: Arc::new(Mutex::new(VecDeque::new())),
            config,
        }
    }

    /// Route an incoming IOPub message to the appropriate handler(s)
    pub fn route_message(&self, msg: Message) {
        log::trace!("Routing message: {:#?}", msg);

        // First, send to all global subscribers
        self.send_to_global_subscribers(&msg);

        // Extract parent header to correlate with request
        let parent_id = self.extract_parent_id(&msg);

        // Try to route to specific request
        if let Some(parent_id) = parent_id {
            self.route_to_request(&parent_id, msg);
        } else {
            // No parent ID, handle as orphan
            self.handle_orphan(msg);
        }

        // route_to_request already consumed the message whether it succeeded or not
        // (it either sent it or handled it as orphan internally)
    }

    /// Extract the parent message ID from a message
    fn extract_parent_id(&self, msg: &Message) -> Option<String> {
        match msg {
            Message::Status(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::ExecuteResult(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::ExecuteError(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::ExecuteInput(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::Stream(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::DisplayData(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::UpdateDisplayData(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::CommOpen(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::CommMsg(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            Message::CommClose(m) => m.parent_header.as_ref().map(|h| h.msg_id.clone()),
            _ => None,
        }
    }

    /// Get the message type as a string for logging
    fn message_type(&self, msg: &Message) -> &str {
        match msg {
            Message::Status(_) => "Status",
            Message::ExecuteResult(_) => "ExecuteResult",
            Message::ExecuteError(_) => "ExecuteError",
            Message::ExecuteInput(_) => "ExecuteInput",
            Message::Stream(_) => "Stream",
            Message::DisplayData(_) => "DisplayData",
            Message::UpdateDisplayData(_) => "UpdateDisplayData",
            Message::CommOpen(_) => "CommOpen",
            Message::CommMsg(_) => "CommMsg",
            Message::CommClose(_) => "CommClose",
            Message::Welcome(_) => "Welcome",
            _ => "Other",
        }
    }

    /// Route a message to a specific request's channels
    /// Returns whether the message was successfully routed (and consumed)
    /// If routing fails, the message is buffered as an orphan
    fn route_to_request(&self, parent_id: &str, msg: Message) -> bool {
        let active = self.active_requests.read().unwrap();

        if let Some(ctx) = active.get(parent_id) {
            // Route based on message type
            let result = match &msg {
                Message::Status(_) => {
                    ctx.channels.status_tx.send(msg)
                }
                Message::Stream(_) => {
                    ctx.channels.stream_tx.send(msg)
                }
                Message::ExecuteResult(_) | Message::ExecuteError(_) | Message::ExecuteInput(_) => {
                    ctx.channels.execution_tx.send(msg)
                }
                Message::DisplayData(_) | Message::UpdateDisplayData(_) => {
                    ctx.channels.display_tx.send(msg)
                }
                Message::CommOpen(_) | Message::CommMsg(_) | Message::CommClose(_) => {
                    ctx.channels.comm_tx.send(msg)
                }
                _ => {
                    log::warn!("Unhandled message type for routing: {}", self.message_type(&msg));
                    // Drop msg by moving it into orphan buffer
                    drop(active); // Release lock before calling handle_orphan
                    self.handle_orphan(msg);
                    return true; // Message consumed
                }
            };

            if result.is_err() {
                log::warn!(
                    "Failed to send message to request {}: receiver dropped",
                    parent_id
                );
            }

            true // Message was sent (or attempted to send)
        } else {
            // No matching request found - buffer as orphan
            drop(active); // Release lock before calling handle_orphan
            self.handle_orphan(msg);
            true // Message consumed
        }
    }

    /// Send message to all global subscribers
    fn send_to_global_subscribers(&self, msg: &Message) {
        let subscribers = self.global_subscribers.read().unwrap();
        for sub in subscribers.iter() {
            // Ignore send errors - subscribers may have disconnected
            let _ = sub.send(msg.clone());
        }
    }

    /// Handle a message that doesn't match any active request
    fn handle_orphan(&self, msg: Message) {
        log::debug!(
            "Orphan message (type: {}): no matching request found",
            self.message_type(&msg)
        );

        let mut buffer = self.orphan_buffer.lock().unwrap();
        buffer.push_back((msg, Instant::now()));

        // Keep buffer size bounded
        while buffer.len() > self.config.orphan_buffer_max {
            if let Some((dropped_msg, _)) = buffer.pop_front() {
                log::trace!(
                    "Dropped old orphan message (type: {}) due to buffer limit",
                    self.message_type(&dropped_msg)
                );
            }
        }
    }

    /// Register a new request that expects IOPub messages
    pub fn register_request(&self, request_id: RequestId, channels: RequestChannels) {
        log::trace!("Registering request: {}", request_id);

        let ctx = RequestContext {
            request_id: request_id.clone(),
            started_at: Instant::now(),
            channels,
        };

        self.active_requests.write().unwrap()
            .insert(request_id, ctx);
    }

    /// Unregister a completed request
    pub fn unregister_request(&self, request_id: &RequestId) {
        log::trace!("Unregistering request: {}", request_id);

        self.active_requests.write().unwrap()
            .remove(request_id);
    }

    /// Clean up stale requests that have exceeded the timeout
    pub fn cleanup_stale_requests(&self) {
        let timeout = self.config.request_timeout;
        let now = Instant::now();

        let mut active = self.active_requests.write().unwrap();
        let before_count = active.len();

        active.retain(|id, ctx| {
            let age = now.duration_since(ctx.started_at);
            if age >= timeout {
                log::warn!(
                    "Removing stale request {} (age: {:?})",
                    id,
                    age
                );
                false
            } else {
                true
            }
        });

        let removed = before_count - active.len();
        if removed > 0 {
            log::info!("Cleaned up {} stale request(s)", removed);
        }
    }

    /// Clean up old orphan messages
    pub fn cleanup_orphans(&self) {
        let max_age = self.config.orphan_max_age;
        let now = Instant::now();

        let mut buffer = self.orphan_buffer.lock().unwrap();
        let before_count = buffer.len();

        buffer.retain(|(_, timestamp)| {
            now.duration_since(*timestamp) < max_age
        });

        let removed = before_count - buffer.len();
        if removed > 0 {
            log::debug!("Cleaned up {} old orphan message(s)", removed);
        }
    }

    /// Perform all cleanup operations
    pub fn cleanup(&self) {
        self.cleanup_stale_requests();
        self.cleanup_orphans();
    }

    /// Add a global subscriber that receives all IOPub messages
    pub fn add_global_subscriber(&self, tx: Sender<Message>) {
        self.global_subscribers.write().unwrap().push(tx);
    }

    /// Remove all global subscribers
    pub fn clear_global_subscribers(&self) {
        self.global_subscribers.write().unwrap().clear();
    }

    /// Get the number of active requests
    pub fn active_request_count(&self) -> usize {
        self.active_requests.read().unwrap().len()
    }

    /// Get the number of orphan messages in the buffer
    pub fn orphan_count(&self) -> usize {
        self.orphan_buffer.lock().unwrap().len()
    }

    /// Get statistics about the broker
    pub fn stats(&self) -> BrokerStats {
        BrokerStats {
            active_requests: self.active_request_count(),
            orphan_messages: self.orphan_count(),
            global_subscribers: self.global_subscribers.read().unwrap().len(),
        }
    }
}

/// Statistics about the broker's current state
#[derive(Debug, Clone)]
pub struct BrokerStats {
    pub active_requests: usize,
    pub orphan_messages: usize,
    pub global_subscribers: usize,
}

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
            stream_tx,
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

/*
 * broker.rs
 *
 * Message broker for routing IOPub messages to appropriate handlers
 * based on parent message correlation.
 *
 */

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::msg::wire::jupyter_message::Message;

/// Uniquely identifies a request-response cycle using the request's msg_id
pub type RequestId = String;

/// Context for tracking an active request that expects IOPub messages
struct RequestContext {
    request_id: RequestId,
    started_at: Instant,
    channel: Sender<Message>,
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
            orphan_buffer: Arc::new(Mutex::new(VecDeque::new())),
            config,
        }
    }

    /// Route an incoming IOPub message to the appropriate handler(s)
    pub fn route(&self, msg: Message) {
        log::trace!("Routing message: {}", msg.kind());

        if let Some(parent_id) = msg.parent_id() {
            self.route_to_request(&parent_id, msg);
        } else {
            // No parent ID, handle as orphan
            log::warn!("Orphaning message: {}", msg.kind());
            self.handle_orphan(msg);
        }
    }

    /// Route a message to a specific request's channels
    /// Returns whether the message was successfully routed (and consumed)
    /// If routing fails, the message is buffered as an orphan
    fn route_to_request(&self, parent_id: &str, msg: Message) -> bool {
        let active = self.active_requests.read().unwrap();

        if let Some(ctx) = active.get(parent_id) {
            let result = ctx.channel.send(msg);

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

    /// Handle a message that doesn't match any active request
    fn handle_orphan(&self, msg: Message) {
        log::debug!("Orphan message {:#?}: no matching request found", &msg);

        let mut buffer = self.orphan_buffer.lock().unwrap();
        buffer.push_back((msg, Instant::now()));

        // Keep buffer size bounded
        while buffer.len() > self.config.orphan_buffer_max {
            if let Some((dropped_msg, _)) = buffer.pop_front() {
                log::trace!(
                    "Dropped old orphan message {:#?} due to buffer limit",
                    &dropped_msg
                );
            }
        }
    }

    /// Register a new request that expects IOPub messages
    pub fn register_request(&self, request_id: RequestId, channel: Sender<Message>) {
        log::trace!("Registering request: {}", request_id);
        self.active_requests.write().unwrap().insert(
            request_id.clone(),
            RequestContext {
                request_id: request_id,
                started_at: Instant::now(),
                channel,
            },
        );
    }

    /// Unregister a completed request
    pub fn unregister_request(&self, request_id: &RequestId) {
        log::trace!("Unregistering request: {}", request_id);
        self.active_requests.write().unwrap().remove(request_id);
    }

    /// Clean up stale requests that have exceeded the timeout
    pub fn drop_stale_requests(&self) {
        let timeout = self.config.request_timeout;
        let now = Instant::now();

        let mut active = self.active_requests.write().unwrap();
        let before_count = active.len();

        active.retain(|id, ctx| {
            let age = now.duration_since(ctx.started_at);
            if age >= timeout {
                log::warn!("Removing stale request {} (age: {:?})", id, age);
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
    pub fn drop_orphan_requests(&self) {
        let max_age = self.config.orphan_max_age;
        let now = Instant::now();

        let mut buffer = self.orphan_buffer.lock().unwrap();
        let before_count = buffer.len();

        buffer.retain(|(_, timestamp)| now.duration_since(*timestamp) < max_age);

        let removed = before_count - buffer.len();
        if removed > 0 {
            log::debug!("Cleaned up {} old orphan message(s)", removed);
        }
    }

    /// Perform all cleanup operations
    pub fn clean(&self) {
        self.drop_stale_requests();
        self.drop_orphan_requests();
    }

    /// Get statistics about the broker
    pub fn stats(&self) -> BrokerStats {
        BrokerStats {
            active_requests: self.active_requests.read().unwrap().len(),
            orphan_messages: self.orphan_buffer.lock().unwrap().len(),
        }
    }
}

/// Statistics about the broker's current state
#[derive(Debug, Clone)]
pub struct BrokerStats {
    pub active_requests: usize,
    pub orphan_messages: usize,
}

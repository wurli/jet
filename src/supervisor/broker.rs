/*
 * broker.rs
 *
 * Message broker for routing messages to appropriate handlers
 * based on parent message correlation.
 *
 */

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::msg::wire::jupyter_message::Message;
use crate::msg::wire::status::ExecutionState;

/// Uniquely identifies a request-response cycle using the request's msg_id
pub type RequestId = String;

/// Context for tracking an active request that expects messages to be returned from the kernel
struct RequestContext {
    started_at: Instant,
    channel: Sender<Message>,
}

/// Configuration for the broker
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

/// Central message broker that routes messages based on parent headers
pub struct Broker {
    name: String,

    /// Active requests waiting for messages
    active_requests: Arc<RwLock<HashMap<RequestId, RequestContext>>>,

    /// Buffer for "orphan" messages (no matching request)
    orphan_buffer: Arc<Mutex<VecDeque<(Message, Instant)>>>,

    /// Configuration
    pub config: BrokerConfig,
}

impl Broker {
    /// Create a new broker with default configuration
    pub fn new(name: String) -> Self {
        Self::with_config(name, BrokerConfig::default())
    }

    /// Create a new broker with custom configuration
    pub fn with_config(name: String, config: BrokerConfig) -> Self {
        Self {
            name,
            active_requests: Arc::new(RwLock::new(HashMap::new())),
            orphan_buffer: Arc::new(Mutex::new(VecDeque::new())),
            config,
        }
    }

    /// Route an incoming message to the appropriate handler(s)
    pub fn route(&self, msg: Message) {
        log::trace!(
            "{}: Routing message: {}<{}>",
            self.name,
            msg.kind(),
            msg.parent_id().unwrap_or(String::from("unparented"))
        );

        if let Some(parent_id) = msg.parent_id() {
            self.route_to_request(&parent_id, msg);
        } else {
            // No parent ID, handle as orphan
            log::trace!("{}: Routing unparented message: {}", self.name, msg.kind());
            self.route_to_request("unparented", msg);
        }
    }

    /// Route a message to a specific request's channels
    /// Returns whether the message was successfully routed (and consumed)
    /// If routing fails, the message is buffered as an orphan
    fn route_to_request(&self, parent_id: &str, msg: Message) {
        let active_requests = self.active_requests.read().unwrap();

        // TODO: clean up this nasty thing
        let should_unregister = if let Some(ctx) = active_requests.get(parent_id) {
            let complete_status = match &msg {
                // Unregister requests with iopub broker when we get an idle status
                Message::Status(m) if m.content.execution_state == ExecutionState::Idle => {
                    Some("received idle status")
                }
                // Unregister requests with shell broker when we get a reply
                Message::InterruptReply(_)
                | Message::CommInfoReply(_)
                | Message::ExecuteReply(_)
                | Message::ExecuteReplyException(_)
                | Message::HandshakeReply(_)
                | Message::InputReply(_)
                | Message::InspectReply(_)
                | Message::IsCompleteReply(_)
                | Message::KernelInfoReply(_) => {
                    Some("reply received")
                }
                _ => None
            };

            if let Err(_) = ctx.channel.send(msg) {
                log::warn!(
                    "{}: Failed to send message to request {}: receiver dropped",
                    self.name,
                    parent_id
                );
            }

            complete_status
        } else {
            // No matching request found - buffer as orphan
            self.handle_orphan(msg);
            None
        };

        // Now unregister if needed (requires write lock)
        if let Some(reason) = should_unregister {
            drop(active_requests);
            self.unregister_request(&String::from(parent_id), reason);
        }
    }

    /// Handle a message that doesn't match any active request
    fn handle_orphan(&self, msg: Message) {
        log::trace!(
            "{}: Orphan {} message {:#?}: no matching request found",
            self.name,
            msg.kind(),
            &msg
        );

        let mut buffer = self.orphan_buffer.lock().unwrap();
        buffer.push_back((msg, Instant::now()));

        // Keep buffer size bounded
        while buffer.len() > self.config.orphan_buffer_max {
            if let Some((dropped_msg, _)) = buffer.pop_front() {
                log::trace!(
                    "{}: Dropped old {} orphan message {:#?} due to buffer limit",
                    self.name,
                    dropped_msg.kind(),
                    &dropped_msg
                );
            }
        }
    }

    /// Register a new request that expects messages
    pub fn register_request(&self, request_id: RequestId, channel: Sender<Message>) {
        log::trace!("{}: Registering request: {}", self.name, request_id);
        self.active_requests.write().unwrap().insert(
            request_id.clone(),
            RequestContext {
                started_at: Instant::now(),
                channel,
            },
        );
    }

    /// Unregister a completed request
    pub fn unregister_request(&self, request_id: &RequestId, reason: &str) {
        log::trace!(
            "{}: Unregistering request {}: {:?}",
            self.name,
            request_id,
            reason
        );
        self.active_requests.write().unwrap().remove(request_id);
    }

    pub fn is_active(&self, request_id: &RequestId) -> bool {
        self.active_requests
            .read()
            .unwrap()
            .contains_key(request_id)
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
                log::warn!(
                    "{}: Removing stale broker request {} (age: {:?})",
                    self.name,
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
            log::info!("{}: Cleaned up {} stale request(s)", self.name, removed);
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
            log::trace!(
                "{}: Cleaned up {} old orphan message(s)",
                self.name,
                removed
            );
        }
    }

    /// Perform all cleanup operations
    pub fn purge(&self) {
        self.log_stats();
        log::trace!("{}: Performing broker cleanup", self.name);
        self.drop_stale_requests();
        self.drop_orphan_requests();
        self.log_stats();
    }

    pub fn log_stats(&self) {
        log::trace!(
            "{} broker stats: {} active requests, {} orphans",
            self.name,
            self.active_requests.read().unwrap().len(),
            self.orphan_buffer.lock().unwrap().len(),
        );
    }
}

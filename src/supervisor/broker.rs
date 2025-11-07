/*
 * broker.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::msg::wire::jupyter_message::Message;
use crate::msg::wire::message_id::Id;

/// Context for tracking an active request that expects messages to be returned from the kernel
struct RequestContext {
    started_at: Instant,
    channel: Sender<Message>,
}

/// Configuration for the broker
#[derive(Debug, Clone)]
pub struct BrokerConfig {
    /// Maximum age of stale requests before cleanup
    pub request_timeout: Duration,
    /// Interval between cleanup operations
    pub cleanup_interval: Duration,
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(30),
        }
    }
}

/// Central message broker that routes messages based on parent headers
pub struct Broker {
    name: String,

    /// Active requests waiting for messages
    active_requests: Arc<RwLock<HashMap<Id, RequestContext>>>,

    /// Open 'comms'
    /// https://jupyter-client.readthedocs.io/en/latest/messaging.html#custom-messages
    open_comms: Arc<RwLock<HashMap<Id, RequestContext>>>,

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
            open_comms: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Route an incoming message to the appropriate handler(s)
    pub fn route(&self, msg: Message) {
        log::trace!("{}: Routing message: {}", self.name, msg.describe(),);

        let parent_id = msg.parent_id().unwrap_or(&Id::unparented()).clone();

        match msg {
            Message::CommOpen(inner) => {
                self.register_comm(&Id::from(inner.content.comm_id), inner.content.target_name);
                return;
            }
            Message::CommClose(inner) => {
                self.unregister_comm(
                    &Id::from(inner.content.comm_id),
                    "received request to close from the kernel",
                );
                return;
            }
            Message::CommMsg(_) => {
                match self.open_comms.read().unwrap().get(&parent_id) {
                    // If there's no corresponding active request, the reply is an orphan
                    None => log::warn!("{}: Dropping orphan comm message {:#?}", self.name, msg),
                    // If there _is_ a corresponding active request, try routing to the corresponding
                    // receiver
                    Some(request) => {
                        let description = msg.describe();
                        if request.channel.send(msg).is_err() {
                            log::warn!(
                                "{}: Failed to route {} for request {}: receiver dropped",
                                self.name,
                                description,
                                parent_id
                            );
                        }
                    }
                }
                return;
            }
            _ => {}
        }

        match self.active_requests.read().unwrap().get(&parent_id) {
            // If there's no corresponding active request, the reply is an orphan
            None => log::warn!("{}: Dropping orphan message {:#?}", self.name, msg.clone()),
            // If there _is_ a corresponding active request, try routing to the corresponding
            // receiver
            Some(request) => {
                let description = msg.describe();
                if request.channel.send(msg).is_err() {
                    log::warn!(
                        "{}: Failed to route {} for request {}: receiver dropped",
                        self.name,
                        description,
                        parent_id
                    );
                }
            }
        }
    }

    /// Register a new request that expects messages
    pub fn register_comm(&self, request_id: &Id, name: String) -> Receiver<Message> {
        log::trace!(
            "{}: Registering new comm: {}{}",
            self.name,
            name,
            request_id
        );
        let (tx, rx) = std::sync::mpsc::channel();
        self.open_comms.write().unwrap().insert(
            request_id.clone(),
            RequestContext {
                started_at: Instant::now(),
                channel: tx,
            },
        );
        rx
    }

    /// Register a new request that expects messages
    pub fn unregister_comm(&self, request_id: &Id, reason: &str) {
        match self.open_comms.write().unwrap().remove(request_id) {
            Some(_) => log::trace!(
                "{}: Unregistered comm {}: {:?}",
                self.name,
                request_id,
                reason
            ),
            None => log::warn!(
                "{}: Attempted to unregister non-present comm {}: {:?}",
                self.name,
                request_id,
                reason
            ),
        }
    }

    /// Register a new request that expects messages
    pub fn register_request(&self, request_id: &Id) -> Receiver<Message> {
        log::trace!("{}: Registering request: {}", self.name, request_id);
        let (tx, rx) = std::sync::mpsc::channel();
        self.active_requests.write().unwrap().insert(
            request_id.clone(),
            RequestContext {
                started_at: Instant::now(),
                channel: tx,
            },
        );
        rx
    }

    /// Unregister a completed request
    pub fn unregister_request(&self, request_id: &Id, reason: &str) {
        match self.active_requests.write().unwrap().remove(request_id) {
            Some(_) => log::trace!(
                "{}: Unregistered request {}: {:?}",
                self.name,
                request_id,
                reason
            ),
            None => log::warn!(
                "{}: Attempted to unregister non-present request {}: {:?}",
                self.name,
                request_id,
                reason
            ),
        }
    }

    pub fn is_active(&self, request_id: &Id) -> bool {
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

    /// Perform all cleanup operations
    pub fn purge(&self) {
        self.log_stats();
        log::trace!("{}: Performing broker cleanup", self.name);
        self.drop_stale_requests();
        self.log_stats();
    }

    pub fn log_stats(&self) {
        log::trace!(
            "{} broker stats: {} active requests",
            self.name,
            self.active_requests.read().unwrap().len(),
        );
    }
}

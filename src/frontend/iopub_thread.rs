/*
 * iopub_thread.rs
 *
 * Thread management for IOPub message processing with broker-based routing
 *
 */

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::frontend::iopub::Iopub;
use crate::msg::broker::IopubBroker;

/// Spawn a thread that continuously receives IOPub messages and routes them through the broker
pub fn start_iopub_thread(
    iopub: Iopub,
    broker: Arc<IopubBroker>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        log::info!("IOPub thread started");
        
        let cleanup_interval = broker.config.cleanup_interval;
        let mut last_cleanup = Instant::now();
        
        loop {
            // Receive with a short timeout to allow periodic cleanup
            match iopub.recv_timeout(Duration::from_millis(100)) {
                Some(msg) => {
                    log::trace!("IOPub received: {}", msg.message_type());
                    broker.route_message(msg);
                }
                None => {
                    // Timeout - this is normal, gives us a chance to do cleanup
                }
            }
            
            // Periodic cleanup of stale requests and orphan messages
            if last_cleanup.elapsed() >= cleanup_interval {
                log::trace!("Performing IOPub broker cleanup");
                broker.cleanup();
                
                let stats = broker.stats();
                log::debug!(
                    "IOPub broker stats: {} active requests, {} orphans, {} subscribers",
                    stats.active_requests,
                    stats.orphan_messages,
                    stats.global_subscribers
                );
                
                last_cleanup = Instant::now();
            }
        }
    })
}

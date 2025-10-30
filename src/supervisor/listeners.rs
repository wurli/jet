/*
 * iopub_thread.rs
 *
 * Thread management for IOPub message processing with broker-based routing
 *
 */

use rand::Rng;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::connection::heartbeat::Heartbeat;
use crate::connection::iopub::Iopub;
use crate::supervisor::broker::Broker;

/// Spawn a thread that continuously receives IOPub messages and routes them through the broker
pub fn listen_iopub(iopub: Iopub, broker: Arc<Broker>) -> JoinHandle<()> {
    thread::spawn(move || {
        log::info!("IOPub thread started");

        let cleanup_interval = broker.config.cleanup_interval;
        let mut last_cleanup = Instant::now();

        loop {
            // Receive with a short timeout to allow periodic cleanup
            if let Some(msg) = iopub.recv_with_timeout(100) {
                log::trace!("Message received on iopub: {}", msg.describe(),);
                broker.route(msg);
            };

            // Periodic cleanup of stale requests and orphan messages
            if last_cleanup.elapsed() >= cleanup_interval {
                broker.purge();
                broker.log_stats();
                last_cleanup = Instant::now();
            }
        }
    })
}

pub fn loop_heartbeat(heartbeat: Heartbeat) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            let mut rng = rand::rng();
            // We just send some random number to the kernel
            let bytes: Vec<u8> = (0..32).map(|_| rng.random_range(0..10)).collect();

            heartbeat.send(zmq::Message::from(&bytes));

            // Then we (hopefully) wait to receive the same message back
            let reply = heartbeat
                .recv_with_timeout(1000)
                .expect("Heartbeat timed out");

            let reply_slice: &[u8] = reply.as_ref();

            if reply_slice != bytes.as_slice() {
                log::warn!(
                    "Heartbeat reply not the same as request: {:?} != {:?}",
                    bytes,
                    reply_slice,
                )
            }

            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    })
}

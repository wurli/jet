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

use crate::frontend::heartbeat::Heartbeat;
use crate::frontend::iopub::Iopub;
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
                log::trace!(
                    "Message received on iopub: {}<{}>",
                    msg.kind(),
                    msg.parent_id().unwrap_or(String::from("no parent"))
                );
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

// NOTE::
// We can't really move the shell socket into its own thread since it, unlike iopub, needs
// to be able to _send_ as well as receive.
// /// Spawn a thread that continuously receives shell messages and routes them through the broker
// pub fn listen_shell(shell: Shell, broker: Arc<Broker>) -> JoinHandle<()> {
//     thread::spawn(move || {
//         log::info!("Shell thread started");
//
//         let cleanup_interval = broker.config.cleanup_interval;
//         let mut last_cleanup = Instant::now();
//
//         loop {
//             // Receive with a short timeout to allow periodic cleanup
//             // This timeout is quite a bit less than the iopub one since we receive a lot less
//             // messages on the shell.
//             if let Some(msg) = shell.recv_with_timeout(30_000) {
//                 log::trace!("Message received on shell: {}", msg.kind());
//                 broker.route(msg);
//             };
//
//             // Periodic cleanup of stale requests and orphan messages
//             if last_cleanup.elapsed() >= cleanup_interval {
//                 broker.purge();
//                 last_cleanup = Instant::now();
//             }
//         }
//     })
// }

pub fn loop_heartbeat(heartbeat: Heartbeat) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            let mut rng = rand::rng();
            // We just send some random number to the kernel
            let bytes: Vec<u8> = (0..32).map(|_| rng.random_range(0..10)).collect();

            heartbeat.send(zmq::Message::from(bytes));

            // Then we (hopefully) wait to receive the same message back
            let _ = heartbeat.recv_with_timeout(1000).expect("Heartbeat timed out");

            // TODO: check the message we receive is the one we sent
            // assert_eq!(bytes, msg.);
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    })
}

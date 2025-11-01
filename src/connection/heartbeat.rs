/*
 * heartbeat.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::{connection::connection::ConnectionOptions, msg::socket::Socket};

pub struct Heartbeat {
    socket: Socket,
}

impl Heartbeat {
    pub fn init(opts: &ConnectionOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Heartbeat"),
            zmq::REQ,
            None,
            endpoint,
        )
        .unwrap();

        Self { socket }
    }

    /// Receives a (raw) message from the heartbeat socket
    pub fn recv(&self) -> zmq::Message {
        let mut msg = zmq::Message::new();
        self.socket.recv(&mut msg).unwrap();
        msg
    }

    /// Receives a (raw) message from the heartbeat socket
    ///
    /// Returns an error if no message is received within 1 second.
    pub fn recv_with_timeout(&self, timeout: i64) -> Result<zmq::Message, anyhow::Error> {
        if self.socket.poll_incoming(timeout).unwrap() {
            Ok(self.recv())
        } else {
            Err(anyhow::anyhow!("Heartbeat timeout after {timeout} ms"))
        }
    }

    /// Sends a (raw) message to the heartbeat socket
    pub fn send(&self, msg: zmq::Message) {
        self.socket.send(msg).unwrap();
    }
}

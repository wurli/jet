/*
 * control.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::{
    connection::connection::ConnectionOptions,
    msg::{
        socket::Socket,
        wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage},
    },
};

pub struct Control {
    socket: Socket,
}

impl Control {
    pub fn init(opts: &ConnectionOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Control"),
            zmq::DEALER,
            None,
            endpoint,
        )
        .unwrap();

        Self { socket }
    }

    pub fn recv(&self) -> Message {
        Message::read_from_socket(&self.socket).unwrap()
    }

    // TODO: this really needs more granular error handling
    pub fn try_recv(&self) -> anyhow::Result<Message> {
        if self.socket.has_incoming_data()? {
            // Just unwrapping here because I don't _think_ this should go wrong
            // and currently not sure how to handle if it does.
            Ok(Message::read_from_socket(&self.socket)?)
        } else {
            Err(anyhow::anyhow!("No incoming data on shell socket"))
        }
    }

    pub fn send<T: ProtocolMessage>(&self, msg: JupyterMessage<T>) {
        msg.send(&self.socket).unwrap();
    }
}

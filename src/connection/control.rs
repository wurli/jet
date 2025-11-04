/*
 * control.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::{
    connection::connection::ConnectionOptions, error::Error, msg::{
        socket::Socket,
        wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage},
    }
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

    pub fn try_recv(&self) -> Result<Message, Error> {
        match self.socket.has_incoming_data() {
            Ok(true) => Ok(Message::read_from_socket(&self.socket)?),
            Ok(false) => Err(Error::NoIncomingData(String::from("stdin"))),
            Err(e) => Err(Error::Anyhow(anyhow::anyhow!(
                "Error checking for incoming data on control socket: {}",
                e,
            ))),
        }
    }

    pub fn send<T: ProtocolMessage>(&self, msg: JupyterMessage<T>) {
        msg.send(&self.socket).unwrap();
    }
}

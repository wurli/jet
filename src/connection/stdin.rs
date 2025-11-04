/*
 * stdin.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::error::Error;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage};
use crate::{connection::connection::ConnectionOptions, msg::socket::Socket};
use assert_matches::assert_matches;

pub struct Stdin {
    socket: Socket,
}

impl Stdin {
    pub fn init(opts: &ConnectionOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Stdin"),
            zmq::DEALER,
            Some(&opts.shell_id),
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
                "Error checking for incoming data on stdin socket: {}",
                e,
            ))),
        }
    }

    pub fn send<T: ProtocolMessage>(&self, msg: JupyterMessage<T>) {
        msg.send(&self.socket).unwrap();
    }

    /// Receive from Stdin and assert `InputRequest` message.
    /// Returns the `prompt`.
    pub fn recv_stdin_input_request(&self) -> String {
        let msg = self.recv();

        assert_matches!(msg, Message::InputRequest(data) => {
            data.content.prompt
        })
    }
}

/*
 * shell.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::error::Error;
use crate::msg::wire::is_complete_reply::IsCompleteReply;
use crate::msg::wire::jupyter_message::Status;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage};
use crate::{connection::connection::ConnectionOptions, msg::socket::Socket};
use assert_matches::assert_matches;

pub struct Shell {
    socket: Socket,
}

impl Shell {
    pub fn init(opts: &ConnectionOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Shell"),
            zmq::DEALER,
            Some(&opts.shell_id),
            endpoint,
        )
        .unwrap();

        Self { socket }
    }

    pub fn recv(&self) -> Message {
        let msg = Message::read_from_socket(&self.socket).unwrap();
        log::trace!("Shell: received {}", msg.describe());
        msg
    }

    pub fn try_recv(&self) -> Result<Message, Error> {
        match self.socket.has_incoming_data() {
            Ok(true) => Ok(Message::read_from_socket(&self.socket)?),
            Ok(false) => Err(Error::NoIncomingData(String::from("stdin"))),
            Err(e) => Err(Error::Anyhow(anyhow::anyhow!(
                "Error checking for incoming data on shell socket: {}",
                e,
            ))),
        }
    }

    pub fn recv_with_timeout(&self, timeout: i64) -> Option<Message> {
        if self.socket.poll_incoming(timeout).unwrap() {
            return Message::read_from_socket(&self.socket).ok();
        }
        None
    }

    pub fn send<T: ProtocolMessage>(&self, msg: JupyterMessage<T>) {
        msg.send(&self.socket).unwrap();
    }

    /// Receive from Shell and assert `ExecuteReply` message.
    /// Returns `execution_count`.
    pub fn recv_execute_reply(&self) -> anyhow::Result<u32> {
        let msg = self.recv();

        assert_matches!(msg, Message::ExecuteReply(data) => {
            assert_eq!(data.content.status, Status::Ok);
            Ok(data.content.execution_count)
        })
    }

    /// Receive from Shell and assert `ExecuteReplyException` message.
    /// Returns `execution_count`.
    pub fn recv_execute_reply_exception(&self) -> u32 {
        let msg = self.recv();

        assert_matches!(msg, Message::ExecuteReplyException(data) => {
            assert_eq!(data.content.status, Status::Error);
            data.content.execution_count
        })
    }

    pub fn recv_is_complete_reply(&self) -> IsCompleteReply {
        let msg = self.recv();

        assert_matches!(msg, Message::IsCompleteReply(data) => {
            data.content
        })
    }
}

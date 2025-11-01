/*
 * iopub.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::msg::wire::execute_input::ExecuteInput;
use crate::msg::wire::jupyter_message::Message;
use crate::msg::wire::status::ExecutionState;
use crate::msg::wire::stream::Stream;
use crate::{connection::connection::ConnectionOptions, msg::socket::Socket};
use assert_matches::assert_matches;
use serde_json::Value;

pub struct Iopub {
    socket: Socket,
}

impl Iopub {
    pub fn init(opts: &ConnectionOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("IOPub"),
            zmq::SUB,
            None,
            endpoint,
        )
        .unwrap();

        Self { socket }
    }

    pub fn recv(&self) -> Message {
        Message::read_from_socket(&self.socket).unwrap()
    }

    pub fn recv_with_timeout(&self, timeout: i64) -> Option<Message> {
        if self.socket.poll_incoming(timeout).unwrap() {
            return Message::read_from_socket(&self.socket).ok();
        }
        None
    }

    /// Receive from IOPub and assert Busy message
    pub fn recv_busy(&self) {
        let msg = self.recv();

        assert_matches!(msg, Message::Status(data) => {
            assert_eq!(data.content.execution_state, ExecutionState::Busy);
        });
    }

    /// Receive from IOPub and assert Idle message
    pub fn recv_idle(&self) {
        let msg = self.recv();

        assert_matches!(msg, Message::Status(data) => {
            assert_eq!(data.content.execution_state, ExecutionState::Idle);
        });
    }

    /// Receive from IOPub and assert ExecuteInput message
    pub fn recv_execute_input(&self) -> ExecuteInput {
        let msg = self.recv();

        assert_matches!(msg, Message::ExecuteInput(data) => {
            data.content
        })
    }

    /// Receive from IOPub and assert ExecuteResult message. Returns compulsory
    /// `plain/text` result.
    pub fn recv_execute_result(&self) -> String {
        let msg = self.recv();

        assert_matches!(msg, Message::ExecuteResult(data) => {
            assert_matches!(data.content.data["text/plain"], Value::String(ref string) => {
                string.clone()
            })
        })
    }

    pub fn recv_display_data(&self) {
        let msg = self.recv();
        assert_matches!(msg, Message::DisplayData(_))
    }

    pub fn recv_update_display_data(&self) {
        let msg = self.recv();
        assert_matches!(msg, Message::UpdateDisplayData(_))
    }

    pub fn recv_stream_stdout(&self, expect: &str) {
        self.recv_stream(expect, Stream::Stdout)
    }

    pub fn recv_stream_stderr(&self, expect: &str) {
        self.recv_stream(expect, Stream::Stderr)
    }

    pub fn recv_comm_close(&self) -> String {
        let msg = self.recv();

        assert_matches!(msg, Message::CommClose(data) => {
            data.content.comm_id
        })
    }

    /// Receive from IOPub Stream
    ///
    /// Stdout and Stderr Stream messages are buffered, so to reliably test against them
    /// we have to collect the messages in batches on the receiving end and compare against
    /// an expected message.
    fn recv_stream(&self, expect: &str, stream: Stream) {
        let mut out = String::new();

        loop {
            // Receive a piece of stream output (with a timeout)
            let msg = self.recv();

            // Assert its type
            let piece = assert_matches!(msg, Message::Stream(data) => {
                assert_eq!(data.content.name, stream);
                data.content.text
            });

            // Add to what we've already collected
            out += piece.as_str();

            if out == expect {
                // Done, found the entire `expect` string
                return;
            }

            if !expect.starts_with(out.as_str()) {
                // Something is wrong, message doesn't match up
                panic!("Expected IOPub stream of '{expect}'. Actual stream of '{out}'.");
            }

            // We have a prefix of `expect`, but not the whole message yet.
            // Wait on the next IOPub Stream message.
        }
    }

    /// Receive from IOPub and assert ExecuteResult message. Returns compulsory
    /// `evalue` field.
    pub fn recv_execute_error(&self) -> String {
        let msg = self.recv();

        assert_matches!(msg, Message::ExecuteError(data) => {
            data.content.exception.evalue
        })
    }
}

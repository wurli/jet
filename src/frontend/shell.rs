use crate::frontend::frontend::ExecuteRequestOptions;
use crate::msg::session::Session;
use crate::msg::wire::execute_request::ExecuteRequest;
use crate::msg::wire::is_complete_reply::IsCompleteReply;
use crate::msg::wire::is_complete_request::IsCompleteRequest;
use crate::msg::wire::jupyter_message::Status;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage};
use crate::msg::wire::wire_message::WireMessage;
use crate::{frontend::frontend::FrontendOptions, msg::socket::Socket};
use assert_matches::assert_matches;

pub struct Shell {
    socket: Socket,
    session: Session,
}

impl Shell {
    pub fn init(opts: &FrontendOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Shell"),
            zmq::DEALER,
            Some(&opts.shell_id),
            endpoint,
        )
        .unwrap();

        Self {
            socket: socket,
            session: opts.session.clone(),
        }
    }

    // fn flush_incoming(&self, name: &str) {
    //     eprintln!("{name} has incoming data:");
    //
    //     while self.socket.has_incoming_data().unwrap() {
    //         dbg!(WireMessage::read_from_socket(&self.socket).unwrap());
    //         eprintln!("---");
    //     }
    // }

    pub fn recv(&self) -> Message {
        Message::read_from_socket(&self.socket).unwrap()
    }

    pub fn send<T: ProtocolMessage>(&self, msg: T) -> String {
        let message = JupyterMessage::create(msg, None, &self.session);
        let id = message.header.msg_id.clone();
        message.send(&self.socket).unwrap();
        id
    }

    /// Receive from Shell and assert `ExecuteReply` message.
    /// Returns `execution_count`.
    pub fn recv_execute_reply(&self) -> u32 {
        let msg = self.recv();

        assert_matches!(msg, Message::ExecuteReply(data) => {
            assert_eq!(data.content.status, Status::Ok);
            data.content.execution_count
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

    pub fn send_is_complete_request(&self, code: &str) -> String {
        self.send(IsCompleteRequest {
            code: String::from(code),
        })
    }


    pub fn send_execute_request(&self, code: &str, options: ExecuteRequestOptions) -> String {
        self.send(ExecuteRequest {
            code: String::from(code),
            silent: false,
            store_history: true,
            user_expressions: serde_json::Value::Null,
            allow_stdin: options.allow_stdin,
            stop_on_error: false,
        })
    }

}

use crate::msg::session::Session;
use crate::msg::wire::input_reply::InputReply;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage};
use crate::{frontend::frontend::FrontendOptions, msg::socket::Socket};
use assert_matches::assert_matches;

pub struct Stdin {
    socket: Socket,
    session: Session,
}

impl Stdin {
    pub fn init(opts: &FrontendOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Stdin"),
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

    fn send_stdin<T: ProtocolMessage>(&self, msg: T) {
        let message = JupyterMessage::create(msg, None, &self.session);
        // let id = message.header.msg_id.clone();
        message.send(&self.socket).unwrap();
        // id
    }

    /// Receive from Stdin and assert `InputRequest` message.
    /// Returns the `prompt`.
    pub fn recv_stdin_input_request(&self) -> String {
        let msg = self.recv();

        assert_matches!(msg, Message::InputRequest(data) => {
            data.content.prompt
        })
    }

    /// Send back an `InputReply` to an `InputRequest` over Stdin
    pub fn send_stdin_input_reply(&self, value: String) {
        self.send_stdin(InputReply { value })
    }
}

use crate::msg::session::Session;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage};
use crate::{connection::connection::ConnectionOptions, msg::socket::Socket};
use assert_matches::assert_matches;

pub struct Stdin {
    socket: Socket,
    session: Session,
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

        Self {
            socket: socket,
            session: opts.session.clone(),
        }
    }

    pub fn recv(&self) -> Message {
        Message::read_from_socket(&self.socket).unwrap()
    }

    pub fn try_recv(&self) -> anyhow::Result<Message> {
        if self.socket.has_incoming_data()? {
            // Just unwrapping here because I don't _think_ this should go wrong
            // and currently not sure how to handle if it does.
            return Ok(Message::read_from_socket(&self.socket)?);
        } else {
            return Err(anyhow::anyhow!("No incoming data on stdin socket"));
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

use crate::{
    connection::connection::ConnectionOptions,
    msg::{
        session::Session,
        socket::Socket,
        wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage},
    },
};

pub struct Control {
    socket: Socket,
    session: Session,
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

        Self {
            socket,
            session: opts.session.clone(),
        }
    }

    pub fn recv(&self) -> Message {
        Message::read_from_socket(&self.socket).unwrap()
    }

    /// TODO: do we need to register ids with brokers _before_ sending the message to avoid
    /// orphaned requests? This might be a good idea :'(
    pub fn send<T: ProtocolMessage>(&self, msg: T) -> String {
        let message = JupyterMessage::create(msg, None, &self.session);
        let id = message.header.msg_id.clone();
        message.send(&self.socket).unwrap();
        id
    }
}

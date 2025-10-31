use crate::{
    connection::connection::ConnectionOptions,
    msg::{
        session::Session,
        socket::Socket,
        wire::{jupyter_message::{JupyterMessage, Message, ProtocolMessage}, message_id::Id},
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

    // TODO: this really needs more granular error handling
    pub fn try_recv(&self) -> anyhow::Result<Message> {
        if self.socket.has_incoming_data()? {
            // Just unwrapping here because I don't _think_ this should go wrong
            // and currently not sure how to handle if it does.
            return Ok(Message::read_from_socket(&self.socket)?);
        } else {
            return Err(anyhow::anyhow!("No incoming data on shell socket"));
        }
    }

    /// TODO: do we need to register ids with brokers _before_ sending the message to avoid
    /// orphaned requests? This might be a good idea :'(
    pub fn send<T: ProtocolMessage>(&self, msg: T) -> Id {
        let message = JupyterMessage::create(msg, None, &self.session);
        let id = message.header.msg_id.clone();
        message.send(&self.socket).unwrap();
        id
    }
}

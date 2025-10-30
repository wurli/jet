use crate::{
    connection::connection::ConnectionOptions,
    msg::{socket::Socket, wire::jupyter_message::Message},
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
}

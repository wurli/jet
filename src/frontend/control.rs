use crate::{
    frontend::frontend::FrontendOptions,
    msg::{socket::Socket, wire::jupyter_message::Message},
};

pub struct Control {
    socket: Socket,
}

impl Control {
    pub fn init(opts: &FrontendOptions, endpoint: String) -> Self {
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

    // fn flush_incoming(&self, name: &str) {
    //     eprintln!("{name} has incoming data:");
    //     while self.socket.has_incoming_data().unwrap() {
    //         dbg!(WireMessage::read_from_socket(&self.socket).unwrap());
    //         eprintln!("---");
    //     }
    // }

    pub fn recv(&self) -> Message {
        Message::read_from_socket(&self.socket).unwrap()
    }
}

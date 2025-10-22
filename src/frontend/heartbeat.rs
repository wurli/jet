use crate::{
    frontend::frontend::FrontendOptions,
    msg::{socket::Socket, wire::wire_message::WireMessage},
};

pub struct Heartbeat {
    socket: Socket,
}

impl Heartbeat {
    pub fn init(opts: &FrontendOptions, endpoint: String) -> Self {
        let socket = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Heartbeat"),
            zmq::REQ,
            None,
            endpoint,
        )
        .unwrap();

        Self { socket }
    }

    // fn flush_incoming(&self, name: &str) {
    //     eprintln!("{name} has incoming data:");
    //
    //     while self.socket.has_incoming_data().unwrap() {
    //         dbg!(WireMessage::read_from_socket(&self.socket).unwrap());
    //         eprintln!("---");
    //     }
    // }

    /// Receives a (raw) message from the heartbeat socket
    pub fn recv(&self) -> zmq::Message {
        let mut msg = zmq::Message::new();
        self.socket.recv(&mut msg).unwrap();
        msg
    }

    /// Receives a (raw) message from the heartbeat socket
    ///
    /// Returns an error if no message is received within 1 second.
    pub fn recv_with_timeout(&self) -> Result<zmq::Message, anyhow::Error> {
        let timeout_ms = 1000;
        if self.socket.poll_incoming(timeout_ms).unwrap() {
            Ok(self.recv())
        } else {
            Err(anyhow::anyhow!("Heartbeat timeout after {timeout_ms} ms"))
        }
    }

    /// Sends a (raw) message to the heartbeat socket
    pub fn send(&self, msg: zmq::Message) {
        self.socket.send(msg).unwrap();
    }
}

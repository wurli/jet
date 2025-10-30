use std::path::PathBuf;

use assert_matches::assert_matches;
use rand::Rng;

use crate::connection::{
    control::Control, heartbeat::Heartbeat, iopub::Iopub, shell::Shell, stdin::Stdin,
};
use crate::msg::connection_file::ConnectionFile;
use crate::msg::registration_file::RegistrationFile;
use crate::msg::session::Session;
use crate::msg::socket::Socket;
use crate::msg::wire::handshake_reply::HandshakeReply;
use crate::msg::wire::jupyter_message::Status;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message};

pub struct ConnectionOptions {
    pub ctx: zmq::Context,
    pub session: Session,
    pub key: String,
    pub ip: String,
    pub transport: String,
    pub signature_scheme: String,
    pub shell_id: [u8; 16],
}

pub struct ExecuteRequestOptions {
    pub allow_stdin: bool,
}

impl ConnectionOptions {
    pub fn init() -> Self {
        // Create a random HMAC key for signing messages.
        let key_bytes = rand::rng().random::<[u8; 16]>();
        let key = hex::encode(key_bytes);

        // Create a new kernel session from the key
        let session = Session::create(&key).unwrap();

        // Create a zmq context for all sockets we create in this session
        let ctx = zmq::Context::new();

        let ip = String::from("127.0.0.1");
        let transport = String::from("tcp");
        let signature_scheme = String::from("hmac-sha256");
        let shell_id = rand::rng().random::<[u8; 16]>();

        Self {
            ctx,
            session,
            key,
            ip,
            transport,
            signature_scheme,
            shell_id,
        }
    }

    fn endpoint(&self, port: u16) -> String {
        format!("{}://{}:{}", &self.transport, &self.ip, port)
    }
}

pub struct RegistrationSockets {
    pub registration: Socket,
}

impl RegistrationSockets {
    pub fn from(opts: &ConnectionOptions) -> Self {
        RegistrationSockets {
            registration: Socket::new(
                opts.session.clone(),
                opts.ctx.clone(),
                String::from("Registration"),
                zmq::REP,
                None,
                opts.endpoint(0),
            )
            .unwrap(),
        }
    }

    pub fn to_file(&self, opts: &ConnectionOptions, path: PathBuf) {
        let registration_file = RegistrationFile {
            transport: opts.transport.clone(),
            signature_scheme: opts.signature_scheme.clone(),
            ip: opts.ip.clone(),
            key: opts.key.clone(),
            registration_port: self.registration.get_port().unwrap(),
        };

        registration_file.to_file(path).unwrap();
    }
}

pub struct Connection {
    pub _control: Control,
    pub shell: Shell,
    pub iopub: Iopub,
    pub stdin: Stdin,
    pub heartbeat: Heartbeat,
    // session: Session,
}

impl Connection {
    pub fn init_with_connection_file(
        mut kernel_cmd: std::process::Command,
        path: PathBuf,
    ) -> Self {
        log::info!("Starting kernel using connection file");

        let opts = ConnectionOptions::init();

        let mut connection_file = ConnectionFile::new();
        connection_file.key = opts.key.clone();
        connection_file.to_file(path).unwrap();

        let _ = kernel_cmd.spawn();

        // We need to give the kernel a chance to start up and read the connection file
        std::thread::sleep(std::time::Duration::from_millis(2000));

        let control_endpoint = connection_file.endpoint(connection_file.control_port);
        let shell_endpoint = connection_file.endpoint(connection_file.shell_port);
        let iopub_endpoint = connection_file.endpoint(connection_file.iopub_port);
        let stdin_endpoint = connection_file.endpoint(connection_file.stdin_port);
        let heartbeat_endpoint = connection_file.endpoint(connection_file.hb_port);

        Self {
            _control: Control::init(&opts, control_endpoint),
            shell: Shell::init(&opts, shell_endpoint),
            iopub: Iopub::init(&opts, iopub_endpoint),
            stdin: Stdin::init(&opts, stdin_endpoint),
            heartbeat: Heartbeat::init(&opts, heartbeat_endpoint),
            // session: opts.session,
        }
    }

    pub fn init_with_registration_file(
        mut kernel_cmd: std::process::Command,
        path: PathBuf,
    ) -> Self {
        log::info!("Starting kernel using registration file");

        let opts = ConnectionOptions::init();

        let sockets = RegistrationSockets::from(&opts);
        sockets.to_file(&opts, path.into());

        let _ = kernel_cmd.spawn();

        // Wait to receive the handshake request so we know what ports to connect on.
        // Note that `recv()` times out.
        let message = Message::read_from_socket(&sockets.registration).unwrap();
        let handshake = assert_matches!(message, Message::HandshakeRequest(message) => {
            message.content
        });

        let reply = HandshakeReply { status: Status::Ok };
        let reply_msg = JupyterMessage::create(reply, None, &opts.session);
        reply_msg.send(&sockets.registration).unwrap();

        Self {
            _control: Control::init(&opts, opts.endpoint(handshake.control_port)),
            shell: Shell::init(&opts, opts.endpoint(handshake.shell_port)),
            iopub: Iopub::init(&opts, opts.endpoint(handshake.iopub_port)),
            stdin: Stdin::init(&opts, opts.endpoint(handshake.stdin_port)),
            heartbeat: Heartbeat::init(&opts, opts.endpoint(handshake.hb_port)),
            // session: opts.session,
        }
    }
}

impl Default for ExecuteRequestOptions {
    fn default() -> Self {
        Self { allow_stdin: true }
    }
}

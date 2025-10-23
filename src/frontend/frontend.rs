/*
 * dummy_frontend.rs
 *
 * Copyright (C) 2022-2024 Posit Software, PBC. All rights reserved.
 *
 */

use std::path::PathBuf;

use assert_matches::assert_matches;
use rand::Rng;

use crate::frontend::{
    control::Control, heartbeat::Heartbeat, iopub::Iopub, shell::Shell, stdin::Stdin,
};
use crate::msg::connection_file::ConnectionFile;
use crate::msg::registration_file::RegistrationFile;
use crate::msg::session::Session;
use crate::msg::socket::Socket;
use crate::msg::wire::handshake_reply::HandshakeReply;
use crate::msg::wire::jupyter_message::Status;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message};
use crate::msg::wire::kernel_info_full_reply::KernelInfoReply;
use crate::msg::wire::kernel_info_request::KernelInfoRequest;
use crate::msg::wire::status::ExecutionState;

pub struct FrontendOptions {
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

impl FrontendOptions {
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
    pub fn from(opts: &FrontendOptions) -> Self {
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

    pub fn to_file(&self, opts: &FrontendOptions, path: PathBuf) {
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

pub struct Frontend {
    pub _control: Control,
    pub shell: Shell,
    pub iopub: Iopub,
    pub stdin: Stdin,
    pub heartbeat: Heartbeat,
    // session: Session,
}

impl Frontend {
    pub fn start_with_connection_file(
        mut kernel_cmd: std::process::Command,
        path: PathBuf,
    ) -> Self {
        log::info!("Starting kernel using connection file");

        let opts = FrontendOptions::init();

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

    pub fn start_with_registration_file(
        mut kernel_cmd: std::process::Command,
        path: PathBuf,
    ) -> Self {
        log::info!("Starting kernel using registration file");

        let opts = FrontendOptions::init();

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

    pub fn subscribe(&self) -> KernelInfoReply {
        // Not all kernels implement the XPUB socket which provides the welcome message which confirms
        // the connection is established. PEP 65 recommends dealing with this by:
        // 1. Sending a kernel info request
        // 2. Checking the protocol version in the reply
        // 3. Waiting for the welcome message if the protocol supports it
        //
        // Docs: https://jupyter.org/enhancement-proposals/65-jupyter-xpub/jupyter-xpub.html#impact-on-existing-implementations
        self.shell.send(KernelInfoRequest {});
        self.iopub.recv_busy();
        let reply = self.shell.recv();

        let kernel_info = match reply {
            Message::KernelInfoReply(reply) => reply.content,
            _ => panic!("Expected kernel_info_reply, but got {:#?}", reply),
        };

        log::info!(
            "Kernel is using protocol version: {}",
            kernel_info.protocol_version
        );

        // Receive the Welcome message for kernels which support it
        // Unfortunately, although JEP 65 is accepted, I can't find the version of the jupyter protocol
        // in which it becomes effective. Ark _does_ support it and is 5.4, ipython doesn't and is 5.3.
        if kernel_info.protocol_version >= String::from("5.4") {
            // Immediately block until we've received the IOPub welcome message from the XPUB server side
            // socket. This confirms that we've fully subscribed and avoids dropping any of the initial
            // IOPub messages that a server may send if we start to perform requests immediately (in
            // particular, busy/idle messages). https://github.com/posit-dev/ark/pull/577
            assert_matches!(self.iopub.recv(), Message::Welcome(data) => {
                assert_eq!(data.content.subscription, String::from(""));
            });
            // We also go ahead and handle the `ExecutionState::Starting` status that we know is coming
            // from the kernel right after the `Welcome` message.
            assert_matches!(self.iopub.recv(), Message::Status(data) => {
                assert_eq!(data.content.execution_state, ExecutionState::Starting);
            });
        }

        // Consume the Idle status
        self.iopub.recv_idle();

        kernel_info
    }
}

impl Default for ExecuteRequestOptions {
    fn default() -> Self {
        Self { allow_stdin: false }
    }
}

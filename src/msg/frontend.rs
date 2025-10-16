/*
 * dummy_frontend.rs
 *
 * Copyright (C) 2022-2024 Posit Software, PBC. All rights reserved.
 *
 */

use std::path::PathBuf;

use assert_matches::assert_matches;
use rand::Rng;
use serde_json::Value;

use crate::msg::connection_file::ConnectionFile;
use crate::msg::registration_file::RegistrationFile;
use crate::msg::session::Session;
use crate::msg::socket::Socket;
use crate::msg::wire::execute_input::ExecuteInput;
use crate::msg::wire::execute_request::ExecuteRequest;
use crate::msg::wire::handshake_reply::HandshakeReply;
use crate::msg::wire::input_reply::InputReply;
use crate::msg::wire::jupyter_message::JupyterMessage;
use crate::msg::wire::jupyter_message::Message;
use crate::msg::wire::jupyter_message::ProtocolMessage;
use crate::msg::wire::jupyter_message::Status;
use crate::msg::wire::status::ExecutionState;
use crate::msg::wire::stream::Stream;
use crate::msg::wire::wire_message::WireMessage;

pub struct FrontendOptions {
    pub ctx: zmq::Context,
    pub session: Session,
    pub key: String,
    pub ip: String,
    pub transport: String,
    pub signature_scheme: String,
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

        Self {
            ctx,
            session,
            key,
            ip,
            transport,
            signature_scheme,
        }
    }

    fn endpoint(&self, port: u16) -> String {
        format!("{}://{}:{}", &self.transport, &self.ip, port)
    }
}

pub struct FrontEndSockets {
    pub control: Socket,
    pub shell: Socket,
    pub iopub: Socket,
    pub stdin: Socket,
    pub heartbeat: Socket,
}

impl FrontEndSockets {
    pub fn from_endpoints(
        opts: &FrontendOptions,
        control_endpoint: String,
        shell_endpoint: String,
        iopub_endpoint: String,
        stdin_endpoint: String,
        heartbeat_endpoint: String,
    ) -> Self {
        let shell_id = rand::rng().random::<[u8; 16]>();

        let control = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Control"),
            zmq::DEALER,
            None,
            control_endpoint,
        )
        .unwrap();

        let shell = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Shell"),
            zmq::DEALER,
            Some(&shell_id),
            shell_endpoint,
        )
        .unwrap();

        let iopub = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("IOPub"),
            zmq::SUB,
            None,
            iopub_endpoint,
        )
        .unwrap();

        let stdin = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Stdin"),
            zmq::DEALER,
            Some(&shell_id),
            stdin_endpoint,
        )
        .unwrap();

        let heartbeat = Socket::new(
            opts.session.clone(),
            opts.ctx.clone(),
            String::from("Heartbeat"),
            zmq::REQ,
            None,
            heartbeat_endpoint,
        )
        .unwrap();

        Self {
            control,
            shell,
            iopub,
            stdin,
            heartbeat,
        }
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
    pub _control_socket: Socket,
    pub shell_socket: Socket,
    pub iopub_socket: Socket,
    pub stdin_socket: Socket,
    pub heartbeat_socket: Socket,
    session: Session,
}

impl Frontend {
    pub fn start_with_connection_file(
        path: PathBuf,
        mut kernel_cmd: std::process::Command,
    ) -> Self {
        let opts = FrontendOptions::init();

        let mut connection_file = ConnectionFile::new();
        connection_file.key = opts.key.clone();
        connection_file.to_file(path).unwrap();

        let _ = kernel_cmd.spawn();

        // We need to give the kernel a chance to start up and read the connection file
        std::thread::sleep(std::time::Duration::from_millis(2000));

        let sockets = FrontEndSockets::from_endpoints(
            &opts,
            connection_file.endpoint(connection_file.control_port),
            connection_file.endpoint(connection_file.shell_port),
            connection_file.endpoint(connection_file.iopub_port),
            connection_file.endpoint(connection_file.stdin_port),
            connection_file.endpoint(connection_file.hb_port),
        );

        // // Immediately block until we've received the IOPub welcome message from the XPUB
        // // server side socket. This confirms that we've fully subscribed and avoids
        // // dropping any of the initial IOPub messages that a server may send if we start
        // // to perform requests immediately (in particular, busy/idle messages).
        // // https://github.com/posit-dev/ark/pull/577
        // assert_matches!(Self::recv(&sockets.iopub), Message::Welcome(data) => {
        //     assert_eq!(data.content.subscription, String::from(""));
        // });
        // // We also go ahead and handle the `ExecutionState::Starting` status that we know
        // // is coming from the kernel right after the `Welcome` message, so tests don't
        // // have to care about this.
        // assert_matches!(Self::recv(&sockets.iopub), Message::Status(data) => {
        //     assert_eq!(data.content.execution_state, ExecutionState::Starting);
        // });

        Self {
            _control_socket: sockets.control,
            shell_socket: sockets.shell,
            iopub_socket: sockets.iopub,
            stdin_socket: sockets.stdin,
            heartbeat_socket: sockets.heartbeat,
            session: opts.session,
        }
    }

    pub fn start_with_registration_file(
        path: PathBuf,
        mut kernel_cmd: std::process::Command,
    ) -> Self {
        let opts = FrontendOptions::init();

        let sockets = RegistrationSockets::from(&opts);
        sockets.to_file(&opts, path.into());

        let _ = kernel_cmd.spawn();

        // Wait to receive the handshake request so we know what ports to connect on.
        // Note that `recv()` times out.
        let message = Self::recv(&sockets.registration);
        let handshake = assert_matches!(message, Message::HandshakeRequest(message) => {
            message.content
        });

        // Immediately send back a handshake reply so the kernel can start up
        Self::send(
            &sockets.registration,
            &opts.session,
            HandshakeReply { status: Status::Ok },
        );

        let sockets = FrontEndSockets::from_endpoints(
            &opts,
            opts.endpoint(handshake.control_port),
            opts.endpoint(handshake.shell_port),
            opts.endpoint(handshake.iopub_port),
            opts.endpoint(handshake.stdin_port),
            opts.endpoint(handshake.hb_port),
        );

        // // Immediately block until we've received the IOPub welcome message from the XPUB
        // // server side socket. This confirms that we've fully subscribed and avoids
        // // dropping any of the initial IOPub messages that a server may send if we start
        // // to perform requests immediately (in particular, busy/idle messages).
        // // https://github.com/posit-dev/ark/pull/577
        // assert_matches!(Self::recv(&sockets.iopub), Message::Welcome(data) => {
        //     assert_eq!(data.content.subscription, String::from(""));
        // });
        // // We also go ahead and handle the `ExecutionState::Starting` status that we know
        // // is coming from the kernel right after the `Welcome` message, so tests don't
        // // have to care about this.
        // assert_matches!(Self::recv(&sockets.iopub), Message::Status(data) => {
        //     assert_eq!(data.content.execution_state, ExecutionState::Starting);
        // });

        Self {
            _control_socket: sockets.control,
            shell_socket: sockets.shell,
            iopub_socket: sockets.iopub,
            stdin_socket: sockets.stdin,
            heartbeat_socket: sockets.heartbeat,
            session: opts.session,
        }
    }

    /// Sends a Jupyter message on the Shell socket; returns the ID of the newly
    /// created message
    pub fn send_shell<T: ProtocolMessage>(&self, msg: T) -> String {
        Self::send(&self.shell_socket, &self.session, msg)
    }

    pub fn send_execute_request(&self, code: &str, options: ExecuteRequestOptions) -> String {
        self.send_shell(ExecuteRequest {
            code: String::from(code),
            silent: false,
            store_history: true,
            user_expressions: serde_json::Value::Null,
            allow_stdin: options.allow_stdin,
            stop_on_error: false,
        })
    }

    /// Sends a Jupyter message on the Stdin socket
    pub fn send_stdin<T: ProtocolMessage>(&self, msg: T) {
        Self::send(&self.stdin_socket, &self.session, msg);
    }

    fn send<T: ProtocolMessage>(socket: &Socket, session: &Session, msg: T) -> String {
        let message = JupyterMessage::create(msg, None, session);
        let id = message.header.msg_id.clone();
        message.send(socket).unwrap();
        id
    }

    pub fn recv(socket: &Socket) -> Message {
        // It's important to wait with a timeout because the kernel thread might have
        // panicked, preventing it from sending the expected message. The tests would then
        // hang indefinitely. We wait a decently long time (10s), as test processes are
        // run in parallel and we think they seem to slow each other down occasionally
        // (we've definitely seen false positive failures with a timeout of just 1s,
        // particularly when running with nextest).
        //
        // Note that the panic hook will still have run to record the panic, so we'll get
        // expected panic information in the test output.
        if socket.poll_incoming(10000).unwrap() {
            return Message::read_from_socket(socket).unwrap();
        }

        panic!("Timeout while expecting message on socket {}", socket.name);
    }

    /// Receives a Jupyter message from the Shell socket
    pub fn recv_shell(&self) -> Message {
        Self::recv(&self.shell_socket)
    }

    /// Receives a Jupyter message from the IOPub socket
    pub fn recv_iopub(&self) -> Message {
        Self::recv(&self.iopub_socket)
    }

    /// Receives a Jupyter message from the Stdin socket
    pub fn recv_stdin(&self) -> Message {
        Self::recv(&self.stdin_socket)
    }

    /// Receive from Shell and assert `ExecuteReply` message.
    /// Returns `execution_count`.
    pub fn recv_shell_execute_reply(&self) -> u32 {
        let msg = self.recv_shell();

        assert_matches!(msg, Message::ExecuteReply(data) => {
            assert_eq!(data.content.status, Status::Ok);
            data.content.execution_count
        })
    }

    /// Receive from Shell and assert `ExecuteReplyException` message.
    /// Returns `execution_count`.
    pub fn recv_shell_execute_reply_exception(&self) -> u32 {
        let msg = self.recv_shell();

        assert_matches!(msg, Message::ExecuteReplyException(data) => {
            assert_eq!(data.content.status, Status::Error);
            data.content.execution_count
        })
    }

    /// Receive from IOPub and assert Busy message
    pub fn recv_iopub_busy(&self) -> () {
        let msg = self.recv_iopub();

        assert_matches!(msg, Message::Status(data) => {
            assert_eq!(data.content.execution_state, ExecutionState::Busy);
        });
    }

    /// Receive from IOPub and assert Idle message
    pub fn recv_iopub_idle(&self) -> () {
        let msg = self.recv_iopub();

        assert_matches!(msg, Message::Status(data) => {
            assert_eq!(data.content.execution_state, ExecutionState::Idle);
        });
    }

    /// Receive from IOPub and assert ExecuteInput message
    pub fn recv_iopub_execute_input(&self) -> ExecuteInput {
        let msg = self.recv_iopub();

        assert_matches!(msg, Message::ExecuteInput(data) => {
            data.content
        })
    }

    /// Receive from IOPub and assert ExecuteResult message. Returns compulsory
    /// `plain/text` result.
    pub fn recv_iopub_execute_result(&self) -> String {
        let msg = self.recv_iopub();

        assert_matches!(msg, Message::ExecuteResult(data) => {
            assert_matches!(data.content.data, Value::Object(map) => {
                assert_matches!(map["text/plain"], Value::String(ref string) => {
                    string.clone()
                })
            })
        })
    }

    pub fn recv_iopub_display_data(&self) {
        let msg = self.recv_iopub();
        assert_matches!(msg, Message::DisplayData(_))
    }

    pub fn recv_iopub_update_display_data(&self) {
        let msg = self.recv_iopub();
        assert_matches!(msg, Message::UpdateDisplayData(_))
    }

    pub fn recv_iopub_stream_stdout(&self, expect: &str) {
        self.recv_iopub_stream(expect, Stream::Stdout)
    }

    pub fn recv_iopub_stream_stderr(&self, expect: &str) {
        self.recv_iopub_stream(expect, Stream::Stderr)
    }

    pub fn recv_iopub_comm_close(&self) -> String {
        let msg = self.recv_iopub();

        assert_matches!(msg, Message::CommClose(data) => {
            data.content.comm_id
        })
    }

    /// Receive from IOPub Stream
    ///
    /// Stdout and Stderr Stream messages are buffered, so to reliably test against them
    /// we have to collect the messages in batches on the receiving end and compare against
    /// an expected message.
    fn recv_iopub_stream(&self, expect: &str, stream: Stream) {
        let mut out = String::new();

        loop {
            // Receive a piece of stream output (with a timeout)
            let msg = self.recv_iopub();

            // Assert its type
            let piece = assert_matches!(msg, Message::Stream(data) => {
                assert_eq!(data.content.name, stream);
                data.content.text
            });

            // Add to what we've already collected
            out += piece.as_str();

            if out == expect {
                // Done, found the entire `expect` string
                return;
            }

            if !expect.starts_with(out.as_str()) {
                // Something is wrong, message doesn't match up
                panic!("Expected IOPub stream of '{expect}'. Actual stream of '{out}'.");
            }

            // We have a prefix of `expect`, but not the whole message yet.
            // Wait on the next IOPub Stream message.
        }
    }

    /// Receive from IOPub and assert ExecuteResult message. Returns compulsory
    /// `evalue` field.
    pub fn recv_iopub_execute_error(&self) -> String {
        let msg = self.recv_iopub();

        assert_matches!(msg, Message::ExecuteError(data) => {
            data.content.exception.evalue
        })
    }

    /// Receive from Stdin and assert `InputRequest` message.
    /// Returns the `prompt`.
    pub fn recv_stdin_input_request(&self) -> String {
        let msg = self.recv_stdin();

        assert_matches!(msg, Message::InputRequest(data) => {
            data.content.prompt
        })
    }

    /// Send back an `InputReply` to an `InputRequest` over Stdin
    pub fn send_stdin_input_reply(&self, value: String) {
        self.send_stdin(InputReply { value })
    }

    /// Receives a (raw) message from the heartbeat socket
    pub fn recv_heartbeat(&self) -> zmq::Message {
        let mut msg = zmq::Message::new();
        self.heartbeat_socket.recv(&mut msg).unwrap();
        msg
    }

    /// Sends a (raw) message to the heartbeat socket
    pub fn send_heartbeat(&self, msg: zmq::Message) {
        self.heartbeat_socket.send(msg).unwrap();
    }

    /// Asserts that no socket has incoming data
    pub fn assert_no_incoming(&mut self) {
        let mut has_incoming = false;

        if self.iopub_socket.has_incoming_data().unwrap() {
            has_incoming = true;
            Self::flush_incoming("IOPub", &self.iopub_socket);
        }
        if self.shell_socket.has_incoming_data().unwrap() {
            has_incoming = true;
            Self::flush_incoming("Shell", &self.shell_socket);
        }
        if self.stdin_socket.has_incoming_data().unwrap() {
            has_incoming = true;
            Self::flush_incoming("StdIn", &self.stdin_socket);
        }
        if self.heartbeat_socket.has_incoming_data().unwrap() {
            has_incoming = true;
            Self::flush_incoming("Heartbeat", &self.heartbeat_socket);
        }

        if has_incoming {
            panic!("Sockets must be empty on exit (see details above)");
        }
    }

    fn flush_incoming(name: &str, socket: &Socket) {
        eprintln!("{name} has incoming data:");

        while socket.has_incoming_data().unwrap() {
            dbg!(WireMessage::read_from_socket(socket).unwrap());
            eprintln!("---");
        }
    }
}

impl Default for ExecuteRequestOptions {
    fn default() -> Self {
        Self { allow_stdin: false }
    }
}

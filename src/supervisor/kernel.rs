use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use assert_matches::assert_matches;
use serde::{Deserialize, Serialize};

use crate::connection::connection::JupyterChannels;
use crate::connection::control::Control;
use crate::connection::shell::Shell;
use crate::connection::stdin::Stdin;
use crate::kernel::kernel_spec::KernelSpec;
use crate::kernel::startup_method::ConnectionMethod;
use crate::msg::session::Session;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage};
use crate::msg::wire::kernel_info_reply::KernelInfoReply;
use crate::msg::wire::kernel_info_request::KernelInfoRequest;
use crate::msg::wire::language_info::LanguageInfo;
use crate::msg::wire::message_id::Id;
use crate::msg::wire::status::ExecutionState;
use crate::supervisor::broker::Broker;
use crate::supervisor::listeners::{listen_heartbeat, listen_iopub};

pub struct Kernel {
    pub id: Id,
    pub info: KernelInfo,
    pub comm: KernelComm,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfo {
    /// The path to the kernel's spec file
    pub spec_path: String,
    /// The spec file's `display_name`
    pub display_name: String,
    /// The banner given by the kernel's `KernelInfoReply`
    pub banner: String,
    /// The language info given by the kernel's `KernelInfoReply`
    pub language: LanguageInfo,
}

/// These are the channels on which we might want to send data (as well as receive)
pub struct KernelComm {
    /// The session used to communicate with the kernel
    pub session: Session,
    /// The broker which routes messages received on the iopub channel
    pub iopub_broker: Arc<Broker>,
    /// The shell channel; used to send generl requests and receive replies
    pub shell_channel: Mutex<Shell>,
    /// The broker which routes messages received on the shell channel
    pub shell_broker: Arc<Broker>,
    /// The stdin channel; used to receive requests from the kernel and send responses
    pub stdin_channel: Mutex<Stdin>,
    /// The broker which routes messages received on the stdin channel
    pub stdin_broker: Arc<Broker>,
    /// The broker which routes messages received on the control channel
    pub control_channel: Mutex<Control>,
    /// The control channel; used to request shutdowns and receive replies.
    pub control_broker: Arc<Broker>,
}

impl KernelComm {
    pub fn send_shell<T: ProtocolMessage>(&self, msg: T) -> ReplyReceivers {
        let (msg, request_id) = self.make_jupyter_message(msg);
        let receivers = self.register_request(&request_id);
        self.shell_channel.lock().unwrap().send(msg);
        receivers
    }

    pub fn send_stdin<T: ProtocolMessage>(&self, msg: T) -> ReplyReceivers {
        let (msg, request_id) = self.make_jupyter_message(msg);
        let receivers = self.register_request(&request_id);
        self.stdin_channel.lock().unwrap().send(msg);
        receivers
    }

    pub fn send_control<T: ProtocolMessage>(&self, msg: T) -> ReplyReceivers {
        let (msg, request_id) = self.make_jupyter_message(msg);
        let receivers = self.register_request(&request_id);
        self.control_channel.lock().unwrap().send(msg);
        receivers
    }

    fn make_jupyter_message<T: ProtocolMessage>(&self, msg: T) -> (JupyterMessage<T>, Id) {
        let message = JupyterMessage::create(msg, None, &self.session);
        let id = message.header.msg_id.clone();
        (message, id)
    }

    fn register_request(&self, request_id: &Id) -> ReplyReceivers {
        ReplyReceivers {
            id: request_id.clone(),
            iopub: self.iopub_broker.register_request(&request_id),
            stdin: self.stdin_broker.register_request(&request_id),
            shell: self.shell_broker.register_request(&request_id),
            control: self.control_broker.register_request(&request_id),
        }
    }

    pub fn recv_shell(&self) -> Message {
        self.shell_channel.lock().unwrap().recv()
    }

    pub fn recv_stdin(&self) -> Message {
        self.stdin_channel.lock().unwrap().recv()
    }

    pub fn recv_control(&self) -> Message {
        self.control_channel.lock().unwrap().recv()
    }

    pub fn await_reply_shell(&self, request_id: &Id) {
        loop {
            let msg = self.recv_shell();
            let is_reply = msg.parent_id().unwrap_or(Id::unparented()) == *request_id;
            self.shell_broker.route(msg);
            if is_reply {
                break;
            }
        }
    }

    pub fn await_reply_stdin(&self, request_id: &Id) {
        loop {
            let msg = self.recv_stdin();
            let is_reply = msg.parent_id().unwrap_or(Id::unparented()) == *request_id;
            self.stdin_broker.route(msg);
            if is_reply {
                break;
            }
        }
    }

    pub fn await_reply_control(&self, request_id: &Id) {
        loop {
            let msg = self.recv_control();
            let is_reply = msg.parent_id().unwrap_or(Id::unparented()) == *request_id;
            self.control_broker.route(msg);
            if is_reply {
                break;
            }
        }
    }

    pub fn route_all_incoming_shell(&self) {
        while let Ok(msg) = self.shell_channel.lock().unwrap().try_recv() {
            self.shell_broker.route(msg);
        }
    }

    pub fn route_all_incoming_stdin(&self) {
        while let Ok(msg) = self.stdin_channel.lock().unwrap().try_recv() {
            self.stdin_broker.route(msg);
        }
    }

    pub fn route_all_incoming_control(&self) {
        while let Ok(msg) = self.control_channel.lock().unwrap().try_recv() {
            self.control_broker.route(msg);
        }
    }

    fn subscribe(&self) -> KernelInfoReply {
        // When kernels up they often send a welcome message with no parent ID.
        let welcome_rx = self.iopub_broker.register_request(&Id::unparented());
        log::info!("Sending kernel info request for subscription");

        let receivers = self.send_shell(KernelInfoRequest {});
        self.await_reply_shell(&receivers.id);

        let reply = receivers.shell.recv().unwrap();
        log::info!("Received reply on the shell");

        let kernel_info = match reply {
            Message::KernelInfoReply(reply) => reply.content,
            _ => panic!("Expected kernel_info_reply but got {:#?}", reply),
        };

        log::info!("Kernel info reply: {:#?}", kernel_info);

        if let Some(version) = &kernel_info.protocol_version {
            if version >= &String::from("5.4") {
                assert_matches!(welcome_rx.recv().unwrap(), Message::Welcome(data) => {
                    assert_eq!(data.content.subscription, String::from(""));
                    log::info!("Received the welcome message from the kernel");
                });
                assert_matches!(welcome_rx.recv().unwrap(), Message::Status(data) => {
                    assert_eq!(data.content.execution_state, ExecutionState::Starting);
                    log::info!("Received the starting message from the kernel");
                });
            }
        }

        self.iopub_broker
            .unregister_request(&Id::unparented(), "all expected startup messages received");

        log::info!("Subscription complete");

        kernel_info
    }
}

/// When you send a request on stdin, any replies which come back from the kernel will be routed
/// via these sockets. This allows you to handle replies _only_ related to the original request,
/// without worrying about dropping any unrelated messages.
pub struct ReplyReceivers {
    /// The ID of the original request message
    pub id: Id,
    /// A receiver for replies to `id` on the iopub socket
    pub iopub: Receiver<Message>,
    /// A receiver for replies to `id` on the shell socket
    pub shell: Receiver<Message>,
    /// A receiver for replies to `id` on the stdin socket
    pub stdin: Receiver<Message>,
    /// A receiver for replies to `id` on the control socket
    pub control: Receiver<Message>,
}

impl Kernel {
    pub fn start(spec_path: String, spec: KernelSpec) -> Self {
        log::info!("Using kernel '{}'", spec.display_name);

        let kernel_id = Id::new();
        let connection_file_path = format!(
            ".connection_files/carpo_connection_file_{}.json",
            String::from(kernel_id.clone())
        );
        let kernel_cmd = spec.build_command(&connection_file_path);

        let jupyter_channels = match spec.get_connection_method() {
            ConnectionMethod::RegistrationFile => JupyterChannels::init_with_registration_file(
                kernel_cmd,
                connection_file_path.into(),
            ),
            ConnectionMethod::ConnectionFile => {
                JupyterChannels::init_with_connection_file(kernel_cmd, connection_file_path.into())
            }
        };

        let kernel_comm = KernelComm {
            session: jupyter_channels.session.clone(),
            shell_channel: Mutex::new(jupyter_channels.shell),
            stdin_channel: Mutex::new(jupyter_channels.stdin),
            control_channel: Mutex::new(jupyter_channels.control),
            iopub_broker: Arc::new(Broker::new(format!("IOPub{}", kernel_id))),
            shell_broker: Arc::new(Broker::new(format!("Shell{}", kernel_id))),
            stdin_broker: Arc::new(Broker::new(format!("Stdin{}", kernel_id))),
            control_broker: Arc::new(Broker::new(format!("Control{}", kernel_id))),
        };

        listen_heartbeat(jupyter_channels.heartbeat);
        listen_iopub(
            jupyter_channels.iopub,
            Arc::clone(&kernel_comm.iopub_broker),
        );

        let kernel_info_reply = kernel_comm.subscribe();

        Self {
            id: kernel_id,
            comm: kernel_comm,
            info: KernelInfo {
                spec_path: spec_path,
                display_name: spec.display_name,
                banner: kernel_info_reply.banner,
                language: kernel_info_reply.language_info,
            },
        }
    }
}


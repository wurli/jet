use crate::connection::control::Control;
use crate::connection::shell::Shell;
use crate::connection::stdin::Stdin;
use crate::error::Error;
use crate::msg::session::Session;
use crate::msg::wire::jupyter_message::{JupyterMessage, Message, ProtocolMessage};
use crate::msg::wire::kernel_info_reply::KernelInfoReply;
use crate::msg::wire::kernel_info_request::KernelInfoRequest;
use crate::msg::wire::message_id::Id;
use crate::msg::wire::shutdown_request::ShutdownRequest;
use crate::msg::wire::status::ExecutionState;
use crate::supervisor::broker::Broker;
use crate::supervisor::reply_receivers::ReplyReceivers;
use assert_matches::assert_matches;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// These are the channels on which we might want to send data (as well as receive)
pub struct KernelComm {
    /// The session used to communicate with the kernel
    pub session: Session,

    /// A sender which can be used to stop the heartbeat loop
    pub heartbeat_tx: Sender<()>,

    /// A sender which can be used to stop the iopub loop
    pub iopub_tx: Sender<()>,

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
    pub fn stop_heartbeat(&self) -> Result<(), Error> {
        match self.heartbeat_tx.send(()) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::CannotStopThread(String::from("heartbeat"))),
        }
    }

    pub fn stop_iopub(&self) -> Result<(), Error> {
        match self.iopub_tx.send(()) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::CannotStopThread(String::from("iopub"))),
        }
    }

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
            iopub: self.iopub_broker.register_request(request_id),
            stdin: self.stdin_broker.register_request(request_id),
            shell: self.shell_broker.register_request(request_id),
            control: self.control_broker.register_request(request_id),
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

    pub fn is_request_active(&self, request_id: &Id) -> bool {
        self.is_request_active_shell(request_id)
            | self.is_request_active_stdin(request_id)
            | self.is_request_active_control(request_id)
    }

    pub fn is_request_active_shell(&self, request_id: &Id) -> bool {
        self.shell_broker.is_active(request_id)
    }

    pub fn is_request_active_stdin(&self, request_id: &Id) -> bool {
        self.stdin_broker.is_active(request_id)
    }

    pub fn is_request_active_control(&self, request_id: &Id) -> bool {
        self.control_broker.is_active(request_id)
    }

    pub fn subscribe(&self) -> KernelInfoReply {
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

        if let Some(version) = &kernel_info.protocol_version
            && version >= &String::from("5.4") {
                assert_matches!(welcome_rx.recv().unwrap(), Message::Welcome(data) => {
                    assert_eq!(data.content.subscription, String::from(""));
                    log::info!("Received the welcome message from the kernel");
                });
                assert_matches!(welcome_rx.recv().unwrap(), Message::Status(data) => {
                    assert_eq!(data.content.execution_state, ExecutionState::Starting);
                    log::info!("Received the starting message from the kernel");
                });
            }

        self.iopub_broker
            .unregister_request(&Id::unparented(), "all expected startup messages received");

        log::info!("Subscription complete");

        kernel_info
    }

    pub fn request_shutdown(&self) -> anyhow::Result<Message> {
        let res = self.request_shutdown_impl(false);
        self.stop_heartbeat()?;
        self.stop_iopub()?;
        res
    }

    pub fn request_restart(&self) -> anyhow::Result<Message> {
        self.request_shutdown_impl(true)
    }

    fn request_shutdown_impl(&self, restart: bool) -> anyhow::Result<Message> {
        self.route_all_incoming_shell();
        let receivers = self.send_control(ShutdownRequest { restart });

        loop {
            while let Ok(reply) = receivers.iopub.try_recv() {
                match reply {
                    Message::ShutdownReply(_) => {
                        log::info!("Received shutdown_reply on iopub (non-standard)");
                        return Ok(reply);
                    }
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                    }
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                        break;
                    }
                    _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
                }
            }

            self.route_all_incoming_stdin();

            if let Ok(reply) = receivers.stdin.try_recv() {
                match reply {
                    Message::InputRequest(_) => return Ok(reply),
                    other => log::warn!("Received unexpected reply {}", other.describe()),
                }
            };

            self.route_all_incoming_control();

            if let Ok(reply) = receivers.control.try_recv() {
                match reply {
                    Message::ShutdownReply(_) => {
                        log::info!("Received shutdown_reply on control (standard)");
                        self.control_broker
                            .unregister_request(&receivers.id, "reply received");
                        return Ok(reply);
                    }
                    other => {
                        log::warn!(
                            "Expected shutdown_reply but received unexpected message: {:#?}",
                            other
                        );
                        return Err(anyhow::anyhow!(
                            "Expected shutdown_reply but received unexpected message: {:#?}",
                            other
                        ));
                    }
                }
            }
        }
    }
}

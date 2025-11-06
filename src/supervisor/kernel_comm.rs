/*
 * kernel_comm.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

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
use crate::supervisor::kernel::{HeartbeatFailed, StopHeartbeat, StopIopub};
use crate::supervisor::reply_receivers::ReplyReceivers;
use assert_matches::assert_matches;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

/// These are the channels on which we might want to send data (as well as receive)
pub struct KernelComm {
    /// The session used to communicate with the kernel
    pub session: Session,

    /// A sender which can be used to stop the heartbeat loop
    pub heartbeat_stopper: Sender<StopHeartbeat>,

    /// A receiver which notifies when the heartbeat has failed
    pub heartbeat_monitor: Mutex<Receiver<HeartbeatFailed>>,

    /// A sender which can be used to stop the iopub loop
    pub iopub_stopper: Sender<StopIopub>,

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
        match self.heartbeat_stopper.send(StopHeartbeat) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::CannotStopThread(String::from("heartbeat"))),
        }
    }

    pub fn stop_iopub(&self) -> Result<(), Error> {
        match self.iopub_stopper.send(StopIopub) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::CannotStopThread(String::from("iopub"))),
        }
    }

    pub fn send_shell<T: ProtocolMessage>(&self, msg: T) -> Result<ReplyReceivers, Error> {
        let msg = self.make_jupyter_message(msg);
        let request_id = &msg.header.msg_id;
        let receivers = self.register_request(request_id)?;
        self.shell_channel.lock().unwrap().send(msg);
        Ok(receivers)
    }

    pub fn send_stdin<T: ProtocolMessage>(&self, msg: T) -> Result<ReplyReceivers, Error> {
        let msg = self.make_jupyter_message(msg);
        let receivers = self.register_request(&msg.header.msg_id)?;
        self.stdin_channel.lock().unwrap().send(msg);
        Ok(receivers)
    }

    pub fn send_control<T: ProtocolMessage>(&self, msg: T) -> Result<ReplyReceivers, Error> {
        let msg = self.make_jupyter_message(msg);
        let receivers = self.register_request(&msg.header.msg_id)?;
        self.control_channel.lock().unwrap().send(msg);
        Ok(receivers)
    }

    fn make_jupyter_message<T: ProtocolMessage>(&self, msg: T) -> JupyterMessage<T> {
        JupyterMessage::create(msg, None, &self.session)
    }

    fn register_request(&self, request_id: &Id) -> Result<ReplyReceivers, Error> {
        self.check_heartbeat()?;
        Ok(ReplyReceivers {
            id: request_id.clone(),
            iopub: self.iopub_broker.register_request(request_id),
            stdin: self.stdin_broker.register_request(request_id),
            shell: self.shell_broker.register_request(request_id),
            control: self.control_broker.register_request(request_id),
        })
    }

    pub fn unregister_request(&self, request_id: &Id, reason: &str) {
        // We don't unregister from iopub here since that is done automatically when we receive an
        // idle status
        self.stdin_broker.unregister_request(request_id, reason);
        self.shell_broker.unregister_request(request_id, reason);
        self.control_broker.unregister_request(request_id, reason);
    }

    fn check_heartbeat(&self) -> Result<(), Error> {
        match self.heartbeat_monitor.lock().unwrap().try_recv() {
            Ok(HeartbeatFailed) => Err(Error::HeartbeatFailed(String::from(
                "Heartbeat monitor reported failure",
            ))),
            Err(TryRecvError::Disconnected) => Err(Error::HeartbeatFailed(String::from(
                "Heartbeat monitor disconnected",
            ))),
            Err(TryRecvError::Empty) => Ok(()),
        }
    }

    pub fn recv_shell(&self) -> Result<Message, Error> {
        self.check_heartbeat()?;
        Ok(self.shell_channel.lock().unwrap().recv())
    }

    pub fn recv_stdin(&self) -> Result<Message, Error> {
        self.check_heartbeat()?;
        Ok(self.stdin_channel.lock().unwrap().recv())
    }

    pub fn recv_control(&self) -> Result<Message, Error> {
        self.check_heartbeat()?;
        Ok(self.control_channel.lock().unwrap().recv())
    }

    pub fn try_recv_shell(&self) -> Result<Message, Error> {
        self.check_heartbeat()?;
        self.shell_channel.lock().unwrap().try_recv()
    }

    pub fn try_recv_stdin(&self) -> Result<Message, Error> {
        self.check_heartbeat()?;
        self.stdin_channel.lock().unwrap().try_recv()
    }

    pub fn try_recv_control(&self) -> Result<Message, Error> {
        self.check_heartbeat()?;
        self.control_channel.lock().unwrap().try_recv()
    }

    pub fn await_reply_shell(&self, request_id: &Id) -> Result<(), Error> {
        loop {
            let msg = self.recv_shell()?;
            let is_reply = msg.parent_id().unwrap_or(&Id::unparented()) == request_id;
            self.shell_broker.route(msg);
            if is_reply {
                break;
            }
        }
        Ok(())
    }

    pub fn await_reply_stdin(&self, request_id: &Id) -> Result<(), Error> {
        loop {
            let msg = self.recv_stdin()?;
            let is_reply = msg.parent_id().unwrap_or(&Id::unparented()) == request_id;
            self.stdin_broker.route(msg);
            if is_reply {
                break;
            }
        }
        Ok(())
    }

    pub fn await_reply_control(&self, request_id: &Id) -> Result<(), Error> {
        loop {
            let msg = self.recv_control()?;
            let is_reply = msg.parent_id().unwrap_or(&Id::unparented()) == request_id;
            self.control_broker.route(msg);
            if is_reply {
                break;
            }
        }
        Ok(())
    }

    pub fn route_all_incoming_shell(&self) {
        while let Ok(msg) = self.try_recv_shell() {
            self.shell_broker.route(msg);
        }
    }

    pub fn route_all_incoming_stdin(&self) {
        while let Ok(msg) = self.try_recv_stdin() {
            self.stdin_broker.route(msg);
        }
    }

    pub fn route_all_incoming_control(&self) {
        while let Ok(msg) = self.try_recv_control() {
            self.control_broker.route(msg);
        }
    }

    /// Check if a request is still active on any of the input channels
    ///
    /// We don't check the iopub channel since requests on iopub are automatically unregistered
    /// when we receive an idle status.
    pub fn is_request_active(&self, request_id: &Id) -> bool {
        self.is_request_active_shell(request_id)
            | self.is_request_active_stdin(request_id)
            | self.is_request_active_control(request_id)
            | self.is_request_active_iopub(request_id)
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

    pub fn is_request_active_iopub(&self, request_id: &Id) -> bool {
        self.iopub_broker.is_active(request_id)
    }

    pub fn subscribe(&self) -> Result<KernelInfoReply, Error> {
        // When kernels up they often send a welcome message with no parent ID.
        let welcome_rx = self.iopub_broker.register_request(&Id::unparented());
        log::info!("Sending kernel info request for subscription");

        let receivers = self.send_shell(KernelInfoRequest {})?;
        self.await_reply_shell(&receivers.id)?;

        let reply = receivers.shell.recv().unwrap();
        log::info!("Received reply on the shell");

        let kernel_info = match reply {
            Message::KernelInfoReply(reply) => reply.content,
            _ => panic!("Expected kernel_info_reply but got {:#?}", reply),
        };

        log::info!("Kernel info reply: {:#?}", kernel_info);

        if let Some(version) = &kernel_info.protocol_version
            && version >= &String::from("5.4")
        {
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

        Ok(kernel_info)
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
        let receivers = self.send_control(ShutdownRequest { restart })?;

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

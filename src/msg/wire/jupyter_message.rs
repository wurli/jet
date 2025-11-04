/*
 * jupyter_message.rs
 *
 * Copyright (C) 2022 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;

use super::display_data::DisplayData;
use super::handshake_reply::HandshakeReply;
use super::handshake_request::HandshakeRequest;
use super::kernel_info_reply::KernelInfoReply;
use super::stream::StreamOutput;
use super::update_display_data::UpdateDisplayData;
use super::welcome::Welcome;
// use crate::comm::base_comm::JsonRpcReply;
// use crate::comm::ui_comm::UiFrontendRequest;
use crate::error::Error;
use crate::msg::session::Session;
use crate::msg::socket::Socket;
use crate::msg::wire::comm_close::CommClose;
use crate::msg::wire::comm_info_reply::CommInfoReply;
use crate::msg::wire::comm_info_request::CommInfoRequest;
use crate::msg::wire::comm_msg::CommWireMsg;
use crate::msg::wire::comm_open::CommOpen;
use crate::msg::wire::complete_reply::CompleteReply;
use crate::msg::wire::complete_request::CompleteRequest;
// use crate::msg::wire::error_reply::ErrorReply;
// use crate::msg::wire::exception::Exception;
use crate::msg::wire::execute_error::ExecuteError;
use crate::msg::wire::execute_input::ExecuteInput;
use crate::msg::wire::execute_reply::ExecuteReply;
use crate::msg::wire::execute_reply_exception::ExecuteReplyException;
use crate::msg::wire::execute_request::ExecuteRequest;
use crate::msg::wire::execute_result::ExecuteResult;
use crate::msg::wire::header::JupyterHeader;
use crate::msg::wire::input_reply::InputReply;
use crate::msg::wire::input_request::InputRequest;
use crate::msg::wire::inspect_reply::InspectReply;
use crate::msg::wire::inspect_request::InspectRequest;
use crate::msg::wire::interrupt_reply::InterruptReply;
use crate::msg::wire::interrupt_request::InterruptRequest;
use crate::msg::wire::is_complete_reply::IsCompleteReply;
use crate::msg::wire::is_complete_request::IsCompleteRequest;
use crate::msg::wire::kernel_info_request::KernelInfoRequest;
use crate::msg::wire::message_id::Id;
use crate::msg::wire::shutdown_reply::ShutdownReply;
// use crate::msg::wire::originator::Originator;
use crate::msg::wire::shutdown_request::ShutdownRequest;
use crate::msg::wire::status::KernelStatus;
use crate::msg::wire::wire_message::WireMessage;

/// Represents a Jupyter message
#[derive(Debug, Clone)]
pub struct JupyterMessage<T> {
    /// The ZeroMQ identities (for ROUTER sockets)
    pub zmq_identities: Vec<Vec<u8>>,

    /// The header for this message
    pub header: JupyterHeader,

    /// The header of the message from which this message originated. Optional;
    /// not all messages have a parent.
    pub parent_header: Option<JupyterHeader>,

    /// The body (payload) of the message
    pub content: T,
}

/// Trait used to extract the wire message type from a Jupyter message
pub trait Describe {
    /// The type of message
    fn message_type() -> String;
    /// Will always give the `message_type()`
    fn kind(&self) -> String {
        Self::message_type()
    }
    /// Any additional information about the message we want to display in logs
    fn info(&self) -> Option<String> {
        None
    }
}

/// Convenience trait for grouping traits that must be present on all Jupyter
/// protocol messages
pub trait ProtocolMessage: Describe + Serialize + std::fmt::Debug + Clone {}
impl<T> ProtocolMessage for T where T: Describe + Serialize + std::fmt::Debug + Clone {}

/// List of all known/implemented messages
#[derive(Debug, Clone)]
pub enum Message {
    // Shell
    KernelInfoReply(JupyterMessage<KernelInfoReply>),
    KernelInfoRequest(JupyterMessage<KernelInfoRequest>),
    CompleteReply(JupyterMessage<CompleteReply>),
    CompleteRequest(JupyterMessage<CompleteRequest>),
    ExecuteReply(JupyterMessage<ExecuteReply>),
    ExecuteReplyException(JupyterMessage<ExecuteReplyException>),
    ExecuteRequest(JupyterMessage<ExecuteRequest>),
    InspectReply(JupyterMessage<InspectReply>),
    InspectRequest(JupyterMessage<InspectRequest>),
    IsCompleteReply(JupyterMessage<IsCompleteReply>),
    IsCompleteRequest(JupyterMessage<IsCompleteRequest>),
    CommInfoReply(JupyterMessage<CommInfoReply>),
    CommInfoRequest(JupyterMessage<CommInfoRequest>),
    // CommRequest(JupyterMessage<UiFrontendRequest>),
    // CommReply(JupyterMessage<JsonRpcReply>),
    InputReply(JupyterMessage<InputReply>),
    InputRequest(JupyterMessage<InputRequest>),
    // Control
    InterruptReply(JupyterMessage<InterruptReply>),
    InterruptRequest(JupyterMessage<InterruptRequest>),
    ShutdownRequest(JupyterMessage<ShutdownRequest>),
    ShutdownReply(JupyterMessage<ShutdownReply>),
    // Registration
    HandshakeRequest(JupyterMessage<HandshakeRequest>),
    HandshakeReply(JupyterMessage<HandshakeReply>),
    // IOPub
    Status(JupyterMessage<KernelStatus>),
    ExecuteResult(JupyterMessage<ExecuteResult>),
    ExecuteError(JupyterMessage<ExecuteError>),
    ExecuteInput(JupyterMessage<ExecuteInput>),
    Stream(JupyterMessage<StreamOutput>),
    DisplayData(JupyterMessage<DisplayData>),
    UpdateDisplayData(JupyterMessage<UpdateDisplayData>),
    Welcome(JupyterMessage<Welcome>),
    // IOPub/Shell
    CommMsg(JupyterMessage<CommWireMsg>),
    CommOpen(JupyterMessage<CommOpen>),
    CommClose(JupyterMessage<CommClose>),
}

/// Associates a `Message` to a 0MQ socket.
///
/// At a high level, outbound messages originate from kernel components on a
/// crossbeam channel and are transfered to the client via a 0MQ socket owned by
/// the forwarding thread.
pub enum OutboundMessage {
    StdIn(Message),
    IOPub(Message),
}

/// Represents status returned from kernel inside messages.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Error,
}

/// Conversion from a `Message` to a `WireMessage`; used to send messages over a
/// socket
impl TryFrom<&Message> for WireMessage {
    type Error = crate::error::Error;

    fn try_from(msg: &Message) -> Result<Self, Error> {
        match msg {
            Message::CompleteReply(msg) => WireMessage::try_from(msg),
            Message::CompleteRequest(msg) => WireMessage::try_from(msg),
            Message::ExecuteReply(msg) => WireMessage::try_from(msg),
            Message::ExecuteReplyException(msg) => WireMessage::try_from(msg),
            Message::ExecuteRequest(msg) => WireMessage::try_from(msg),
            Message::ExecuteResult(msg) => WireMessage::try_from(msg),
            Message::ExecuteError(msg) => WireMessage::try_from(msg),
            Message::ExecuteInput(msg) => WireMessage::try_from(msg),
            Message::InputReply(msg) => WireMessage::try_from(msg),
            Message::InputRequest(msg) => WireMessage::try_from(msg),
            Message::InspectReply(msg) => WireMessage::try_from(msg),
            Message::InspectRequest(msg) => WireMessage::try_from(msg),
            Message::InterruptReply(msg) => WireMessage::try_from(msg),
            Message::InterruptRequest(msg) => WireMessage::try_from(msg),
            Message::IsCompleteReply(msg) => WireMessage::try_from(msg),
            Message::IsCompleteRequest(msg) => WireMessage::try_from(msg),
            Message::KernelInfoReply(msg) => WireMessage::try_from(msg),
            Message::KernelInfoRequest(msg) => WireMessage::try_from(msg),
            Message::ShutdownRequest(msg) => WireMessage::try_from(msg),
            Message::ShutdownReply(msg) => WireMessage::try_from(msg),
            Message::Status(msg) => WireMessage::try_from(msg),
            Message::CommInfoReply(msg) => WireMessage::try_from(msg),
            Message::CommInfoRequest(msg) => WireMessage::try_from(msg),
            Message::CommOpen(msg) => WireMessage::try_from(msg),
            Message::CommMsg(msg) => WireMessage::try_from(msg),
            Message::CommClose(msg) => WireMessage::try_from(msg),
            // Message::CommRequest(msg) => WireMessage::try_from(msg),
            // Message::CommReply(msg) => WireMessage::try_from(msg),
            Message::Stream(msg) => WireMessage::try_from(msg),
            Message::HandshakeReply(msg) => WireMessage::try_from(msg),
            Message::HandshakeRequest(msg) => WireMessage::try_from(msg),
            Message::DisplayData(msg) => WireMessage::try_from(msg),
            Message::UpdateDisplayData(msg) => WireMessage::try_from(msg),
            Message::Welcome(msg) => WireMessage::try_from(msg),
        }
    }
}

impl TryFrom<&WireMessage> for Message {
    type Error = crate::error::Error;

    /// Converts from a wire message to a Jupyter message by examining the message
    /// type and attempting to coerce the content into the appropriate
    /// structure.
    ///
    /// Note that not all message types are supported here; this handles only
    /// messages that are received from the frontend.
    fn try_from(msg: &WireMessage) -> Result<Self, Error> {
        let kind = msg.header.msg_type.clone();

        if kind == KernelInfoRequest::message_type() {
            return Ok(Message::KernelInfoRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == KernelInfoReply::message_type() {
            return Ok(Message::KernelInfoReply(JupyterMessage::try_from(msg)?));
        }
        if kind == IsCompleteRequest::message_type() {
            return Ok(Message::IsCompleteRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == IsCompleteReply::message_type() {
            return Ok(Message::IsCompleteReply(JupyterMessage::try_from(msg)?));
        }
        if kind == InspectRequest::message_type() {
            return Ok(Message::InspectRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == InspectReply::message_type() {
            return Ok(Message::InspectReply(JupyterMessage::try_from(msg)?));
        }
        if kind == ExecuteReplyException::message_type()
            && let Ok(data) = JupyterMessage::try_from(msg)
        {
            return Ok(Message::ExecuteReplyException(data));
        }
        // else fallthrough to try `ExecuteRequest` which has the same message type
        if kind == ExecuteRequest::message_type() {
            return Ok(Message::ExecuteRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == ExecuteReply::message_type() {
            return Ok(Message::ExecuteReply(JupyterMessage::try_from(msg)?));
        }
        if kind == ExecuteResult::message_type() {
            return Ok(Message::ExecuteResult(JupyterMessage::try_from(msg)?));
        }
        if kind == ExecuteError::message_type() {
            return Ok(Message::ExecuteError(JupyterMessage::try_from(msg)?));
        }
        if kind == ExecuteInput::message_type() {
            return Ok(Message::ExecuteInput(JupyterMessage::try_from(msg)?));
        }
        if kind == CompleteRequest::message_type() {
            return Ok(Message::CompleteRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == CompleteReply::message_type() {
            return Ok(Message::CompleteReply(JupyterMessage::try_from(msg)?));
        }
        if kind == DisplayData::message_type() {
            return Ok(Message::DisplayData(JupyterMessage::try_from(msg)?));
        }
        if kind == UpdateDisplayData::message_type() {
            return Ok(Message::UpdateDisplayData(JupyterMessage::try_from(msg)?));
        }
        if kind == ShutdownRequest::message_type() {
            return Ok(Message::ShutdownRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == ShutdownReply::message_type() {
            return Ok(Message::ShutdownReply(JupyterMessage::try_from(msg)?));
        }
        if kind == KernelStatus::message_type() {
            return Ok(Message::Status(JupyterMessage::try_from(msg)?));
        }
        if kind == CommInfoRequest::message_type() {
            return Ok(Message::CommInfoRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == CommInfoReply::message_type() {
            return Ok(Message::CommInfoReply(JupyterMessage::try_from(msg)?));
        }
        if kind == CommOpen::message_type() {
            return Ok(Message::CommOpen(JupyterMessage::try_from(msg)?));
        }
        if kind == CommWireMsg::message_type() {
            return Ok(Message::CommMsg(JupyterMessage::try_from(msg)?));
        }
        if kind == CommClose::message_type() {
            return Ok(Message::CommClose(JupyterMessage::try_from(msg)?));
        }
        if kind == InterruptRequest::message_type() {
            return Ok(Message::InterruptRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == InterruptReply::message_type() {
            return Ok(Message::InterruptReply(JupyterMessage::try_from(msg)?));
        }
        if kind == InputReply::message_type() {
            return Ok(Message::InputReply(JupyterMessage::try_from(msg)?));
        }
        if kind == InputRequest::message_type() {
            return Ok(Message::InputRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == StreamOutput::message_type() {
            return Ok(Message::Stream(JupyterMessage::try_from(msg)?));
        }
        // if kind == UiFrontendRequest::message_type() {
        //     return Ok(Message::CommRequest(JupyterMessage::try_from(msg)?));
        // }
        // if kind == JsonRpcReply::message_type() {
        //     return Ok(Message::CommReply(JupyterMessage::try_from(msg)?));
        // }
        if kind == HandshakeRequest::message_type() {
            return Ok(Message::HandshakeRequest(JupyterMessage::try_from(msg)?));
        }
        if kind == HandshakeReply::message_type() {
            return Ok(Message::HandshakeReply(JupyterMessage::try_from(msg)?));
        }
        if kind == Welcome::message_type() {
            return Ok(Message::Welcome(JupyterMessage::try_from(msg)?));
        }
        Err(Error::UnknownMessageType(kind))
    }
}

impl Message {
    pub fn read_from_socket(socket: &Socket) -> Result<Self, Error> {
        let msg = WireMessage::read_from_socket(socket)?;
        Message::try_from(&msg)
    }

    pub fn send(&self, socket: &Socket) -> Result<(), Error> {
        let msg = WireMessage::try_from(self)?;
        msg.send(socket)?;
        Ok(())
    }
}

impl Message {
    pub fn parent_header(&self) -> &Option<JupyterHeader> {
        match self {
            Message::KernelInfoReply(m) => &m.parent_header,
            Message::KernelInfoRequest(m) => &m.parent_header,
            Message::CompleteReply(m) => &m.parent_header,
            Message::CompleteRequest(m) => &m.parent_header,
            Message::ExecuteReply(m) => &m.parent_header,
            Message::ExecuteReplyException(m) => &m.parent_header,
            Message::ExecuteRequest(m) => &m.parent_header,
            Message::InspectReply(m) => &m.parent_header,
            Message::InspectRequest(m) => &m.parent_header,
            Message::IsCompleteReply(m) => &m.parent_header,
            Message::IsCompleteRequest(m) => &m.parent_header,
            Message::CommInfoReply(m) => &m.parent_header,
            Message::CommInfoRequest(m) => &m.parent_header,
            Message::InputReply(m) => &m.parent_header,
            Message::InputRequest(m) => &m.parent_header,
            Message::InterruptReply(m) => &m.parent_header,
            Message::InterruptRequest(m) => &m.parent_header,
            Message::ShutdownRequest(m) => &m.parent_header,
            Message::ShutdownReply(m) => &m.parent_header,
            Message::HandshakeRequest(m) => &m.parent_header,
            Message::HandshakeReply(m) => &m.parent_header,
            Message::Status(m) => &m.parent_header,
            Message::ExecuteResult(m) => &m.parent_header,
            Message::ExecuteError(m) => &m.parent_header,
            Message::ExecuteInput(m) => &m.parent_header,
            Message::Stream(m) => &m.parent_header,
            Message::DisplayData(m) => &m.parent_header,
            Message::UpdateDisplayData(m) => &m.parent_header,
            Message::Welcome(m) => &m.parent_header,
            Message::CommMsg(m) => &m.parent_header,
            Message::CommOpen(m) => &m.parent_header,
            Message::CommClose(m) => &m.parent_header,
        }
    }

    pub fn kind(&self) -> String {
        let msg_type = match self {
            Message::KernelInfoReply(msg) => msg.content.kind(),
            Message::KernelInfoRequest(msg) => msg.content.kind(),
            Message::CompleteReply(msg) => msg.content.kind(),
            Message::CompleteRequest(msg) => msg.content.kind(),
            Message::ExecuteReply(msg) => msg.content.kind(),
            Message::ExecuteReplyException(msg) => msg.content.kind(),
            Message::ExecuteRequest(msg) => msg.content.kind(),
            Message::InspectReply(msg) => msg.content.kind(),
            Message::InspectRequest(msg) => msg.content.kind(),
            Message::IsCompleteReply(msg) => msg.content.kind(),
            Message::IsCompleteRequest(msg) => msg.content.kind(),
            Message::CommInfoReply(msg) => msg.content.kind(),
            Message::CommInfoRequest(msg) => msg.content.kind(),
            Message::InputReply(msg) => msg.content.kind(),
            Message::InputRequest(msg) => msg.content.kind(),
            Message::InterruptReply(msg) => msg.content.kind(),
            Message::InterruptRequest(msg) => msg.content.kind(),
            Message::ShutdownRequest(msg) => msg.content.kind(),
            Message::ShutdownReply(msg) => msg.content.kind(),
            Message::HandshakeRequest(msg) => msg.content.kind(),
            Message::HandshakeReply(msg) => msg.content.kind(),
            Message::Status(msg) => msg.content.kind(),
            Message::ExecuteResult(msg) => msg.content.kind(),
            Message::ExecuteError(msg) => msg.content.kind(),
            Message::ExecuteInput(msg) => msg.content.kind(),
            Message::Stream(msg) => msg.content.kind(),
            Message::DisplayData(msg) => msg.content.kind(),
            Message::UpdateDisplayData(msg) => msg.content.kind(),
            Message::Welcome(msg) => msg.content.kind(),
            Message::CommMsg(msg) => msg.content.kind(),
            Message::CommOpen(msg) => msg.content.kind(),
            Message::CommClose(msg) => msg.content.kind(),
        };

        msg_type
    }

    pub fn info(&self) -> Option<String> {
        match self {
            Message::KernelInfoReply(msg) => msg.content.info(),
            Message::KernelInfoRequest(msg) => msg.content.info(),
            Message::CompleteReply(msg) => msg.content.info(),
            Message::CompleteRequest(msg) => msg.content.info(),
            Message::ExecuteReply(msg) => msg.content.info(),
            Message::ExecuteReplyException(msg) => msg.content.info(),
            Message::ExecuteRequest(msg) => msg.content.info(),
            Message::InspectReply(msg) => msg.content.info(),
            Message::InspectRequest(msg) => msg.content.info(),
            Message::IsCompleteReply(msg) => msg.content.info(),
            Message::IsCompleteRequest(msg) => msg.content.info(),
            Message::CommInfoReply(msg) => msg.content.info(),
            Message::CommInfoRequest(msg) => msg.content.info(),
            Message::InputReply(msg) => msg.content.info(),
            Message::InputRequest(msg) => msg.content.info(),
            Message::InterruptReply(msg) => msg.content.info(),
            Message::InterruptRequest(msg) => msg.content.info(),
            Message::ShutdownRequest(msg) => msg.content.info(),
            Message::ShutdownReply(msg) => msg.content.info(),
            Message::HandshakeRequest(msg) => msg.content.info(),
            Message::HandshakeReply(msg) => msg.content.info(),
            Message::Status(msg) => msg.content.info(),
            Message::ExecuteResult(msg) => msg.content.info(),
            Message::ExecuteError(msg) => msg.content.info(),
            Message::ExecuteInput(msg) => msg.content.info(),
            Message::Stream(msg) => msg.content.info(),
            Message::DisplayData(msg) => msg.content.info(),
            Message::UpdateDisplayData(msg) => msg.content.info(),
            Message::Welcome(msg) => msg.content.info(),
            Message::CommMsg(msg) => msg.content.info(),
            Message::CommOpen(msg) => msg.content.info(),
            Message::CommClose(msg) => msg.content.info(),
        }
    }

    pub fn parent_id(&self) -> Option<&Id> {
        self.parent_header().as_ref().map(|msg| &msg.msg_id)
    }

    /// Gives the message kind, the parent id, and possibly additional info
    pub fn describe(&self) -> String {
        let unparented = Id::unparented();
        let id = self.parent_id().unwrap_or(&unparented);
        let info = if let Some(info) = self.info() {
            format!("[{}]", info)
        } else {
            String::from("")
        };

        format!("{}{}{}", self.kind(), info, id)
    }
}

impl<T> JupyterMessage<T>
where
    T: ProtocolMessage,
{
    pub fn parent_id(&self) -> Option<&Id> {
        self.parent_header
            .as_ref()
            .map(|header| &header.msg_id)
    }

    /// Sends this Jupyter message to the designated ZeroMQ socket.
    pub fn send(self, socket: &Socket) -> Result<(), Error> {
        let msg = WireMessage::try_from(&self)?;
        msg.send(socket)?;
        Ok(())
    }

    /// Create a new Jupyter message, optionally as a child (reply) to an
    /// existing message.
    pub fn create(
        content: T,
        parent: Option<JupyterHeader>,
        session: &Session,
    ) -> JupyterMessage<T> {
        JupyterMessage::<T> {
            zmq_identities: Vec::new(),
            header: JupyterHeader::create(
                T::message_type(),
                session.session_id.clone(),
                session.username.clone(),
            ),
            parent_header: parent,
            content,
        }
    }

    // /// Create a new Jupyter message with a specific ZeroMQ identity.
    // pub fn create_with_identity(
    //     originator: Originator,
    //     content: T,
    //     session: &Session,
    // ) -> JupyterMessage<T> {
    //     JupyterMessage::<T> {
    //         zmq_identities: originator.zmq_identities,
    //         header: JupyterHeader::create(
    //             T::message_type(),
    //             session.session_id.clone(),
    //             session.username.clone(),
    //         ),
    //         parent_header: Some(originator.header),
    //         content,
    //     }
    // }

    // /// Sends a reply to the message; convenience method combining creating the
    // /// reply and sending it.
    // pub fn send_reply<R: ProtocolMessage>(&self, content: R, socket: &Socket) -> crate::Result<()> {
    //     let reply = self.reply_msg(content, &socket.session)?;
    //     reply.send(&socket)
    // }

    // /// Sends an error reply to the message.
    // pub fn send_error<R: ProtocolMessage>(
    //     &self,
    //     exception: Exception,
    //     socket: &Socket,
    // ) -> crate::Result<()> {
    //     let reply = self.error_reply::<R>(exception, &socket.session);
    //     reply.send(&socket)
    // }

    // pub fn send_execute_error(
    //     &self,
    //     exception: Exception,
    //     exec_count: u32,
    //     socket: &Socket,
    // ) -> crate::Result<()> {
    //     let rep = ExecuteReplyException {
    //         status: Status::Error,
    //         execution_count: exec_count,
    //         exception,
    //     };
    //     self.send_reply(rep, socket)
    // }

    // /// Create a raw reply message to this message.
    // fn reply_msg<R: ProtocolMessage>(
    //     &self,
    //     content: R,
    //     session: &Session,
    // ) -> Result<WireMessage, Error> {
    //     let reply = self.create_reply(content, session);
    //     WireMessage::try_from(&reply)
    // }

    // /// Create a reply to this message with the given content.
    // pub fn create_reply<R: ProtocolMessage>(
    //     &self,
    //     content: R,
    //     session: &Session,
    // ) -> JupyterMessage<R> {
    //     // Note that the message we are creating needs to use the kernel session
    //     // (given as an argument), not the client session (which we could
    //     // otherwise copy from the message itself)
    //     JupyterMessage::<R> {
    //         zmq_identities: self.zmq_identities.clone(),
    //         header: JupyterHeader::create(
    //             R::message_type(),
    //             session.session_id.clone(),
    //             session.username.clone(),
    //         ),
    //         parent_header: Some(self.header.clone()),
    //         content,
    //     }
    // }

    //     /// Creates an error reply to this message; used on ROUTER/DEALER sockets to
    //     /// indicate that an error occurred while processing a Request message.
    //     ///
    //     /// Error replies are special cases; they use the message type of a
    //     /// successful reply, but their content is an Exception instead.
    //     pub fn error_reply<R: ProtocolMessage>(
    //         &self,
    //         exception: Exception,
    //         session: &Session,
    //     ) -> JupyterMessage<ErrorReply> {
    //         JupyterMessage::<ErrorReply> {
    //             zmq_identities: self.zmq_identities.clone(),
    //             header: JupyterHeader::create(
    //                 R::message_type(),
    //                 session.session_id.clone(),
    //                 session.username.clone(),
    //             ),
    //             parent_header: Some(self.header.clone()),
    //             content: ErrorReply {
    //                 status: Status::Error,
    //                 exception,
    //             },
    //         }
    //     }
}

use crate::{
    kernel::kernel_spec::KernelSpecFull,
    msg::wire::{
        complete_request::CompleteRequest,
        execute_request::ExecuteRequest,
        is_complete_request::IsCompleteRequest,
        jupyter_message::{Message, MessageType},
        status::ExecutionState,
    },
    supervisor::frontend::Frontend,
};
use std::collections::HashMap;

pub fn discover_kernels() -> Vec<KernelSpecFull> {
    KernelSpecFull::get_all()
}

pub fn start_kernel(spec_path: String) -> anyhow::Result<String> {
    Frontend::start_kernel(spec_path)
}

pub fn provide_stdin(value: String) {
    Frontend::provide_stdin(value);
}

pub fn execute_code(
    code: String,
    user_expressions: HashMap<String, String>,
) -> impl Fn() -> Option<Message> {
    log::trace!("Sending execute request `{}`", code);

    // First let's try routing any incoming messages from the shell.
    Frontend::recv_all_incoming_shell();

    let request = Frontend::send_request(ExecuteRequest {
        code: code.clone(),
        silent: false,
        store_history: true,
        allow_stdin: true,
        stop_on_error: true,
        user_expressions: serde_json::to_value(user_expressions).unwrap(),
    });

    // We return a closure which can be repeatedly called as a function from Lua to get the
    // response from the kernel
    move || {
        loop {
            // --------------------------------------------------------------------------------------------------------
            // First we check if the request is still active. If not we return an empty result.
            // --------------------------------------------------------------------------------------------------------
            // If the request id is no longer registered as active then we've evidently already
            // received the reply and we can just return an empty result.
            if !Frontend::is_request_active(&request.id) {
                return None;
            }

            // First let's try routing any incoming messages from the shell. In theory there should
            // be only one - the reply to this execute request. However there may be more, e.g.
            // late responses to previous requests.
            Frontend::recv_all_incoming_shell();

            // --------------------------------------------------------------------------------------------------------
            // The request _is_ active, so let's see if there's anything on iopub
            // --------------------------------------------------------------------------------------------------------
            if let Ok(reply) = request.iopub.try_recv() {
                log::trace!("Receiving message from iopub: {}", reply.kind());
                match reply {
                    // These are the message types we want to surface in Lua
                    Message::ExecuteResult(_) | Message::ExecuteError(_) | Message::Stream(_) => {
                        return Some(reply);
                    }
                    // NB, it's possible that here we should also check if we have already received
                    // a busy status. However, I don't see any reason to confirm that the kernel is
                    // conforming to this pattern, so I'm not going to for now.
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                        return None;
                    }
                    // Here we can just add a sense check to ensure the code matches what we sent
                    Message::ExecuteInput(msg) => {
                        if msg.content.code != code {
                            log::warn!(
                                "Received {} with unexpected code: {}",
                                msg.content.kind(),
                                msg.content.code
                            );
                        };
                    }
                    // This is expected immediately after sending the execute request.
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                    }
                    _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
                }
            }

            // --------------------------------------------------------------------------------------------------------
            // Since there was nothing on iopub, let's see if the kernel wants input from the user
            // --------------------------------------------------------------------------------------------------------
            Frontend::recv_all_incoming_stdin();

            if let Ok(msg) = request.stdin.try_recv() {
                log::trace!("Received message from stdin: {}", msg.kind());
                if let Message::InputRequest(_) = msg {
                    return Some(msg);
                }
                log::warn!("Dropping unexpected stdin message {}", msg.kind());
            }

            // --------------------------------------------------------------------------------------------------------
            // Last of all we check if the request is complete. If not we loop again.
            // --------------------------------------------------------------------------------------------------------
            // Now let's check any shell replies related to this execute request. In theory there
            // should only be one, the final execute reply.
            while let Ok(msg) = request.shell.try_recv() {
                match msg {
                    Message::ExecuteReply(_) | Message::ExecuteReplyException(_) => {}
                    _ => log::warn!("Unexpected reply received on shell: {}", msg.kind()),
                }
                Frontend::stdin_broker().unregister_request(&request.id, "reply received");
                return None;
            }
            // If we didn't get a reply from the shell then let's try looping again
        }
    }
}

pub fn get_completions(code: String, cursor_pos: u32) -> anyhow::Result<Message> {
    log::trace!("Sending is completion request `{}`", code);

    // First let's try routing any incoming messages from the shell.
    Frontend::recv_all_incoming_shell();

    let request = Frontend::send_request(CompleteRequest { code, cursor_pos });

    let mut out = Err(anyhow::anyhow!("Failed to obtain a reply from the kernel"));

    while let Ok(reply) = request.iopub.recv() {
        match reply {
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                log::trace!("Received iopub busy status for completion_request");
            }
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                log::trace!("Received iopub idle status for completion_request");
                break;
            }
            _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
        }
    }

    // First let's try routing any incoming messages from the shell. In theory there should
    // be only one - the reply to this execute request. However there may be more, e.g.
    // late responses to previous requests.
    Frontend::route_shell();

    if let Ok(reply) = request.shell.recv() {
        match reply {
            Message::CompleteReply(_) => {
                log::trace!("Received completion_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.kind()),
        }
        Frontend::stdin_broker().unregister_request(&request.id, "reply received");
    } else {
        log::warn!("Failed to obtain completion_reply from the shell");
    }

    out
}

pub fn is_complete(code: String) -> anyhow::Result<Message> {
    log::trace!("Sending is complete request `{}`", code);

    Frontend::recv_all_incoming_shell();

    let request = Frontend::send_request(IsCompleteRequest { code: code.clone() });

    let mut out = Err(anyhow::anyhow!("Failed to obtain a reply from the kernel"));

    while let Ok(reply) = request.iopub.recv() {
        match reply {
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                log::trace!("Received iopub busy status for is_complete_request");
            }
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                log::trace!("Received iopub idle status for is_complete_request");
                break;
            }
            _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
        }
    }

    // First let's try routing any incoming messages from the shell. In theory there should
    // be only one - the reply to this execute request. However there may be more, e.g.
    // late responses to previous requests.
    Frontend::route_shell();

    if let Ok(reply) = request.shell.recv() {
        match reply {
            Message::IsCompleteReply(_) => {
                log::trace!("Received is_complete_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.kind()),
        }
        Frontend::stdin_broker().unregister_request(&request.id, "reply received");
    } else {
        log::warn!("Failed to obtain is_complete_reply from the shell");
    }

    out
}

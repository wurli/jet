use crate::{
    kernel::kernel_spec::KernelSpecFull,
    msg::wire::{
        complete_request::CompleteRequest,
        execute_request::ExecuteRequest,
        is_complete_request::IsCompleteRequest,
        jupyter_message::{Describe, Message},
        message_id::Id,
        status::ExecutionState,
    },
    supervisor::{frontend::Frontend, manager::KernelInfo},
};
use std::collections::HashMap;

pub fn discover_kernels() -> Vec<KernelSpecFull> {
    KernelSpecFull::get_all()
}

pub fn start_kernel(spec_path: String) -> anyhow::Result<(Id, KernelInfo)> {
    let matched_spec = KernelSpecFull::get_all()
        .into_iter()
        .filter(|x| x.path.to_string_lossy() == spec_path)
        .nth(0);

    let spec_full = matched_spec.expect(&format!("No kernel found with name '{}'", spec_path));
    let spec = spec_full.spec?;

    Frontend::start_kernel(spec_path, spec)
}

pub fn request_shutdown(kernel_id: &Id) -> anyhow::Result<Message> {
    Frontend::request_shutdown(&kernel_id)
}

pub fn list_kernels() -> HashMap<String, KernelInfo> {
    Frontend::kernel_manager().list_kernels()
}

pub fn provide_stdin(kernel_id: &Id, value: String) -> anyhow::Result<()> {
    Frontend::provide_stdin(&kernel_id, value)
}

/// Long term this should maybe return a coroutine (i.e. generator) once they're stable:
/// https://doc.rust-lang.org/beta/unstable-book/language-features/coroutines.html
pub fn execute_code(
    kernel_id: Id,
    code: String,
    user_expressions: HashMap<String, String>,
) -> anyhow::Result<impl Fn() -> Option<Message>> {
    log::trace!("Sending execute request `{}` to kernel {}", code, kernel_id);

    Frontend::recv_all_incoming_shell(&kernel_id).ok();

    let request = match Frontend::send_request(
        &kernel_id,
        ExecuteRequest {
            code: code.clone(),
            silent: false,
            store_history: true,
            allow_stdin: true,
            stop_on_error: true,
            user_expressions: serde_json::to_value(user_expressions).unwrap(),
        },
    ) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Failed to send execute request: {}", e);
            return Err(anyhow::anyhow!("Failed to send execute request: {}", e));
        }
    };

    Ok(move || {
        loop {
            if let Ok(false) = Frontend::is_request_active(&kernel_id, &request.id) {
                log::trace!("Request {} is no longer active, returning None", request.id);
                return None;
            }

            Frontend::recv_all_incoming_shell(&kernel_id).ok();

            if let Ok(reply) = request.iopub.try_recv() {
                log::trace!("Receiving message from iopub: {}", reply.describe());
                match reply {
                    Message::ExecuteResult(_)
                    | Message::ExecuteError(_)
                    | Message::Stream(_)
                    | Message::DisplayData(_) => {
                        return Some(reply);
                    }
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                        return None;
                    }
                    Message::ExecuteInput(msg) => {
                        if msg.content.code != code {
                            log::warn!(
                                "Received {} with unexpected code: {}",
                                msg.content.kind(),
                                msg.content.code
                            );
                        };
                    }
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                    }
                    _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
                }
            }

            Frontend::recv_all_incoming_stdin(&kernel_id).ok();

            if let Ok(msg) = request.stdin.try_recv() {
                log::trace!("Received message from stdin: {}", msg.describe());
                if let Message::InputRequest(_) = msg {
                    return Some(msg);
                }
                log::warn!("Dropping unexpected stdin message {}", msg.describe());
            }

            while let Ok(msg) = request.shell.try_recv() {
                match msg {
                    Message::ExecuteReply(_) | Message::ExecuteReplyException(_) => {}
                    _ => log::warn!("Unexpected reply received on shell: {}", msg.describe()),
                }
                if let Ok(stdin_broker) = Frontend::get_stdin_broker(&kernel_id) {
                    stdin_broker.unregister_request(&request.id, "reply received");
                }
            }
        }
    })
}

pub fn get_completions(kernel_id: Id, code: String, cursor_pos: u32) -> anyhow::Result<Message> {
    log::trace!(
        "Sending completion request `{}` to kernel {}",
        code,
        kernel_id
    );

    Frontend::recv_all_incoming_shell(&kernel_id)?;

    let request = Frontend::send_request(&kernel_id, CompleteRequest { code, cursor_pos })?;

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
            _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
        }
    }

    Frontend::route_shell_reply(&kernel_id, &request.id)?;

    if let Ok(reply) = request.shell.recv() {
        match reply {
            Message::CompleteReply(_) => {
                log::trace!("Received completion_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.describe()),
        }
        if let Ok(broker) = Frontend::get_stdin_broker(&kernel_id) {
            broker.unregister_request(&request.id, "reply received");
        }
    } else {
        log::warn!("Failed to obtain completion_reply from the shell");
    }

    out
}

pub fn is_complete(kernel_id: Id, code: String) -> anyhow::Result<Message> {
    log::trace!(
        "Sending is complete request `{}` to kernel {}",
        code,
        kernel_id
    );

    Frontend::recv_all_incoming_shell(&kernel_id)?;

    let request = Frontend::send_request(&kernel_id, IsCompleteRequest { code: code.clone() })?;

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
            _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
        }
    }

    Frontend::route_shell_reply(&kernel_id, &request.id)?;

    if let Ok(reply) = request.shell.recv() {
        match reply {
            Message::IsCompleteReply(_) => {
                log::trace!("Received is_complete_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.describe()),
        }
        if let Ok(broker) = Frontend::get_stdin_broker(&kernel_id) {
            broker.unregister_request(&request.id, "reply received");
        }
    } else {
        log::warn!("Failed to obtain is_complete_reply from the shell");
    }

    out
}

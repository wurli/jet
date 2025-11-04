/*
 * api.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::{
    kernel::kernel_spec::KernelSpec,
    msg::wire::{
        complete_request::CompleteRequest,
        execute_request::ExecuteRequest,
        input_reply::InputReply,
        is_complete_request::IsCompleteRequest,
        jupyter_message::{Describe, Message},
        message_id::Id,
        status::ExecutionState,
    },
    supervisor::{kernel::Kernel, kernel_info::KernelInfo, kernel_manager::KernelManager},
};
use std::{collections::HashMap, path::PathBuf};

pub fn list_available_kernels() -> HashMap<PathBuf, KernelSpec> {
    KernelSpec::find_valid()
}

pub fn list_running_kernels() -> HashMap<String, KernelInfo> {
    KernelManager::list()
}

pub fn start_kernel(spec_path: PathBuf) -> anyhow::Result<(Id, KernelInfo)> {
    let spec = KernelSpec::from_file(&spec_path)?;

    let kernel = Kernel::start(spec_path, spec)?;
    let out = (kernel.id.clone(), kernel.info.clone());

    KernelManager::add(kernel)?;

    Ok(out)
}

/// Long term this should maybe return a coroutine (i.e. generator) once they're stable:
/// https://doc.rust-lang.org/beta/unstable-book/language-features/coroutines.html
pub fn execute_code(
    kernel_id: Id,
    code: String,
    user_expressions: HashMap<String, String>,
) -> anyhow::Result<impl Fn() -> Option<Message>> {
    log::trace!("Sending execute request `{}` to kernel {}", code, kernel_id);

    let kernel = KernelManager::get(&kernel_id)?;

    kernel.comm.route_all_incoming_shell();

    let receivers = kernel.comm.send_shell(ExecuteRequest {
        code: code.clone(),
        silent: false,
        store_history: true,
        allow_stdin: true,
        stop_on_error: true,
        user_expressions: serde_json::to_value(user_expressions).unwrap(),
    })?;

    Ok(move || {
        loop {
            if !kernel.comm.is_request_active(&receivers.id) {
                log::trace!(
                    "Request {} is no longer active, returning None",
                    receivers.id
                );
                return None;
            }

            kernel.comm.route_all_incoming_shell();

            if let Ok(reply) = receivers.iopub.try_recv() {
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

            kernel.comm.route_all_incoming_stdin();

            if let Ok(msg) = receivers.stdin.try_recv() {
                log::trace!("Received message from stdin: {}", msg.describe());
                if let Message::InputRequest(_) = msg {
                    return Some(msg);
                }
                log::warn!("Dropping unexpected stdin message {}", msg.describe());
            }

            while let Ok(msg) = receivers.shell.try_recv() {
                match msg {
                    Message::ExecuteReply(_) | Message::ExecuteReplyException(_) => {}
                    _ => log::warn!("Unexpected reply received on shell: {}", msg.describe()),
                }
                kernel
                    .comm
                    .stdin_broker
                    .unregister_request(&receivers.id, "reply received");
            }
        }
    })
}

pub fn request_shutdown(kernel_id: &Id) -> anyhow::Result<()> {
    log::info!("Requesting shutdown of kernel {}", kernel_id);
    KernelManager::shutdown(kernel_id)
}

pub fn request_restart(kernel_id: &Id) -> anyhow::Result<Message> {
    log::info!("Requesting restart of kernel {}", kernel_id);
    let kernel = KernelManager::get(kernel_id)?;
    kernel.comm.request_restart()
}

pub fn provide_stdin(kernel_id: &Id, value: String) -> anyhow::Result<()> {
    let kernel = KernelManager::get(kernel_id)?;
    kernel.comm.send_stdin(InputReply { value })?;
    Ok(())
}

pub fn get_completions(kernel_id: Id, code: String, cursor_pos: u32) -> anyhow::Result<Message> {
    log::trace!(
        "Sending completion request `{}` to kernel {}",
        code,
        kernel_id
    );

    let kernel = KernelManager::get(&kernel_id)?;

    let receivers = kernel.comm.send_shell(CompleteRequest { code, cursor_pos })?;

    loop {
        // We need to loop here because it's possible that the shell channel may receive any number
        // of replies to previous messages before we get the reply we're looking for.
        kernel.comm.route_all_incoming_shell();

        if let Ok(reply) = receivers.shell.try_recv() {
            match reply {
                Message::CompleteReply(_) => {
                    log::trace!("Received completion_reply on the shell");
                    kernel
                        .comm
                        .stdin_broker
                        .unregister_request(&receivers.id, "reply received");
                    return Ok(reply);
                }
                _ => {
                    log::warn!("Unexpected reply received on shell: {}", reply.describe());
                    return Err(anyhow::anyhow!("Unexpected reply: {}", reply.describe()));
                }
            }
        }
    }
}

pub fn is_complete(kernel_id: Id, code: String) -> anyhow::Result<Message> {
    log::trace!(
        "Sending is complete request `{}` to kernel {}",
        code,
        kernel_id
    );

    let kernel = KernelManager::get(&kernel_id)?;

    let receivers = kernel
        .comm
        .send_shell(IsCompleteRequest { code: code.clone() })?;

    loop {
        // We need to loop here because it's possible that the shell channel may receive any number
        // of replies to previous messages before we get the reply we're looking for.
        kernel.comm.route_all_incoming_shell();

        if let Ok(reply) = receivers.shell.try_recv() {
            match reply {
                Message::IsCompleteReply(_) => {
                    log::trace!("Received is_complete_reply on the shell");
                    kernel
                        .comm
                        .stdin_broker
                        .unregister_request(&receivers.id, "reply received");
                    return Ok(reply);
                }
                _ => {
                    log::warn!("Unexpected reply received on shell: {}", reply.describe());
                    return Err(anyhow::anyhow!("Unexpected reply: {}", reply.describe()));
                }
            }
        }
    }
}

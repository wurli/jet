/*
 * api.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use serde_json::Value;

use crate::{
    callback_output::CallbackOutput,
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

pub fn request_shutdown(kernel_id: &Id) -> anyhow::Result<()> {
    log::info!("Requesting shutdown of kernel {kernel_id}");
    KernelManager::shutdown(kernel_id)
}

pub fn request_restart(kernel_id: Id) -> anyhow::Result<Message> {
    log::info!("Requesting restart of kernel {kernel_id}");
    let kernel = KernelManager::get(&kernel_id)?;
    kernel.comm.request_restart()
}

pub fn provide_stdin(kernel_id: &Id, value: String) -> anyhow::Result<()> {
    let kernel = KernelManager::get(kernel_id)?;
    kernel.comm.send_stdin(InputReply { value })?;
    Ok(())
}

pub fn comm_open(
    kernel_id: Id,
    target_name: String,
    data: Value,
) -> anyhow::Result<(Id, impl Fn() -> CallbackOutput)> {
    log::trace!("Opening new comm `{target_name}` for kernel {kernel_id}");

    let kernel = KernelManager::get(&kernel_id)?;
    let (comm_id, receiver) = kernel.comm.comm_open(target_name, data);
    let comm_id_out = comm_id.clone();

    let callback = move || {
        // Just to make things a bit more readable
        let comm = &kernel.comm;

        if !comm.iopub_broker.is_comm_open(&comm_id) {
            log::trace!("Comm {comm_id} is no longer active, returning None");
            return CallbackOutput::Idle;
        }

        while let Ok(reply) = receiver.try_recv() {
            log::trace!("Receiving message from iopub: {}", reply.describe());
            match reply {
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {}
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                    return if comm.iopub_broker.is_comm_open(&comm_id) {
                        CallbackOutput::Busy(None)
                    } else {
                        CallbackOutput::Idle
                    };
                }
                Message::CommMsg(msg) => {
                    return CallbackOutput::Busy(Some(Message::CommMsg(msg)));
                }
                _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
            }
        }

        CallbackOutput::Busy(None)
    };

    Ok((comm_id_out, callback))
}

pub fn comm_send(
    kernel_id: Id,
    comm_id: Id,
    data: Value,
) -> anyhow::Result<impl Fn() -> CallbackOutput> {
    log::trace!("Sending comm message to comm {comm_id} on kernel {kernel_id}");

    let kernel = KernelManager::get(&kernel_id)?;
    let (id, receiver) = kernel.comm.comm_send(comm_id, data)?;

    Ok(move || {
        // Just to make things a bit more readable
        let comm = &kernel.comm;

        if !comm.is_request_active(&id) {
            log::trace!("Request {id} is no longer active, returning None");
            return CallbackOutput::Idle;
        }

        while let Ok(reply) = receiver.try_recv() {
            log::trace!("Receiving message from iopub: {}", reply.describe());
            match reply {
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {}
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                    comm.iopub_broker
                        .unregister_request(&id, "idle status received");
                    return if comm.is_request_active(&id) {
                        CallbackOutput::Busy(None)
                    } else {
                        CallbackOutput::Idle
                    };
                }
                Message::CommMsg(msg) => {
                    return CallbackOutput::Busy(Some(Message::CommMsg(msg)));
                }
                _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
            }
        }

        CallbackOutput::Busy(None)
    })
}

pub fn execute_code(
    kernel_id: Id,
    code: String,
    user_expressions: HashMap<String, String>,
) -> anyhow::Result<impl Fn() -> CallbackOutput> {
    log::trace!("Sending execute request `{code}` to kernel {kernel_id}");

    let kernel = KernelManager::get(&kernel_id)?;

    let receivers = kernel.comm.send_shell(ExecuteRequest {
        code: code.clone(),
        silent: false,
        store_history: true,
        allow_stdin: true,
        stop_on_error: true,
        user_expressions: serde_json::to_value(user_expressions).unwrap(),
    })?;

    Ok(move || {
        // Just to make things a bit more readable
        let comm = &kernel.comm;

        if !comm.is_request_active(&receivers.id) {
            log::trace!(
                "Request {} is no longer active, returning None",
                receivers.id
            );
            return CallbackOutput::Idle;
        }

        while let Ok(reply) = receivers.iopub.try_recv() {
            log::trace!("Receiving message from iopub: {}", reply.describe());
            match reply {
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {}
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                    comm.iopub_broker
                        .unregister_request(&receivers.id, "idle status received");
                    return if comm.is_request_active(&receivers.id) {
                        CallbackOutput::Busy(None)
                    } else {
                        CallbackOutput::Idle
                    };
                }
                Message::ExecuteResult(_)
                | Message::ExecuteError(_)
                | Message::Stream(_)
                | Message::DisplayData(_) => {
                    return CallbackOutput::Busy(Some(reply));
                }
                Message::ExecuteInput(ref msg) => {
                    if msg.content.code != code {
                        log::warn!(
                            "Received {} with unexpected code: {}",
                            msg.content.kind(),
                            msg.content.code
                        );
                    };
                    return CallbackOutput::Busy(Some(reply));
                }
                _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
            }
        }

        comm.route_all_incoming_stdin();
        while let Ok(msg) = receivers.stdin.try_recv() {
            log::trace!("Received message from stdin: {}", msg.describe());
            if let Message::InputRequest(_) = msg {
                return CallbackOutput::Busy(Some(msg));
            }
            log::warn!("Dropping unexpected stdin message {}", msg.describe());
        }

        comm.route_all_incoming_shell();
        while let Ok(msg) = receivers.shell.try_recv() {
            match msg {
                Message::ExecuteReply(_) | Message::ExecuteReplyException(_) => {}
                _ => log::warn!("Unexpected reply received on shell: {}", msg.describe()),
            }
            comm.unregister_request(&receivers.id, "reply received");
            return if comm.is_request_active(&receivers.id) {
                CallbackOutput::Busy(None)
            } else {
                CallbackOutput::Idle
            };
        }

        CallbackOutput::Busy(None)
    })
}

pub fn get_completions(
    kernel_id: Id,
    code: String,
    cursor_pos: u32,
) -> anyhow::Result<impl Fn() -> CallbackOutput> {
    log::trace!("Sending completion request `{code}` to kernel {kernel_id}");

    let kernel = KernelManager::get(&kernel_id)?;

    let receivers = kernel
        .comm
        .send_shell(CompleteRequest { code, cursor_pos })?;

    Ok(move || {
        // We need to loop here because it's possible that the shell channel may receive any number
        // of replies to previous messages before we get the reply we're looking for.
        let comm = &kernel.comm;
        comm.route_all_incoming_shell();

        if !comm.is_request_active(&receivers.id) {
            log::trace!(
                "Request {} is no longer active, returning None",
                receivers.id
            );
            return CallbackOutput::Idle;
        }

        while let Ok(reply) = receivers.shell.try_recv() {
            match reply {
                Message::CompleteReply(_) => {
                    log::trace!("Received completion_reply on the shell");
                    comm.unregister_request(&receivers.id, "reply received");
                }
                _ => log::warn!("Unexpected reply received on shell: {}", reply.describe()),
            }
            return CallbackOutput::Busy(Some(reply));
        }

        return CallbackOutput::Busy(None);
    })
}

pub fn is_complete(kernel_id: Id, code: String) -> anyhow::Result<impl Fn() -> CallbackOutput> {
    log::trace!("Sending is complete request `{code}` to kernel {kernel_id}");

    let kernel = KernelManager::get(&kernel_id)?;

    let receivers = kernel
        .comm
        .send_shell(IsCompleteRequest { code: code.clone() })?;

    Ok(move || {
        // We need to loop here because it's possible that the shell channel may receive any number
        // of replies to previous messages before we get the reply we're looking for.
        let comm = &kernel.comm;
        comm.route_all_incoming_shell();

        while let Ok(reply) = receivers.shell.try_recv() {
            match reply {
                Message::IsCompleteReply(_) => {
                    log::trace!("Received is_complete_reply on the shell");
                    comm.unregister_request(&receivers.id, "reply received");
                }
                _ => log::warn!("Unexpected reply received on shell: {}", reply.describe()),
            }
            return CallbackOutput::Busy(Some(reply));
        }

        return CallbackOutput::Busy(None);
    })
}

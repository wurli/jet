/*
 * api.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use serde_json::Value;

use crate::{
    callback_output::KernelResponse,
    error::Error,
    kernel::kernel_spec::KernelSpec,
    msg::wire::{
        input_reply::InputReply, jupyter_message::Message, message_id::Id, status::ExecutionState,
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
) -> anyhow::Result<(Id, impl Fn() -> KernelResponse)> {
    log::trace!("Opening new comm `{target_name}` for kernel {kernel_id}");

    let kernel = KernelManager::get(&kernel_id)?;
    let (comm_id, receiver) = kernel.comm.comm_open(target_name, data);
    let comm_id_out = comm_id.clone();

    let callback = move || {
        // Just to make things a bit more readable
        let comm = &kernel.comm;

        if !comm.iopub_broker.is_comm_open(&comm_id) {
            log::trace!("Comm {comm_id} is no longer active, returning None");
            return KernelResponse::Idle;
        }

        while let Ok(reply) = receiver.try_recv() {
            log::trace!("Receiving message from iopub: {}", reply.describe());
            match reply {
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {}
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                    return if comm.iopub_broker.is_comm_open(&comm_id) {
                        KernelResponse::Busy(None)
                    } else {
                        KernelResponse::Idle
                    };
                }
                Message::CommMsg(msg) => {
                    return KernelResponse::Busy(Some(Message::CommMsg(msg)));
                }
                _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
            }
        }

        KernelResponse::Busy(None)
    };

    Ok((comm_id_out, callback))
}

pub fn comm_send(
    kernel_id: Id,
    comm_id: Id,
    data: Value,
) -> anyhow::Result<impl Fn() -> KernelResponse> {
    log::trace!("Sending comm message to comm {comm_id} on kernel {kernel_id}");

    let kernel = KernelManager::get(&kernel_id)?;
    let (id, receiver) = kernel.comm.comm_send(comm_id, data)?;

    Ok(move || {
        // Just to make things a bit more readable
        let comm = &kernel.comm;

        if !comm.is_request_active(&id) {
            log::trace!("Request {id} is no longer active, returning None");
            return KernelResponse::Idle;
        }

        while let Ok(reply) = receiver.try_recv() {
            log::trace!("Receiving message from iopub: {}", reply.describe());
            match reply {
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {}
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                    comm.iopub_broker
                        .unregister_request(&id, "idle status received");
                    return if comm.is_request_active(&id) {
                        KernelResponse::Busy(None)
                    } else {
                        KernelResponse::Idle
                    };
                }
                Message::CommMsg(msg) => {
                    return KernelResponse::Busy(Some(Message::CommMsg(msg)));
                }
                _ => log::warn!("Dropping unexpected iopub message {}", reply.describe()),
            }
        }

        KernelResponse::Busy(None)
    })
}

pub fn interrupt(kernel_id: Id) -> Result<Option<Message>, Error> {
    KernelManager::get(&kernel_id)?.interrupt()
}

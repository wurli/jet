/*
 * api.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use crate::{
    error::Error,
    kernel::kernel_spec::KernelSpec,
    msg::wire::{input_reply::InputReply, jupyter_message::Message, message_id::Id},
    supervisor::{kernel::Kernel, kernel_info::KernelInfo, kernel_manager::KernelManager},
};
use std::path::PathBuf;

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

pub fn interrupt(kernel_id: Id) -> Result<Option<Message>, Error> {
    KernelManager::get(&kernel_id)?.interrupt()
}

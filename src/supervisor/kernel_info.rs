/*
 * kernel_info.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::path::PathBuf;

use crate::{kernel::kernel_spec::KernelSpec, msg::wire::kernel_info_reply::KernelInfoReply};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct KernelInfo {
    /// The path to the kernel's spec file
    pub spec_path: PathBuf,
    pub spec: KernelSpec,
    pub info: KernelInfoReply,
    /// The time the kernel was started in seconds since the UNIX epoch We don't use an Instant
    /// because they're not supported by Serde, and we don't use a SystemTime because they
    /// (currently) don't play nicely with mlua.
    pub start_time: u64,
}

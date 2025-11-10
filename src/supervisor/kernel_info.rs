/*
 * kernel_info.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::{path::PathBuf};

use crate::msg::wire::language_info::LanguageInfo;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct KernelInfo {
    /// The path to the kernel's spec file
    pub spec_path: PathBuf,
    /// The spec file's `display_name`
    pub display_name: String,
    /// The banner given by the kernel's `KernelInfoReply`
    pub banner: String,
    /// The language info given by the kernel's `KernelInfoReply`
    pub language: LanguageInfo,
    /// The time the kernel was started in seconds since the UNIX epoch We don't use an Instant
    /// because they're not supported by Serde, and we don't use a SystemTime because they
    /// (currently) don't play nicely with mlua.
    pub start_time: u64,
}


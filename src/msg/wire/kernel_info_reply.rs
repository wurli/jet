/*
 * kernel_info_reply.rs
 *
 * Copyright (C) 2022-2024 Posit Software, PBC. All rights reserved.
 *
 */

use serde::Deserialize;
use serde::Serialize;

use crate::msg::wire::help_link::HelpLink;
use crate::msg::wire::jupyter_message::Status;
use crate::msg::wire::language_info::LanguageInfo;

/// Represents a reply to a `kernel_info_request`
///
/// When implementing a kernel, use this struct. Amalthea is in charge of
/// providing the `protocol_version` to complete the reply.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KernelInfoReply {
    /// The execution status ("ok" or "error")
    pub status: Status,

    /// Version of messaging protocol.
    pub protocol_version: Option<String>,

    /// The kernel implementation name
    pub implementation: Option<String>,

    /// Information about the language the kernel supports
    pub language_info: LanguageInfo,

    /// A startup banner
    pub banner: String,

    /// Whether debugging is supported
    pub debugger: bool,

    /// A list of help links
    pub help_links: Vec<HelpLink>,

    /// Optional: A list of optional features such as 'debugger' and 'kernel subshells'. Introduced
    /// by Jupyter Enhancement Proposal 92
    ///
    /// docs: <https://github.com/jupyter/enhancement-proposals/pull/92>
    pub supported_features: Option<Vec<String>>,
}

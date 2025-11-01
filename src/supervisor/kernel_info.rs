use std::path::PathBuf;

use crate::msg::wire::language_info::LanguageInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KernelInfo {
    /// The path to the kernel's spec file
    pub spec_path: PathBuf,
    /// The spec file's `display_name`
    pub display_name: String,
    /// The banner given by the kernel's `KernelInfoReply`
    pub banner: String,
    /// The language info given by the kernel's `KernelInfoReply`
    pub language: LanguageInfo,
}

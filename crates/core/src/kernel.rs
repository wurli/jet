//! Jupyter kernelspec parsing.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::kallichore::api::types::InterruptMode;

/// A parsed Jupyter `kernel.json` kernelspec.
///
/// Spec: https://jupyter-client.readthedocs.io/en/latest/kernels.html#kernel-specs
#[derive(Debug, Deserialize)]
pub struct KernelSpec {
    pub argv: Vec<String>,
    pub language: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// `signal` (default) or `message`. Per the Jupyter spec, kernels that
    /// don't set this expect interrupts via OS signals.
    #[serde(default = "default_interrupt_mode")]
    pub interrupt_mode: InterruptMode,
}

fn default_interrupt_mode() -> InterruptMode {
    InterruptMode::Signal
}

impl KernelSpec {
    /// Read and parse a `kernel.json` file.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading kernelspec at {}", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing kernelspec at {}", path.display()))
    }
}

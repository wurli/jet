//! Jupyter kernelspec: `kernel.json` parsing and on-disk discovery.
//!
//! docs: <https://jupyter-client.readthedocs.io/en/latest/kernels.html#kernel-specs>
//! schema: <https://github.com/jupyter/enhancement-proposals/blob/master/105-kernelspec-spec/kernelspec.schema.json>

use std::collections::{HashMap, HashSet};
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How the kernel expects to be interrupted: `Signal` (default) means
/// SIGINT to the kernel process group; `Message` means an
/// `interrupt_request` on the control channel.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InterruptMode {
    #[default]
    Signal,
    Message,
}

/// A parsed Jupyter `kernel.json` kernelspec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSpec {
    /// Command line used to start the kernel. `{connection_file}` in any
    /// argument is replaced with the connection file path at spawn time.
    pub argv: Vec<String>,

    /// Language of the kernel (used for notebook → kernel matching).
    pub language: String,

    /// Display name shown in UIs. Optional in practice — some kernelspecs
    /// in the wild omit it.
    #[serde(default)]
    pub display_name: Option<String>,

    /// How clients should interrupt cell execution. Defaults to `Signal`.
    #[serde(default)]
    pub interrupt_mode: InterruptMode,

    /// Environment variables added to the kernel process. Values may use
    /// `${VAR}` to reference the parent environment.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Free-form metadata for client-specific kernel selection.
    #[serde(default)]
    pub metadata: HashMap<String, Value>,

    /// Protocol version the kernel implements. `>=5.5` advertises JEP 66
    /// handshake support.
    #[serde(default)]
    pub kernel_protocol_version: Option<String>,
}

impl KernelSpec {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading kernelspec at {}", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing kernelspec at {}", path.display()))
    }

    /// Discover installed kernelspecs and return those that parse cleanly.
    pub fn find_valid() -> Vec<(PathBuf, Self)> {
        Self::discover_specs()
            .into_iter()
            .filter_map(|path| match Self::load(&path) {
                Ok(spec) => Some((path, spec)),
                Err(e) => {
                    log::debug!("skipping kernelspec {}: {e}", path.display());
                    None
                }
            })
            .collect()
    }

    /// Jupyter kernels live in well-known directories on disk, in
    /// descending priority:
    ///
    /// - `$JUPYTER_PATH/kernels`
    /// - `$XDG_DATA_HOME/jupyter/kernels` (defaults to `~/.local/share/...` on Linux)
    /// - `~/Library/Jupyter/kernels` (Mac)
    /// - `{sys.prefix}/share/jupyter/kernels` (Python)
    /// - `$CONDA_PREFIX/share/jupyter/kernels`
    /// - `/usr/local/share/jupyter/kernels`, `/usr/share/jupyter/kernels`
    ///
    /// Windows is not supported yet.
    pub fn discover_specs() -> Vec<PathBuf> {
        log::info!("discovering installed kernels");
        let mut dirs: Vec<PathBuf> = Vec::new();

        if let Some(var) = std::env::var_os("JUPYTER_PATH") {
            for path in std::env::split_paths(&var) {
                dirs.push(path.join("kernels"));
            }
        }

        // Linux user dir: XDG_DATA_HOME if set, else $HOME/.local/share.
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(xdg).join("jupyter/kernels"));
        } else if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&home).join(".local/share/jupyter/kernels"));
        }
        // Mac user dir.
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&home).join("Library/Jupyter/kernels"));
        }

        // Python kernels under {sys.prefix}/share/jupyter/kernels. We
        // intentionally don't probe `.venv/bin/python` directly: kernels
        // discovered that way start up against the wrong interpreter
        // unless the venv is already activated, in which case `python3`
        // resolves correctly via PATH.
        if let Some(sys_prefix) = ["python3", "python"].into_iter().find_map(get_sys_prefix) {
            dirs.push(sys_prefix.join("share/jupyter/kernels"));
        }

        if let Some(var) = std::env::var_os("CONDA_PREFIX") {
            dirs.push(PathBuf::from(var).join("share/jupyter/kernels"));
        }

        dirs.push("/usr/local/share/jupyter/kernels".into());
        dirs.push("/usr/share/jupyter/kernels".into());

        // Dedup preserving first occurrence so JUPYTER_PATH wins over a
        // colliding system path.
        let mut seen = HashSet::new();
        dirs.retain(|d| seen.insert(d.clone()));

        dirs.into_iter()
            .flat_map(|dir| match read_dir(&dir) {
                Ok(entries) => entries.collect::<Vec<_>>(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                Err(e) => {
                    log::warn!("skipping kernel dir {}: {e}", dir.display());
                    Vec::new()
                }
            })
            .flatten()
            .map(|entry| entry.path().join("kernel.json"))
            .filter(|path| path.exists())
            .collect()
    }
}

fn get_sys_prefix(py_cmd: &str) -> Option<PathBuf> {
    let output = Command::new(py_cmd)
        .arg("-c")
        .arg("import sys; print(sys.prefix)")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(prefix.into())
}

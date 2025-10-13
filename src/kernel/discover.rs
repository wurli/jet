//! Jupyter kernels should be identified through files in the following directories:
//!
//! System
//!   /usr/share/jupyter/kernels
//!   /usr/local/share/jupyter/kernels
//!
//! Env
//!   {sys.prefix}/share/jupyter/kernels
//!
//! User
//!
//!   ~/.local/share/jupyter/kernels (Linux)
//!   ~/Library/Jupyter/kernels (Mac)
//!
//! Other locations may also be searched if the JUPYTER_PATH environment variable is set.
//!
//! Windows exists too but I'm not supporting it yet.
//!
//! docs: https://jupyter-client.readthedocs.io/en/latest/kernels.html#kernel-specs

use std::fs;
use std::path::{Path, PathBuf};

fn path_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}

/// Discover Jupyter kernels by searching known directories.
///
/// Returns a vector of paths to the discovered kernel.json files, ordered by precedence (env,
/// user, system).
pub fn discover_kernels() -> Vec<PathBuf> {
    let mut dirs: Vec<String> = Vec::new();

    // TODO: split this variable up on `:` and recurse
    if let Some(var) = std::env::var_os("JUPYTER_PATH") {
        dirs.push(format!("{}", var.to_string_lossy()));
    }

    if let Some(var) = std::env::var_os("HOME") {
        dirs.push(format!(
            "{}/.local/share/jupyter/kernels",
            var.to_string_lossy()
        ));
        dirs.push(format!("{}/Library/Jupyter/kernels", var.to_string_lossy()));
    }

    dirs.push("/usr/share/jupyter/kernels".to_string());
    dirs.push("/usr/local/share/jupyter/kernels".to_string());

    // TODO: Are there any other prefix env vars we should check?
    if let Some(var) = std::env::var_os("CONDA_PREFIX") {
        dirs.push(format!("{}/share/jupyter/kernels", var.to_string_lossy()));
    }


    dirs.into_iter()
        .filter(|dir| path_exists(Path::new(dir)))
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path().join("kernel.json"))
        .filter(|path| path.exists())
        .collect()
}

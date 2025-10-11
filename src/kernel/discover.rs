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
//! Windows exists too but I'm not supporting it yet.
//!
//! docs: https://jupyter-client.readthedocs.io/en/latest/kernels.html#kernel-specs

use std::fs;
use std::path::Path;

fn path_exists(path: &Path) -> bool {
    fs::metadata(path).is_ok()
}

pub fn discover_kernels() -> Vec<String> {
    let mut kernels = Vec::new();

    let mut dirs = vec![
        "/usr/share/jupyter/kernels".to_string(),
        "/usr/local/share/jupyter/kernels".to_string(),
    ];

    if let Some(var) = std::env::var_os("CONDA_PREFIX") {
        dirs.push(format!("{}/share/jupyter/kernels", var.to_string_lossy()));
    }

    if let Some(var) = std::env::var_os("HOME") {
        dirs.push(format!(
            "{}/.local/share/jupyter/kernels",
            var.to_string_lossy()
        ));
        dirs.push(format!("{}/Library/Jupyter/kernels", var.to_string_lossy()));
    }

    for dir in dirs {
        let path = Path::new(&dir);
        if path_exists(path) {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Some(name) = entry.path().to_str() {
                            kernels.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    kernels
}

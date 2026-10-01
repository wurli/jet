//! Resolve Jupyter's standard directories.
//!
//! Matches the precedence rules from
//! <https://docs.jupyter.org/en/stable/use/jupyter-directories.html>.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Jupyter's user data directory.
///
/// Precedence:
/// 1. `$JUPYTER_DATA_DIR` if set and non-empty.
/// 2. Platform default:
///    - macOS: `~/Library/Jupyter`
///    - Linux: `$XDG_DATA_HOME/jupyter` else `~/.local/share/jupyter`
pub fn jupyter_data_dir() -> Result<PathBuf> {
    if let Some(v) = non_empty_env("JUPYTER_DATA_DIR") {
        return Ok(v);
    }
    let home = std::env::var_os("HOME").context("$HOME not set")?;
    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(home).join("Library/Jupyter"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(xdg) = non_empty_env("XDG_DATA_HOME") {
            return Ok(xdg.join("jupyter"));
        }
        Ok(PathBuf::from(home).join(".local/share/jupyter"))
    }
}

/// Jupyter's runtime directory — where connection files live.
///
/// Precedence:
/// 1. `$JUPYTER_RUNTIME_DIR` if set and non-empty.
/// 2. [`jupyter_data_dir`] joined with `runtime`.
pub fn jupyter_runtime_dir() -> Result<PathBuf> {
    if let Some(v) = non_empty_env("JUPYTER_RUNTIME_DIR") {
        return Ok(v);
    }
    Ok(jupyter_data_dir()?.join("runtime"))
}

/// Env var name for the runtime directory. Used by tests to override.
pub fn jupyter_runtime_dir_env_var() -> &'static str {
    "JUPYTER_RUNTIME_DIR"
}

/// Set `$JUPYTER_RUNTIME_DIR` for tests.
///
/// # Safety
///
/// For single-threaded `#[serial]` tests only — nothing else may touch the
/// environment concurrently.
pub unsafe fn set_jupyter_runtime_dir_env_var(dir: &Path) {
    // SAFETY: caller guarantees serialized access.
    unsafe { std::env::set_var(jupyter_runtime_dir_env_var(), dir) }
}

fn non_empty_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Save/restore an env var across a closure.
    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let prev: Vec<_> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var_os(k)))
            .collect();
        // SAFETY: serialized via #[serial]
        unsafe {
            for (k, v) in vars {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        f();
        // SAFETY: serialized via #[serial]
        unsafe {
            for (k, v) in prev {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    #[test]
    #[serial]
    fn jupyter_runtime_dir_wins() {
        with_env(
            &[
                ("JUPYTER_RUNTIME_DIR", Some("/tmp/explicit-runtime")),
                ("JUPYTER_DATA_DIR", Some("/tmp/data")),
            ],
            || {
                assert_eq!(
                    jupyter_runtime_dir().unwrap(),
                    PathBuf::from("/tmp/explicit-runtime")
                );
            },
        );
    }

    #[test]
    #[serial]
    fn jupyter_data_dir_feeds_runtime_dir() {
        with_env(
            &[
                ("JUPYTER_RUNTIME_DIR", None),
                ("JUPYTER_DATA_DIR", Some("/tmp/my-data")),
            ],
            || {
                assert_eq!(jupyter_data_dir().unwrap(), PathBuf::from("/tmp/my-data"));
                assert_eq!(
                    jupyter_runtime_dir().unwrap(),
                    PathBuf::from("/tmp/my-data/runtime")
                );
            },
        );
    }

    #[test]
    #[serial]
    fn empty_env_var_is_treated_as_unset() {
        with_env(
            &[
                ("JUPYTER_RUNTIME_DIR", Some("")),
                ("JUPYTER_DATA_DIR", Some("/tmp/not-empty")),
            ],
            || {
                assert_eq!(
                    jupyter_runtime_dir().unwrap(),
                    PathBuf::from("/tmp/not-empty/runtime")
                );
            },
        );
    }

    #[test]
    #[serial]
    fn platform_default_ends_with_jupyter_runtime() {
        with_env(
            &[
                ("JUPYTER_RUNTIME_DIR", None),
                ("JUPYTER_DATA_DIR", None),
            ],
            || {
                let got = jupyter_runtime_dir().unwrap();
                #[cfg(target_os = "macos")]
                assert!(
                    got.ends_with("Library/Jupyter/runtime"),
                    "got {got:?}"
                );
                #[cfg(not(target_os = "macos"))]
                assert!(got.ends_with("jupyter/runtime"), "got {got:?}");
            },
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[serial]
    fn xdg_data_home_honored_on_linux() {
        with_env(
            &[
                ("JUPYTER_RUNTIME_DIR", None),
                ("JUPYTER_DATA_DIR", None),
                ("XDG_DATA_HOME", Some("/tmp/xdg-data")),
            ],
            || {
                assert_eq!(
                    jupyter_data_dir().unwrap(),
                    PathBuf::from("/tmp/xdg-data/jupyter")
                );
                assert_eq!(
                    jupyter_runtime_dir().unwrap(),
                    PathBuf::from("/tmp/xdg-data/jupyter/runtime")
                );
            },
        );
    }
}

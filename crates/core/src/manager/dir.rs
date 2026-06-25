//! Resolve the jet data dir: `$XDG_DATA_HOME/jet`, falling back to
//! `$HOME/.local/share/jet`.

use std::path::PathBuf;

use anyhow::{Context, Result};

pub(super) fn jet_data_dir() -> Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return Ok(PathBuf::from(xdg).join("jet"));
    }
    let home = std::env::var_os("HOME").context("$HOME not set")?;
    Ok(PathBuf::from(home).join(".local/share/jet"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn honors_xdg_data_home() {
        let prev = std::env::var_os("XDG_DATA_HOME");
        // SAFETY: serialized via serial_test
        unsafe { std::env::set_var("XDG_DATA_HOME", "/tmp/xdg-test") };
        let got = jet_data_dir().unwrap();
        assert_eq!(got, PathBuf::from("/tmp/xdg-test/jet"));
        match prev {
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
    }

    #[test]
    #[serial]
    fn falls_back_when_xdg_unset() {
        let prev = std::env::var_os("XDG_DATA_HOME");
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
        let got = jet_data_dir().unwrap();
        assert!(
            got.ends_with("jet"),
            "expected path ending in 'jet', got {got:?}"
        );
        if let Some(v) = prev {
            unsafe { std::env::set_var("XDG_DATA_HOME", v) };
        }
    }
}

//! Formatting helpers for user-facing output.

use std::path::{Path, PathBuf};

/// `/Users/me/foo/x.json` → `~/foo/x.json` (under HOME) or `./x.json`
/// (under cwd), whichever produces a shorter display path. Falls back to
/// the absolute path. cwd wins ties — relative paths read more locally.
pub fn shorten_path(path: &Path, cwd: bool) -> String {
    let mut best = path.to_string_lossy().into_owned();

    if cwd
        && let Ok(cwd) = std::env::current_dir()
        && let Ok(rel) = path.strip_prefix(&cwd)
    {
        let cand = format!("./{}", rel.display());
        if cand.len() < best.len() {
            best = cand;
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && let Ok(rel) = path.strip_prefix(&home)
    {
        let cand = format!("~/{}", rel.display());
        if cand.len() < best.len() {
            best = cand;
        }
    }
    best
}

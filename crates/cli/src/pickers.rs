//! Interactive pickers for kernels (jet connect) and sessions (jet attach).

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::picker;
use jet_core::session::{SessionStatus, SessionStore};

/// Interactive picker over kernelspecs discovered on disk. Returns
/// `Ok(None)` on cancel or empty list.
pub async fn pick_kernelspec() -> Result<Option<PathBuf>> {
    let specs = jet_core::kernel_spec::KernelSpec::find_valid();
    if specs.is_empty() {
        eprintln!("No kernelspecs found on disk");
        return Ok(None);
    }

    let cwd = std::env::current_dir().ok();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let rows: Vec<Vec<picker::Cell>> = specs
        .iter()
        .map(|(path, spec)| {
            let name = spec.display_name.as_deref().unwrap_or(&spec.language);
            vec![
                picker::Cell::plain(name),
                picker::Cell::dim(shorten_path(path, cwd.as_deref(), home.as_deref())),
            ]
        })
        .collect();
    let idx =
        tokio::task::spawn_blocking(move || picker::pick("Start a new session:", &rows)).await??;
    Ok(idx.map(|i| specs[i].0.clone()))
}

/// Interactive picker over open sessions in the current working directory.
/// Returns `Ok(None)` if the user cancels (Esc / ^C) or there's nothing
/// to attach to.
pub async fn pick_session(message: &str) -> Result<Option<String>> {
    let store = SessionStore::default()?;
    store.probe_open().await?;
    let cwd = std::env::current_dir()?;
    let sessions: Vec<_> = store
        .list()?
        .into_iter()
        .filter(|s| s.status == SessionStatus::Open && s.working_dir == cwd)
        .collect();

    if sessions.is_empty() {
        eprintln!("No open sessions in {}", cwd.display());
        return Ok(None);
    }

    let rows: Vec<Vec<picker::Cell>> = sessions
        .iter()
        .map(|s| {
            vec![
                picker::Cell::dim(&s.id),
                picker::Cell::plain(&s.name),
                picker::Cell::plain(&s.created_at),
            ]
        })
        .collect();
    let message = message.to_owned();
    let idx = tokio::task::spawn_blocking(move || picker::pick(&message, &rows)).await??;
    Ok(idx.map(|i| sessions[i].id.clone()))
}

/// `/Users/me/foo/x.json` → `~/foo/x.json` (under HOME) or `./x.json`
/// (under cwd), whichever produces a shorter display path. Falls back to
/// the absolute path. cwd wins ties — relative paths read more locally.
fn shorten_path(path: &Path, cwd: Option<&Path>, home: Option<&Path>) -> String {
    let abs = path.to_string_lossy().into_owned();
    let mut best = abs;

    if let Some(cwd) = cwd {
        if let Ok(rel) = path.strip_prefix(cwd) {
            let cand = format!("./{}", rel.display());
            if cand.len() < best.len() {
                best = cand;
            }
        }
    }
    if let Some(home) = home {
        if let Ok(rel) = path.strip_prefix(home) {
            let cand = format!("~/{}", rel.display());
            if cand.len() < best.len() {
                best = cand;
            }
        }
    }
    best
}

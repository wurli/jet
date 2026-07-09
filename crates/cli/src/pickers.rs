//! Interactive pickers for kernels (jet start) and sessions (jet attach).

use std::path::PathBuf;

use anyhow::Result;

use crate::fmt::shorten_path;
use crate::picker;
use jet_core::manager::{SessionStatus, SessionStore};

/// Interactive picker over kernelspecs discovered on disk. Returns
/// `Ok(None)` on cancel or empty list.
pub async fn pick_kernelspec() -> Result<Option<PathBuf>> {
    let specs = jet_core::kernel_spec::KernelSpec::find_valid();
    if specs.is_empty() {
        eprintln!("No kernelspecs found on disk");
        return Ok(None);
    }

    let rows: Vec<Vec<picker::Cell>> = specs
        .iter()
        .map(|(path, spec)| {
            let name = spec.display_name.as_deref().unwrap_or(&spec.language);
            vec![
                picker::Cell::plain(name),
                picker::Cell::dim(shorten_path(path, true)),
            ]
        })
        .collect();
    let idx =
        tokio::task::spawn_blocking(move || picker::pick("Start a new session:", &rows, Some(1)))
            .await??;
    Ok(idx.map(|i| specs[i].0.clone()))
}

/// Interactive picker over open sessions in the current working directory.
/// Returns `Ok(None)` if the user cancels (Esc / ^C) or there's nothing
/// to attach to.
pub async fn pick_session(message: &str) -> Result<Option<String>> {
    let (sessions, rows) = match open_sessions_in_cwd().await? {
        Some(v) => v,
        None => return Ok(None),
    };
    let message = message.to_owned();
    let idx = tokio::task::spawn_blocking(move || picker::pick(&message, &rows, None)).await??;
    Ok(idx.map(|i| sessions[i].session_id.clone()))
}

/// Multi-select variant of [`pick_session`]. Returns the ids of every
/// session the user toggled on (Space to toggle, Enter to confirm).
/// Returns `Ok(vec![])` on cancel or empty selection.
pub async fn pick_sessions_multi(message: &str) -> Result<Vec<String>> {
    let (sessions, rows) = match open_sessions_in_cwd().await? {
        Some(v) => v,
        None => return Ok(vec![]),
    };
    let message = message.to_owned();
    let idxs = tokio::task::spawn_blocking(move || picker::pick_multi(&message, &rows)).await??;
    Ok(idxs
        .into_iter()
        .map(|i| sessions[i].session_id.clone())
        .collect())
}

/// List open sessions in the current working directory and build picker
/// rows. Returns `None` (after printing a hint) if there are none.
async fn open_sessions_in_cwd()
-> Result<Option<(Vec<jet_core::manager::SessionMeta>, Vec<Vec<picker::Cell>>)>> {
    let store = SessionStore::default()?;
    store.probe_open().await?;
    let cwd = std::env::current_dir()?;
    let sessions: Vec<_> = store
        .list()?
        .into_iter()
        .filter(|s| s.status == SessionStatus::Open && s.working_dir == cwd)
        .collect();

    if sessions.is_empty() {
        eprintln!("No open sessions in {}", shorten_path(&cwd, false));
        return Ok(None);
    }

    let rows: Vec<Vec<picker::Cell>> = sessions
        .iter()
        .map(|s| {
            vec![
                picker::Cell::dim(&s.session_id),
                picker::Cell::plain(&s.display_name),
                picker::Cell::plain(&s.created_at),
            ]
        })
        .collect();
    Ok(Some((sessions, rows)))
}

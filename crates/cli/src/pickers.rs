//! Interactive pickers for kernels (jet connect) and sessions (jet attach).

use std::path::PathBuf;

use anyhow::Result;

use crate::fmt::shorten_path;
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
    let idx = tokio::task::spawn_blocking(move || {
        picker::pick("Start a new session:", &rows, Some(1))
    })
    .await??;
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
        eprintln!("No open sessions in {}", shorten_path(&cwd, false));
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
    let idx = tokio::task::spawn_blocking(move || picker::pick(&message, &rows, None)).await??;
    Ok(idx.map(|i| sessions[i].id.clone()))
}

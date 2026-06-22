//! Listing existing sessions on disk.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::task::JoinSet;

use super::dir::jet_data_dir;
use super::session::{Session, SessionMeta, SessionStatus, read_meta};
use crate::connection_file;
use crate::kernel::probe_kernel_alive;

/// Scan the jet data dir and return metadata for every session whose
/// `session.json` parses. Malformed/missing entries are silently skipped.
/// Returns an empty vec if the data dir does not exist.
pub fn list_sessions() -> Result<Vec<SessionMeta>> {
    list_sessions_in(jet_data_dir()?.as_path())
}

pub fn list_sessions_in(data_dir: &Path) -> Result<Vec<SessionMeta>> {
    if !data_dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(data_dir)
        .with_context(|| format!("reading {}", data_dir.display()))?;

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(meta) = read_meta(&path) {
            out.push(meta);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Probe every Open session in the default data dir; mark dead ones
/// Closed. Best-effort and fully parallel — a single hung probe does
/// not block the others past its own 200ms TCP timeout.
pub async fn probe_open_sessions() -> Result<()> {
    let data_dir = jet_data_dir()?;
    probe_open_sessions_in(&data_dir).await
}

pub async fn probe_open_sessions_in(data_dir: &Path) -> Result<()> {
    let metas = list_sessions_in(data_dir)?;
    let mut tasks: JoinSet<()> = JoinSet::new();
    for meta in metas.into_iter().filter(|m| m.status == SessionStatus::Open) {
        let data_dir = data_dir.to_path_buf();
        tasks.spawn(async move {
            if probe_one(&data_dir, &meta).await {
                return; // alive
            }
            // Dead — load the Session and flip it.
            match Session::open_in(&data_dir, &meta.id) {
                Ok(mut s) => s.mark_closed(),
                Err(e) => log::warn!("probe: failed to reopen {}: {e}", meta.id),
            }
        });
    }
    while tasks.join_next().await.is_some() {}
    Ok(())
}

/// `true` if the session looks alive. Fast path: `kill(pid, 0)` if the
/// pid was recorded. Otherwise a TCP probe to the kernel's shell port.
async fn probe_one(data_dir: &Path, meta: &SessionMeta) -> bool {
    if let Some(pid) = meta.kernel_pid {
        #[cfg(unix)]
        // SAFETY: signal 0 is the standard liveness check; never sends a real signal.
        if unsafe { libc::kill(pid as libc::pid_t, 0) } != 0 {
            return false;
        }
    }
    let conn_path = data_dir.join(&meta.id).join(&meta.connection_file);
    let Ok(info) = connection_file::read(&conn_path) else {
        return false;
    };
    let probe = probe_kernel_alive(&info);
    tokio::time::timeout(Duration::from_millis(300), probe)
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::session::{CreateParams, Session};
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    fn tempdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "jet-sessions-test-{tag}-{:x}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn empty_when_dir_missing() {
        let p = std::env::temp_dir().join(format!(
            "jet-sessions-nope-{:x}",
            rand::random::<u64>()
        ));
        assert!(list_sessions_in(&p).unwrap().is_empty());
    }

    #[test]
    fn lists_created_sessions() {
        let data = tempdir("list");
        let cwd_a = PathBuf::from("/tmp/a");
        let cwd_b = PathBuf::from("/tmp/b");
        let t1 = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let t2 = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        Session::create_in(
            &data,
            CreateParams {
                lang: "python",
                name: "python3",
                kernelspec_path: std::path::Path::new("/k1"),
                working_dir: &cwd_a,
            },
            t1,
        )
        .unwrap();
        Session::create_in(
            &data,
            CreateParams {
                lang: "r",
                name: "ark",
                kernelspec_path: std::path::Path::new("/k2"),
                working_dir: &cwd_b,
            },
            t2,
        )
        .unwrap();

        let listed = list_sessions_in(&data).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].working_dir, cwd_a);
        assert_eq!(listed[1].working_dir, cwd_b);
        std::fs::remove_dir_all(&data).ok();
    }

    #[tokio::test]
    async fn probe_marks_dead_sessions_closed() {
        let data = tempdir("probe");
        let cwd = PathBuf::from("/tmp/probe");
        // Create two Open sessions. Each writes a connection file at
        // its connection_file_path with random free ports — by the time
        // the probe runs, the listeners are dropped, so TCP-connect to
        // shell_port will fail and the probe flips them Closed.
        let s1 = Session::create_in(
            &data,
            CreateParams {
                lang: "python",
                name: "python3",
                kernelspec_path: std::path::Path::new("/k"),
                working_dir: &cwd,
            },
            UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        )
        .unwrap();
        crate::connection_file::generate(&s1.connection_file_path()).unwrap();

        let s2 = Session::create_in(
            &data,
            CreateParams {
                lang: "r",
                name: "ark",
                kernelspec_path: std::path::Path::new("/k"),
                working_dir: &cwd,
            },
            UNIX_EPOCH + Duration::from_secs(1_700_000_001),
        )
        .unwrap();
        crate::connection_file::generate(&s2.connection_file_path()).unwrap();

        probe_open_sessions_in(&data).await.unwrap();

        let listed = list_sessions_in(&data).unwrap();
        assert_eq!(listed.len(), 2);
        for m in &listed {
            assert_eq!(m.status, SessionStatus::Closed, "{} not flipped: {m:?}", m.id);
            assert!(m.closed_at.is_some());
        }
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn skips_malformed_subdirs() {
        let data = tempdir("malformed");
        std::fs::create_dir_all(data.join("no-session-json-here")).unwrap();
        let bad = data.join("bad-json");
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(bad.join("session.json"), b"not json").unwrap();

        let cwd = PathBuf::from("/tmp/good");
        Session::create_in(
            &data,
            CreateParams {
                lang: "python",
                name: "python3",
                kernelspec_path: std::path::Path::new("/k"),
                working_dir: &cwd,
            },
            UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        )
        .unwrap();

        let listed = list_sessions_in(&data).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].working_dir, cwd);
        std::fs::remove_dir_all(&data).ok();
    }
}

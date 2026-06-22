//! Per-session directory + metadata.
//!
//! A `Session` owns one subdir of [`jet_data_dir`] containing:
//! - `session.json` — [`SessionMeta`] serialized
//! - `connection-file.json` — written by the kernel layer (not here)

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use super::dir::jet_data_dir;
use super::naming::{format_iso8601, generate_session_name};

const SESSION_JSON: &str = "session.json";
const CONNECTION_FILE: &str = "connection-file.json";
const SCHEMA_VERSION: u32 = 1;

/// Inputs to [`Session::create`].
pub struct CreateParams<'a> {
    pub lang: &'a str,
    pub kernel_name: &'a str,
    pub kernelspec_path: &'a Path,
    pub working_dir: &'a Path,
}

/// Lifecycle of a session. `Open` means the kernel should be reachable;
/// `Closed` means it exited cleanly. Sessions that crashed will still
/// read as `Open` until something (a future `jet list` probe, etc.)
/// notices the kernel is gone and updates the file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Open,
    Closed,
}

/// Contents of `session.json`. Kept stable; bump `schema_version` and
/// migrate if you need to change the shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionMeta {
    pub name: String,
    pub created_at: String,
    pub working_dir: PathBuf,
    pub lang: String,
    pub kernel_name: String,
    pub kernelspec_path: PathBuf,
    /// Relative to the session dir.
    pub connection_file: String,
    pub status: SessionStatus,
    /// ISO8601 UTC, set when status transitions to Closed.
    pub closed_at: Option<String>,
    /// OS pid of the kernel process, recorded at spawn. None for attached
    /// sessions (we don't own the child).
    pub kernel_pid: Option<u32>,
    pub jet_version: String,
    pub schema_version: u32,
}

#[derive(Debug)]
pub struct Session {
    dir: PathBuf,
    meta: SessionMeta,
}

impl Session {
    pub fn create(params: CreateParams<'_>) -> Result<Session> {
        Self::create_in(jet_data_dir()?.as_path(), params, SystemTime::now())
    }

    /// Test/internal hook: create under an explicit data dir at a fixed instant.
    pub fn create_in(data_dir: &Path, params: CreateParams<'_>, now: SystemTime) -> Result<Session> {
        let name = generate_session_name(now, params.lang, params.working_dir);
        let dir = data_dir.join(&name);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating session dir {}", dir.display()))?;

        let meta = SessionMeta {
            name,
            created_at: format_iso8601(now),
            working_dir: params.working_dir.to_path_buf(),
            lang: params.lang.to_string(),
            kernel_name: params.kernel_name.to_string(),
            kernelspec_path: params.kernelspec_path.to_path_buf(),
            connection_file: CONNECTION_FILE.to_string(),
            status: SessionStatus::Open,
            closed_at: None,
            kernel_pid: None,
            jet_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: SCHEMA_VERSION,
        };
        write_meta(&dir, &meta)?;
        Ok(Session { dir, meta })
    }

    /// Record the kernel's OS pid. Rewrites session.json. Best-effort:
    /// a write failure is logged but not returned, so caller doesn't
    /// have to error out of the spawn flow on a metadata-only hiccup.
    pub fn set_kernel_pid(&mut self, pid: u32) {
        self.meta.kernel_pid = Some(pid);
        if let Err(e) = write_meta(&self.dir, &self.meta) {
            log::warn!("session: failed to record kernel pid: {e}");
        }
    }

    /// Mark the session as closed. Rewrites session.json with
    /// `status=closed` and `closed_at=now`. Best-effort.
    pub fn mark_closed(&mut self) {
        self.mark_closed_at(SystemTime::now());
    }

    /// Test/internal hook: mark closed at a fixed instant.
    pub fn mark_closed_at(&mut self, now: SystemTime) {
        self.meta.status = SessionStatus::Closed;
        self.meta.closed_at = Some(format_iso8601(now));
        if let Err(e) = write_meta(&self.dir, &self.meta) {
            log::warn!("session: failed to record closed status: {e}");
        }
    }

    pub fn open(name: &str) -> Result<Session> {
        Self::open_in(jet_data_dir()?.as_path(), name)
    }

    pub fn open_in(data_dir: &Path, name: &str) -> Result<Session> {
        let dir = data_dir.join(name);
        let meta = read_meta(&dir)?;
        Ok(Session { dir, meta })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn meta(&self) -> &SessionMeta {
        &self.meta
    }

    pub fn connection_file_path(&self) -> PathBuf {
        self.dir.join(&self.meta.connection_file)
    }
}

pub(super) fn read_meta(dir: &Path) -> Result<SessionMeta> {
    let path = dir.join(SESSION_JSON);
    let bytes = std::fs::read(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing {}", path.display()))
}

fn write_meta(dir: &Path, meta: &SessionMeta) -> Result<()> {
    let path = dir.join(SESSION_JSON);
    let json = serde_json::to_vec_pretty(meta).map_err(|e| anyhow!("serialize session.json: {e}"))?;
    std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::time::UNIX_EPOCH;

    fn tempdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "jet-session-test-{tag}-{:x}",
            rand::random::<u64>()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn sample_params<'a>(cwd: &'a Path) -> CreateParams<'a> {
        CreateParams {
            lang: "python",
            kernel_name: "python3",
            kernelspec_path: Path::new("/fake/kernels/python3/kernel.json"),
            working_dir: cwd,
        }
    }

    #[test]
    fn create_writes_session_json() {
        let data = tempdir("create");
        let cwd = PathBuf::from("/Users/x/Repos/jet");
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let sess = Session::create_in(&data, sample_params(&cwd), t).unwrap();

        assert!(sess.dir().exists());
        assert!(sess.dir().join("session.json").exists());
        assert_eq!(
            sess.connection_file_path(),
            sess.dir().join("connection-file.json")
        );
        assert_eq!(sess.meta().lang, "python");
        assert_eq!(sess.meta().working_dir, cwd);
        assert_eq!(sess.meta().created_at, "2026-06-22T14:03:11Z");
        assert_eq!(sess.meta().schema_version, SCHEMA_VERSION);
        assert_eq!(sess.meta().status, SessionStatus::Open);
        assert_eq!(sess.meta().closed_at, None);
        assert_eq!(sess.meta().kernel_pid, None);

        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn mark_closed_persists() {
        let data = tempdir("close");
        let cwd = PathBuf::from("/tmp/foo");
        let t0 = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let t1 = UNIX_EPOCH + Duration::from_secs(1_782_140_000);
        let mut sess = Session::create_in(&data, sample_params(&cwd), t0).unwrap();
        sess.set_kernel_pid(12345);
        sess.mark_closed_at(t1);

        let reopened = Session::open_in(&data, &sess.meta().name).unwrap();
        assert_eq!(reopened.meta().status, SessionStatus::Closed);
        assert_eq!(reopened.meta().closed_at.as_deref(), Some("2026-06-22T14:53:20Z"));
        assert_eq!(reopened.meta().kernel_pid, Some(12345));
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn open_round_trips() {
        let data = tempdir("open");
        let cwd = PathBuf::from("/tmp/foo");
        let t = UNIX_EPOCH + Duration::from_secs(1_782_136_991);
        let created = Session::create_in(&data, sample_params(&cwd), t).unwrap();
        let opened = Session::open_in(&data, &created.meta().name).unwrap();
        assert_eq!(created.meta(), opened.meta());
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn open_missing_errors() {
        let data = tempdir("missing");
        let err = Session::open_in(&data, "nope").unwrap_err();
        assert!(format!("{err:#}").contains("session.json"));
        std::fs::remove_dir_all(&data).ok();
    }
}

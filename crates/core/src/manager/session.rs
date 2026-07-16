//! Per-session directory + metadata.
//!
//! A `Session` owns one subdir of the data dir, containing:
//! - `session.json` — [`SessionMeta`] serialized
//! - `connection-file.json` — written by the kernel layer (not here)

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::naming::{format_iso8601, generate_session_name};

pub(super) const SESSION_JSON: &str = "session.json";
const CONNECTION_FILE: &str = "connection-file.json";

/// Lifecycle of a session. `Open` means the kernel should be reachable;
/// `Closed` means it exited cleanly. Sessions that crashed will still
/// read as `Open` until [`super::probe_open_sessions`] notices the
/// kernel is gone and flips them.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Open,
    Closed,
}

/// Contents of `session.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionMeta {
    pub session_id: String,
    pub created_at: String,
    pub working_dir: PathBuf,
    /// Lifted from the kernelspec.json
    pub language: String,
    /// Lifted from the kernelspec.json
    pub display_name: String,
    pub kernelspec_path: PathBuf,
    /// Relative to the session dir.
    pub connection_file: String,
    pub status: SessionStatus,
    /// ISO8601 UTC, set when status transitions to Closed.
    pub closed_at: Option<String>,
    /// OS pid of the kernel process, recorded at spawn. None for attached
    /// sessions (we don't own the child).
    pub kernel_pid: Option<u32>,
}

#[derive(Debug)]
pub struct Session {
    dir: PathBuf,
    meta: SessionMeta,
}

impl Session {
    pub(super) fn create(
        data_dir: &Path,
        lang: &str,
        name: &str,
        kernelspec_path: &Path,
        working_dir: &Path,
    ) -> Result<Session> {
        let now = SystemTime::now();
        let id = generate_session_name(now, lang, working_dir);
        Self::create_inner(data_dir, &id, lang, name, kernelspec_path, working_dir, now)
    }

    /// Like [`Self::create`], but uses the caller-supplied `id` as the
    /// session-dir name instead of generating one. Fails if the dir
    /// already exists — collisions almost certainly mean the caller is
    /// confused about which session it owns.
    pub(super) fn create_with_id(
        data_dir: &Path,
        id: &str,
        lang: &str,
        name: &str,
        kernelspec_path: &Path,
        working_dir: &Path,
    ) -> Result<Session> {
        Self::create_inner(
            data_dir,
            id,
            lang,
            name,
            kernelspec_path,
            working_dir,
            SystemTime::now(),
        )
    }

    fn create_inner(
        data_dir: &Path,
        id: &str,
        lang: &str,
        name: &str,
        kernelspec_path: &Path,
        working_dir: &Path,
        now: SystemTime,
    ) -> Result<Session> {
        let id = id.to_string();
        let dir = data_dir.join(&id);
        if dir.exists() {
            anyhow::bail!("session dir already exists: {}", dir.display());
        }
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("creating session dir {}", dir.display()))?;

        let session = Session {
            meta: SessionMeta {
                session_id: id,
                created_at: format_iso8601(now),
                working_dir: working_dir.to_path_buf(),
                language: lang.to_string(),
                display_name: name.to_string(),
                kernelspec_path: kernelspec_path.to_path_buf(),
                connection_file: CONNECTION_FILE.to_string(),
                status: SessionStatus::Open,
                closed_at: None,
                kernel_pid: None,
            },
            dir,
        };
        session.persist()?;
        Ok(session)
    }

    pub(super) fn open(data_dir: &Path, id: &str) -> Result<Session> {
        let dir = data_dir.join(id);
        let meta = read_meta(&dir)?;
        Ok(Session { dir, meta })
    }

    /// Record the kernel's OS pid. Best-effort: a write failure is
    /// logged but not returned, so the caller doesn't have to error out
    /// of the spawn flow on a metadata-only hiccup.
    pub fn set_kernel_pid(&mut self, pid: u32) {
        self.meta.kernel_pid = Some(pid);
        self.persist_best_effort("record kernel pid");
    }

    /// Mark the session as closed. Best-effort (see [`Self::set_kernel_pid`]).
    pub fn mark_closed(&mut self) {
        self.mark_closed_at(SystemTime::now());
    }

    pub(super) fn mark_closed_at(&mut self, now: SystemTime) {
        self.meta.status = SessionStatus::Closed;
        self.meta.closed_at = Some(format_iso8601(now));
        self.meta.kernel_pid = None;
        self.persist_best_effort("record closed status");
    }

    pub fn meta(&self) -> &SessionMeta {
        &self.meta
    }

    pub fn connection_file_path(&self) -> PathBuf {
        self.dir.join(&self.meta.connection_file)
    }

    fn persist(&self) -> Result<()> {
        let path = self.dir.join(SESSION_JSON);
        let json = serde_json::to_vec_pretty(&self.meta)
            .with_context(|| format!("serializing {}", path.display()))?;
        std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    fn persist_best_effort(&self, what: &str) {
        if let Err(e) = self.persist() {
            log::warn!("session: failed to {what}: {e}");
        }
    }
}

pub(super) fn read_meta(dir: &Path) -> Result<SessionMeta> {
    let path = dir.join(SESSION_JSON);
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};
    use tempfile::TempDir;

    fn create(dir: &Path, cwd: &Path) -> Result<Session> {
        Session::create(
            dir,
            "python",
            "python3",
            Path::new("/fake/kernels/python3/kernel.json"),
            cwd,
        )
    }

    #[test]
    fn create_writes_session_json() {
        let data = TempDir::new().unwrap();
        let cwd = PathBuf::from("/Users/x/Repos/jet");
        let sess = create(data.path(), &cwd).unwrap();

        assert!(sess.connection_file_path().parent().unwrap().exists());
        assert!(
            sess.connection_file_path()
                .parent()
                .unwrap()
                .join("session.json")
                .exists()
        );
        assert_eq!(
            sess.connection_file_path().file_name().unwrap(),
            "connection-file.json"
        );
        assert_eq!(sess.meta().language, "python");
        assert_eq!(sess.meta().working_dir, cwd);
        assert!(!sess.meta().created_at.is_empty());
        assert_eq!(sess.meta().status, SessionStatus::Open);
        assert_eq!(sess.meta().closed_at, None);
        assert_eq!(sess.meta().kernel_pid, None);
    }

    #[test]
    fn mark_closed_persists() {
        let data = TempDir::new().unwrap();
        let cwd = PathBuf::from("/tmp/foo");
        let t1 = UNIX_EPOCH + Duration::from_secs(1_782_140_000);
        let mut sess = create(data.path(), &cwd).unwrap();
        sess.set_kernel_pid(12345);
        sess.mark_closed_at(t1);

        let reopened = Session::open(data.path(), &sess.meta().session_id).unwrap();
        assert_eq!(reopened.meta().status, SessionStatus::Closed);
        assert_eq!(
            reopened.meta().closed_at.as_deref(),
            Some("2026-06-22T14:53:20Z")
        );
        assert_eq!(
            reopened.meta().kernel_pid,
            None,
            "mark_closed should clear the recorded pid"
        );
    }

    #[test]
    fn set_kernel_pid_round_trips() {
        let data = TempDir::new().unwrap();
        let mut sess = create(data.path(), Path::new("/tmp/foo")).unwrap();
        assert_eq!(sess.meta().kernel_pid, None);

        sess.set_kernel_pid(98765);
        assert_eq!(sess.meta().kernel_pid, Some(98765));

        // Persisted to disk synchronously, so a separate Session::open sees it.
        let reopened = Session::open(data.path(), &sess.meta().session_id).unwrap();
        assert_eq!(reopened.meta().kernel_pid, Some(98765));
    }

    #[test]
    fn mark_closed_clears_kernel_pid() {
        let data = TempDir::new().unwrap();
        let mut sess = create(data.path(), Path::new("/tmp/foo")).unwrap();
        sess.set_kernel_pid(12345);
        assert_eq!(sess.meta().kernel_pid, Some(12345));

        sess.mark_closed();
        assert_eq!(sess.meta().kernel_pid, None);

        let reopened = Session::open(data.path(), &sess.meta().session_id).unwrap();
        assert_eq!(reopened.meta().kernel_pid, None);
    }

    #[test]
    fn open_round_trips() {
        let data = TempDir::new().unwrap();
        let created = create(data.path(), Path::new("/tmp/foo")).unwrap();
        let opened = Session::open(data.path(), &created.meta().session_id).unwrap();
        assert_eq!(created.meta(), opened.meta());
    }

    #[test]
    fn open_missing_errors() {
        let data = TempDir::new().unwrap();
        let err = Session::open(data.path(), "nope").unwrap_err();
        assert!(format!("{err:#}").contains("session.json"));
    }
}

//! [`SessionStore`] — a data dir bound to a path, with create/open/list/probe.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::task::JoinSet;

use super::dir::jet_data_dir;
use super::session::{Session, SessionMeta, SessionStatus, read_meta};
use crate::connection_file;
use crate::kernel::probe_kernel_alive;

/// Which sessions [`SessionStore::list_filtered`] should return.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StatusFilter {
    Open,
    Closed,
    All,
}

impl std::str::FromStr for StatusFilter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "all" => Ok(Self::All),
            other => anyhow::bail!("invalid status {other:?}: expected \"open\", \"closed\", or \"all\""),
        }
    }
}

/// A jet data dir. Wraps a path so callers don't have to thread it
/// through every call; tests construct one over a tempdir.
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    /// `$XDG_DATA_HOME/jet`, falling back to `$HOME/.local/share/jet`.
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Result<Self> {
        Ok(Self {
            dir: jet_data_dir()?,
        })
    }

    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn create(
        &self,
        lang: &str,
        name: &str,
        kernelspec_path: &Path,
        working_dir: &Path,
    ) -> Result<Session> {
        Session::create(&self.dir, lang, name, kernelspec_path, working_dir)
    }

    /// Like [`Self::create`], but uses the caller-supplied `id` as the
    /// session-dir name. Fails if a session with that id already exists.
    pub fn create_with_id(
        &self,
        id: &str,
        lang: &str,
        name: &str,
        kernelspec_path: &Path,
        working_dir: &Path,
    ) -> Result<Session> {
        Session::create_with_id(&self.dir, id, lang, name, kernelspec_path, working_dir)
    }

    pub fn open(&self, id: &str) -> Result<Session> {
        Session::open(&self.dir, id)
    }

    /// Reverse-lookup: find the session whose `connection_file_path()`
    /// matches `path`. Paths are canonicalized on both sides so symlinks
    /// and `..` normalize away; missing files fall back to a plain `==`
    /// compare. Returns `Ok(None)` if no session matches.
    pub fn find_by_connection_file(&self, path: &Path) -> Result<Option<Session>> {
        let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        for meta in self.list()? {
            let candidate = self.dir.join(&meta.id).join(&meta.connection_file);
            let canonical = std::fs::canonicalize(&candidate).unwrap_or(candidate);
            if canonical == target {
                return Ok(Some(self.open(&meta.id)?));
            }
        }
        Ok(None)
    }

    /// Scan the data dir and return metadata for every session whose
    /// `session.json` parses. Malformed/missing entries are silently
    /// skipped. Empty vec if the dir doesn't exist.
    pub fn list(&self) -> Result<Vec<SessionMeta>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let entries = std::fs::read_dir(&self.dir)
            .with_context(|| format!("reading {}", self.dir.display()))?;
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Ok(meta) = read_meta(&path)
            {
                out.push(meta);
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Probe Open sessions, then return entries matching `status` and (unless
    /// `all_dirs`) the current working directory. The filter pair `jet list-sessions`
    /// and `jet.list_sessions` both apply.
    pub async fn list_filtered(
        &self,
        status: StatusFilter,
        all_dirs: bool,
    ) -> Result<Vec<SessionMeta>> {
        self.probe_open().await?;
        let cwd = std::env::current_dir()?;
        Ok(self
            .list()?
            .into_iter()
            .filter(|s| match status {
                StatusFilter::Open => s.status == SessionStatus::Open,
                StatusFilter::Closed => s.status == SessionStatus::Closed,
                StatusFilter::All => true,
            })
            .filter(|s| all_dirs || s.working_dir == cwd)
            .collect())
    }

    /// Probe every Open session; mark dead ones Closed. Best-effort and
    /// fully parallel — a single hung probe doesn't block the others
    /// past its own ~300ms TCP timeout.
    pub async fn probe_open(&self) -> Result<()> {
        let metas = self.list()?;
        let mut tasks: JoinSet<()> = JoinSet::new();
        for meta in metas
            .into_iter()
            .filter(|m| m.status == SessionStatus::Open)
        {
            let dir = self.dir.clone();
            tasks.spawn(async move {
                if probe_one(&dir, &meta).await {
                    return;
                }
                log::warn!("probe: session {} appears dead, marking closed", meta.id);
                match Session::open(&dir, &meta.id) {
                    Ok(mut s) => s.mark_closed(),
                    Err(e) => log::warn!("probe: failed to reopen {}: {e}", meta.id),
                }
            });
        }
        tasks.join_all().await;
        Ok(())
    }
}

/// `true` if the session looks alive. Fast path: `kill(pid, 0)` if the
/// pid was recorded. Otherwise a TCP probe to the kernel's shell port.
async fn probe_one(data_dir: &Path, meta: &SessionMeta) -> bool {
    if let Some(pid) = meta.kernel_pid {
        #[cfg(unix)]
        // SAFETY: signal 0 is the standard liveness check; never sends a real signal.
        return unsafe { libc::kill(pid as libc::pid_t, 0) } == 0;
    }
    let conn_path = data_dir.join(&meta.id).join(&meta.connection_file);
    let Ok(info) = connection_file::read(&conn_path) else {
        return false;
    };
    tokio::time::timeout(Duration::from_millis(300), probe_kernel_alive(&info))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

/// Convenience: `SessionStore::default()?.list()`.
pub fn list_sessions() -> Result<Vec<SessionMeta>> {
    SessionStore::default()?.list()
}

/// Convenience: `SessionStore::default()?.probe_open().await`.
pub async fn probe_open_sessions() -> Result<()> {
    SessionStore::default()?.probe_open().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create(store: &SessionStore, lang: &str, cwd: &Path) -> Session {
        Session::create(&store.dir, lang, "kernel", Path::new("/k"), cwd).unwrap()
    }

    #[test]
    fn empty_when_dir_missing() {
        let p = std::env::temp_dir().join(format!("jet-store-nope-{:x}", rand::random::<u64>()));
        assert!(SessionStore::at(&p).list().unwrap().is_empty());
    }

    #[test]
    fn lists_created_sessions() {
        let data = TempDir::new().unwrap();
        let store = SessionStore::at(data.path());
        let cwd_a = PathBuf::from("/tmp/a");
        let cwd_b = PathBuf::from("/tmp/b");
        create(&store, "python", &cwd_a);
        create(&store, "r", &cwd_b);

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 2);
    }

    #[tokio::test]
    async fn probe_marks_dead_sessions_closed() {
        let data = TempDir::new().unwrap();
        let store = SessionStore::at(data.path());
        let cwd = PathBuf::from("/tmp/probe");
        // Two Open sessions with no recorded pid (forces TCP probe).
        // Each writes a connection file with free ports; by the time
        // probe runs the listeners are dropped, so the probe flips them.
        let s1 = create(&store, "python", &cwd);
        connection_file::generate(&s1.connection_file_path()).unwrap();
        let s2 = create(&store, "r", &cwd);
        connection_file::generate(&s2.connection_file_path()).unwrap();

        store.probe_open().await.unwrap();

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 2);
        for m in &listed {
            assert_eq!(
                m.status,
                SessionStatus::Closed,
                "{} not flipped: {m:?}",
                m.id
            );
            assert!(m.closed_at.is_some());
        }
    }

    #[test]
    fn skips_malformed_subdirs() {
        let data = TempDir::new().unwrap();
        let store = SessionStore::at(data.path());
        std::fs::create_dir_all(data.path().join("no-session-json-here")).unwrap();
        let bad = data.path().join("bad-json");
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(bad.join("session.json"), b"not json").unwrap();

        let cwd = PathBuf::from("/tmp/good");
        create(&store, "python", &cwd);

        let listed = store.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].working_dir, cwd);
    }
}

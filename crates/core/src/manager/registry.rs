//! Process-global registry of live in-process kernel clients + a
//! background liveness cache for on-disk sessions this process doesn't
//! own a client for.
//!
//! Two consumers today:
//! - Lua's `jet.list_connections()` reads [`ClientRegistry::snapshot`].
//! - Both Lua and CLI's `list_sessions()` read
//!   [`ClientRegistry::live_session_ids`] + the poller's cache via
//!   [`SessionStore::list_filtered`].
//!
//! In-process clients are authoritative: their `watch_status()` is driven
//! by SIGCHLD (spawn) or the always-on heartbeat watcher (both paths), so
//! we know instantly whether they're alive or wedged. On-disk sessions
//! belonging to another process fall back to a background heartbeat probe
//! (~1Hz) whose result is cached; the hot path never awaits.
//!
//! The registry itself is sync — all state lives behind `std::sync`
//! locks. The poller uses the shared tokio runtime returned by [`runtime`].
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinSet;

use crate::client::{Client, KernelStatus};
use crate::connection_file;
use crate::kernel::probe_kernel_alive;
use crate::manager::session::{SessionMeta, SessionStatus};
use crate::manager::store::SessionStore;

/// Shared multi-threaded tokio runtime. Callers that don't already have a
/// runtime (Lua, background poller) use this; the CLI has its own
/// `#[tokio::main]` and doesn't touch it.
pub fn runtime() -> &'static Runtime {
    static RT: Lazy<Runtime> = Lazy::new(|| {
        Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime")
    });
    &RT
}

/// Shared handle to a live [`Client`]. The mutex keeps per-kernel state
/// safe when sync callers (Lua) race on lifecycle methods (`interrupt`,
/// `shutdown`) and per-request sends.
pub type ClientHandle = Arc<tokio::sync::Mutex<Client>>;

/// Snapshot entry returned by [`ClientRegistry::snapshot`].
#[derive(Debug, Clone)]
pub struct ClientView {
    pub client_id: String,
    pub session_id: Option<String>,
}

/// Cached heartbeat-probe result for an on-disk session this process
/// doesn't own a `Client` for.
#[derive(Debug, Clone, Copy)]
struct Liveness {
    alive: bool,
    #[allow(dead_code)]
    checked_at: Instant,
}

/// Process-global registry: live in-process clients + background liveness
/// cache for foreign on-disk sessions. Use [`ClientRegistry::global`].
/// Cached snapshot of the on-disk session list plus the sessions-dir
/// mtime it was taken from. The mtime lets readers detect create/delete
/// churn (including from other jet processes) without re-parsing every
/// `session.json` on the hot path.
#[derive(Clone)]
struct MetaCache {
    metas: Arc<Vec<SessionMeta>>,
    dir_mtime: Option<SystemTime>,
}

pub struct ClientRegistry {
    clients: RwLock<HashMap<String, ClientHandle>>,
    cache: RwLock<HashMap<String, Liveness>>,
    /// Cached result of `SessionStore::default().list()` paired with the
    /// sessions-dir mtime at scan time. Refreshed each poll tick and
    /// on-demand when a reader spots a newer mtime. `None` until the
    /// first read lands.
    metas: RwLock<Option<MetaCache>>,
}

impl ClientRegistry {
    /// The process-global registry. The background liveness poller is
    /// spawned the first time this is called; subsequent calls just
    /// hand back the same static.
    ///
    /// The CLI never touches this (it's a one-shot process and uses the
    /// async `SessionStore::list_filtered` path directly), so the poller
    /// only runs in long-lived embeddings — today, the lua cdylib.
    pub fn global() -> &'static Self {
        static REG: Lazy<&'static ClientRegistry> = Lazy::new(|| {
            let reg: &'static ClientRegistry = Box::leak(Box::new(ClientRegistry {
                clients: RwLock::new(HashMap::new()),
                cache: RwLock::new(HashMap::new()),
                metas: RwLock::new(None),
            }));
            // Seed the meta cache off-thread so `global()` returns
            // immediately — even with thousands of on-disk sessions,
            // module load stays cheap. `list_filtered_cached()` falls
            // back to a live read while the cache is still empty, so
            // callers that race the seed just pay the read once.
            runtime().spawn_blocking(move || {
                if let Ok(store) = SessionStore::default()
                    && let Ok(metas) = store.list()
                {
                    let dir_mtime = dir_mtime(store.dir());
                    *reg.metas.write().unwrap() = Some(MetaCache {
                        metas: Arc::new(metas),
                        dir_mtime,
                    });
                }
            });
            runtime().spawn(reg.poll_loop());
            reg
        });
        *REG
    }

    pub fn insert(&self, client_id: String, handle: ClientHandle) {
        self.clients.write().unwrap().insert(client_id, handle);
    }

    pub fn remove(&self, client_id: &str) -> Option<ClientHandle> {
        self.clients.write().unwrap().remove(client_id)
    }

    pub fn get(&self, client_id: &str) -> Option<ClientHandle> {
        self.clients.read().unwrap().get(client_id).cloned()
    }

    /// Like [`Self::get`] but returns an error with the standard
    /// "no kernel with session id …" message when the id isn't
    /// registered. Convenience for lua/CLI callers that want to bail
    /// with `?` on a missing handle.
    pub fn require(&self, client_id: &str) -> anyhow::Result<ClientHandle> {
        self.get(client_id)
            .ok_or_else(|| anyhow::anyhow!("no kernel with session id {client_id}"))
    }

    /// Snapshot of every registered client. Reads each client's
    /// `session_id()` via `try_lock` so a locked handle doesn't block the
    /// caller; the field just shows up as `None` in that case.
    pub fn snapshot(&self) -> Vec<ClientView> {
        let map = self.clients.read().unwrap();
        map.iter()
            .map(|(client_id, handle)| {
                let session_id = handle
                    .try_lock()
                    .ok()
                    .and_then(|c| c.session_id().map(str::to_string));
                ClientView {
                    client_id: client_id.clone(),
                    session_id,
                }
            })
            .collect()
    }

    /// Session ids this process has a live client for. Filters out
    /// clients whose `watch_status()` has reached `Exited` — a shutdown
    /// that hasn't yet been cleaned up from the registry (e.g. the
    /// child-wait watcher fired but no one has called `remove` yet)
    /// shouldn't count as alive.
    pub fn live_session_ids(&self) -> HashSet<String> {
        let map = self.clients.read().unwrap();
        let mut out = HashSet::with_capacity(map.len());
        for handle in map.values() {
            let Ok(client) = handle.try_lock() else {
                // Someone's mid-call. Assume alive — a wedged client
                // would have flipped Exited by now and freed the lock
                // for the exit watcher's transition.
                continue;
            };
            if *client.watch_status().borrow() == KernelStatus::Exited {
                continue;
            }
            if let Some(sid) = client.session_id() {
                out.insert(sid.to_string());
            }
        }
        out
    }

    /// Read a cached liveness result for `session_id`, if the background
    /// poller has probed it. Returns `None` when the entry doesn't exist
    /// yet — the caller can decide whether to treat that as "assume
    /// alive" (matches the on-disk status) or trigger a fresh probe.
    pub fn cached_alive(&self, session_id: &str) -> Option<bool> {
        self.cache.read().unwrap().get(session_id).map(|l| l.alive)
    }

    /// Patch the cached [`SessionMeta`] for `session_id` to `Closed`.
    /// Called after `Session::mark_closed` writes the change to disk on
    /// graceful shutdown: the sessions-dir mtime doesn't move for
    /// in-file writes, so without this the cache would keep serving the
    /// stale Open status until the next 1Hz poll tick. No-op if the
    /// cache hasn't been populated yet or the id isn't present.
    pub fn mark_meta_closed(&self, session_id: &str) {
        let mut guard = self.metas.write().unwrap();
        let Some(cache) = guard.as_ref() else { return };
        if !cache.metas.iter().any(|m| m.session_id == session_id) {
            return;
        }
        let mut metas = (*cache.metas).clone();
        for m in &mut metas {
            if m.session_id == session_id {
                m.status = SessionStatus::Closed;
            }
        }
        *guard = Some(MetaCache {
            metas: Arc::new(metas),
            dir_mtime: cache.dir_mtime,
        });
    }

    /// Snapshot of the on-disk `SessionMeta` list. Returns the cached
    /// list when the sessions-dir mtime is unchanged since the last
    /// scan; otherwise re-scans synchronously and updates the cache.
    /// Errors only if the underlying `SessionStore::default()` /
    /// `list()` fail.
    pub fn cached_metas(&self) -> anyhow::Result<Arc<Vec<SessionMeta>>> {
        let store = SessionStore::default()?;
        let current_mtime = dir_mtime(store.dir());
        {
            let guard = self.metas.read().unwrap();
            if let Some(cache) = guard.as_ref()
                && cache.dir_mtime == current_mtime
            {
                return Ok(cache.metas.clone());
            }
        }
        // mtime changed (or cache empty) — rescan and publish.
        let cache = MetaCache {
            metas: Arc::new(store.list()?),
            dir_mtime: current_mtime,
        };
        let arc = cache.metas.clone();
        *self.metas.write().unwrap() = Some(cache);
        Ok(arc)
    }

    async fn poll_loop(&'static self) {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(e) = self.refresh_once().await {
                log::debug!("liveness poll: {e}");
            }
        }
    }

    /// One tick of the poller. Probes every Open on-disk session this
    /// process doesn't own a `Client` for, updates the cache, and flips
    /// dead sessions to `Closed` on disk. Public for testing.
    pub async fn refresh_once(&self) -> anyhow::Result<()> {
        let store = SessionStore::default()?;
        let metas = store.list()?;
        let dir_mtime = dir_mtime(store.dir());
        *self.metas.write().unwrap() = Some(MetaCache {
            metas: Arc::new(metas.clone()),
            dir_mtime,
        });
        let owned = self.live_session_ids();

        let mut tasks: JoinSet<(String, bool)> = JoinSet::new();
        let mut candidate_ids: HashSet<String> = HashSet::new();
        for meta in metas
            .into_iter()
            .filter(|m| m.status == SessionStatus::Open)
            .filter(|m| !owned.contains(&m.session_id))
        {
            let session_id = meta.session_id.clone();
            candidate_ids.insert(session_id.clone());
            let store_dir = store.dir().to_path_buf();
            tasks.spawn(async move {
                let alive = probe_session(&store_dir, &meta).await;
                (session_id, alive)
            });
        }

        let mut results = Vec::new();
        while let Some(joined) = tasks.join_next().await {
            if let Ok(r) = joined {
                results.push(r);
            }
        }

        // Update cache: drop entries for sessions no longer on disk, then
        // insert fresh results.
        {
            let mut cache = self.cache.write().unwrap();
            cache.retain(|k, _| candidate_ids.contains(k) || owned.contains(k));
            let now = Instant::now();
            for (sid, alive) in &results {
                cache.insert(
                    sid.clone(),
                    Liveness {
                        alive: *alive,
                        checked_at: now,
                    },
                );
            }
        }

        // Persist Open → Closed transitions so cross-process readers agree.
        for (sid, alive) in results {
            if !alive {
                log::warn!("liveness: session {sid} appears dead, marking closed");
                if let Ok(mut s) = store.open(&sid) {
                    s.mark_closed();
                }
            }
        }
        Ok(())
    }
}

/// Directory mtime, or `None` if the dir doesn't exist / can't be
/// stat'd. Used as a cheap change-detector for the sessions dir:
/// creating or removing an entry bumps this on macOS/Linux, so a
/// matching mtime means the entry set is unchanged since the last scan.
/// (In-place writes to files *inside* the dir don't bump it — the 1s
/// poll picks those up.)
fn dir_mtime(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir).ok()?.modified().ok()
}

/// Heartbeat probe with a short timeout. `false` on any error/timeout,
/// so the caller can treat it as "not answering."
async fn probe_session(data_dir: &std::path::Path, meta: &crate::manager::SessionMeta) -> bool {
    let conn_path = data_dir.join(&meta.session_id).join(&meta.connection_file);
    let Ok(info) = connection_file::read(&conn_path) else {
        return false;
    };
    probe_kernel_alive(&info).await.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::session::Session;
    use std::path::Path;
    use tempfile::TempDir;

    fn create(store: &SessionStore, lang: &str, cwd: &Path) -> Session {
        store.create(lang, "kernel", Path::new("/k"), cwd).unwrap()
    }

    #[tokio::test]
    async fn refresh_marks_dead_sessions_closed() {
        let data = TempDir::new().unwrap();
        // Force default() to point at the tempdir.
        // SAFETY: single-threaded test.
        // SessionStore::default() reads XDG_DATA_HOME; override it.
        unsafe {
            std::env::set_var("XDG_DATA_HOME", data.path());
        }
        let store = SessionStore::at(data.path().join("jet"));
        std::fs::create_dir_all(store.dir()).unwrap();
        let cwd = std::path::PathBuf::from("/tmp/registry");
        let s1 = create(&store, "python", &cwd);
        connection_file::generate(&s1.connection_file_path()).unwrap();

        // Fresh registry (bypass global — global is a singleton across tests).
        let reg = ClientRegistry {
            clients: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
            metas: RwLock::new(None),
        };
        reg.refresh_once().await.unwrap();

        // Cache should record it as dead (no kernel behind the ports),
        // and the on-disk status should have flipped.
        let sid = s1.meta().session_id.clone();
        assert_eq!(reg.cached_alive(&sid), Some(false));
        let reopened = SessionStore::at(data.path().join("jet"))
            .open(&sid)
            .unwrap();
        assert_eq!(reopened.meta().status, SessionStatus::Closed);
    }
}

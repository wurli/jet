//! Probe unmanaged Jupyter kernels by their connection files.
//!
//! Enumerates connection files under the Jupyter runtime directory (or
//! takes an explicit path), heartbeat-probes each for liveness, and —
//! when alive and we have no cached reply — sends a one-shot
//! `kernel_info_request` to learn what the kernel is. Replies are cached
//! on disk keyed by the absolute connection-file path; liveness is never
//! cached.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use jupyter_protocol::{ConnectionInfo, JupyterMessage, KernelInfoRequest};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::task::JoinSet;

use crate::connection_file;
use crate::jupyter_zmq_client::{
    create_client_iopub_connection, create_client_shell_connection_with_identity,
    peer_identity_for_session,
};
use crate::kernel::probe_kernel_alive;

/// Bounded time we'll wait for `kernel_info_reply` on a live kernel.
/// Short by design — `jet list-external` should be snappy, and a wedged-
/// but-heartbeat-live kernel shouldn't hold up the whole listing.
const KERNEL_INFO_TIMEOUT: Duration = Duration::from_secs(3);

/// Result of probing one external kernel.
#[derive(Debug, Clone, Serialize)]
pub struct ExternalKernelReport {
    pub connection_file_path: PathBuf,
    pub alive: bool,
    /// Parsed connection info. `None` when the file failed to parse
    /// (`error` is set in that case).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_file: Option<ConnectionInfo>,
    /// `kernel_info_reply.content` as JSON. Sourced from the cache when
    /// present (whether the kernel is alive or not); otherwise fetched
    /// fresh when alive. `None` when we have no cached reply and either
    /// the kernel is dead or the info request errored/timed out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_info: Option<Value>,
    /// Populated when parsing the connection file failed, or when the
    /// info request errored/timed out on a live kernel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Enumerate `*.json` files under the Jupyter runtime directory that
/// parse as Jupyter kernel connection files. Non-connection-file JSON
/// (e.g. `nbserver-*.json`) is silently skipped. A missing runtime dir
/// yields an empty list. See [`crate::jupyter_dirs::jupyter_runtime_dir`]
/// for the directory resolution rules.
pub fn discover_connection_files() -> Result<Vec<PathBuf>> {
    let dir = crate::jupyter_dirs::jupyter_runtime_dir()?;
    let read_dir = match std::fs::read_dir(&dir) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(anyhow!("read_dir {}: {e}", dir.display())),
    };
    let mut out = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if connection_file::read(&path).is_ok() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Probe one connection file: heartbeat + (if alive and uncached) kernel_info.
/// Cache-aware: a cached `kernel_info` is used as-is and reported whether
/// or not the kernel is alive.
pub async fn probe_external(path: &Path) -> ExternalKernelReport {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let info = match connection_file::read(&abs) {
        Ok(i) => i,
        Err(e) => {
            return ExternalKernelReport {
                connection_file_path: abs,
                alive: false,
                connection_file: None,
                kernel_info: None,
                error: Some(format!("{e:#}")),
            };
        }
    };

    let alive = probe_kernel_alive(&info).await.is_ok();
    let cached = load_cached_kernel_info(&abs);

    let (kernel_info, error) = if let Some(cached) = cached {
        // Cache hit — skip the round-trip regardless of liveness.
        log::debug!("external: cache hit for {}", abs.display());
        (Some(cached), None)
    } else if alive {
        match fetch_kernel_info(&info).await {
            Ok(v) => {
                if let Err(e) = store_cached_kernel_info(&abs, &v) {
                    log::warn!(
                        "external: failed to cache kernel_info for {}: {e:#}",
                        abs.display()
                    );
                }
                (Some(v), None)
            }
            Err(e) => (None, Some(format!("{e:#}"))),
        }
    } else {
        (None, None)
    };

    ExternalKernelReport {
        connection_file_path: abs,
        alive,
        connection_file: Some(info),
        kernel_info,
        error,
    }
}

/// Probe many connection files in parallel. Output order matches input order.
/// When `include_closed` is `false`, dead kernels are dropped from the result.
pub async fn probe_external_many(paths: &[PathBuf], include_closed: bool) -> Vec<ExternalKernelReport> {
    let mut set: JoinSet<(usize, ExternalKernelReport)> = JoinSet::new();
    for (idx, path) in paths.iter().enumerate() {
        let path = path.clone();
        set.spawn(async move {
            let report = probe_external(&path).await;
            (idx, report)
        });
    }
    let mut collected: Vec<Option<ExternalKernelReport>> = (0..paths.len()).map(|_| None).collect();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((idx, report)) => collected[idx] = Some(report),
            Err(e) => log::warn!("external probe task panicked: {e}"),
        }
    }
    // Any slot still None (task panicked) becomes a stub error entry so
    // the output length still matches paths.
    collected
        .into_iter()
        .zip(paths.iter())
        .map(|(slot, path)| {
            slot.unwrap_or_else(|| ExternalKernelReport {
                connection_file_path: path.clone(),
                alive: false,
                connection_file: None,
                kernel_info: None,
                error: Some("probe task panicked".to_string()),
            })
        })
        .filter(|r| include_closed || r.alive)
        .collect()
}

/// One-shot `kernel_info_request`: open shell + iopub, send the request,
/// wait for the matching reply within [`KERNEL_INFO_TIMEOUT`], return
/// its `content` as JSON. Sockets are dropped on return. Modelled on
/// `client::handshake` but standalone — no `Client`/`FrameRouter` needed
/// for a one-shot probe.
async fn fetch_kernel_info(info: &ConnectionInfo) -> Result<Value> {
    let session_id = format!("jet-external-{:x}", rand::thread_rng().r#gen::<u64>());
    let identity =
        peer_identity_for_session(&session_id).map_err(|e| anyhow!("peer_identity: {e}"))?;

    let mut shell =
        create_client_shell_connection_with_identity(info, &session_id, identity.clone())
            .await
            .map_err(|e| anyhow!("shell start: {e}"))?;
    let mut iopub = create_client_iopub_connection(info, "", &session_id)
        .await
        .map_err(|e| anyhow!("iopub start: {e}"))?;

    let req: JupyterMessage = KernelInfoRequest {}.into();
    let info_id = req.header.msg_id.clone();
    shell
        .send(req)
        .await
        .map_err(|e| anyhow!("shell.send (kernel_info): {e}"))?;

    let wait = async {
        loop {
            tokio::select! {
                biased;
                read = shell.read() => {
                    let msg = read.map_err(|e| anyhow!("shell.read: {e}"))?;
                    let parent = msg.parent_header.as_ref().map(|h| h.msg_id.as_str()).unwrap_or("");
                    if parent == info_id && msg.message_type() == "kernel_info_reply" {
                        return serde_json::to_value(&msg.content)
                            .map_err(|e| anyhow!("serialize kernel_info_reply: {e}"));
                    }
                }
                // Drain iopub so its HWM doesn't back up the kernel while we wait.
                read = iopub.read() => {
                    let _ = read.map_err(|e| anyhow!("iopub.read: {e}"))?;
                }
            }
        }
    };
    tokio::time::timeout(KERNEL_INFO_TIMEOUT, wait)
        .await
        .map_err(|_| {
            anyhow!(
                "timed out waiting for kernel_info_reply after {}s",
                KERNEL_INFO_TIMEOUT.as_secs()
            )
        })?
}

// --- Cache ---------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    connection_file_path: PathBuf,
    kernel_info: Value,
}

fn cache_dir() -> Result<PathBuf> {
    Ok(crate::manager::jet_data_dir()?.join("external-cache"))
}

/// Filename for a cache entry: hex-encoded absolute path. Deterministic,
/// filesystem-safe, and human-inspectable via `xxd`/`hex -d`.
fn cache_filename(abs_path: &Path) -> PathBuf {
    let bytes = abs_path.as_os_str().to_string_lossy();
    let name = hex::encode(bytes.as_bytes());
    PathBuf::from(format!("{name}.json"))
}

fn load_cached_kernel_info(abs_path: &Path) -> Option<Value> {
    let dir = cache_dir().ok()?;
    let path = dir.join(cache_filename(abs_path));
    let bytes = std::fs::read(&path).ok()?;
    let entry: CacheEntry = serde_json::from_slice(&bytes).ok()?;
    Some(entry.kernel_info)
}

fn store_cached_kernel_info(abs_path: &Path, kernel_info: &Value) -> Result<()> {
    let dir = cache_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating cache dir {}", dir.display()))?;
    let path = dir.join(cache_filename(abs_path));
    let entry = CacheEntry {
        connection_file_path: abs_path.to_path_buf(),
        kernel_info: kernel_info.clone(),
    };
    let json = serde_json::to_vec_pretty(&entry).map_err(|e| anyhow!("serialize cache: {e}"))?;
    std::fs::write(&path, json).with_context(|| format!("writing cache {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jupyter_protocol::Transport;
    use serde_json::json;
    use serial_test::serial;

    fn tmpdir(prefix: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "jet-external-test-{prefix}-{:x}",
            rand::thread_rng().r#gen::<u64>()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn fake_conn_file(dir: &Path, name: &str) -> PathBuf {
        let info = ConnectionInfo {
            ip: "127.0.0.1".to_string(),
            transport: Transport::TCP,
            shell_port: 1,
            iopub_port: 2,
            stdin_port: 3,
            control_port: 4,
            hb_port: 5,
            key: "deadbeef".to_string(),
            signature_scheme: "hmac-sha256".to_string(),
            kernel_name: None,
        };
        let path = dir.join(name);
        std::fs::write(&path, serde_json::to_vec(&info).unwrap()).unwrap();
        path
    }

    #[test]
    fn cache_filename_is_stable_and_reversible() {
        let p = Path::new("/tmp/kernel-42.json");
        let n1 = cache_filename(p);
        let n2 = cache_filename(p);
        assert_eq!(n1, n2);
        // Different paths → different filenames.
        assert_ne!(cache_filename(Path::new("/tmp/kernel-43.json")), n1);
    }

    #[test]
    #[serial]
    fn discover_skips_non_connection_files() {
        let dir = tmpdir("discover");
        let real = fake_conn_file(&dir, "kernel-1.json");
        std::fs::write(dir.join("nbserver-99.json"), r#"{"url":"x","token":"y"}"#).unwrap();
        std::fs::write(dir.join("not-json.txt"), "hello").unwrap();

        // SAFETY: single-threaded test; we restore afterwards.
        let prev = std::env::var_os("JUPYTER_RUNTIME_DIR");
        unsafe { std::env::set_var("JUPYTER_RUNTIME_DIR", &dir) };
        let found = discover_connection_files().unwrap();
        match prev {
            Some(v) => unsafe { std::env::set_var("JUPYTER_RUNTIME_DIR", v) },
            None => unsafe { std::env::remove_var("JUPYTER_RUNTIME_DIR") },
        }

        assert_eq!(found.len(), 1);
        assert_eq!(found[0], real);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn probe_dead_connection_reports_alive_false() {
        let dir = tmpdir("dead");
        let path = fake_conn_file(&dir, "kernel-x.json");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let report = rt.block_on(probe_external(&path));
        assert!(!report.alive, "dead kernel should report alive=false");
        assert!(report.connection_file.is_some());
        // No cache entry → dead → kernel_info stays None.
        assert!(report.kernel_info.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[serial]
    fn cached_kernel_info_is_reported_even_when_dead() {
        // Redirect XDG_DATA_HOME so the cache doesn't touch the real
        // jet data dir.
        let cache_root = tmpdir("cache-root");
        let dir = tmpdir("cache-dead");
        let path = fake_conn_file(&dir, "kernel-y.json");
        let abs = std::fs::canonicalize(&path).unwrap();

        // SAFETY: single-threaded test; restored below.
        let prev_xdg = std::env::var_os("XDG_DATA_HOME");
        unsafe { std::env::set_var("XDG_DATA_HOME", &cache_root) };

        store_cached_kernel_info(&abs, &json!({"language_info": {"name": "python"}})).unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let report = rt.block_on(probe_external(&abs));

        match prev_xdg {
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }

        assert!(!report.alive);
        let info = report.kernel_info.expect("cached kernel_info should surface");
        assert_eq!(info["language_info"]["name"], "python");

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&cache_root).ok();
    }
}

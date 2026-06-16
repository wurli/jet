//! kcserver process lifecycle and connection file.

use std::future::Future;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use rand::Rng;
use serde::Deserialize;

async fn poll_until<F, Fut, T>(deadline: Instant, interval: Duration, mut f: F) -> Option<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    while Instant::now() < deadline {
        if let Some(v) = f().await {
            return Some(v);
        }
        tokio::time::sleep(interval).await;
    }
    None
}

#[derive(Debug, Deserialize)]
pub struct ConnectionFile {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub base_path: Option<String>,
    pub bearer_token: String,
}

impl ConnectionFile {
    pub fn read(path: &Path) -> Result<Self> {
        let mut s = String::new();
        std::fs::File::open(path)
            .with_context(|| format!("opening {path:?}"))?
            .read_to_string(&mut s)?;
        Ok(serde_json::from_str(&s)?)
    }
}

/// RAII wrapper that kills the spawned kcserver on drop, unless detached.
pub struct ChildGuard {
    child: std::process::Child,
    detached: bool,
}

impl ChildGuard {
    pub fn new(child: std::process::Child) -> Self {
        Self {
            child,
            detached: false,
        }
    }

    /// Leave the child running when this guard is dropped.
    pub fn detach(&mut self) {
        self.detached = true;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.detached {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub async fn spawn_kcserver(
    bin: &str,
    connection_file: Option<PathBuf>,
    persist: bool,
) -> Result<(ConnectionFile, ChildGuard)> {
    let conn_path = connection_file.unwrap_or_else(|| {
        let path =
            std::env::temp_dir().join(format!("jet-kc-{:x}.json", rand::thread_rng().gen::<u64>()));
        // Make sure stale file doesn't trick us.
        let _ = std::fs::remove_file(&path);
        path
    });

    // When kcserver may outlive jet (`--persist`), we must inherit our
    // stderr fd directly: a piped stderr would close on jet's exit and any
    // later kcserver output (e.g. its zeromq panic on shutdown) would be
    // lost. When jet owns the lifetime, pipe + prefix each line with
    // `[kcserver]` so the source is clear in mixed output.
    let stderr_setting = if persist {
        Stdio::inherit()
    } else {
        Stdio::piped()
    };

    log::debug!("spawning {bin} with connection file {conn_path:?}");
    let mut command = Command::new(bin);
    command
        .arg("--connection-file")
        .arg(&conn_path)
        .arg("--transport")
        .arg("tcp")
        .stdout(Stdio::null())
        .stderr(stderr_setting);
    // Put kcserver — and the kernels it spawns — in a new process group.
    // Otherwise they share jet's foreground pgrp and a ^C at the tty
    // (cooked-mode SIGINT to the whole pgrp) kills the kernel before our
    // interrupt_session HTTP call can reach it.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn {bin}"))?;

    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let stderr_out = std::io::stderr();
            for line in reader.lines().map_while(|r| r.ok()) {
                let mut h = stderr_out.lock();
                let _ = writeln!(h, "[kcserver] {line}");
            }
            // Reaching here means kcserver closed its stderr — usually
            // because the process exited. If jet is still running, log this
            // so a silent kcserver death is at least visible in jet's logs.
            log::warn!("[kcserver] stderr closed (likely process exit)");
        });
    }

    let guard = ChildGuard::new(child);

    let deadline = Instant::now() + Duration::from_secs(10);
    let conn = poll_until(deadline, Duration::from_millis(100), || async {
        if !conn_path.exists() {
            return None;
        }
        // Give the server a moment to finish writing.
        tokio::time::sleep(Duration::from_millis(50)).await;
        ConnectionFile::read(&conn_path).ok()
    })
    .await;
    match conn {
        Some(c) => Ok((c, guard)),
        None => bail!("timed out waiting for kcserver connection file at {conn_path:?}"),
    }
}

pub async fn wait_for_status(api: &super::api::Client, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    match poll_until(deadline, Duration::from_millis(100), || async {
        api.server_status().await.ok().map(|_| ())
    })
    .await
    {
        Some(()) => Ok(()),
        None => bail!("kcserver /status never became ready"),
    }
}

/// Single-shot liveness check — does not poll.
pub async fn probe_status(api: &super::api::Client) -> Result<()> {
    api.server_status()
        .await
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("kcserver /status probe failed: {e}"))
}

//! kcserver process lifecycle and connection file.

use std::future::Future;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
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

/// RAII wrapper that kills the spawned kcserver on drop.
pub struct ChildGuard(std::process::Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub async fn spawn_kcserver(bin: &str) -> Result<(ConnectionFile, ChildGuard)> {
    let conn_path = std::env::temp_dir().join(format!(
        "jet-kc-{:x}.json",
        rand::thread_rng().gen::<u64>()
    ));
    // Make sure stale file doesn't trick us.
    let _ = std::fs::remove_file(&conn_path);

    log::debug!("spawning {bin} with connection file {conn_path:?}");
    let child = Command::new(bin)
        .arg("--connection-file")
        .arg(&conn_path)
        .arg("--transport")
        .arg("tcp")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {bin}"))?;
    let guard = ChildGuard(child);

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

pub async fn wait_for_status(api: &super::api::Client) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    match poll_until(deadline, Duration::from_millis(100), || async {
        api.server_status().await.ok().map(|_| ())
    })
    .await
    {
        Some(()) => Ok(()),
        None => bail!("kcserver /status never became ready"),
    }
}

//! kcserver process lifecycle and connection file.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use rand::Rng;
use serde::Deserialize;

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
    while Instant::now() < deadline {
        if conn_path.exists() {
            // Give the server a moment to finish writing.
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Ok(c) = ConnectionFile::read(&conn_path) {
                return Ok((c, guard));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("timed out waiting for kcserver connection file at {conn_path:?}");
}

pub async fn wait_for_status(http: &reqwest::Client, base: &str) -> Result<()> {
    let url = format!("{base}/status");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(r) = http.get(&url).send().await {
            if r.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("kcserver /status never became ready at {url}");
}

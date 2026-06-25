//! Generate a Jupyter kernel connection file (the per-kernel one with
//! `shell_port` / `iopub_port` / etc. + an HMAC `key`).
//!
//! Mirrors what kallichore's `kcshared` and `kcserver` do, just for jet's
//! single-kernel case: pick five free TCP ports via the bind-and-drop
//! pattern, generate a 16-byte hex HMAC key, write the JSON, hand the
//! `ConnectionInfo` back to the caller.

use std::net::TcpListener;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use jupyter_protocol::{ConnectionInfo, Transport};
use rand::Rng;

/// Bind a TCP listener on port 0, read the OS-assigned port, drop the
/// listener. There's a small race between this and the kernel binding the
/// same port — kallichore lives with it, so do we.
fn pick_free_port() -> Result<u16> {
    let listener =
        TcpListener::bind("127.0.0.1:0").context("bind ephemeral port to discover free port")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

/// Build a [`ConnectionInfo`] with a fresh HMAC key and five distinct free
/// ports, write it to `path`, and return it.
pub fn generate(path: &Path) -> Result<ConnectionInfo> {
    let mut ports = std::collections::HashSet::new();
    while ports.len() < 5 {
        ports.insert(pick_free_port()?);
    }
    let mut iter = ports.into_iter();
    let shell_port = iter.next().unwrap();
    let iopub_port = iter.next().unwrap();
    let stdin_port = iter.next().unwrap();
    let control_port = iter.next().unwrap();
    let hb_port = iter.next().unwrap();

    let key_bytes: [u8; 16] = rand::thread_rng().r#gen();
    let key = hex::encode(key_bytes);

    let info = ConnectionInfo {
        ip: "127.0.0.1".to_string(),
        transport: Transport::TCP,
        shell_port,
        iopub_port,
        stdin_port,
        control_port,
        hb_port,
        key,
        signature_scheme: "hmac-sha256".to_string(),
        kernel_name: None,
    };

    write_to(&info, path)?;
    Ok(info)
}

/// Read an existing connection file from disk.
pub fn read(path: &Path) -> Result<ConnectionInfo> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading connection file at {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing connection file at {}", path.display()))
}

fn write_to(info: &ConnectionInfo, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating dir {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(info).map_err(|e| anyhow!("serialize: {e}"))?;
    std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_writes_well_formed_file() {
        let tmp = tempfile_path("conn-gen");
        let info = generate(&tmp).unwrap();
        assert_eq!(info.ip, "127.0.0.1");
        assert_eq!(info.signature_scheme, "hmac-sha256");
        assert_eq!(info.key.len(), 32);
        let ports = [
            info.shell_port,
            info.iopub_port,
            info.stdin_port,
            info.control_port,
            info.hb_port,
        ];
        let unique: std::collections::HashSet<_> = ports.iter().copied().collect();
        assert_eq!(unique.len(), 5, "ports must be distinct");

        let round = read(&tmp).unwrap();
        assert_eq!(round.key, info.key);
        assert_eq!(round.shell_port, info.shell_port);
        let _ = std::fs::remove_file(&tmp);
    }

    fn tempfile_path(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "jet-test-{prefix}-{:x}.json",
            rand::thread_rng().r#gen::<u64>()
        ))
    }
}

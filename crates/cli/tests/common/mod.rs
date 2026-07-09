//! Shared helpers for the per-area integration test files
//! (`kernel_lifecycle.rs`, `sessions.rs`, `execute.rs`, `connect.rs`,
//! plus `snapshots.rs` which has its own copy because mixing snapshot
//! plumbing with general PTY plumbing is more friction than dedup is
//! worth there).
//!
//! ## Parallelism
//!
//! Tests run in parallel (cargo's default). Each one isolates state by:
//! - spawning jet with `XDG_DATA_HOME=scratch_xdg_dir()` so session
//!   storage is per-test;
//! - killing only its own kernel — by recorded pid where possible, else
//!   `pkill -f <connection-file-path>` which is unique to the test.
//!
//! Don't reintroduce `pkill -f ark` / `pkill -f ipykernel_launcher` —
//! those cross-kill concurrent tests.

#![allow(dead_code)] // each test file uses a subset

use std::process::Command;

use anyhow::Result;
use rand::Rng;

// ─────────────────────────────────────────────────────────────────────
// Skip / availability gates
// ─────────────────────────────────────────────────────────────────────

pub fn which(name: &str) -> Option<String> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

pub fn skip(reason: &str) {
    eprintln!("SKIP: {reason}");
}

/// True when `scripts/install-dev-kernels.sh` has been run and the
/// python3 kernelspec is available. Callers that need it should still
/// call `ensure_python_kernelspec()` to actually get the path.
pub fn ipykernel_available() -> bool {
    dev_kernel("python3").is_some()
}

/// Path to the repo's `kernels/` dir — populated by
/// `scripts/install-dev-kernels.sh`. Returns `None` if the layout doesn't
/// match a workspace checkout (e.g. running tests from a published crate).
fn repo_kernels_dir() -> Option<std::path::PathBuf> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/cli`; go up two.
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join("test-kernels");
    p.exists().then_some(p)
}

/// Locate a kernelspec provisioned by `scripts/install-dev-kernels.sh`
/// (`kernels/<name>/kernel.json` in the repo). Returns `None` when the
/// dev-kernel isn't present, so tests can skip with an instructive
/// message rather than failing.
pub fn dev_kernel(name: &str) -> Option<std::path::PathBuf> {
    let p = repo_kernels_dir()?.join(name).join("kernel.json");
    p.exists().then_some(p)
}

// ─────────────────────────────────────────────────────────────────────
// Kernelspec preparation
// ─────────────────────────────────────────────────────────────────────

/// Path to the dev-installed python3 kernelspec. Errors (which callers
/// convert to `skip`) when the install script hasn't been run — no
/// fallback to user installs or ambient python3.
pub fn ensure_python_kernelspec() -> Result<std::path::PathBuf> {
    dev_kernel("python3").ok_or_else(|| {
        anyhow::anyhow!("python3 kernelspec missing; run scripts/install-dev-kernels.sh")
    })
}

/// Path to the dev-installed ark kernelspec, or `None` if the install
/// script hasn't been run (test skips).
pub fn ark_kernelspec() -> Option<std::path::PathBuf> {
    dev_kernel("ark")
}

// ─────────────────────────────────────────────────────────────────────
// Test-local scratch state
// ─────────────────────────────────────────────────────────────────────

/// Find the single session subdir under `<xdg>/jet/` and parse its
/// session.json. Panics if there isn't exactly one.
pub fn read_only_session(xdg: &std::path::Path) -> serde_json::Value {
    let jet_dir = xdg.join("jet");
    let mut subdirs: Vec<std::path::PathBuf> = std::fs::read_dir(&jet_dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", jet_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    assert_eq!(
        subdirs.len(),
        1,
        "expected exactly one session subdir under {}, got {subdirs:?}",
        jet_dir.display()
    );
    let dir = subdirs.pop().unwrap();
    let bytes = std::fs::read(dir.join("session.json"))
        .unwrap_or_else(|e| panic!("read session.json: {e}"));
    serde_json::from_slice(&bytes).expect("parse session.json")
}

pub fn scratch_xdg_dir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "jet-xdg-test-{:x}",
        rand::thread_rng().r#gen::<u64>()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

// ─────────────────────────────────────────────────────────────────────
// jet process helpers
// ─────────────────────────────────────────────────────────────────────

/// Spawn `jet start <kernel_json>` with piped stdin and null stdout/stderr.
pub fn spawn_jet_start(
    kernel_json: &std::path::Path,
    xdg: &std::path::Path,
) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_jet"))
        .args(["start", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", xdg)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn jet start")
}

/// Spawn `jet attach --connection-file <conn>` with piped stdin and null
/// stdout/stderr.
pub fn spawn_jet_attach(conn: &std::path::Path, xdg: &std::path::Path) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_jet"))
        .args(["attach", "--connection-file", conn.to_str().unwrap()])
        .env("XDG_DATA_HOME", xdg)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn jet attach")
}

// ─────────────────────────────────────────────────────────────────────
// PTY plumbing
// ─────────────────────────────────────────────────────────────────────

/// Writer that the test caller and the PTY reader thread can share, so
/// the reader can answer reedline's cursor-position query while the
/// caller writes input.
pub struct SharedWriter(pub std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>);

impl std::io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// Spawn a reader thread that mirrors the PTY's output into `output`
/// and answers reedline's `ESC [ 6 n` cursor-position query with a
/// fixed `1;1R` reply (the PTY harness has no real terminal to do it).
/// Returns the caller-facing shared writer plus the reader join handle.
pub fn spawn_pty_reader(
    master: &dyn portable_pty::MasterPty,
) -> (
    Box<dyn std::io::Write + Send>,
    std::sync::Arc<std::sync::Mutex<String>>,
    std::thread::JoinHandle<()>,
) {
    use std::io::{Read, Write};
    let mut reader = master.try_clone_reader().expect("clone reader");
    let raw_writer = master.take_writer().expect("take writer");
    let shared = std::sync::Arc::new(std::sync::Mutex::new(raw_writer));
    let writer: Box<dyn std::io::Write + Send> = Box::new(SharedWriter(shared.clone()));
    let writer_for_reader = shared;
    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let output_clone = output.clone();
    let handle = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let chunk = &buf[..n];
                    if chunk.windows(4).any(|w| w == b"\x1b[6n") {
                        let mut w = writer_for_reader.lock().unwrap();
                        let _ = w.write_all(b"\x1b[1;1R");
                        let _ = w.flush();
                    }
                    let s = String::from_utf8_lossy(chunk).to_string();
                    output_clone.lock().unwrap().push_str(&s);
                }
            }
        }
    });
    (writer, output, handle)
}

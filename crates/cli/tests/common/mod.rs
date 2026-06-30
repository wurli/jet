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

use std::process::{Command, Stdio};

use anyhow::Result;
use rand::Rng;
use serde_json::json;

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

pub fn ipykernel_available() -> bool {
    Command::new("python3")
        .args(["-c", "import ipykernel"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────
// Kernelspec preparation
// ─────────────────────────────────────────────────────────────────────

pub fn ensure_python_kernelspec() -> Result<std::path::PathBuf> {
    let user = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/python3/kernel.json");
    if user.exists() {
        return Ok(user);
    }
    let python = which("python3").ok_or_else(|| anyhow::anyhow!("python3 not on PATH"))?;
    let dir = std::env::temp_dir().join(format!(
        "jet-test-kernelspec-{:x}",
        rand::thread_rng().r#gen::<u64>()
    ));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("kernel.json");
    let spec = json!({
        "argv": [python, "-m", "ipykernel_launcher", "-f", "{connection_file}"],
        "display_name": "Python (jet test)",
        "language": "python",
        "interrupt_mode": "signal",
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&spec)?)?;
    Ok(path)
}

/// Locate the user-installed ark R kernelspec, or `None` if it isn't
/// present (skip the test).
pub fn ark_kernelspec() -> Option<std::path::PathBuf> {
    let p = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/ark/kernel.json");
    if p.exists() { Some(p) } else { None }
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

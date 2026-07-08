//! Tests for the `jet execute` non-REPL subcommand.

#![allow(clippy::zombie_processes)]

use std::process::{Command, Stdio};
use std::time::Duration;

use rand::Rng;

mod common;
use common::*;

/// Spawn an ipykernel via `jet start --persist --connection-file <path>`
/// in a child process, then EOF its stdin to exit jet (kernel keeps
/// running). Returns the connection-file path; caller is responsible for
/// killing the kernel and cleaning up.
fn spawn_persisted_ipykernel() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return None;
        }
    };
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();
    let conn = std::env::temp_dir().join(format!(
        "jet-execute-test-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    let mut child = Command::new(bin)
        .args([
            "start",
            "--connection-file",
            &conn_str,
            "--persist",
            kernel_json.to_str().unwrap(),
        ])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet (persist)");
    std::thread::sleep(Duration::from_secs(3));
    drop(child.stdin.take());
    let _ = child.wait();
    assert!(conn.exists(), "connection file not written");
    Some((conn, xdg))
}

/// `jet execute --connection-file <path> "<code>"` streams kernel output
/// to stdout and exits once the kernel reports idle.
#[test]
fn execute_with_positional_code_streams_to_stdout() {
    let Some((conn, xdg)) = spawn_persisted_ipykernel() else {
        return;
    };
    let conn_str = conn.to_string_lossy().to_string();
    let bin = env!("CARGO_BIN_EXE_jet");

    let out = Command::new(bin)
        .args([
            "execute",
            "--connection-file",
            &conn_str,
            "--no-graphics",
            "print(6 * 7)",
        ])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::null())
        .output()
        .expect("run jet execute");

    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", &conn_str])
        .status();
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        out.status.success(),
        "jet execute failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("42"),
        "expected '42' in stdout, got: {stdout}",
    );
}

/// With no positional code, `jet execute` reads from stdin.
#[test]
fn execute_reads_code_from_stdin_when_positional_omitted() {
    let Some((conn, xdg)) = spawn_persisted_ipykernel() else {
        return;
    };
    let conn_str = conn.to_string_lossy().to_string();
    let bin = env!("CARGO_BIN_EXE_jet");

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["execute", "--connection-file", &conn_str, "--no-graphics"])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn jet execute");

    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin
        .write_all(b"print('hello-stdin')\n")
        .expect("write code");
    drop(stdin);

    let out = child.wait_with_output().expect("wait jet execute");

    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", &conn_str])
        .status();
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        out.status.success(),
        "jet execute failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("hello-stdin"),
        "expected 'hello-stdin' in stdout, got: {stdout}",
    );
}

/// A kernel-side error still surfaces on stdout and `jet execute` exits
/// (rather than hanging waiting for an idle that never comes — error
/// frames are followed by idle just like normal completions).
#[test]
fn execute_surfaces_kernel_error_and_exits() {
    let Some((conn, xdg)) = spawn_persisted_ipykernel() else {
        return;
    };
    let conn_str = conn.to_string_lossy().to_string();
    let bin = env!("CARGO_BIN_EXE_jet");

    let out = Command::new(bin)
        .args([
            "execute",
            "--connection-file",
            &conn_str,
            "--no-graphics",
            "raise RuntimeError('boom')",
        ])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::null())
        .output()
        .expect("run jet execute");

    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", &conn_str])
        .status();
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        out.status.success(),
        "jet execute failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("RuntimeError") && stdout.contains("boom"),
        "expected traceback for RuntimeError('boom') in stdout, got: {stdout}",
    );
}

/// `jet execute` with neither a session id nor `--connection-file` is a
/// usage error (no interactive picker for this command).
#[test]
fn execute_requires_session_or_connection_file() {
    let bin = env!("CARGO_BIN_EXE_jet");
    let out = Command::new(bin)
        .args(["execute", "print(1)"])
        .stdin(Stdio::null())
        .output()
        .expect("run jet execute");
    assert!(
        !out.status.success(),
        "jet execute with no target should fail",
    );
}

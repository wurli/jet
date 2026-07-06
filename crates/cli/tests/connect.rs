//! Tests for the `jet connect` subcommand and how parent/spec env merge.

#![allow(clippy::zombie_processes)]

use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::Result;
use rand::Rng;
use serde_json::json;

mod common;
use common::*;

/// Write a python kernelspec whose `env` field contains the given entries.
/// Used by the env-inheritance tests to seed a conflict between the spec
/// and the parent process's env.
fn write_python_kernelspec_with_env(env: &[(&str, &str)]) -> Result<std::path::PathBuf> {
    let python = which("python3").ok_or_else(|| anyhow::anyhow!("python3 not on PATH"))?;
    let dir = std::env::temp_dir().join(format!(
        "jet-test-kernelspec-env-{:x}",
        rand::thread_rng().r#gen::<u64>()
    ));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("kernel.json");
    let env_map: serde_json::Map<String, serde_json::Value> = env
        .iter()
        .map(|(k, v)| {
            (
                (*k).to_string(),
                serde_json::Value::String((*v).to_string()),
            )
        })
        .collect();
    let spec = json!({
        "argv": [python, "-m", "ipykernel_launcher", "-f", "{connection_file}"],
        "display_name": "Python (jet env test)",
        "language": "python",
        "interrupt_mode": "signal",
        "env": env_map,
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&spec)?)?;
    Ok(path)
}

/// Spawn `jet start --persist` with extra args and a parent env, then
/// EOF stdin to exit. Returns the connection file path on success.
fn spawn_persisted_with_env(
    kernel_json: &std::path::Path,
    xdg: &std::path::Path,
    parent_env: &[(&str, &str)],
) -> std::path::PathBuf {
    let bin = env!("CARGO_BIN_EXE_jet");
    let conn = std::env::temp_dir().join(format!(
        "jet-env-test-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    let mut cmd = Command::new(bin);
    cmd.args(["start", "--connection-file", &conn_str, "--persist"]);
    cmd.arg(kernel_json);
    cmd.env("XDG_DATA_HOME", xdg);
    for (k, v) in parent_env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet (persist)");
    std::thread::sleep(Duration::from_secs(3));
    drop(child.stdin.take());
    let _ = child.wait();
    assert!(conn.exists(), "connection file not written");
    conn
}

/// Run `jet execute` against an existing connection file and return the
/// stdout output as a UTF-8 string. Fails the test if execute exits non-zero.
fn run_execute(conn: &std::path::Path, xdg: &std::path::Path, code: &str) -> String {
    let bin = env!("CARGO_BIN_EXE_jet");
    let out = Command::new(bin)
        .args([
            "execute",
            "--connection-file",
            conn.to_str().unwrap(),
            "--no-graphics",
            code,
        ])
        .env("XDG_DATA_HOME", xdg)
        .stdin(Stdio::null())
        .output()
        .expect("run jet execute");
    assert!(
        out.status.success(),
        "jet execute failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `jet start` refuses to spawn if the target connection file already
/// exists. Error message tells the user how to reconnect — by session id
/// when the path resolves to a tracked session, by --connection-file
/// otherwise.
#[test]
fn connect_refuses_if_connection_file_exists() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();
    let conn = std::env::temp_dir().join(format!(
        "jet-collide-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    // Pre-create the connection file so start bails before kernel launch.
    std::fs::write(&conn, b"{}").unwrap();
    let conn_str = conn.to_string_lossy().to_string();

    let out = Command::new(bin)
        .args([
            "start",
            "--connection-file",
            &conn_str,
            kernel_json.to_str().unwrap(),
        ])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::null())
        .output()
        .expect("run jet start");

    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        !out.status.success(),
        "jet start should have failed; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("connection file already exists"),
        "expected 'connection file already exists' in stderr, got: {stderr}",
    );
    // Untracked path → suggestion uses --connection-file form.
    assert!(
        stderr.contains("--connection-file"),
        "expected --connection-file in reattach hint, got: {stderr}",
    );
}

/// Parent env should reach the kernel; the kernelspec's `env` field wins
/// on conflict (matches Jupyter convention — the spec author chose that
/// value intentionally).
#[test]
fn connect_inherits_parent_env_with_spec_winning_on_conflict() {
    if !ipykernel_available() {
        skip("ipykernel not installed");
        return;
    }
    let kernel_json = match write_python_kernelspec_with_env(&[
        ("JET_TEST_SPEC_ONLY", "from-spec"),
        ("JET_TEST_OVERRIDE", "from-spec"),
    ]) {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let xdg = scratch_xdg_dir();
    let parent_env = [
        ("JET_TEST_PARENT_ONLY", "from-parent"),
        ("JET_TEST_OVERRIDE", "from-parent"),
    ];
    let conn = spawn_persisted_with_env(&kernel_json, &xdg, &parent_env);

    let code = "import os; print('SPEC_ONLY=' + os.environ.get('JET_TEST_SPEC_ONLY','<unset>')); \
                print('PARENT_ONLY=' + os.environ.get('JET_TEST_PARENT_ONLY','<unset>')); \
                print('OVERRIDE=' + os.environ.get('JET_TEST_OVERRIDE','<unset>'))";
    let out = run_execute(&conn, &xdg, code);

    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", conn.to_str().unwrap()])
        .status();
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);
    let _ = std::fs::remove_dir_all(kernel_json.parent().unwrap());

    assert!(
        out.contains("SPEC_ONLY=from-spec"),
        "spec-only key missing from kernel env; got:\n{out}",
    );
    assert!(
        out.contains("PARENT_ONLY=from-parent"),
        "parent-only key not inherited into kernel env; got:\n{out}",
    );
    assert!(
        out.contains("OVERRIDE=from-spec"),
        "spec env did not override parent on conflict; got:\n{out}",
    );
}

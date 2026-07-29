//! Tests for the `jet connect` subcommand and how parent/spec env merge.

#![allow(clippy::zombie_processes)]

use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rand::Rng;

mod common;
use common::*;

/// Best-effort read of a file, returning "<empty>" or "<missing: ...>"
/// rather than propagating errors. Used to enrich CI failure messages.
fn read_or_placeholder(path: &std::path::Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) if s.trim().is_empty() => "<empty>".to_string(),
        Ok(s) => s,
        Err(e) => format!("<missing: {e}>"),
    }
}

/// Snapshot of everything worth showing when a persist-then-execute test
/// fails: jet's captured stderr from both invocations, the connection
/// file, and any kernel-side log jet may have written.
struct PersistedContext {
    persist_stderr: std::path::PathBuf,
    kernel_log: Option<std::path::PathBuf>,
}

impl PersistedContext {
    fn dump(&self, label: &str) -> String {
        let mut out = format!("=== {label} ===\n");
        out.push_str(&format!(
            "--- jet start --persist stderr ({}) ---\n{}\n",
            self.persist_stderr.display(),
            read_or_placeholder(&self.persist_stderr),
        ));
        if let Some(p) = &self.kernel_log {
            out.push_str(&format!(
                "--- kernel log ({}) ---\n{}\n",
                p.display(),
                read_or_placeholder(p),
            ));
        }
        out
    }
}

/// Spawn `jet start --persist` with extra args and a parent env, then
/// EOF stdin to exit. Returns the connection file path plus a diagnostic
/// context callers can dump on failure.
fn spawn_persisted_with_env(
    kernel_json: &std::path::Path,
    xdg: &std::path::Path,
    parent_env: &[(&str, &str)],
) -> (std::path::PathBuf, PersistedContext) {
    let bin = env!("CARGO_BIN_EXE_jet");
    let conn = std::env::temp_dir().join(format!(
        "jet-env-test-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();
    let stderr_path = std::env::temp_dir().join(format!(
        "jet-env-test-persist-stderr-{:x}.log",
        rand::thread_rng().r#gen::<u64>()
    ));
    let stderr_file = std::fs::File::create(&stderr_path).expect("open persist stderr file");

    let mut cmd = Command::new(bin);
    cmd.args(["start", "--connection-file", &conn_str, "--persist"]);
    cmd.arg(kernel_json);
    cmd.env("XDG_DATA_HOME", xdg);
    // Turn on jet's debug logging so the captured stderr actually tells
    // us something on CI (default is warn-and-above, which is silent for
    // a normal graceful path).
    cmd.env("RUST_LOG", "debug");
    for (k, v) in parent_env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .expect("spawn jet (persist)");
    // Wait for jet to write the connection file — proof it reached the
    // point of picking ports and generating the file. On a slow runner
    // this can take several seconds after spawn.
    let deadline = Instant::now() + Duration::from_secs(20);
    while !conn.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    // A brief additional wait so the REPL has landed on the readline
    // prompt (i.e. finished the kernel_info round-trip) before we EOF
    // its stdin. Closing stdin mid-startup can race the detach path.
    std::thread::sleep(Duration::from_secs(2));
    drop(child.stdin.take());
    let _ = child.wait();

    // Jet writes a kernel-side log next to the connection file as
    // `<conn>.log`. Include it in the diagnostic if it exists.
    let kernel_log = {
        let mut p = conn.clone();
        let ext = format!(
            "{}.log",
            p.extension().and_then(|e| e.to_str()).unwrap_or("")
        );
        p.set_extension(ext);
        if p.exists() { Some(p) } else { None }
    };
    let ctx = PersistedContext {
        persist_stderr: stderr_path,
        kernel_log,
    };

    assert!(
        conn.exists(),
        "connection file not written\n{}",
        ctx.dump("spawn_persisted_with_env"),
    );
    // The kernel should still be listening after jet detached. Poll the
    // shell port until it accepts a TCP start, so `jet execute` doesn't
    // race the kernel's post-detach settling.
    wait_for_kernel_reachable(&conn, Duration::from_secs(10), &ctx);
    (conn, ctx)
}

/// Read the shell port out of `conn` and poll TCP-start against it until
/// the kernel accepts or `timeout` elapses. Panics with a useful message
/// on timeout — the alternative is `run_execute` failing later with the
/// opaque "kernel not reachable" error.
fn wait_for_kernel_reachable(conn: &std::path::Path, timeout: Duration, ctx: &PersistedContext) {
    let info: serde_json::Value =
        serde_json::from_slice(&std::fs::read(conn).expect("read connection file"))
            .expect("parse connection file");
    let ip = info["ip"].as_str().unwrap_or("127.0.0.1");
    let port = info["shell_port"].as_u64().expect("shell_port");
    let addr = format!("{ip}:{port}");
    let deadline = Instant::now() + timeout;
    let mut last_err = String::from("never attempted");
    while Instant::now() < deadline {
        match TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_millis(200)) {
            Ok(_) => return,
            Err(e) => last_err = e.to_string(),
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "kernel never became reachable at {addr}: {last_err}\n{}",
        ctx.dump("wait_for_kernel_reachable"),
    );
}

/// Run `jet execute` against an existing connection file and return the
/// stdout output as a UTF-8 string. Fails the test if execute exits non-zero.
/// `ctx` is dumped on failure to enrich the diagnostic with jet's own logs.
fn run_execute(
    conn: &std::path::Path,
    xdg: &std::path::Path,
    code: &str,
    ctx: &PersistedContext,
) -> String {
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
        .env("RUST_LOG", "debug")
        .stdin(Stdio::null())
        .output()
        .expect("run jet execute");
    assert!(
        out.status.success(),
        "jet execute failed: stdout={} stderr={}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
        ctx.dump("run_execute"),
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
///
/// The dev python3 kernelspec bakes `JET_TEST_SPEC_VAR=from-spec` into
/// its `env` — we reuse that here rather than writing a bespoke spec, so
/// this test doesn't depend on whatever `python3` happens to be on PATH.
#[test]
fn connect_inherits_parent_env_with_spec_winning_on_conflict() {
    if !ipykernel_available() {
        skip("ipykernel not installed");
        return;
    }
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let xdg = scratch_xdg_dir();
    let parent_env = [
        // Not in the spec — should reach the kernel unchanged.
        ("JET_TEST_PARENT_VAR", "from-parent"),
        // Conflicts with the spec's JET_TEST_SPEC_VAR=from-spec; the
        // spec should win.
        ("JET_TEST_SPEC_VAR", "from-parent"),
    ];
    let (conn, ctx) = spawn_persisted_with_env(&kernel_json, &xdg, &parent_env);

    let code = "import os; print('SPEC_VAR=' + os.environ.get('JET_TEST_SPEC_VAR','<unset>')); \
                print('PARENT_VAR=' + os.environ.get('JET_TEST_PARENT_VAR','<unset>'))";
    let out = run_execute(&conn, &xdg, code, &ctx);

    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", conn.to_str().unwrap()])
        .status();
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        out.contains("PARENT_VAR=from-parent"),
        "parent env not inherited into kernel env; got:\n{out}",
    );
    assert!(
        out.contains("SPEC_VAR=from-spec"),
        "spec env did not override parent on conflict; got:\n{out}",
    );
}

//! Tests for jet's session.json bookkeeping: graceful exit marks the
//! session closed, --persist leaves it open, kernel pid is recorded,
//! `jet stop` shuts down a persisted kernel, and `--connection-file`
//! skips the session-storage path entirely.

#![allow(clippy::zombie_processes)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rand::Rng;

mod common;
use common::*;

/// Graceful exit (no --persist) should leave the session marked closed
/// and the kernel_pid cleared — pid only makes sense for live kernels.
#[test]
fn session_marked_closed_on_graceful_exit() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    let mut child = Command::new(bin)
        .args(["start", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    std::thread::sleep(Duration::from_secs(3));
    drop(child.stdin.take()); // EOF → graceful exit

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut exited = false;
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&xdg);
        panic!("jet did not exit within 15s");
    }

    let meta = read_only_session(&xdg);
    assert_eq!(
        meta["status"], "closed",
        "session not marked closed: {meta}"
    );
    assert!(meta["closed_at"].is_string(), "closed_at missing: {meta}");
    assert!(
        meta["kernel_pid"].is_null(),
        "kernel_pid should be cleared on close: {meta}"
    );
    assert_eq!(meta["language"], "python");
    let conn_path = std::path::PathBuf::from(meta["working_dir"].as_str().unwrap());
    assert!(conn_path.is_absolute());

    let _ = std::fs::remove_dir_all(&xdg);
}

/// The kernel pid must be written to session.json *while the REPL is running*,
/// not after the user quits. Earlier versions only wrote the pid after `drive_repl`
/// returned, so external readers (e.g. the nvim plugin polling `list_sessions`)
/// always saw a null pid during the kernel's actual lifetime. Regression test for
/// that timing bug — the pid we read mid-session must match a live process.
#[test]
fn session_records_kernel_pid_while_open() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    let mut child = Command::new(bin)
        .args(["start", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    // Give the REPL time to spawn the kernel and persist the pid, but don't
    // close stdin yet — we want to inspect session.json while it's still Open.
    std::thread::sleep(Duration::from_secs(3));

    let meta = read_only_session(&xdg);
    let cleanup = |child: &mut std::process::Child| {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&xdg);
    };

    assert_eq!(
        meta["status"], "open",
        "session should still be open mid-REPL: {meta}"
    );
    let Some(pid) = meta["kernel_pid"].as_i64() else {
        cleanup(&mut child);
        panic!("kernel_pid not recorded while session open: {meta}");
    };
    let pid = pid as i32;
    let alive = unsafe { libc::kill(pid, 0) } == 0;
    if !alive {
        cleanup(&mut child);
        panic!("kernel_pid {pid} recorded but process is not alive");
    }

    cleanup(&mut child);
}

/// With --persist, the session stays open and the kernel keeps running.
#[test]
fn session_left_open_with_persist() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    let mut child = Command::new(bin)
        .args(["start", "--persist", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    std::thread::sleep(Duration::from_secs(3));
    drop(child.stdin.take());

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut exited = false;
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&xdg);
        panic!("jet did not exit within 15s");
    }

    let meta = read_only_session(&xdg);
    assert_eq!(
        meta["status"], "open",
        "session unexpectedly closed: {meta}"
    );
    assert!(
        meta["closed_at"].is_null(),
        "closed_at set on persist: {meta}"
    );
    assert!(meta["kernel_pid"].is_number(), "kernel_pid missing: {meta}");

    // Kill the kernel by its recorded pid. Then `jet list` (which
    // probes) should flip the session to Closed even though jet never
    // observed the death. We deliberately avoid `pkill -f
    // ipykernel_launcher` here — under parallel test execution, that
    // pattern would cross-kill any other concurrent Python-kernel test.
    let kernel_pid = meta["kernel_pid"].as_i64().expect("kernel_pid recorded") as i32;
    unsafe {
        libc::kill(kernel_pid, libc::SIGKILL);
    }
    // kill is async; give the OS a beat to actually reap.
    std::thread::sleep(Duration::from_millis(500));

    let status = Command::new(bin)
        .args(["list-sessions", "--status", "all", "--all-dirs"])
        .env("XDG_DATA_HOME", &xdg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run jet list");
    assert!(status.success(), "jet list failed");

    let after = read_only_session(&xdg);
    assert_eq!(
        after["status"], "closed",
        "probe did not flip dead session to closed: {after}"
    );
    assert!(after["closed_at"].is_string());

    let _ = std::fs::remove_dir_all(&xdg);
}

/// If the jet parent process dies abruptly (terminal closed, SIGKILL,
/// crash) without --persist, the kernel must NOT outlive it. Drop on
/// `ChildGuard` won't run after SIGKILL, so the kernel layer has to
/// arrange its own death — otherwise every closed terminal leaks a
/// kernel that only `jet stop` can clean up.
#[test]
fn kernel_dies_when_parent_killed_without_persist() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    let mut child = Command::new(bin)
        .args(["start", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    std::thread::sleep(Duration::from_secs(3));

    let meta = read_only_session(&xdg);
    let kernel_pid = meta["kernel_pid"].as_i64().expect("kernel_pid recorded") as i32;
    assert_eq!(
        unsafe { libc::kill(kernel_pid, 0) },
        0,
        "kernel pid {kernel_pid} should be alive while jet is running"
    );

    // SIGKILL the jet parent — no chance for Drop or a Rust signal
    // handler to clean up. Simulates the harshest version of "terminal
    // closed": kernel must die anyway.
    let _ = child.kill();
    let _ = child.wait();

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut died = false;
    while Instant::now() < deadline {
        if unsafe { libc::kill(kernel_pid, 0) } != 0 {
            died = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if !died {
        unsafe {
            libc::kill(kernel_pid, libc::SIGKILL);
        }
    }
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        died,
        "kernel pid {kernel_pid} still alive 10s after jet parent killed — \
         kernel leaked instead of shutting down with its owner"
    );
}

/// `jet stop --connection-file <path>` attaches to a persisted kernel,
/// sends shutdown_request on control, and the kernel actually dies.
#[test]
fn jet_stop_shuts_down_persisted_kernel() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    // Spawn persisted: kernel survives jet's exit. No --connection-file —
    // that opts out of session tracking, which we need below to recover
    // the kernel pid.
    let mut spawn = Command::new(bin)
        .args(["start", "--persist", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet (persist)");
    std::thread::sleep(Duration::from_secs(3));
    drop(spawn.stdin.take()); // EOF → jet exits, kernel keeps running
    let _ = spawn.wait();

    // Grab the kernel pid AND the connection file path from the session
    // record so we can verify the kernel actually dies (not just that
    // jet stop returned 0).
    let meta = read_only_session(&xdg);
    let kernel_pid = meta["kernel_pid"].as_i64().expect("kernel_pid recorded") as i32;
    let conn = {
        let jet_dir = xdg.join("jet");
        let session_id = meta["session_id"].as_str().expect("session id");
        let rel = meta["connection_file"].as_str().expect("connection_file");
        jet_dir.join(session_id).join(rel)
    };
    let conn_str = conn.to_string_lossy().to_string();
    assert!(
        conn.exists(),
        "connection file {conn_str} should still exist after --persist"
    );
    // Sanity: pid is alive right now.
    assert_eq!(
        unsafe { libc::kill(kernel_pid, 0) },
        0,
        "expected persisted kernel pid {kernel_pid} to still be alive before stop"
    );

    // Run `jet stop --connection-file <conn>`.
    let status = Command::new(bin)
        .args(["stop", "--connection-file", &conn_str])
        .env("XDG_DATA_HOME", &xdg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run jet stop");
    assert!(status.success(), "jet stop exited non-zero: {status:?}");

    // The shutdown_request is best-effort: poll for the pid to disappear.
    // ipykernel exits cleanly on shutdown_request; if jet stop's control
    // channel were wired up wrong, the pid would linger here.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut died = false;
    while Instant::now() < deadline {
        if unsafe { libc::kill(kernel_pid, 0) } != 0 {
            died = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Cleanup: kill any straggler before asserting so we don't leak.
    if !died {
        unsafe {
            libc::kill(kernel_pid, libc::SIGKILL);
        }
    }
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        died,
        "kernel pid {kernel_pid} still alive 10s after `jet stop` — \
         shutdown_request was not delivered or kernel ignored it"
    );
}

/// `jet start --connection-file <path>` opts out of session tracking:
/// no session.json is written and `jet list-sessions` shows nothing.
#[test]
fn connect_with_connection_file_skips_session_tracking() {
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
        "jet-untracked-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    let mut child = Command::new(bin)
        .args([
            "start",
            "--connection-file",
            &conn_str,
            kernel_json.to_str().unwrap(),
        ])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    std::thread::sleep(Duration::from_secs(3));
    drop(child.stdin.take());

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut exited = false;
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&xdg);
        let _ = std::fs::remove_file(&conn);
        panic!("jet did not exit within 15s");
    }

    // No session dir should have been created under <xdg>/jet/.
    let jet_dir = xdg.join("jet");
    let subdirs: Vec<_> = std::fs::read_dir(&jet_dir)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    let _ = std::fs::remove_dir_all(&xdg);
    let _ = std::fs::remove_file(&conn);

    assert!(
        subdirs.is_empty(),
        "expected no session dirs under {} with --connection-file, got {subdirs:?}",
        jet_dir.display(),
    );
}

//! Tests for the kernel-liveness paths in jet's REPL:
//!  - ^C interrupts the running kernel without killing it;
//!  - jet exits cleanly on stdin EOF;
//!  - jet exits promptly when the kernel dies mid-execute (Python
//!    `os._exit`, R `quit()`, kernel `quit`-without-handshake);
//!  - state round-trips via `--persist` + `attach`.

#![allow(clippy::zombie_processes)]

use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::Result;
use rand::Rng;

mod common;
use common::*;

/// Drive the jet binary through a real PTY, send `code`, then send a
/// literal ^C byte. Returns the captured tty output until either the
/// REPL recovers a prompt or `timeout` elapses.
fn drive_jet_with_interrupt(
    code: &str,
    kernel_json: &std::path::Path,
    xdg: &std::path::Path,
    busy_grace: Duration,
    timeout: Duration,
) -> Result<String> {
    use portable_pty::{CommandBuilder, PtySize, native_pty_system};

    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows: 40,
            cols: 120,
            ..Default::default()
        })
        .expect("openpty");

    let bin = env!("CARGO_BIN_EXE_jet");
    let mut cmd = CommandBuilder::new(bin);
    cmd.args(["start", kernel_json.to_str().unwrap()]);
    cmd.env("XDG_DATA_HOME", xdg);
    cmd.cwd(std::env::current_dir()?);
    let mut child = pair.slave.spawn_command(cmd).expect("spawn jet under pty");
    drop(pair.slave);

    let (mut writer, output, reader_handle) = spawn_pty_reader(&*pair.master);

    let banner_deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < banner_deadline {
        if output.lock().unwrap().contains("> ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    writer.write_all(code.as_bytes())?;
    writer.flush()?;

    std::thread::sleep(busy_grace);

    writer.write_all(&[0x03])?;
    writer.flush()?;

    let deadline = Instant::now() + timeout;
    let interrupt_marker = "^C";
    let mut saw_interrupt = false;
    while Instant::now() < deadline {
        let s = output.lock().unwrap().clone();
        if !saw_interrupt && s.contains(interrupt_marker) {
            saw_interrupt = true;
        }
        if saw_interrupt
            && let Some(idx) = s.find(interrupt_marker)
            && s[idx + interrupt_marker.len()..].contains("> ")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let _ = writer.write_all(&[0x04]); // ^D
    let _ = writer.flush();
    drop(writer);
    let _ = child.wait();
    drop(pair.master);
    let _ = reader_handle.join();

    let result = output.lock().unwrap().clone();
    Ok(result)
}

#[test]
fn ctrl_c_interrupts_running_kernel_in_repl() {
    // ark on SIGINT exits, which surfaces the bug clearly: if ^C from the
    // tty propagates to the kernel's process group, the kernel dies and
    // jet prints "kernel exited". With the kernel in its own pgid, the
    // SIGINT only reaches jet, which forwards it via interrupt(); ark
    // survives.
    let Some(ark_kernel) = ark_kernelspec() else {
        skip("ark kernelspec missing; run scripts/install-dev-kernels.sh");
        return;
    };

    let xdg = scratch_xdg_dir();
    let out = drive_jet_with_interrupt(
        "Sys.sleep(30)\n",
        &ark_kernel,
        &xdg,
        Duration::from_secs(2),
        Duration::from_secs(15),
    )
    .expect("drive_jet_with_interrupt");
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        out.contains("^C"),
        "expected '^C' echo in jet output, got:\n{out}"
    );
    assert!(
        !out.contains("kernel exited"),
        "kernel exited after ^C — interrupt should have been delivered to \
         the kernel pgid, not killed it. Output:\n{out}"
    );
}

#[test]
fn jet_exits_on_eof() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let xdg = scratch_xdg_dir();
    let mut child = spawn_jet_start(&kernel_json, &xdg);

    std::thread::sleep(Duration::from_secs(3));
    drop(child.stdin.take());

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                let _ = std::fs::remove_dir_all(&xdg);
                return;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&xdg);
    panic!("jet did not exit within 10s after stdin closed");
}

/// Regression: ark's `quit()` (and any kernel that exits mid-execute
/// without sending an idle status) used to hang jet for 300s because
/// the inner `wait_for_idle` loop didn't watch the `closed` notify.
/// Simulate the same shape with `os._exit(0)` on ipykernel — it skips
/// the normal shutdown handshake, so no idle ever arrives.
#[test]
fn jet_exits_when_kernel_dies_mid_execute() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let xdg = scratch_xdg_dir();
    let mut child = spawn_jet_start(&kernel_json, &xdg);

    std::thread::sleep(Duration::from_secs(3));
    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin
        .write_all(b"import os; os._exit(0)\n")
        .expect("write to jet stdin");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut exited = false;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                exited = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }
    drop(stdin);
    let _ = std::fs::remove_dir_all(&xdg);
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "jet did not exit within 10s after the kernel died mid-execute \
             without sending idle — wait_for_idle is probably blocking on \
             a Closed signal it never receives"
        );
    }
}

#[test]
fn jet_exits_when_kernel_quits() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };
    let xdg = scratch_xdg_dir();
    let mut child = spawn_jet_start(&kernel_json, &xdg);

    std::thread::sleep(Duration::from_secs(3));

    // Record the kernel pid so we can verify it actually died (not just
    // that jet exited). session.json is cleared on Close, so read it now.
    let meta_before = read_only_session(&xdg);
    let kernel_pid = meta_before["kernel_pid"]
        .as_u64()
        .expect("kernel_pid recorded in session meta before quit()")
        as libc::pid_t;

    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin.write_all(b"quit()\n").expect("write to jet stdin");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut exited = false;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                exited = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }
    drop(stdin);
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&xdg);
        panic!("jet did not exit within 10s after the kernel quit");
    }

    // The kernel process itself should be gone (not merely that jet exited).
    // `kill(pid, 0)` returns 0 while the pid exists and -1/ESRCH otherwise.
    let alive = unsafe { libc::kill(kernel_pid, 0) } == 0;
    assert!(
        !alive,
        "kernel pid {kernel_pid} still alive after quit() — did the kernel actually shut down?"
    );

    // After jet noticed the kernel exit, the session should be marked closed.
    let meta = read_only_session(&xdg);
    assert_eq!(
        meta["status"], "closed",
        "kernel-quit path did not mark session closed: {meta}"
    );
    assert!(meta["closed_at"].is_string(), "closed_at missing: {meta}");
    let _ = std::fs::remove_dir_all(&xdg);
}

/// R's `quit()` cleanly closes the kernel sockets and exits. On the
/// spawn path the waitpid watcher catches this; this test guards that
/// path. (`jet_exits_when_kernel_quits` covers Python+ipykernel.)
#[test]
fn jet_exits_when_r_kernel_quits_spawn() {
    let Some(kernel_json) = ark_kernelspec() else {
        skip("ark kernelspec not found");
        return;
    };
    let xdg = scratch_xdg_dir();
    let mut child = spawn_jet_start(&kernel_json, &xdg);

    std::thread::sleep(Duration::from_secs(3));
    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin.write_all(b"quit()\n").expect("write to jet stdin");

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut exited = false;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                exited = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }
    drop(stdin);
    let _ = std::fs::remove_dir_all(&xdg);
    if !exited {
        let _ = child.kill();
        let _ = child.wait();
        panic!("jet did not exit within 15s after R kernel quit() (spawn path)");
    }
}

/// Regression for the attach-path liveness watcher (heartbeat). With no
/// child pid, jet relies on the heartbeat REQ/REP echo to detect that
/// the kernel has exited; without it, `quit()` would hang the REPL
/// indefinitely because ZMQ DEALER/SUB reads on a closed peer never
/// error.
#[test]
fn jet_exits_when_r_kernel_quits_attach() {
    let Some(kernel_json) = ark_kernelspec() else {
        skip("ark kernelspec not found");
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = std::env::temp_dir().join(format!(
        "jet-attach-quit-test-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    // Spawn persisted: get a kernel that survives jet exiting.
    {
        let mut child = Command::new(env!("CARGO_BIN_EXE_jet"))
            .args([
                "start",
                "--connection-file",
                &conn_str,
                "--persist",
                kernel_json.to_str().unwrap(),
            ])
            .env("XDG_DATA_HOME", &xdg)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn jet (persist)");
        std::thread::sleep(Duration::from_secs(3));
        let mut stdin = child.stdin.take().expect("stdin piped");
        // ^D to exit jet without quitting the kernel.
        stdin.write_all(&[0x04]).expect("write ^D");
        drop(stdin);
        let _ = child.wait();
    }
    assert!(
        conn.exists(),
        "connection file {conn_str} should still exist after --persist"
    );

    // Attach + quit().
    let mut attach = spawn_jet_attach(&conn, &xdg);

    std::thread::sleep(Duration::from_secs(3));
    let mut stdin = attach.stdin.take().expect("stdin piped");
    stdin.write_all(b"quit()\n").expect("write quit()");

    // Heartbeat poll cadence is 2s + up to 5s recv timeout, so allow
    // generous wall-clock for the dead detection to fire.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut exited = false;
    while Instant::now() < deadline {
        match attach.try_wait() {
            Ok(Some(_)) => {
                exited = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }
    drop(stdin);
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);
    if !exited {
        let _ = attach.kill();
        let _ = attach.wait();
        // Also try to clear any straggler kernel — scope by connection
        // file path so we don't cross-kill other ark tests running in
        // parallel.
        let _ = std::process::Command::new("pkill")
            .args(["-9", "-f", &conn_str])
            .status();
        panic!(
            "jet did not exit within 20s after R kernel quit() on attach path \
             — heartbeat liveness watcher should have noticed the closed socket"
        );
    }
}

/// Connect with `--persist`, set a variable, exit. Then `attach` to the
/// connection file and read the variable back. Round-trips state through
/// a kernel that survived past jet's exit.
#[test]
fn detach_and_attach_round_trip() {
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare python kernelspec: {e}"));
            return;
        }
    };

    use portable_pty::{CommandBuilder, PtySize, native_pty_system};
    use std::io::Write;

    let conn = std::env::temp_dir().join(format!(
        "jet-detach-test-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));

    fn drive(
        bin: &str,
        args: &[&str],
        xdg: &std::path::Path,
        code: &str,
        expected: Option<&str>,
        timeout: Duration,
    ) -> String {
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows: 40,
                cols: 120,
                ..Default::default()
            })
            .expect("openpty");
        let mut cmd = CommandBuilder::new(bin);
        for a in args {
            cmd.arg(a);
        }
        cmd.env("XDG_DATA_HOME", xdg);
        cmd.cwd(std::env::current_dir().expect("cwd"));
        let mut child = pair.slave.spawn_command(cmd).expect("spawn");
        drop(pair.slave);
        let (mut writer, output, h) = spawn_pty_reader(&*pair.master);

        // Wait for the prompt.
        let banner = Instant::now() + Duration::from_secs(20);
        while Instant::now() < banner {
            if output.lock().unwrap().contains("> ") {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        writer.write_all(code.as_bytes()).expect("write code");
        writer.flush().ok();

        // Wait for the expected substring (if any), or just give the
        // kernel a beat to handle the line.
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(needle) = expected
                && output.lock().unwrap().contains(needle)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // ^D to exit.
        let _ = writer.write_all(&[0x04]);
        let _ = writer.flush();
        drop(writer);
        let _ = child.wait();
        drop(pair.master);
        let _ = h.join();

        output.lock().unwrap().clone()
    }

    let bin = env!("CARGO_BIN_EXE_jet");
    let conn_str = conn.to_string_lossy().to_string();
    let xdg = scratch_xdg_dir();

    // Connect+persist: set x = 42, exit jet. We don't wait on a return
    // prompt — readline goes back to "> " whether or not the cell ran;
    // a fixed grace period is more reliable than a substring match.
    let _ = drive(
        bin,
        &[
            "start",
            "--connection-file",
            &conn_str,
            "--persist",
            kernel_json.to_str().unwrap(),
        ],
        &xdg,
        "x = 42\n",
        None,
        Duration::from_secs(3),
    );
    assert!(
        conn.exists(),
        "connection file {conn_str} should still exist after --persist"
    );

    // Attach: read x back. Wait until we see "42" in jet's output.
    let out = drive(
        bin,
        &["attach", "--connection-file", &conn_str],
        &xdg,
        "print(x)\n",
        Some("42"),
        Duration::from_secs(10),
    );
    assert!(
        out.contains("42"),
        "expected '42' in attach output; got:\n{out}"
    );

    // Cleanup: the kernel is still running. waitpid won't help — we
    // don't own it. Best-effort kill via pgrep on the connection file
    // path — the only argv string guaranteed unique to this kernel.
    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", &conn_str])
        .status();
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);
}

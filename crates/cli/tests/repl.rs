// Tests explicitly `.kill()` their children and the process exits shortly after; the OS
// reaps before any zombie lingers. Wiring `.wait()` into every error path adds noise
// without changing behaviour.
#![allow(clippy::zombie_processes)]

//! End-to-end tests that drive the `jet` binary through a real PTY.
//! Skipped (printed as `SKIP: …` and pass) if `python -m ipykernel` or
//! the ark R kernel are missing.
//!
//! No kallichore: jet now spawns the Jupyter kernel directly via ZMQ,
//! so `JET_KCSERVER` is no longer used.
//!
//! ## Tearing down PTY-based tests
//!
//! Tear down a `spawn_jet_pty` session by calling `child.kill()` (see
//! `shutdown_jet_pty`) — do **not** rely on writing `0x04` (^D) to the
//! master to provoke a clean exit. Under a real PTY, rustyline reads
//! `/dev/tty` in raw mode, and `0x04` only resolves to `Eof` when the
//! editor's input buffer is empty AND the upstream end of the master is
//! actually closed. `drop(writer)` only drops a clone of the master fd
//! (`pair.master` still owns the original), so the slave never sees true
//! EOF and `child.wait()` hangs forever. The graceful EOF path is
//! covered separately by `jet_exits_on_eof`, which uses `Stdio::piped()`
//! where pipe-close is reliable.
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

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::Result;
use rand::Rng;
use serde_json::json;

/// Writer that the test caller and the PTY reader thread can share, so
/// the reader can answer reedline's cursor-position query while the
/// caller writes input.
struct SharedWriter(std::sync::Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>);

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
fn spawn_pty_reader(
    master: &dyn portable_pty::MasterPty,
) -> (
    Box<dyn std::io::Write + Send>,
    std::sync::Arc<std::sync::Mutex<String>>,
    std::thread::JoinHandle<()>,
) {
    use std::io::Read;
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

fn which(name: &str) -> Option<String> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn skip(reason: &str) {
    eprintln!("SKIP: {reason}");
}

fn ipykernel_available() -> bool {
    Command::new("python3")
        .args(["-c", "import ipykernel"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

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
    use std::io::Write;

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
    let ark_kernel = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/ark/kernel.json");
    if !ark_kernel.exists() {
        skip(&format!("ark kernelspec not found at {ark_kernel:?}"));
        return;
    }

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
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["start", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

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
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["start", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    std::thread::sleep(Duration::from_secs(3));
    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin.write_all(b"exit()\n").expect("write to jet stdin");

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

    // After jet noticed the kernel exit, the session should be marked closed.
    let meta = read_only_session(&xdg);
    assert_eq!(
        meta["status"], "closed",
        "kernel-quit path did not mark session closed: {meta}"
    );
    assert!(meta["closed_at"].is_string(), "closed_at missing: {meta}");
    let _ = std::fs::remove_dir_all(&xdg);
}

/// Locate the user-installed ark R kernelspec, or `None` if it isn't
/// present (skip the test).
fn ark_kernelspec() -> Option<std::path::PathBuf> {
    let p = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/ark/kernel.json");
    if p.exists() { Some(p) } else { None }
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
    let bin = env!("CARGO_BIN_EXE_jet");
    let xdg = scratch_xdg_dir();

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["start", kernel_json.to_str().unwrap()])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

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
    let bin = env!("CARGO_BIN_EXE_jet");

    let xdg = scratch_xdg_dir();
    let conn = std::env::temp_dir().join(format!(
        "jet-attach-quit-test-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    // Spawn persisted: get a kernel that survives jet exiting.
    {
        use std::io::Write;
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
    use std::io::Write;
    let mut attach = Command::new(bin)
        .args(["attach", "--connection-file", &conn_str])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet (attach)");

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

fn ensure_python_kernelspec() -> Result<std::path::PathBuf> {
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

/// Connect with `--persist`, set a variable, exit. Then `attach` to the
/// connection file and read the variable back. Round-trips state through
/// a kernel that survived past jet's exit.
#[test]
fn detach_and_attach_round_trip() {
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

#[test]
fn input_request_prompts_user_and_replies() {
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

    use portable_pty::{CommandBuilder, PtySize, native_pty_system};
    use std::io::Write;

    let xdg = scratch_xdg_dir();
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
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.cwd(std::env::current_dir().expect("cwd"));
    let mut child = pair.slave.spawn_command(cmd).expect("spawn jet under pty");
    drop(pair.slave);

    let (mut writer, output, reader_handle) = spawn_pty_reader(&*pair.master);

    let banner_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < banner_deadline {
        if output.lock().unwrap().contains("> ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Reedline puts the slave into raw mode; in raw mode crossterm only
    // recognises `\r` (0x0d) as KeyCode::Enter — `\n` becomes a literal
    // Char('\n') and the buffer never gets committed.
    let code = "v = input('ASK> '); print('GOT:' + v)\r";
    writer.write_all(code.as_bytes()).expect("write code");
    writer.flush().expect("flush");

    let prompt_deadline = Instant::now() + Duration::from_secs(15);
    let mut saw_prompt = false;
    while Instant::now() < prompt_deadline {
        if output.lock().unwrap().contains("ASK> ") {
            saw_prompt = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if !saw_prompt {
        let _ = writer.write_all(&[0x04]);
        let _ = writer.flush();
        drop(writer);
        let _ = child.wait();
        drop(pair.master);
        let _ = reader_handle.join();
        panic!(
            "did not see input prompt 'ASK> ' within 15s; output:\n{}",
            output.lock().unwrap()
        );
    }

    writer.write_all(b"hello-jet\r").expect("write reply");
    writer.flush().expect("flush reply");

    let done_deadline = Instant::now() + Duration::from_secs(15);
    let mut got_value = false;
    while Instant::now() < done_deadline {
        if output.lock().unwrap().contains("GOT:hello-jet") {
            got_value = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let _ = writer.write_all(&[0x04]);
    let _ = writer.flush();
    let _ = child.wait();
    drop(writer);
    drop(pair.master);
    let _ = reader_handle.join();
    let _ = std::fs::remove_dir_all(&xdg);

    let final_out = output.lock().unwrap().clone();
    assert!(
        got_value,
        "kernel did not echo input value back; output:\n{final_out}"
    );
}

/// Spawn jet under a pty, return (child, writer, output buffer, reader thread).
/// Waits for the first `> ` prompt before returning so callers don't race
/// the banner.
#[allow(clippy::type_complexity)]
fn spawn_jet_pty(
    kernel_json: &std::path::Path,
    xdg: &std::path::Path,
) -> (
    Box<dyn portable_pty::Child + Send + Sync>,
    Box<dyn std::io::Write + Send>,
    std::sync::Arc<std::sync::Mutex<String>>,
    std::thread::JoinHandle<()>,
    Box<dyn portable_pty::MasterPty + Send>,
) {
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
    cmd.cwd(std::env::current_dir().expect("cwd"));
    let child = pair.slave.spawn_command(cmd).expect("spawn jet under pty");
    drop(pair.slave);

    let (writer, output, reader_handle) = spawn_pty_reader(&*pair.master);

    let banner_deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < banner_deadline {
        if output.lock().unwrap().contains("> ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    (child, writer, output, reader_handle, pair.master)
}

fn wait_for_substr(
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    needle: &str,
    timeout: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if output.lock().unwrap().contains(needle) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Tear down a `spawn_jet_pty` session by killing the child. We don't try
/// to provoke a clean EOF: under a real PTY, rustyline reads from
/// `/dev/tty` in raw mode, and `0x04` only resolves to `Eof` when the
/// editor's input buffer is empty AND the master fd has actually been
/// closed — which we can't do reliably here without racing the child's
/// own reads. Killing is unambiguous; `jet_exits_on_eof` covers the
/// graceful path on a separate test.
fn shutdown_jet_pty(
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn std::io::Write + Send>,
    reader_handle: std::thread::JoinHandle<()>,
    master: Box<dyn portable_pty::MasterPty + Send>,
) {
    let _ = child.kill();
    let _ = child.wait();
    drop(writer);
    drop(master);
    let _ = reader_handle.join();
}

/// Regression: KernelSession.start consumes the kernel_info_reply
/// before spawning the reader loops, so without explicit handling
/// the renderer never sees it and no welcome banner is printed.
/// ipykernel's reply.banner starts with "Python ".
#[test]
fn spawn_emits_kernel_banner() {
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
    let (child, writer, output, reader_handle, master) = spawn_jet_pty(&kernel_json, &xdg);
    // spawn_jet_pty already waits for the first "> " prompt, which
    // only fires after KernelSession::start returns — so by the time
    // it returns, the banner sink should have already run.
    let captured = output.lock().unwrap().clone();
    shutdown_jet_pty(child, writer, reader_handle, master);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        captured.contains("Python "),
        "expected kernel banner ('Python …') before first prompt; got:\n{captured}",
    );

    // Banner must appear BEFORE the first `> ` prompt — not sandwiched
    // between two prompts. If kernel_info handshaking returns before
    // the renderer has finished writing the banner, rustyline draws
    // the prompt first and the user sees `> Python … > `.
    let banner_idx = captured.find("Python ").expect("banner present");
    let prompt_idx = captured.find("> ").expect("prompt present");
    assert!(
        banner_idx < prompt_idx,
        "banner must precede the first '> ' prompt; got:\n{captured}",
    );
}

/// A complete one-liner executes immediately — no continuation prompt
/// appears and the result lands.
#[test]
fn complete_one_liner_executes_without_continuation() {
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

    use std::io::Write;
    let xdg = scratch_xdg_dir();
    let (child, mut writer, output, reader_handle, master) = spawn_jet_pty(&kernel_json, &xdg);

    let before_send = output.lock().unwrap().len();
    writer.write_all(b"1+1\n").expect("write code");
    writer.flush().expect("flush");
    let saw_result = wait_for_substr(&output, "2", Duration::from_secs(15));
    let tail = {
        let s = output.lock().unwrap().clone();
        s[before_send..].to_string()
    };

    shutdown_jet_pty(child, writer, reader_handle, master);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(saw_result, "did not see '2' result; tail:\n{tail}");
    assert!(
        !tail.contains("\n+ "),
        "complete code should not show a '+ ' continuation prompt; tail:\n{tail}"
    );
}

/// Incomplete code triggers a continuation prompt; finishing the block
/// with a blank line then executes it.
#[test]
fn incomplete_code_prompts_for_continuation_then_executes() {
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

    use std::io::Write;
    let xdg = scratch_xdg_dir();
    let (child, mut writer, output, reader_handle, master) = spawn_jet_pty(&kernel_json, &xdg);

    let before_send = output.lock().unwrap().len();

    // First line of a function definition. ipykernel reports
    // `incomplete` with `indent: "    "`; jet should NOT echo a fresh
    // top-level `> ` prompt afterwards.
    writer.write_all(b"def f():\n").expect("write line 1");
    writer.flush().expect("flush");
    std::thread::sleep(Duration::from_millis(500));
    let tail_after_first = {
        let s = output.lock().unwrap().clone();
        s[before_send..].to_string()
    };

    // Body line, then a blank line to close the block, then call the
    // function. Once `f()` runs we should see `42` in the output.
    writer
        .write_all(b"    return 42\n")
        .expect("write body line");
    writer.flush().expect("flush");
    writer.write_all(b"\n").expect("write blank");
    writer.flush().expect("flush");
    writer.write_all(b"f()\n").expect("write call");
    writer.flush().expect("flush");
    let saw_result = wait_for_substr(&output, "42", Duration::from_secs(15));
    let final_tail = {
        let s = output.lock().unwrap().clone();
        s[before_send..].to_string()
    };

    shutdown_jet_pty(child, writer, reader_handle, master);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        !tail_after_first.trim_end().ends_with("> "),
        "after incomplete first line, jet rolled back to top-level '> ' \
         prompt — IsCompleteRequest path didn't fire; tail:\n{tail_after_first}"
    );
    assert!(
        saw_result,
        "did not see '42' from f() call; tail:\n{final_tail}"
    );
}

/// Backspace at column 0 of a continuation prompt should merge the prior
/// accumulator line back into the editor. We submit a first line with an
/// unclosed paren (`x = (`), which ipykernel's is_complete reports as
/// Incomplete (still expecting `)`). If merge-back works, Backspace can
/// pop that line back into the editor, where we delete it and submit a
/// clean `print(marker)`. If merge-back is broken (Backspace at an empty
/// continuation line is a no-op), the broken line stays in the
/// accumulator forever and any subsequent input is appended underneath
/// it — the unclosed paren prevents the kernel from ever marking the
/// buffer complete, so `print(marker)` never runs and the marker never
/// appears in the output.
#[test]
fn backspace_at_empty_continuation_merges_prior_line() {
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

    use std::io::Write;
    let xdg = scratch_xdg_dir();
    let (child, mut writer, output, reader_handle, master) = spawn_jet_pty(&kernel_json, &xdg);

    // Submit a line with an unclosed paren. ipykernel's is_complete
    // returns Incomplete with no indent — it's still waiting for `)` —
    // so jet stays on a continuation prompt with an empty editor, and
    // the line is now stuck in the accumulator. There's no kernel-side
    // way to abandon it short of ^C; this is exactly the situation that
    // motivated backspace-merge.
    writer.write_all(b"x = (\n").expect("write malformed line");
    writer.flush().expect("flush");
    std::thread::sleep(Duration::from_millis(500));

    // Walk back through the accumulator with Backspaces:
    //   - editor is currently empty (no indent for the unclosed-paren
    //     continuation), so the first BS triggers merge-back: editor
    //     becomes "x = (" (5 chars).
    //   - 5 BS clears the line; editor and accumulator are now empty.
    //   - 2 spare BS to confirm extra Backspaces at empty/empty are
    //     harmless.
    // DEL (0x7f) is what the terminal sends for the Backspace key in raw
    // mode; reedline binds it to KeyCode::Backspace by default and jet
    // re-routes that through the HOSTCMD_BACKSPACE sentinel.
    for _ in 0..(1 + 5 + 2) {
        writer.write_all(b"\x7f").expect("write backspace");
        writer.flush().expect("flush");
        // Each BS round-trips Rust↔reedline. Pace the bytes so the loop
        // processes them in order — a burst of identical keypresses can
        // otherwise coalesce in ways that hide whether each one took the
        // merge path or the plain-edit path.
        std::thread::sleep(Duration::from_millis(15));
    }
    std::thread::sleep(Duration::from_millis(200));

    // The marker is assembled at runtime (chr(109) → 'm') so the input
    // echo of the typed line doesn't itself contain the marker — only
    // actual kernel-executed `print` output will.
    let marker = "merged-backspace-9f31";
    let line = "print(chr(109) + \"erged-backspace-9f31\")\n";
    writer
        .write_all(line.as_bytes())
        .expect("write print line");
    writer.flush().expect("flush");

    let saw_marker = wait_for_substr(&output, marker, Duration::from_secs(10));
    let final_tail = output.lock().unwrap().clone();

    shutdown_jet_pty(child, writer, reader_handle, master);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        saw_marker,
        "expected to see {marker:?} after Backspace-merging the broken \
         `def f(:` line out of the accumulator and submitting a clean \
         print; without merge-back the unclosed paren keeps the buffer \
         incomplete forever. Output was:\n{final_tail}"
    );
}

fn scratch_xdg_dir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "jet-xdg-test-{:x}",
        rand::thread_rng().r#gen::<u64>()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Find the single session subdir under `<xdg>/jet/` and parse its
/// session.json. Panics if there isn't exactly one.
fn read_only_session(xdg: &std::path::Path) -> serde_json::Value {
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

/// Graceful exit (no --persist) should leave the session marked closed
/// and the kernel_pid cleared — pid only makes sense for live kernels.
#[test]
fn session_marked_closed_on_graceful_exit() {
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

/// Spawn an ipykernel via `jet start --persist --connection-file <path>`
/// in a child process, then EOF its stdin to exit jet (kernel keeps
/// running). Returns the connection-file path; caller is responsible for
/// killing the kernel and cleaning up.
fn spawn_persisted_ipykernel() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    if !ipykernel_available() {
        skip("ipykernel not installed");
        return None;
    }
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

/// When a second client (`jet execute --session-name t2`) drives the
/// same kernel as an in-flight REPL, the REPL must surface that
/// activity to the user:
///   - the foreign client's input line is echoed as `[t2]> <code>`;
///   - the foreign output stream is tagged `[t2]` on every line;
///   - `\n` in foreign output resets to column 0 (no reedline-raw-mode
///     staircase), which we proxy by asserting the streamed text does
///     not appear immediately after the prompt's `> ` cursor — there
///     should be a fresh-line break between the prompt and the foreign
///     output.
///
/// This is the regression test for the reedline switch: rustyline's
/// prompt drew over cooked-mode stdout, so a plain `\n` reset to col 0;
/// reedline holds the tty in raw mode and needs the external printer
/// path in `Renderer::write_foreign` to land output cleanly.
#[test]
fn foreign_session_output_is_prefixed_and_column_zero_aligned() {
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

    use portable_pty::{CommandBuilder, PtySize, native_pty_system};

    let xdg = scratch_xdg_dir();
    let conn = std::env::temp_dir().join(format!(
        "jet-foreign-prefix-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    // Terminal 1: REPL under a real PTY (raw mode active during reedline
    // read_line). --persist so the kernel survives if jet exits, and so
    // we can pkill it by connection-file argv at the end.
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
    cmd.args([
        "start",
        "--connection-file",
        &conn_str,
        "--persist",
        kernel_json.to_str().unwrap(),
    ]);
    cmd.env("XDG_DATA_HOME", &xdg);
    cmd.cwd(std::env::current_dir().expect("cwd"));
    let mut child = pair.slave.spawn_command(cmd).expect("spawn jet (start)");
    drop(pair.slave);
    let (writer, output, reader_handle) = spawn_pty_reader(&*pair.master);

    // Wait for the first prompt — that's how we know the kernel is up
    // and reedline owns the tty.
    let banner_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < banner_deadline {
        if output.lock().unwrap().contains("> ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let before_t2 = output.lock().unwrap().len();

    // Terminal 2: separate `jet execute --session-name t2`. We pick a
    // print that emits two lines so we can assert the second line is
    // also prefixed (proves the line-by-line route through the external
    // printer, not just the first-byte case).
    let exec_out = Command::new(bin)
        .args([
            "execute",
            "--connection-file",
            &conn_str,
            "--session-name",
            "t2",
            "--no-graphics",
            "print('HI'); print('AGAIN')",
        ])
        .env("XDG_DATA_HOME", &xdg)
        .stdin(Stdio::null())
        .output()
        .expect("run jet execute t2");
    assert!(
        exec_out.status.success(),
        "jet execute (t2) failed: stdout={} stderr={}",
        String::from_utf8_lossy(&exec_out.stdout),
        String::from_utf8_lossy(&exec_out.stderr),
    );

    // Give the REPL a moment to receive t2's iopub frames and route
    // them through the external printer.
    let deadline = Instant::now() + Duration::from_secs(10);
    let needle = "AGAIN";
    while Instant::now() < deadline {
        if output.lock().unwrap()[before_t2..].contains(needle) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let tail = {
        let s = output.lock().unwrap().clone();
        s[before_t2..].to_string()
    };

    // Teardown.
    let _ = child.kill();
    let _ = child.wait();
    drop(writer);
    drop(pair.master);
    let _ = reader_handle.join();
    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", &conn_str])
        .status();
    let _ = std::fs::remove_file(&conn);
    let _ = std::fs::remove_dir_all(&xdg);

    // 1. The foreign execute_input echo should appear as `[t2]> ...`.
    //    The dim ANSI escape brackets the tag, but the literal `[t2]`
    //    substring is still present.
    assert!(
        tail.contains("[t2]"),
        "no '[t2]' tag in REPL output after foreign execute — \
         renderer didn't classify the t2 client_id as foreign. tail:\n{tail}",
    );
    assert!(
        tail.contains("> print('HI'); print('AGAIN')"),
        "foreign execute_input echo missing from REPL output. tail:\n{tail}",
    );

    // 2. Both streamed lines should appear AND both should be tagged.
    //    The renderer's per-line prefix path means we should see two
    //    occurrences of `[t2]` between the echo and `AGAIN`.
    assert!(tail.contains("HI"), "missing 'HI' in tail:\n{tail}");
    assert!(tail.contains("AGAIN"), "missing 'AGAIN' in tail:\n{tail}");
    let t2_tags = tail.matches("[t2]").count();
    assert!(
        t2_tags >= 3,
        "expected `[t2]` to appear at least 3 times (echo + 2 stream \
         lines); saw {t2_tags}. tail:\n{tail}",
    );

    // 3. No raw-mode staircase. The bug we're guarding against: in
    //    reedline's raw mode, `\n` in a foreign-output write moves down
    //    one row but does NOT return to column 0, so each subsequent
    //    line starts further right than the last (`HI\n  AGAIN\n    …`).
    //    The external printer fixes this by clearing the prompt line
    //    and emitting each foreign message starting at column 0.
    //
    //    To detect the staircase: strip ANSI escapes from the tail,
    //    split into lines, take the lines containing `HI` and `AGAIN`,
    //    and assert the body of each line starts with the `[t2]` tag —
    //    not with leading spaces from a left-over cursor position.
    let stripped = strip_ansi(&tail);
    let hi_line = stripped
        .lines()
        .find(|l| l.contains("HI") && !l.contains("AGAIN"))
        .unwrap_or_else(|| panic!("no line containing only HI in stripped:\n{stripped}"));
    let again_line = stripped
        .lines()
        .find(|l| l.contains("AGAIN"))
        .unwrap_or_else(|| panic!("no line containing AGAIN in stripped:\n{stripped}"));
    // Each line, after stripping leading whitespace from any prompt-clear
    // padding reedline emits, should start with `[t2]` — not `HI` or
    // `AGAIN` at some non-zero column.
    let hi_trimmed = hi_line.trim_start();
    let again_trimmed = again_line.trim_start();
    assert!(
        hi_trimmed.starts_with("[t2]"),
        "staircase: the `HI` line should start with the [t2] tag, but \
         after trimming leading whitespace it starts with {hi_trimmed:?}. \
         full stripped output:\n{stripped}",
    );
    assert!(
        again_trimmed.starts_with("[t2]"),
        "staircase: the `AGAIN` line should start with the [t2] tag, \
         but after trimming leading whitespace it starts with \
         {again_trimmed:?}. full stripped output:\n{stripped}",
    );
    // Further: HI and AGAIN should land on DIFFERENT lines. Pre-fix the
    // staircase smashed them onto adjacent columns of one PTY line.
    assert_ne!(
        hi_line, again_line,
        "HI and AGAIN landed on the same line — staircase compaction. \
         stripped:\n{stripped}",
    );
}

/// Strip ANSI CSI escapes from `s`. Good enough for test assertions:
/// matches `ESC [ … <letter>` and drops the whole sequence.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                // Other escape (e.g. OSC) — drop until ST or BEL.
                for next in chars.by_ref() {
                    if next == '\x07' || next == '\x1b' {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

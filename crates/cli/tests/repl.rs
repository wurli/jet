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
//! EOF and `child.wait()` hangs forever — which then blocks every other
//! `#[serial_test::serial]` test behind the same mutex. The graceful
//! EOF path is covered separately by `jet_exits_on_eof`, which uses
//! `Stdio::piped()` where pipe-close is reliable.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::Result;
use rand::Rng;
use serde_json::json;

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
    busy_grace: Duration,
    timeout: Duration,
) -> Result<String> {
    use portable_pty::{CommandBuilder, PtySize, native_pty_system};
    use std::io::{Read, Write};

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
    cmd.args(["connect", kernel_json.to_str().unwrap()]);
    cmd.cwd(std::env::current_dir()?);
    let mut child = pair.slave.spawn_command(cmd).expect("spawn jet under pty");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let mut writer = pair.master.take_writer().expect("take writer");

    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let output_clone = output.clone();
    let reader_handle = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    output_clone.lock().unwrap().push_str(&s);
                }
            }
        }
    });

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
        if saw_interrupt {
            if let Some(idx) = s.find(interrupt_marker) {
                if s[idx + interrupt_marker.len()..].contains("> ") {
                    break;
                }
            }
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
#[serial_test::serial]
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

    let out = drive_jet_with_interrupt(
        "Sys.sleep(30)\n",
        &ark_kernel,
        Duration::from_secs(2),
        Duration::from_secs(15),
    )
    .expect("drive_jet_with_interrupt");

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
#[serial_test::serial]
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

    let mut child = Command::new(bin)
        .args(["connect", kernel_json.to_str().unwrap()])
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
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("jet did not exit within 10s after stdin closed");
}

/// Regression: ark's `quit()` (and any kernel that exits mid-execute
/// without sending an idle status) used to hang jet for 300s because
/// the inner `wait_for_idle` loop didn't watch the `closed` notify.
/// Simulate the same shape with `os._exit(0)` on ipykernel — it skips
/// the normal shutdown handshake, so no idle ever arrives.
#[test]
#[serial_test::serial]
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

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["connect", kernel_json.to_str().unwrap()])
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
#[serial_test::serial]
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

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["connect", kernel_json.to_str().unwrap()])
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
        panic!("jet did not exit within 10s after the kernel quit");
    }
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
#[serial_test::serial]
fn jet_exits_when_r_kernel_quits_spawn() {
    let Some(kernel_json) = ark_kernelspec() else {
        skip("ark kernelspec not found");
        return;
    };
    let bin = env!("CARGO_BIN_EXE_jet");

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["connect", kernel_json.to_str().unwrap()])
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
#[serial_test::serial]
fn jet_exits_when_r_kernel_quits_attach() {
    let Some(kernel_json) = ark_kernelspec() else {
        skip("ark kernelspec not found");
        return;
    };
    let bin = env!("CARGO_BIN_EXE_jet");

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
                "connect",
                "--connection-file",
                &conn_str,
                "--persist",
                kernel_json.to_str().unwrap(),
            ])
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
        .args(["attach", &conn_str])
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
    if !exited {
        let _ = attach.kill();
        let _ = attach.wait();
        // Also try to clear any straggler kernel.
        let _ = std::process::Command::new("pkill")
            .args(["-9", "-f", "ark"])
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
#[serial_test::serial]
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
    use std::io::{Read, Write};

    let conn = std::env::temp_dir().join(format!(
        "jet-detach-test-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));

    fn drive(
        bin: &str,
        args: &[&str],
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
        cmd.cwd(std::env::current_dir().expect("cwd"));
        let mut child = pair.slave.spawn_command(cmd).expect("spawn");
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().expect("reader");
        let mut writer = pair.master.take_writer().expect("writer");

        let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let output_clone = output.clone();
        let h = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => output_clone
                        .lock()
                        .unwrap()
                        .push_str(&String::from_utf8_lossy(&buf[..n])),
                }
            }
        });

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
            if let Some(needle) = expected {
                if output.lock().unwrap().contains(needle) {
                    break;
                }
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
        let final_out = output.lock().unwrap().clone();
        final_out
    }

    let bin = env!("CARGO_BIN_EXE_jet");
    let conn_str = conn.to_string_lossy().to_string();

    // Connect+persist: set x = 42, exit jet. We don't wait on a return
    // prompt — readline goes back to "> " whether or not the cell ran;
    // a fixed grace period is more reliable than a substring match.
    let _ = drive(
        bin,
        &[
            "connect",
            "--connection-file",
            &conn_str,
            "--persist",
            kernel_json.to_str().unwrap(),
        ],
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
        &["attach", &conn_str],
        "print(x)\n",
        Some("42"),
        Duration::from_secs(10),
    );
    assert!(
        out.contains("42"),
        "expected '42' in attach output; got:\n{out}"
    );

    // Cleanup: the kernel is still running. waitpid won't help — we
    // don't own it. Best-effort kill via pgrep.
    let _ = std::process::Command::new("pkill")
        .args(["-9", "-f", "ipykernel_launcher"])
        .status();
    let _ = std::fs::remove_file(&conn);
}

#[test]
#[serial_test::serial]
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
    use std::io::{Read, Write};

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
    cmd.args(["connect", kernel_json.to_str().unwrap()]);
    cmd.cwd(std::env::current_dir().expect("cwd"));
    let mut child = pair.slave.spawn_command(cmd).expect("spawn jet under pty");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let mut writer = pair.master.take_writer().expect("take writer");

    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let output_clone = output.clone();
    let reader_handle = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    output_clone.lock().unwrap().push_str(&s);
                }
            }
        }
    });

    let banner_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < banner_deadline {
        if output.lock().unwrap().contains("> ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let code = "v = input('ASK> '); print('GOT:' + v)\n";
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

    writer.write_all(b"hello-jet\n").expect("write reply");
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
    drop(writer);
    let _ = child.wait();
    drop(pair.master);
    let _ = reader_handle.join();

    let final_out = output.lock().unwrap().clone();
    assert!(
        got_value,
        "kernel did not echo input value back; output:\n{final_out}"
    );
}

/// Spawn jet under a pty, return (child, writer, output buffer, reader thread).
/// Waits for the first `> ` prompt before returning so callers don't race
/// the banner.
fn spawn_jet_pty(
    kernel_json: &std::path::Path,
) -> (
    Box<dyn portable_pty::Child + Send + Sync>,
    Box<dyn std::io::Write + Send>,
    std::sync::Arc<std::sync::Mutex<String>>,
    std::thread::JoinHandle<()>,
    Box<dyn portable_pty::MasterPty + Send>,
) {
    use portable_pty::{CommandBuilder, PtySize, native_pty_system};
    use std::io::Read;

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
    cmd.args(["connect", kernel_json.to_str().unwrap()]);
    cmd.cwd(std::env::current_dir().expect("cwd"));
    let child = pair.slave.spawn_command(cmd).expect("spawn jet under pty");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let writer = pair.master.take_writer().expect("take writer");

    let output = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let output_clone = output.clone();
    let reader_handle = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    output_clone.lock().unwrap().push_str(&s);
                }
            }
        }
    });

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

/// A complete one-liner executes immediately — no continuation prompt
/// appears and the result lands.
#[test]
#[serial_test::serial]
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
    let (child, mut writer, output, reader_handle, master) = spawn_jet_pty(&kernel_json);

    let before_send = output.lock().unwrap().len();
    writer.write_all(b"1+1\n").expect("write code");
    writer.flush().expect("flush");
    let saw_result = wait_for_substr(&output, "2", Duration::from_secs(15));
    let tail = {
        let s = output.lock().unwrap().clone();
        s[before_send..].to_string()
    };

    shutdown_jet_pty(child, writer, reader_handle, master);

    assert!(saw_result, "did not see '2' result; tail:\n{tail}");
    assert!(
        !tail.contains("\n+ "),
        "complete code should not show a '+ ' continuation prompt; tail:\n{tail}"
    );
}

/// Incomplete code triggers a continuation prompt; finishing the block
/// with a blank line then executes it.
#[test]
#[serial_test::serial]
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
    let (child, mut writer, output, reader_handle, master) = spawn_jet_pty(&kernel_json);

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

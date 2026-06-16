//! Tests that drive the `jet` binary directly through a pty and assert on
//! REPL behavior. Skipped (printed as `SKIP: …` and pass) if kcserver or
//! ipykernel is missing — same gates as `jet-core/tests/kcserver.rs`.

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

fn locate_kcserver() -> Option<String> {
    if let Ok(p) = std::env::var("JET_KCSERVER") {
        if std::path::Path::new(&p).exists() {
            return Some(p);
        }
    }
    for p in ["/tmp/kc/kcserver"] {
        if std::path::Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    which("kcserver")
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

fn prereqs_ok() -> bool {
    if locate_kcserver().is_none() {
        skip("kcserver not found (set JET_KCSERVER=/path/to/kcserver)");
        return false;
    }
    if !ipykernel_available() {
        skip("ipykernel not installed (`pip install ipykernel`)");
        return false;
    }
    true
}

/// Drive the jet binary through a real PTY, send `code`, then send SIGINT
/// to the jet process. Returns everything jet wrote to its tty up until it
/// either returned to a prompt after the interrupt or `timeout` elapsed.
fn drive_jet_with_interrupt(
    code: &str,
    kc: &str,
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
    cmd.args(["connect", "--kcserver", kc, kernel_json.to_str().unwrap()]);
    cmd.cwd(std::env::current_dir()?);
    let mut child = pair.slave.spawn_command(cmd).expect("spawn jet under pty");
    drop(pair.slave);

    let pid = child.process_id().expect("pid") as i32;

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

    // Wait for the banner / first prompt to appear before sending code.
    let banner_deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < banner_deadline {
        if output.lock().unwrap().contains("> ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    writer.write_all(code.as_bytes())?;
    writer.flush()?;

    // Give the kernel time to enter Busy. We can't watch session status from
    // outside jet, so a short sleep that's well under the kernel sleep.
    std::thread::sleep(busy_grace);

    // Write a literal ^C byte to the master side of the pty. This is the
    // real keystroke path: the tty driver, in cooked mode with ISIG, turns
    // it into SIGINT to jet's process group. Sending SIGINT directly with
    // libc::kill would bypass the tty layer and miss the bug we're testing.
    let _ = pid;
    writer.write_all(&[0x03])?;
    writer.flush()?;

    // Wait for either: jet prints another prompt (recovered) OR timeout.
    let deadline = Instant::now() + timeout;
    let interrupt_marker = "^C";
    let mut saw_interrupt = false;
    while Instant::now() < deadline {
        let s = output.lock().unwrap().clone();
        if !saw_interrupt && s.contains(interrupt_marker) {
            saw_interrupt = true;
        }
        // Recovered prompt = a second occurrence of "> " after the ^C echo.
        if saw_interrupt {
            if let Some(idx) = s.find(interrupt_marker) {
                if s[idx + interrupt_marker.len()..].contains("> ") {
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Cleanly exit jet with EOF so the test doesn't leak processes.
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
    if !prereqs_ok() {
        return;
    }
    let kc = locate_kcserver().expect("kcserver");

    // Use ark (the R kernel) here, not ipykernel. ipykernel installs its
    // own SIGINT handler that converts SIGINT into KeyboardInterrupt and
    // keeps running, which masks the bug we're testing: if ^C from the
    // tty reaches the kernel's process group, the kernel should NOT die.
    // ark just exits on SIGINT, so it surfaces the bug clearly.
    let ark_kernel = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/ark/kernel.json");
    if !ark_kernel.exists() {
        eprintln!("SKIP: ark kernelspec not found at {ark_kernel:?}");
        return;
    }

    let out = drive_jet_with_interrupt(
        "Sys.sleep(30)\n",
        &kc,
        &ark_kernel,
        Duration::from_secs(2),
        Duration::from_secs(15),
    )
    .expect("drive_jet_with_interrupt");

    assert!(
        out.contains("^C"),
        "expected '^C' echo in jet output, got:\n{out}"
    );
    // The kernel must survive ^C. If SIGINT propagates to the kernel's
    // process (because jet shares its tty's foreground process group with
    // the kernel), the kernel dies and jet prints "[jet] kernel exited".
    assert!(
        !out.contains("kernel exited"),
        "kernel exited after ^C — interrupt should have been delivered via \
         interrupt_session, not as a SIGINT to the kernel process. Output:\n{out}"
    );
}

#[test]
#[serial_test::serial]
fn jet_exits_on_eof() {
    if !prereqs_ok() {
        return;
    }
    let kc = locate_kcserver().expect("kcserver located");
    let bin = env!("CARGO_BIN_EXE_jet");

    let mut child = Command::new(bin)
        .args(["--kcserver", &kc])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    // Give jet time to come up, then close stdin to simulate ^D.
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

#[test]
#[serial_test::serial]
fn jet_exits_when_kernel_quits() {
    if !prereqs_ok() {
        return;
    }
    let kc = locate_kcserver().expect("kcserver located");
    let bin = env!("CARGO_BIN_EXE_jet");

    use std::io::Write;
    let mut child = Command::new(bin)
        .args(["--kcserver", &kc])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn jet");

    // Give jet time to come up, then ask the kernel to exit. We KEEP stdin
    // open afterwards — closing it would let rustyline return EOF naturally
    // (the trivial exit path). The bug we're testing is "does jet notice
    // the websocket dying and exit even while still waiting on stdin?"
    std::thread::sleep(Duration::from_secs(3));
    let mut stdin = child.stdin.take().expect("stdin piped");
    stdin.write_all(b"exit()\n").expect("write to jet stdin");
    // Hold stdin open by keeping `stdin` in scope until after we've waited.

    // jet should notice the kernel went away and exit on its own. Without
    // this fix it sits forever in rustyline.
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

/// Locate a Python kernelspec, generating a temporary one if needed.
/// jet requires a kernel.json on disk (its argv comes from there).
fn ensure_python_kernelspec() -> Result<std::path::PathBuf> {
    let user = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/python3/kernel.json");
    if user.exists() {
        return Ok(user);
    }
    // Fall back to a generated kernelspec under the OS tempdir so the
    // test works in CI without a pre-installed kernel.
    let python = which("python3").ok_or_else(|| anyhow::anyhow!("python3 not on PATH"))?;
    let dir = std::env::temp_dir().join(format!(
        "jet-test-kernelspec-{:x}",
        rand::thread_rng().gen::<u64>()
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

#[test]
#[serial_test::serial]
fn input_request_prompts_user_and_replies() {
    if !prereqs_ok() {
        return;
    }
    let kc = locate_kcserver().expect("kcserver");
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: could not prepare python kernelspec: {e}");
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
    cmd.args([
        "connect",
        "--kcserver",
        &kc,
        kernel_json.to_str().unwrap(),
    ]);
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

    // Wait for the first prompt so we know the banner has been drawn.
    let banner_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < banner_deadline {
        if output.lock().unwrap().contains("> ") {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Use a unique marker for the input prompt so we can sync on the
    // kernel asking for input rather than on the REPL prompt itself.
    // Send the code as a single ipython cell using its `%paste`-free
    // multiline form: a semicolon-joined statement. Avoids interleaving
    // a second readline at the REPL level with the input_request.
    let code = "v = input('ASK> '); print('GOT:' + v)\n";
    writer.write_all(code.as_bytes()).expect("write code");
    writer.flush().expect("flush");

    // Wait for the kernel's input prompt to appear via the input_request
    // path (jet writes req.prompt to the tty before reading our reply).
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
        let _ = writer.write_all(&[0x04]); // ^D to release jet
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

    // Send the reply — jet should forward this as input_reply on the
    // stdin channel and the kernel should resume execution.
    writer.write_all(b"hello-jet\n").expect("write reply");
    writer.flush().expect("flush reply");

    // Expect the kernel to print "GOT:hello-jet" and return to a prompt.
    let done_deadline = Instant::now() + Duration::from_secs(15);
    let mut got_value = false;
    while Instant::now() < done_deadline {
        if output.lock().unwrap().contains("GOT:hello-jet") {
            got_value = true;
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

    let final_out = output.lock().unwrap().clone();
    assert!(
        got_value,
        "kernel did not echo input value back; output:\n{final_out}"
    );
}

//! End-to-end integration tests against a real kcserver + ipython kernel.
//!
//! Skipped (printed as `SKIP: …` and pass) if either prerequisite is
//! missing. Set `JET_KCSERVER=/path/to/kcserver` or place the binary at
//! `/tmp/kc/kcserver` or on `PATH`.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

use jet::jupyter;
use jet::kallichore::Client;

fn which(name: &str) -> Option<String> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
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

/// Drive a session end-to-end and return whatever printed to stdout from
/// the iopub handler.
async fn run_one(code: &str) -> Result<String> {
    let kc = locate_kcserver().expect("locate_kcserver should succeed before calling");

    let client = Client::spawn(&kc, None, false).await?;

    let session_id = format!("jet-test-{:x}", rand::thread_rng().gen::<u64>());
    let python = which("python3").ok_or_else(|| anyhow::anyhow!("python3 not on PATH"))?;
    let argv = vec![
        python,
        "-m".into(),
        "ipykernel_launcher".into(),
        "-f".into(),
        "{connection_file}".into(),
    ];
    client
        .create_session(
            &session_id,
            "jet",
            "python",
            &argv,
            &std::collections::HashMap::new(),
            jet::kallichore::api::types::InterruptMode::Signal,
        )
        .await?;

    let ws = client.open_channels(&session_id).await?;
    let (mut sink, mut stream) = ws.split();

    client.start_session(&session_id).await?;

    let msg_id = jupyter::new_msg_id();
    let req = jupyter::message(
        "shell",
        &msg_id,
        "execute_request",
        json!({
            "code": code,
            "silent": false,
            "store_history": true,
            "user_expressions": {},
            "allow_stdin": false,
            "stop_on_error": true,
        }),
    );
    sink.send(Message::Text(req.to_string().into())).await?;

    let result = collect_output(&mut stream, &msg_id).await;
    // Close cleanly so kcserver doesn't log "Connection reset without closing
    // handshake" — kcserver inherits our stderr.
    let _ = sink.send(Message::Close(None)).await;
    let _ = sink.close().await;
    result
}

async fn collect_output(
    stream: &mut futures_util::stream::SplitStream<jet::kallichore::WsStream>,
    msg_id: &str,
) -> Result<String> {
    let mut output = String::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let next = tokio::time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            stream.next(),
        )
        .await;
        let Ok(Some(Ok(Message::Text(t)))) = next else {
            continue;
        };
        let v: Value = serde_json::from_str(&t).unwrap_or(Value::Null);
        let channel = v.get("channel").and_then(|s| s.as_str()).unwrap_or("");
        let msg_type = v
            .pointer("/header/msg_type")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let parent = v
            .pointer("/parent_header/msg_id")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        if channel != "iopub" || parent != msg_id {
            continue;
        }
        let content = v.get("content").cloned().unwrap_or(Value::Null);
        match msg_type {
            "stream" => {
                if let Some(t) = content.get("text").and_then(|s| s.as_str()) {
                    output.push_str(t);
                }
            }
            "execute_result" | "display_data" => {
                if let Some(t) = content
                    .pointer("/data/text~1plain")
                    .and_then(|s| s.as_str())
                {
                    output.push_str(t);
                }
            }
            "error" => {
                bail!(
                    "kernel error: {:?}",
                    content
                        .get("evalue")
                        .and_then(|s| s.as_str())
                        .unwrap_or("?")
                );
            }
            "status" if content.get("execution_state").and_then(|s| s.as_str()) == Some("idle") => {
                return Ok(output);
            }
            _ => {}
        }
    }
    bail!("timed out waiting for execution to complete");
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
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

#[test]
#[serial_test::serial]
fn executes_simple_expression() {
    if !prereqs_ok() {
        return;
    }
    let out = block_on(run_one("2 + 2")).expect("run_one should succeed");
    assert!(out.contains('4'), "expected '4' in output, got: {out:?}");
}

#[test]
#[serial_test::serial]
fn captures_stdout() {
    if !prereqs_ok() {
        return;
    }
    let out = block_on(run_one("print('hello-from-jet-test')")).expect("run_one should succeed");
    assert!(out.contains("hello-from-jet-test"), "got: {out:?}");
}

#[test]
#[serial_test::serial]
fn propagates_kernel_error() {
    if !prereqs_ok() {
        return;
    }
    let err = block_on(run_one("raise RuntimeError('boom')")).unwrap_err();
    assert!(
        err.to_string().contains("boom"),
        "expected 'boom' in error, got: {err}"
    );
}

/// Create + start a Python session on the given client and return its id
/// along with the channels websocket. The ws must be kept alive — kallichore
/// tears the kernel down when its channels client disconnects.
async fn create_python_session(client: &Client) -> Result<(String, jet::kallichore::WsStream)> {
    let session_id = format!("jet-test-{:x}", rand::thread_rng().gen::<u64>());
    let python = which("python3").ok_or_else(|| anyhow::anyhow!("python3 not on PATH"))?;
    let argv = vec![
        python,
        "-m".into(),
        "ipykernel_launcher".into(),
        "-f".into(),
        "{connection_file}".into(),
    ];
    client
        .create_session(
            &session_id,
            "jet",
            "python",
            &argv,
            &std::collections::HashMap::new(),
            jet::kallichore::api::types::InterruptMode::Signal,
        )
        .await?;
    let ws = client.open_channels(&session_id).await?;
    client.start_session(&session_id).await?;

    // kallichore stays in `Starting` until the first message exchange with
    // the kernel. Send a kernel_info_request so the session advances to Idle.
    let (mut sink, stream) = ws.split();
    let info_id = jupyter::new_msg_id();
    let req = jupyter::message("shell", &info_id, "kernel_info_request", json!({}));
    sink.send(Message::Text(req.to_string().into())).await?;
    let ws = sink.reunite(stream).expect("reunite");
    Ok((session_id, ws))
}

/// Wait until `predicate` is true for the listed session, polling
/// `list_sessions()`. Returns the matching ActiveSession.
async fn wait_for_status(
    client: &Client,
    session_id: &str,
    predicate: impl Fn(&jet::kallichore::ActiveSession) -> bool,
) -> Result<jet::kallichore::ActiveSession> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_status = None;
    while Instant::now() < deadline {
        let sessions = client.list_sessions().await?;
        if let Some(s) = sessions.into_iter().find(|s| s.session_id == session_id) {
            last_status = Some(format!("{:?}", s.status));
            if predicate(&s) {
                return Ok(s);
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!(
        "timed out waiting for session {session_id} to match predicate (last status: {:?})",
        last_status
    );
}

#[test]
#[serial_test::serial]
fn kill_session_terminates_running_kernel() {
    if !prereqs_ok() {
        return;
    }
    block_on(async {
        let kc = locate_kcserver().expect("kcserver");
        let client = Client::spawn(&kc, None, false).await.expect("spawn client");
        let (session_id, _ws) = create_python_session(&client)
            .await
            .expect("create session");

        // Wait until the kernel is actually up.
        use jet::kallichore::api::types::Status;
        wait_for_status(&client, &session_id, |s| {
            matches!(s.status, Status::Idle | Status::Ready | Status::Busy)
        })
        .await
        .expect("session reached running state");

        client
            .kill_session(&session_id)
            .await
            .expect("kill_session");

        wait_for_status(&client, &session_id, |s| {
            matches!(s.status, Status::Exited)
        })
        .await
        .expect("session reached Exited");
    });
}

#[test]
#[serial_test::serial]
fn delete_session_rejects_running_then_succeeds_after_kill() {
    if !prereqs_ok() {
        return;
    }
    block_on(async {
        let kc = locate_kcserver().expect("kcserver");
        let client = Client::spawn(&kc, None, false).await.expect("spawn client");
        let (session_id, _ws) = create_python_session(&client)
            .await
            .expect("create session");

        use jet::kallichore::api::types::Status;
        wait_for_status(&client, &session_id, |s| {
            matches!(s.status, Status::Idle | Status::Ready | Status::Busy)
        })
        .await
        .expect("running");

        // Pre-condition for the assertion in `jet stop`: kallichore refuses
        // DELETE while the kernel is alive.
        let err = client
            .delete_session(&session_id)
            .await
            .expect_err("delete on running session should fail");
        assert!(
            err.to_string().contains("400") || err.to_string().to_lowercase().contains("running"),
            "expected 400/running in error, got: {err}"
        );

        client.kill_session(&session_id).await.expect("kill");
        wait_for_status(&client, &session_id, |s| {
            matches!(s.status, Status::Exited)
        })
        .await
        .expect("exited");

        client
            .delete_session(&session_id)
            .await
            .expect("delete on exited session");

        let remaining = client.list_sessions().await.expect("list");
        assert!(
            !remaining.iter().any(|s| s.session_id == session_id),
            "session still present after delete"
        );
    });
}

#[test]
#[serial_test::serial]
fn shutdown_server_stops_the_kcserver() {
    if !prereqs_ok() {
        return;
    }
    block_on(async {
        let kc = locate_kcserver().expect("kcserver");
        // Spawn but DETACH so the ChildGuard's Drop doesn't kill the process
        // out from under shutdown_server — we want the request itself to be
        // what stops the server.
        let mut client = Client::spawn(&kc, None, false).await.expect("spawn client");
        client.detach_server();

        client.shutdown_server().await.expect("shutdown_server");

        // After /shutdown returns, subsequent HTTP calls must fail.
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut down = false;
        while Instant::now() < deadline {
            if client.list_sessions().await.is_err() {
                down = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(down, "kcserver still serving requests after /shutdown");
    });
}

#[test]
#[serial_test::serial]
fn interrupt_session_breaks_long_running_computation() {
    if !prereqs_ok() {
        return;
    }
    block_on(async {
        let kc = locate_kcserver().expect("kcserver");
        let client = Client::spawn(&kc, None, false).await.expect("spawn client");
        let (session_id, ws) = create_python_session(&client)
            .await
            .expect("create session");
        let (mut sink, mut stream) = ws.split();

        // Drain the kernel_info_request reply and any startup chatter so
        // we're synced to the kernel being idle before the long sleep.
        use jet::kallichore::api::types::Status;
        wait_for_status(&client, &session_id, |s| {
            matches!(s.status, Status::Idle | Status::Ready)
        })
        .await
        .expect("session reached idle");

        let msg_id = jupyter::new_msg_id();
        let req = jupyter::message(
            "shell",
            &msg_id,
            "execute_request",
            json!({
                "code": "import time\nfor _ in range(60):\n    time.sleep(1)\n",
                "silent": false,
                "store_history": true,
                "user_expressions": {},
                "allow_stdin": false,
                "stop_on_error": true,
            }),
        );
        sink.send(Message::Text(req.to_string().into()))
            .await
            .expect("send execute_request");

        // Wait until kallichore reports the kernel is busy, so we know the
        // request is in flight before we interrupt.
        wait_for_status(&client, &session_id, |s| {
            matches!(s.status, Status::Busy)
        })
        .await
        .expect("session reached Busy");

        client
            .interrupt_session(&session_id)
            .await
            .expect("interrupt_session");

        // Expect the kernel to surface a KeyboardInterrupt error for our
        // request and then return to idle. Cap at 15s — the sleep is 60s,
        // so anything close to that means interrupt didn't take effect.
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut saw_error = false;
        let mut saw_idle = false;
        while Instant::now() < deadline && !(saw_error && saw_idle) {
            let next = tokio::time::timeout(
                deadline.saturating_duration_since(Instant::now()),
                stream.next(),
            )
            .await;
            let Ok(Some(Ok(Message::Text(t)))) = next else {
                continue;
            };
            let v: Value = serde_json::from_str(&t).unwrap_or(Value::Null);
            let parent = v
                .pointer("/parent_header/msg_id")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if parent != msg_id {
                continue;
            }
            let msg_type = v
                .pointer("/header/msg_type")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            match msg_type {
                "error" => {
                    let ename = v
                        .pointer("/content/ename")
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    assert!(
                        ename.contains("KeyboardInterrupt"),
                        "expected KeyboardInterrupt, got ename={ename:?}"
                    );
                    saw_error = true;
                }
                "status" => {
                    if v.pointer("/content/execution_state").and_then(|s| s.as_str())
                        == Some("idle")
                    {
                        saw_idle = true;
                    }
                }
                _ => {}
            }
        }
        assert!(saw_error, "did not see error reply after interrupt");
        assert!(saw_idle, "kernel did not return to idle after interrupt");

        let _ = sink.send(Message::Close(None)).await;
        let _ = sink.close().await;
    });
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

//! End-to-end integration tests against a real kcserver + ipython kernel.
//!
//! Skipped (printed as `SKIP: …` and pass) if either prerequisite is
//! missing. Set `JET_KCSERVER=/path/to/kcserver` or place the binary at
//! `/tmp/kc/kcserver` or on `PATH`.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

use jet_core::jupyter;
use jet_core::kallichore::Client;

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
            jet_core::kallichore::api::types::InterruptMode::Signal,
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
    stream: &mut futures_util::stream::SplitStream<jet_core::kallichore::WsStream>,
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
async fn create_python_session(client: &Client) -> Result<(String, jet_core::kallichore::WsStream)> {
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
            jet_core::kallichore::api::types::InterruptMode::Signal,
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
    predicate: impl Fn(&jet_core::kallichore::ActiveSession) -> bool,
) -> Result<jet_core::kallichore::ActiveSession> {
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
        use jet_core::kallichore::api::types::Status;
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

        use jet_core::kallichore::api::types::Status;
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
        use jet_core::kallichore::api::types::Status;
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

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
use jet::kernel;

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

    let client = Client::spawn(&kc).await?;

    let session_id = format!("jet-test-{:x}", rand::thread_rng().gen::<u64>());
    let argv = kernel::build_argv(&[]);
    client.create_session(&session_id, "python", &argv).await?;

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
            "status"
                if content.get("execution_state").and_then(|s| s.as_str()) == Some("idle") =>
            {
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
    let out =
        block_on(run_one("print('hello-from-jet-test')")).expect("run_one should succeed");
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

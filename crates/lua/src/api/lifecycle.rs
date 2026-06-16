//! Kernel lifecycle: start, shutdown, interrupt, list.

use anyhow::Context;
use futures_util::StreamExt;
use jet_core::jupyter;
use jet_core::kallichore::{Channel, Client, WsStream};
use jet_core::kernel::KernelSpec;
use mlua::prelude::*;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

use crate::router::{Frame, FrameRouter};
use crate::runtime::{KERNELS, KernelHandle, get, rt};

/// `jet.start_kernel(spec_path) -> (session_id, info)`
///
/// Loads the kernelspec, joins-or-spawns kcserver, creates+starts a session,
/// performs `kernel_info_request`, and returns the kallichore session id
/// plus the kernel's `kernel_info_reply.content` table.
pub fn start_kernel(lua: &Lua, spec_path: String) -> LuaResult<(String, LuaValue)> {
    let spec = KernelSpec::load(&PathBuf::from(&spec_path))
        .with_context(|| format!("loading kernelspec {spec_path}"))
        .into_lua_err()?;

    let (session_id, info, handle) = rt()
        .block_on(start_kernel_async(spec))
        .into_lua_err()?;

    KERNELS
        .lock()
        .unwrap()
        .insert(session_id.clone(), handle);

    Ok((session_id, lua.to_value(&info)?))
}

async fn start_kernel_async(
    spec: KernelSpec,
) -> anyhow::Result<(String, Value, Arc<KernelHandle>)> {
    let kcserver = std::env::var("JET_KCSERVER").unwrap_or_else(|_| "kcserver".to_string());
    // No persistent kcfile from Lua callers — they get a fresh per-session
    // kcserver. Multi-kernel reuse will come when we wire kc files in.
    let client = Client::spawn(&kcserver, None, false).await?;

    let session_id = format!(
        "jet-{:x}",
        rand::random::<u64>(),
    );
    let display_name = spec.display_name.as_deref().unwrap_or("jet");
    client
        .create_session(
            &session_id,
            display_name,
            &spec.language,
            &spec.argv,
            &spec.env,
            spec.interrupt_mode,
        )
        .await?;

    let ws = client.open_channels(&session_id).await?;
    let (sink, stream) = ws.split();
    let mut channel = Channel::new(sink);

    client.start_session(&session_id).await?;

    // Send kernel_info_request and watch for its reply on the stream side
    // BEFORE we hand the stream off to the per-kernel reader task. This
    // way Lua callers see a fully booted kernel by the time start_kernel
    // returns.
    let info_id = jupyter::new_msg_id();
    let info_req = jupyter::message("shell", &info_id, "kernel_info_request", json!({}));
    channel.send(&info_req).await?;
    let info = await_kernel_info_reply(stream, &info_id, Duration::from_secs(10)).await?;

    // info.0 = the parsed content; info.1 = the WsStream handed back to us
    // post-await so we can spawn the reader on the same connection.
    let (info_content, stream) = info;

    let router = Arc::new(FrameRouter::new());
    spawn_reader(stream, router.clone());

    let handle = Arc::new(KernelHandle {
        client: Arc::new(client),
        channel: Arc::new(tokio::sync::Mutex::new(channel)),
        router,
        session_id: session_id.clone(),
    });
    Ok((session_id, info_content, handle))
}

/// Drain the WsStream until the `kernel_info_reply` for `expected_id`
/// arrives, then return its content alongside the (still-open) stream so
/// the caller can hand it to the long-running reader task.
async fn await_kernel_info_reply(
    mut stream: futures_util::stream::SplitStream<WsStream>,
    expected_id: &str,
    timeout: Duration,
) -> anyhow::Result<(Value, futures_util::stream::SplitStream<WsStream>)> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            anyhow::bail!("timed out waiting for kernel_info_reply");
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => {
                let v: Value = match serde_json::from_str(&t) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let parent = v
                    .pointer("/parent_header/msg_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let msg_type = v
                    .pointer("/header/msg_type")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                if parent == expected_id && msg_type == "kernel_info_reply" {
                    let content = v.get("content").cloned().unwrap_or(Value::Null);
                    return Ok((content, stream));
                }
                // Drop everything else during boot — we're not registered
                // with the router yet, and the caller doesn't see this
                // chatter.
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => anyhow::bail!("ws error during boot: {e}"),
            Ok(None) => anyhow::bail!("websocket closed before kernel_info_reply"),
            Err(_) => anyhow::bail!("timed out waiting for kernel_info_reply"),
        }
    }
}

/// Spawn the long-running per-kernel reader. Frames go through
/// `parse_event` for the idle/exited cues; everything else is forwarded
/// to the router as a raw `(msg_type, content)` pair so Lua sees the full
/// kernel payload.
fn spawn_reader(
    stream: futures_util::stream::SplitStream<WsStream>,
    router: Arc<FrameRouter>,
) {
    rt().spawn(async move {
        let mut stream = stream;
        while let Some(msg) = stream.next().await {
            let t = match msg {
                Ok(Message::Text(t)) => t,
                Ok(Message::Close(_)) => break,
                Ok(_) => continue,
                Err(e) => {
                    log::warn!("ws error: {e}");
                    break;
                }
            };
            let v: Value = match serde_json::from_str(&t) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("frame parse failed: {e}");
                    continue;
                }
            };
            // Only act on jupyter-kind frames; ignore kernel-lifecycle
            // (`kind: "kernel"`) noise except for the exited signal, which
            // we treat as "tear everything down."
            let kind = v.get("kind").and_then(|s| s.as_str()).unwrap_or("");
            if kind == "kernel" {
                let exited_status = v
                    .pointer("/status/status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                if v.get("exited").is_some() || exited_status == "exited" {
                    log::info!("kernel exited; reader stopping");
                    break;
                }
                continue;
            }
            let parent_id = v
                .pointer("/parent_header/msg_id")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            let msg_type = v
                .pointer("/header/msg_type")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let channel = v.get("channel").and_then(|s| s.as_str()).unwrap_or("");
            let content = v.get("content").cloned().unwrap_or(Value::Null);

            if channel == "iopub"
                && msg_type == "status"
                && content
                    .get("execution_state")
                    .and_then(|s| s.as_str())
                    == Some("idle")
            {
                if let Some(pid) = parent_id {
                    router.dispatch(None, Frame::Idle { parent_msg_id: pid });
                }
                continue;
            }

            router.dispatch(
                parent_id.as_deref(),
                Frame::Content { msg_type, content },
            );
        }
    });
}

/// `jet.shutdown_kernel(session_id)` — graceful `shutdown_request` over
/// control, then DELETE the session record. Mirrors `jet stop` in the CLI.
pub fn shutdown_kernel(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    rt()
        .block_on(async move {
            // Best-effort graceful shutdown: send shutdown_request on the
            // control channel, wait briefly, then ask kallichore to delete
            // the session record. Errors are logged but don't propagate
            // unless the whole flow fails.
            let msg_id = jupyter::new_msg_id();
            let req = jupyter::message(
                "control",
                &msg_id,
                "shutdown_request",
                json!({ "restart": false }),
            );
            if let Err(e) = handle.channel.lock().await.send(&req).await {
                log::warn!("shutdown_request send failed: {e}");
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = handle.channel.lock().await.close().await;
            // If the kernel didn't honor shutdown_request, kill outright.
            let _ = handle.client.kill_session(&handle.session_id).await;
            handle
                .client
                .delete_session(&handle.session_id)
                .await
                .ok();
            anyhow::Ok(())
        })
        .into_lua_err()?;
    KERNELS.lock().unwrap().remove(&session_id);
    Ok(())
}

/// `jet.interrupt(session_id)` — POST `/sessions/{id}/interrupt`. Reply
/// shape from the kernel (KeyboardInterrupt error → idle) flows through
/// the existing execute_code poll closure.
pub fn interrupt(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    rt()
        .block_on(handle.client.interrupt_session(&handle.session_id))
        .into_lua_err()
}

/// `jet.list_running_kernels()` — `{ [session_id] = { language, ... }, ... }`
/// from kallichore's view. Pulls live data, not the local registry, so any
/// sessions another client created on the same kcserver show up too.
pub fn list_running_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    // Without a registered local kernel there's no kcserver to query, so
    // pick any one and ask it. If none registered, return empty.
    let handle = {
        let map = KERNELS.lock().unwrap();
        map.values().next().cloned()
    };
    let table = lua.create_table()?;
    let Some(handle) = handle else {
        return Ok(table);
    };
    let sessions = rt()
        .block_on(handle.client.list_sessions())
        .into_lua_err()?;
    for s in sessions {
        let entry = lua.create_table()?;
        entry.set("language", s.language.clone())?;
        entry.set("display_name", s.display_name.to_string())?;
        entry.set("status", s.status.to_string())?;
        if let Some(pid) = s.process_id {
            entry.set("pid", pid)?;
        }
        table.set(s.session_id.clone(), entry)?;
    }
    Ok(table)
}

/// `jet.list_available_kernels()` — `{ [path] = KernelSpec }` discovered
/// under the standard Jupyter directories. Scans the same locations
/// `jupyter kernelspec list` would.
pub fn list_available_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    for (path, spec) in discover_kernelspecs() {
        let entry = lua.create_table()?;
        entry.set("language", spec.language)?;
        if let Some(d) = spec.display_name {
            entry.set("display_name", d)?;
        }
        entry.set("argv", spec.argv)?;
        table.set(path.to_string_lossy().to_string(), entry)?;
    }
    Ok(table)
}

fn discover_kernelspecs() -> Vec<(PathBuf, KernelSpec)> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let h = PathBuf::from(home);
        roots.push(h.join("Library/Jupyter/kernels")); // macOS
        roots.push(h.join(".local/share/jupyter/kernels")); // Linux
        roots.push(h.join(".jupyter/kernels"));
    }
    roots.push(PathBuf::from("/usr/local/share/jupyter/kernels"));
    roots.push(PathBuf::from("/usr/share/jupyter/kernels"));

    let mut out = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let kj = entry.path().join("kernel.json");
            if !kj.exists() {
                continue;
            }
            match KernelSpec::load(&kj) {
                Ok(s) => out.push((kj, s)),
                Err(e) => log::debug!("skipping {kj:?}: {e}"),
            }
        }
    }
    out
}

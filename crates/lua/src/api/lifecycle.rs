//! Kernel lifecycle: start, attach, shutdown, interrupt, list.

use anyhow::Context;
use jet_core::events::{Channel, Event, from_message, raw_msg_type_and_content};
use jet_core::jupyter_protocol::{JupyterMessage, KernelInfoRequest};
use jet_core::kernel::{Kernel, KernelSpec};
use mlua::prelude::*;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::router::{Frame, FrameRouter};
use crate::runtime::{KERNELS, KernelHandle, get, rt};

/// `jet.connect(spec_path, connection_file?) -> (session_id, info)`
///
/// Spawn a kernel from `spec_path`. If `connection_file` is given,
/// attach to a kernel already listening there or spawn against that
/// path if the attach fails (mirrors [`Kernel::attach_or_spawn`] and
/// `jet connect --connection-file`).
pub fn connect(
    lua: &Lua,
    (spec_path, connection_file): (String, Option<String>),
) -> LuaResult<(String, LuaValue)> {
    let spec = KernelSpec::load(&PathBuf::from(&spec_path))
        .with_context(|| format!("loading kernelspec {spec_path}"))
        .into_lua_err()?;

    let (session_id, info, handle) = rt()
        .block_on(async move {
            let kernel = match connection_file {
                Some(p) => Kernel::attach_or_spawn(&spec, &PathBuf::from(p)).await?,
                None => Kernel::spawn(&spec, None).await?,
            };
            boot_kernel(kernel).await
        })
        .into_lua_err()?;

    KERNELS.lock().unwrap().insert(session_id.clone(), handle);
    Ok((session_id, lua.to_value(&info)?))
}

/// `jet.attach(connection_file) -> (session_id, info)`
///
/// Attach to a kernel already running on `connection_file`. Mirrors
/// `jet attach`: no kernelspec, never spawns.
pub fn attach(lua: &Lua, connection_file: String) -> LuaResult<(String, LuaValue)> {
    let path = PathBuf::from(&connection_file);
    let (session_id, info, handle) = rt()
        .block_on(async move { boot_kernel(Kernel::attach(&path).await?).await })
        .into_lua_err()?;

    KERNELS.lock().unwrap().insert(session_id.clone(), handle);
    Ok((session_id, lua.to_value(&info)?))
}

/// Move all four channel halves out of the kernel, send
/// `kernel_info_request` synchronously, then spawn the long-running
/// reader/writer tasks and return a populated [`KernelHandle`]. Used
/// by both `start_kernel` and `attach_kernel`.
async fn boot_kernel(
    mut kernel: Kernel,
) -> anyhow::Result<(String, Value, Arc<KernelHandle>)> {
    let session_id = kernel.session_id.clone();

    let mut shell = kernel.channels.take_shell()?;
    let iopub = kernel.channels.take_iopub()?;
    let stdin_sock = kernel.channels.take_stdin()?;

    // kernel_info_request: send + wait for reply on the still-borrowed
    // shell socket, before spawning the long-running shell task.
    let info_req: JupyterMessage = KernelInfoRequest {}.into();
    let info_id = info_req.header.msg_id.clone();
    shell
        .send(info_req)
        .await
        .context("shell.send")?;
    let info = match tokio::time::timeout(
        Duration::from_secs(10),
        await_kernel_info(&mut shell, &info_id),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => anyhow::bail!("timed out waiting for kernel_info_reply"),
    };

    let (shell_tx, shell_rx) = tokio::sync::mpsc::unbounded_channel::<JupyterMessage>();
    let (stdin_tx, stdin_rx) = tokio::sync::mpsc::unbounded_channel::<JupyterMessage>();

    let router = Arc::new(FrameRouter::new());

    spawn_shell_loop(shell, shell_rx, router.clone());
    spawn_iopub_reader(iopub, router.clone());
    spawn_stdin_loop(stdin_sock, stdin_rx, router.clone());

    let handle = Arc::new(KernelHandle {
        kernel: Arc::new(tokio::sync::Mutex::new(kernel)),
        shell_tx,
        stdin_tx,
        router,
    });
    Ok((session_id, info, handle))
}

async fn await_kernel_info(
    shell: &mut jet_core::jupyter_zmq_client::ClientShellConnection,
    expected_id: &str,
) -> anyhow::Result<Value> {
    loop {
        let msg = shell
            .read()
            .await
            .map_err(|e| anyhow::anyhow!("shell.read: {e}"))?;
        let parent = msg
            .parent_header
            .as_ref()
            .map(|h| h.msg_id.as_str())
            .unwrap_or("");
        if parent == expected_id && msg.message_type() == "kernel_info_reply" {
            return Ok(serde_json::to_value(&msg.content)?);
        }
        // Otherwise drop the message — boot-time chatter we don't show.
    }
}

fn spawn_shell_loop(
    mut shell: jet_core::jupyter_zmq_client::ClientShellConnection,
    mut send_rx: UnboundedReceiver<JupyterMessage>,
    router: Arc<FrameRouter>,
) {
    rt().spawn(async move {
        loop {
            tokio::select! {
                send = send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = shell.send(msg).await {
                            log::warn!("shell.send: {e}");
                            return;
                        }
                    }
                    None => return,
                },
                read = shell.read() => match read {
                    Ok(msg) => dispatch(&router, Channel::Shell, &msg),
                    Err(e) => {
                        log::warn!("shell.read: {e}");
                        return;
                    }
                },
            }
        }
    });
}

fn spawn_iopub_reader(
    mut iopub: jet_core::jupyter_zmq_client::ClientIoPubConnection,
    router: Arc<FrameRouter>,
) {
    rt().spawn(async move {
        loop {
            match iopub.read().await {
                Ok(msg) => dispatch(&router, Channel::IoPub, &msg),
                Err(e) => {
                    log::warn!("iopub.read: {e}");
                    return;
                }
            }
        }
    });
}

fn spawn_stdin_loop(
    mut stdin_sock: jet_core::jupyter_zmq_client::ClientStdinConnection,
    mut send_rx: UnboundedReceiver<JupyterMessage>,
    router: Arc<FrameRouter>,
) {
    rt().spawn(async move {
        loop {
            tokio::select! {
                send = send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = stdin_sock.send(msg).await {
                            log::warn!("stdin.send: {e}");
                            return;
                        }
                    }
                    None => return,
                },
                read = stdin_sock.read() => match read {
                    Ok(msg) => dispatch(&router, Channel::Stdin, &msg),
                    Err(e) => {
                        log::warn!("stdin.read: {e}");
                        return;
                    }
                },
            }
        }
    });
}

/// Convert one message into router frames. Idle is its own terminal
/// signal; everything else becomes a `Content` frame keyed by parent_id.
fn dispatch(router: &FrameRouter, channel: Channel, msg: &JupyterMessage) {
    let parent_id = msg.parent_header.as_ref().map(|h| h.msg_id.clone());
    if let Event::Idle { parent_id } = from_message(channel, msg) {
        router.dispatch(None, Frame::Idle { parent_msg_id: parent_id });
        return;
    }
    let (msg_type, content) = raw_msg_type_and_content(msg);
    router.dispatch(parent_id.as_deref(), Frame::Content { msg_type, content });
}

/// `jet.shutdown_kernel(session_id)`
pub fn shutdown_kernel(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    rt()
        .block_on(async move { handle.kernel.lock().await.shutdown().await })
        .into_lua_err()?;
    KERNELS.lock().unwrap().remove(&session_id);
    Ok(())
}

/// `jet.interrupt(session_id)`
pub fn interrupt(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    rt()
        .block_on(async move { handle.kernel.lock().await.interrupt().await })
        .into_lua_err()
}

/// `jet.list_running_kernels()` — local registry only. Without a
/// supervisor, jet_lua only knows about kernels it itself started or
/// attached to in this process.
pub fn list_running_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    let map = KERNELS.lock().unwrap();
    for id in map.keys() {
        let entry = lua.create_table()?;
        entry.set("status", "running")?;
        table.set(id.clone(), entry)?;
    }
    Ok(table)
}

/// `jet.list_available_kernels()` — kernelspecs discovered under the
/// standard Jupyter directories.
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
        roots.push(h.join("Library/Jupyter/kernels"));
        roots.push(h.join(".local/share/jupyter/kernels"));
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

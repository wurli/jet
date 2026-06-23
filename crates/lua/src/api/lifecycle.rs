//! Kernel lifecycle: start, attach, shutdown, interrupt, list.

use anyhow::Context;
use jet_core::client::Client;
use jet_core::kernel::{Kernel, KernelSpec};
use mlua::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::runtime::{KERNELS, KernelHandle, get, rt};

/// `jet.connect(spec_path, connection_file?) -> (session_id, info)`
///
/// Spawn a kernel from `spec_path`. If `connection_file` is given and
/// the path already exists, errors — use `jet.attach` to reconnect.
/// Mirrors `jet connect --connection-file`.
pub fn connect(
    lua: &Lua,
    (spec_path, connection_file, session_name): (String, Option<String>, Option<String>),
) -> LuaResult<(String, LuaValue)> {
    let spec = KernelSpec::load(&PathBuf::from(&spec_path))
        .with_context(|| format!("loading kernelspec {spec_path}"))
        .into_lua_err()?;

    let conn_path = connection_file.map(PathBuf::from);
    if let Some(p) = &conn_path
        && p.exists()
    {
        return Err(LuaError::external(anyhow::anyhow!(
            "connection file already exists at {0}: remove it or call jet.attach({0:?}) to reconnect",
            p.display(),
        )));
    }

    let (session_id, info, handle) = rt()
        .block_on(async move {
            let kernel = Kernel::spawn(&spec, conn_path, session_name.as_deref()).await?;
            wrap(kernel).await
        })
        .into_lua_err()?;

    KERNELS.lock().unwrap().insert(session_id.clone(), handle);
    Ok((session_id, lua.to_value(&info)?))
}

/// `jet.attach(connection_file) -> (session_id, info)`
///
/// Attach to a kernel already running on `connection_file`. Mirrors
/// `jet attach`: no kernelspec, never spawns.
pub fn attach(
    lua: &Lua,
    (connection_file, session_name): (String, Option<String>),
) -> LuaResult<(String, LuaValue)> {
    let path = PathBuf::from(&connection_file);
    let (session_id, info, handle) = rt()
        .block_on(async move {
            let kernel = Kernel::attach(&path, session_name.as_deref()).await?;
            wrap(kernel).await
        })
        .into_lua_err()?;

    KERNELS.lock().unwrap().insert(session_id.clone(), handle);
    Ok((session_id, lua.to_value(&info)?))
}

/// Wrap a freshly built [`Kernel`] in a [`KernelSession`] and the
/// lua-side shared handle, returning the session id (taken from the
/// kernel) and the kernel_info reply.
async fn wrap(kernel: Kernel) -> anyhow::Result<(String, serde_json::Value, KernelHandle)> {
    let session_id = kernel.session_id.clone();
    // KernelSession::start performs a kernel_info handshake, doubling
    // as the is-the-kernel-actually-answering check on attach.
    let (session, info) = Client::start(kernel).await?;
    let handle: KernelHandle = Arc::new(tokio::sync::Mutex::new(session));
    Ok((session_id, info, handle))
}

/// `jet.stop(session_id)`
pub fn shutdown_kernel(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    KERNELS.lock().unwrap().remove(&session_id);
    // Drop our registry reference; if no other handle holds it, take
    // ownership of the session and call shutdown (which consumes self).
    rt().block_on(async move {
        match Arc::try_unwrap(handle) {
            Ok(mutex) => mutex.into_inner().shutdown().await,
            Err(arc) => {
                // Another caller is still holding a handle (unusual);
                // best-effort interrupt+drop instead.
                let mut guard = arc.lock().await;
                guard.kernel_mut().shutdown().await
            }
        }
    })
    .into_lua_err()
}

/// `jet.interrupt(session_id)`
pub fn interrupt(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    rt().block_on(async move { handle.lock().await.interrupt().await })
        .into_lua_err()
}

/// `jet.list_sessions()` — local registry only. Without a
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

/// `jet.list_kernels()` — kernelspecs discovered under the
/// standard Jupyter directories.
pub fn list_available_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    for (path, spec) in KernelSpec::find_valid() {
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

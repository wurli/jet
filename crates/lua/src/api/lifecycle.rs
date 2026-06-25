//! Kernel lifecycle: start, attach, shutdown, interrupt, list.

use anyhow::Context;
use jet_core::client::Client;
use jet_core::kernel::KernelSpec;
use mlua::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::runtime::{KERNELS, KernelHandle, get, runtime};

/// `jet.connect(spec_path, connection_file?) -> (client_id, info)`
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

    let (client, info) = runtime()
        .block_on(Client::spawn(
            &spec,
            conn_path,
            session_name.as_deref(),
            |_| {},
        ))
        .into_lua_err()?;
    register(lua, client, info)
}

/// `jet.attach(connection_file) -> (client_id, info)`
///
/// Attach to a kernel already running on `connection_file`. Mirrors
/// `jet attach`: no kernelspec, never spawns.
pub fn attach(
    lua: &Lua,
    (connection_file, session_name): (String, Option<String>),
) -> LuaResult<(String, LuaValue)> {
    let path = PathBuf::from(&connection_file);
    let (client, info) = runtime()
        .block_on(Client::attach(&path, session_name.as_deref(), |_| {}))
        .into_lua_err()?;
    register(lua, client, info)
}

/// Insert a freshly built [`Client`] into the lua-side registry, returning the id and
/// kernel_info reply in lua-friendly form.
fn register(lua: &Lua, client: Client, info: serde_json::Value) -> LuaResult<(String, LuaValue)> {
    let client_id = client.client_id().to_string();
    let handle: KernelHandle = Arc::new(tokio::sync::Mutex::new(client));
    KERNELS.lock().unwrap().insert(client_id.clone(), handle);
    Ok((client_id, lua.to_value(&info)?))
}

/// `jet.stop(session_id)`
pub fn shutdown_kernel(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    KERNELS.lock().unwrap().remove(&session_id);
    runtime()
        .block_on(async move { handle.lock().await.shutdown().await })
        .into_lua_err()
}

/// `jet.interrupt(session_id)`
pub fn interrupt(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    runtime()
        .block_on(async move { handle.lock().await.interrupt().await })
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
        entry.set("spec", lua.to_value(&spec)?)?;
        entry.set("path", path.to_string_lossy().to_string())?;
        table.push(entry)?;
    }
    Ok(table)
}

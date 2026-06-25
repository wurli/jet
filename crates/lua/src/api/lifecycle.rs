//! Kernel lifecycle: start, attach, shutdown, interrupt, list.

use anyhow::Context;
use jet_core::client::Client;
use jet_core::kernel::KernelSpec;
use jet_core::manager::SessionStore;
use mlua::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

use crate::runtime::{KERNELS, KernelHandle, get, runtime};

/// `jet.connect(spec_path, connection_file?, session_name?) -> (client_id, info)`
///
/// Spawn a kernel from `spec_path`. Mirrors `jet connect`:
/// - no `connection_file` → create a tracked SessionStore entry and use its connection path;
///   the resulting Client carries the SessionStore id.
/// - `connection_file` given → caller owns the path; no SessionStore entry is written and
///   the Client has no `session_id`. The path must not already exist (use `jet.attach`).
pub fn connect(
    lua: &Lua,
    (spec_path, connection_file, session_name): (String, Option<String>, Option<String>),
) -> LuaResult<(String, LuaValue)> {
    let spec = KernelSpec::load(&PathBuf::from(&spec_path))
        .with_context(|| format!("loading kernelspec {spec_path}"))
        .into_lua_err()?;

    let (conn_path, session_id, mut store_entry) = match connection_file {
        Some(p) => {
            let path = PathBuf::from(p);
            if path.exists() {
                return Err(LuaError::external(anyhow::anyhow!(
                    "connection file already exists at {0}: remove it or call jet.attach({0:?}) to reconnect",
                    path.display(),
                )));
            }
            (Some(path), None, None)
        }
        None => {
            let store = SessionStore::default().into_lua_err()?;
            let cwd = std::env::current_dir().into_lua_err()?;
            let entry = store
                .create(
                    &spec.language,
                    spec.display_name.as_deref().unwrap_or(""),
                    &PathBuf::from(&spec_path),
                    &cwd,
                )
                .into_lua_err()?;
            let id = entry.meta().id.clone();
            let path = entry.connection_file_path();
            (Some(path), Some(id), Some(entry))
        }
    };

    let (client, info) = runtime()
        .block_on(Client::spawn(
            &spec,
            conn_path,
            session_name.as_deref(),
            session_id,
            |_| {},
        ))
        .into_lua_err()?;

    if let (Some(pid), Some(s)) = (client.child_pid(), store_entry.as_mut()) {
        s.set_kernel_pid(pid);
    }

    register(lua, client, info)
}

/// `jet.attach(connection_file, session_name?) -> (client_id, info)`
///
/// Attach to a kernel already running on `connection_file`. If the path lives inside a
/// tracked SessionStore entry, the Client carries its `session_id`; otherwise `session_id`
/// is `None`. Mirrors `jet attach --connection-file`.
pub fn attach(
    lua: &Lua,
    (connection_file, session_name): (String, Option<String>),
) -> LuaResult<(String, LuaValue)> {
    let path = PathBuf::from(&connection_file);
    let session_id = SessionStore::default()
        .ok()
        .and_then(|s| s.find_by_connection_file(&path).ok().flatten())
        .map(|s| s.meta().id.clone());

    let (client, info) = runtime()
        .block_on(Client::attach(
            &path,
            session_name.as_deref(),
            session_id,
            |_| {},
        ))
        .into_lua_err()?;
    register(lua, client, info)
}

/// Insert a freshly built [`Client`] into the lua-side registry (keyed by `client_id`),
/// returning the id and kernel_info reply in lua-friendly form.
fn register(lua: &Lua, client: Client, info: serde_json::Value) -> LuaResult<(String, LuaValue)> {
    let client_id = client.client_id().to_string();
    let handle: KernelHandle = Arc::new(tokio::sync::Mutex::new(client));
    KERNELS.lock().unwrap().insert(client_id.clone(), handle);
    Ok((client_id, lua.to_value(&info)?))
}

/// `jet.stop(client_id)`
pub fn shutdown_kernel(_lua: &Lua, client_id: String) -> LuaResult<()> {
    let handle = get(&client_id).into_lua_err()?;
    KERNELS.lock().unwrap().remove(&client_id);
    runtime()
        .block_on(async move {
            let mut guard = handle.lock().await;
            // Mark the SessionStore entry closed so disk-backed `list_sessions` reflects it.
            if let Some(sid) = guard.session_id()
                && let Ok(store) = SessionStore::default()
                && let Ok(mut s) = store.open(sid)
            {
                s.mark_closed();
            }
            guard.shutdown().await
        })
        .into_lua_err()
}

/// `jet.interrupt(client_id)`
pub fn interrupt(_lua: &Lua, client_id: String) -> LuaResult<()> {
    let handle = get(&client_id).into_lua_err()?;
    runtime()
        .block_on(async move { handle.lock().await.interrupt().await })
        .into_lua_err()
}

/// `jet.list_connections()` — clients open *in this process*. Keyed by `client_id`.
/// Each entry carries the bound SessionStore id (if any) so callers can correlate with
/// `jet.list_sessions()`.
pub fn list_connections(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    let map = KERNELS.lock().unwrap();
    // Snapshot client→session mappings under the registry lock without awaiting; reading
    // session_id() needs the per-client Mutex but try_lock avoids parking the runtime.
    for (client_id, handle) in map.iter() {
        let entry = lua.create_table()?;
        let session_id = handle.try_lock().ok().and_then(|c| c.session_id().map(str::to_string));
        if let Some(sid) = session_id {
            entry.set("session_id", sid)?;
        }
        entry.set("status", "running")?;
        table.set(client_id.clone(), entry)?;
    }
    Ok(table)
}

/// `jet.list_sessions()` — Jet sessions on disk (the SessionStore). Returns every
/// `session.json` jet has written, regardless of which process owns the live client (or
/// whether one is open at all). Each entry exposes the full `SessionMeta`.
pub fn list_sessions(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    let store = SessionStore::default().into_lua_err()?;
    for meta in store.list().into_lua_err()? {
        let entry = lua.to_value(&meta)?;
        table.set(meta.id.clone(), entry)?;
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

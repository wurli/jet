//! Kernel lifecycle: start, attach, shutdown, interrupt, list.

use anyhow::Context;
use jet_core::client::{Client, make_client_id};
use jet_core::connection_file;
use jet_core::kernel::{Kernel, KernelSpec, probe_kernel_alive};
use jet_core::manager::{SessionStore, generate_session_name};
use mlua::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use crate::runtime::{KERNELS, KernelHandle, get, runtime};

/// `jet.start(spec_path, connection_file?, session_name?) -> (client_id, info)`
///
/// Spawn a kernel from `spec_path`. Mirrors `jet start`:
/// - no `connection_file` → create a tracked SessionStore entry and use its connection path;
///   the resulting Client carries the SessionStore id.
/// - `connection_file` given → caller owns the path; no SessionStore entry is written and
///   the Client has no `session_id`. The path must not already exist (use `jet.attach`).
pub fn start(
    lua: &Lua,
    (spec_path, connection_file, session_name): (String, Option<String>, Option<String>),
) -> LuaResult<LuaTable> {
    let spec = KernelSpec::load(&PathBuf::from(&spec_path))
        .with_context(|| format!("loading kernelspec {spec_path}"))
        .into_lua_err()?;

    let (conn_path, session_id, mut store_entry) = match connection_file {
        Some(p) => {
            let path = PathBuf::from(p);
            if path.exists() {
                return Err(LuaError::external(anyhow::anyhow!(
                    "connection file already exists at {0}: remove it or call jet.attach(nil, {0:?}) to reconnect",
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
            let id = entry.meta().session_id.clone();
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

/// `jet.attach(session_id, connection_file?, session_name?) -> (client_id, info)`
///
/// Attach to a kernel already running. Mirrors `jet attach`:
/// - `session_id` given (and no `connection_file`) → resolve the connection file via the
///   SessionStore; the Client carries the session id.
/// - `connection_file` given (and no `session_id`) → attach to the path. If it lives inside
///   a tracked SessionStore entry, the Client carries that session id; otherwise none.
/// - both `nil` → error (Lua has no picker; pass one of them).
/// - both set → error (mutually exclusive, matching the CLI's ArgGroup).
pub fn attach(
    lua: &Lua,
    (session_id, connection_file, session_name): (Option<String>, Option<String>, Option<String>),
) -> LuaResult<LuaTable> {
    let (path, session_id) = match (session_id, connection_file) {
        (Some(id), None) => {
            let path = SessionStore::default()
                .into_lua_err()?
                .open(&id)
                .into_lua_err()?
                .connection_file_path();
            (path, Some(id))
        }
        (None, Some(p)) => {
            let path = PathBuf::from(p);
            let id = SessionStore::default()
                .ok()
                .and_then(|s| s.find_by_connection_file(&path).ok().flatten())
                .map(|s| s.meta().session_id.clone());
            (path, id)
        }
        (None, None) => {
            return Err(LuaError::external(anyhow::anyhow!(
                "jet.attach: pass either a session_id or a connection_file"
            )));
        }
        (Some(_), Some(_)) => {
            return Err(LuaError::external(anyhow::anyhow!(
                "jet.attach: session_id and connection_file are mutually exclusive"
            )));
        }
    };

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
fn register(lua: &Lua, client: Client, info: serde_json::Value) -> LuaResult<LuaTable> {
    let client_id = client.client_id().to_string();
    let session_id = client.session_id().map(str::to_string);
    let handle: KernelHandle = Arc::new(tokio::sync::Mutex::new(client));
    KERNELS.lock().unwrap().insert(client_id.clone(), handle);

    let out = lua.create_table()?;

    out.set("client_id", client_id.clone())?;
    out.set("kernel_info", crate::to_lua_value(lua, &info)?)?;

    if let Some(session_id) = session_id {
        out.set("session_id", session_id)?;
    }

    Ok(out)
}

/// `jet.stop(session_id)`
///
/// Resolves the session via the on-disk SessionStore, attaches a fresh
/// client to the kernel's connection file, and sends `shutdown_request`.
/// Mirrors `jet stop <session_id>` — works for any tracked session,
/// regardless of which process owns the live in-memory client.
pub fn shutdown_kernel(_lua: &Lua, session_id: String) -> LuaResult<()> {
    let path = SessionStore::default()
        .into_lua_err()?
        .open(&session_id)
        .into_lua_err()?
        .connection_file_path();
    runtime()
        .block_on(async move {
            let connection = connection_file::read(path.as_path())?;
            if probe_kernel_alive(&connection).await.is_ok() {
                let client_id = make_client_id(None);
                let mut kernel = Kernel::attach(&path, &client_id).await?;
                kernel.shutdown().await
            } else {
                Ok(())
            }
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
    let out = lua.create_table()?;
    let map = KERNELS.lock().unwrap();
    // Snapshot client→session mappings under the registry lock without awaiting; reading
    // session_id() needs the per-client Mutex but try_lock avoids parking the runtime.
    for (client_id, handle) in map.iter() {
        let entry = lua.create_table()?;
        let session_id = handle
            .try_lock()
            .ok()
            .and_then(|c| c.session_id().map(str::to_string));
        if let Some(sid) = session_id {
            entry.set("session_id", sid)?;
        }
        entry.set("client_id", client_id.clone())?;
        out.push(entry)?;
    }
    Ok(out)
}

/// `jet.list_sessions({ status?, all_dirs? })` — Jet sessions on disk (the SessionStore).
/// Returns every `session.json` jet has written, regardless of which process owns the live
/// client (or whether one is open at all). Each entry exposes the full `SessionMeta`.
///
/// Mirrors `jet list-sessions`:
/// - `status`: `"open"` (default), `"closed"`, or `"all"`.
/// - `all_dirs`: when true, return sessions for every working directory; otherwise only
///   sessions whose `working_dir` matches the current dir.
///
/// Probes Open sessions first so kernels that exited while detached are flipped to Closed
/// before filtering.
pub fn list_sessions(lua: &Lua, opts: Option<LuaTable>) -> LuaResult<LuaTable> {
    let (status, all_dirs) = match opts {
        Some(t) => (
            t.get::<Option<String>>("status")?,
            t.get::<Option<bool>>("all_dirs")?.unwrap_or(false),
        ),
        None => (None, false),
    };
    let status: jet_core::manager::StatusFilter =
        status.as_deref().unwrap_or("open").parse().into_lua_err()?;

    let store = SessionStore::default().into_lua_err()?;
    let sessions = runtime()
        .block_on(store.list_filtered(status, all_dirs))
        .into_lua_err()?;

    let table = lua.create_table()?;
    for session in sessions {
        table.push(crate::to_lua_value(lua, &session)?)?;
    }
    Ok(table)
}

/// `jet.show(session_id) -> { session, spec }`
///
/// Look up a session by id and return its `SessionMeta` alongside the
/// parsed kernelspec read from `session.kernelspec_path`.
pub fn show_session(lua: &Lua, session_id: String) -> LuaResult<LuaValue> {
    let view = jet_core::manager::show_session(&session_id).into_lua_err()?;
    crate::to_lua_value(lua, &view)
}

pub fn show_spec(lua: &Lua, spec_path: String) -> LuaResult<LuaValue> {
    let spec = KernelSpec::load(&PathBuf::from(&spec_path))
        .with_context(|| format!("loading kernelspec {spec_path}"))
        .into_lua_err()?;
    crate::to_lua_value(lua, &spec)
}

/// `jet.list_kernels()` — kernelspecs discovered under the
/// standard Jupyter directories.
pub fn list_available_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    for (path, spec) in KernelSpec::find_valid() {
        let entry = lua.create_table()?;
        entry.set("spec", crate::to_lua_value(lua, &spec)?)?;
        entry.set("path", path.to_string_lossy().to_string())?;
        table.push(entry)?;
    }
    Ok(table)
}

/// `jet.make_session_id(lang, cwd?) -> string`
///
/// Mint a session id in jet's canonical format
/// (`<timestamp>_<lang>_<basename>_<rand>`). Use this to pre-allocate an
/// id from Lua, then pass it to `jet start --session-id <id>` so both
/// sides share the same handle without baking the format into Lua.
pub fn make_session_id(_: &Lua, lang: String) -> LuaResult<String> {
    Ok(generate_session_name(
        SystemTime::now(),
        &lang,
        &std::env::current_dir().into_lua_err()?,
    ))
}

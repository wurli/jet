//! Kernel lifecycle: start, attach, shutdown, interrupt, list.

use anyhow::Context;
use jet_core::client::{Client, make_client_id};
use jet_core::connection_file;
use jet_core::kernel::{AttachOptions, Kernel, KernelSpec, probe_kernel_alive};
use jet_core::manager::{Session, SessionStore, generate_session_name};
use mlua::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::oneshot;

use jet_core::client::RequestStream;

use crate::poll::make_poll;
use jet_core::manager::{ClientHandle, ClientRegistry, runtime};

/// `jet.start(spec_path, connection_file?, session_name?) -> poll`
///
/// Spawn a kernel from `spec_path`. Non-blocking: returns a poll closure that
/// the caller drives (e.g. via `vim.schedule`) until it yields the
/// `jet.start.response`. While the kernel is booting the closure returns
/// `{status="pending"}`; on the first call after boot it returns
/// `{status="ready", client_id=..., session_id=..., kernel_info=..., stream=...}`
/// and registers the client. Subsequent calls return `nil`.
///
/// Mirrors `jet start`:
/// - no `connection_file` → create a tracked SessionStore entry and use its connection path;
///   the resulting Client carries the SessionStore id.
/// - `connection_file` given → caller owns the path; no SessionStore entry is written and
///   the Client has no `session_id`. The path must not already exist (use `jet.attach`).
pub fn start(
    lua: &Lua,
    (spec_path, connection_file, session_name): (String, Option<String>, Option<String>),
) -> LuaResult<(LuaFunction, Option<LuaValue>)> {
    let spec = KernelSpec::load(&PathBuf::from(&spec_path))
        .with_context(|| format!("loading kernelspec {spec_path}"))
        .into_lua_err()?;

    let (conn_path, store_entry) = match connection_file {
        Some(p) => {
            let path = PathBuf::from(p);
            if path.exists() {
                return Err(LuaError::external(anyhow::anyhow!(
                    "connection file already exists at {0}: remove it or call jet.attach(nil, {0:?}) to reconnect",
                    path.display(),
                )));
            }
            (Some(path), None)
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
            let path = entry.connection_file_path();
            (Some(path), Some(entry))
        }
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    let id = store_entry.as_ref().map(|s| s.meta().session_id.clone());
    runtime().spawn(async move {
        let _ = tx.send(Client::spawn(&spec, conn_path, session_name.as_deref(), id).await);
    });

    let meta = store_entry
        .as_ref()
        .map(|s| crate::to_lua_value(lua, &s.meta()))
        .transpose()?;

    Ok((make_lifecycle_poll(lua, rx, store_entry)?, meta))
}

/// `jet.attach(session_id, connection_file?, session_name?) -> poll`
///
/// Attach to a kernel already running. Non-blocking with the same poll-closure
/// contract as [`start`]: returns `{status="pending"}` until the handshake
/// completes, then `{status="ready", ..jet.start.response}` once. Mirrors
/// `jet attach`:
/// - `session_id` given (and no `connection_file`) → resolve the connection file via the
///   SessionStore; the Client carries the session id.
/// - `connection_file` given (and no `session_id`) → attach to the path. If it lives inside
///   a tracked SessionStore entry, the Client carries that session id; otherwise none.
/// - both `nil` → error (Lua has no picker; pass one of them).
/// - both set → error (mutually exclusive, matching the CLI's ArgGroup).
pub fn attach(
    lua: &Lua,
    (session_id, connection_file, session_name): (Option<String>, Option<String>, Option<String>),
) -> LuaResult<(LuaFunction, Option<LuaValue>)> {
    let (path, store_entry) = match (session_id, connection_file) {
        (Some(id), None) => {
            let session = SessionStore::default()
                .into_lua_err()?
                .open(&id)
                .into_lua_err()?;
            (session.connection_file_path(), Some(session))
        }
        (None, Some(p)) => {
            let path = PathBuf::from(p);
            let session = SessionStore::default()
                .ok()
                .and_then(|s| s.find_by_connection_file(&path).ok().flatten());
            (path, session)
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

    let (tx, rx) = oneshot::channel();
    let id = store_entry.as_ref().map(|s| s.meta().session_id.clone());
    // Best-effort: recover interrupt_mode + pid from the tracked session so
    // `jet.interrupt(client_id)` can actually forward ^C after an attach.
    // Falls back to signal-mode/no-pid when the connection file isn't part
    // of a tracked session, or the kernelspec on disk has since been removed.
    let attach_opts = match store_entry.as_ref() {
        Some(s) => {
            let meta = s.meta();
            let interrupt_mode = KernelSpec::load(&meta.kernelspec_path)
                .map(|k| k.interrupt_mode)
                .unwrap_or_default();
            AttachOptions {
                interrupt_mode,
                pid: meta.kernel_pid,
            }
        }
        None => AttachOptions::default(),
    };
    runtime().spawn(async move {
        let _ = tx.send(Client::attach(&path, session_name.as_deref(), id, attach_opts).await);
    });

    let meta = store_entry
        .as_ref()
        .map(|s| crate::to_lua_value(lua, &s.meta()))
        .transpose()?;

    Ok((make_lifecycle_poll(lua, rx, store_entry)?, meta))
}

/// Result delivered from the spawned boot future to the Lua-side poll closure.
type BootResult = anyhow::Result<(Client, serde_json::Value, RequestStream)>;

/// Build a Lua-callable poll closure that drives a backgrounded boot
/// (`Client::spawn` / `Client::attach`) to completion.
///
/// While the future is in flight, the closure returns `{status="pending"}`.
/// Once the future completes, the next call registers the client and returns
/// `{status="ready", client_id, session_id?, kernel_info, stream}` (the
/// `jet.start.response` table). After that, the closure returns `nil`.
///
/// If the boot future itself fails, the call that observes the failure
/// raises a Lua error.
fn make_lifecycle_poll(
    lua: &Lua,
    rx: oneshot::Receiver<BootResult>,
    store_entry: Option<Session>,
) -> LuaResult<LuaFunction> {
    let state = RefCell::new(Some((rx, store_entry)));
    lua.create_function(move |lua, ()| {
        let mut borrow = state.borrow_mut();
        let Some((rx, store_entry)) = borrow.as_mut() else {
            return Ok(LuaValue::Nil);
        };
        match rx.try_recv() {
            Err(oneshot::error::TryRecvError::Empty) => {
                let t = lua.create_table()?;
                t.set("status", "pending")?;
                Ok(LuaValue::Table(t))
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                *borrow = None;
                Err(LuaError::external(anyhow::anyhow!(
                    "kernel boot task aborted before completing"
                )))
            }
            Ok(result) => {
                let mut store_entry = store_entry.take();
                *borrow = None;
                let (client, info, boot_stream) = result.into_lua_err()?;
                if let (Some(pid), Some(s)) = (client.child_pid(), store_entry.as_mut()) {
                    s.set_kernel_pid(pid);
                }
                let t = register(lua, client, info, boot_stream)?;
                t.set("status", "ready")?;
                Ok(LuaValue::Table(t))
            }
        }
    })
}

/// Insert a freshly built [`Client`] into the lua-side registry (keyed by `client_id`),
/// returning the id, kernel_info reply, and a `stream` poll closure (the boot-time
/// no-filter listener returned by [`Client::spawn`] / [`Client::attach`]) in
/// lua-friendly form.
fn register(
    lua: &Lua,
    client: Client,
    info: serde_json::Value,
    boot_stream: RequestStream,
) -> LuaResult<LuaTable> {
    let client_id = client.client_id().to_string();
    let session_id = client.session_id().map(str::to_string);
    // The LSP is spawned inside `Client::spawn`/`Client::attach`; read
    // back the bound port so Lua callers can hand it to editors. Boot
    // would have failed if the bind didn't succeed.
    let lsp_port = client.lsp_port();
    let stream = make_poll(lua, boot_stream)?;
    let handle: ClientHandle = Arc::new(tokio::sync::Mutex::new(client));
    ClientRegistry::global().insert(client_id.clone(), handle);

    let out = lua.create_table()?;

    out.set("client_id", client_id.clone())?;
    out.set("kernel_info", crate::to_lua_value(lua, &info)?)?;
    out.set("stream", stream)?;

    if let Some(session_id) = session_id {
        out.set("session_id", session_id)?;
    }
    out.set("lsp_port", lsp_port)?;

    Ok(out)
}

/// Result of the async shutdown future: `Ok(())` on success, `Err(msg)` when
/// the session couldn't be found (unknown id, no live handle, no store entry).
/// Any other shutdown failure comes back as a `Result::Err` on the oneshot
/// value itself and is raised as a Lua error by the poll closure.
type ShutdownOutcome = Result<(), String>;

/// `jet.stop(session_id) -> poll`
///
/// If a Client for this session is live in-process, drive shutdown through it —
/// that waits for the child-wait watcher to flip status to `Exited` (i.e. the
/// kernel process has been reaped), so no fd we inherited is still held by a
/// live descendant when we return. Otherwise falls back to resolving the
/// session via the on-disk SessionStore and attaching a fresh client just to
/// send `shutdown_request`. Mirrors `jet stop <session_id>` — works for any
/// tracked session, regardless of which process owns the live in-memory client.
///
/// Non-blocking: returns a poll closure that the caller drives (e.g. via
/// `vim.schedule`) until it yields `{status="ready", success=bool,
/// failure_msg=string?}`. Subsequent calls return `nil`. Shutdown failures
/// mid-flight raise a Lua error on the observing call; "no such session"
/// comes back as `{success=false, failure_msg=...}`.
pub fn shutdown_kernel(lua: &Lua, session_id: String) -> LuaResult<LuaFunction> {
    // Look for a live in-process handle bound to this session_id.
    let live_handle = ClientRegistry::global()
        .snapshot()
        .into_iter()
        .find(|v| v.session_id.as_deref() == Some(session_id.as_str()))
        .and_then(|v| {
            ClientRegistry::global()
                .get(&v.client_id)
                .map(|h| (v.client_id, h))
        });

    let (tx, rx) = oneshot::channel::<anyhow::Result<ShutdownOutcome>>();

    if let Some((client_id, handle)) = live_handle {
        runtime().spawn(async move {
            let res = handle.lock().await.shutdown().await;
            // Drop the Client from the registry so its background tasks tear down
            // and the ChildGuard (if any) is released — the process is already gone
            // by the time Client::shutdown returns.
            if res.is_ok() {
                ClientRegistry::global().remove(&client_id);
            }
            let _ = tx.send(res.map(|()| Ok(())));
        });
    } else {
        runtime().spawn(async move {
            let store = match SessionStore::default() {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            let session = match store.open(&session_id) {
                Ok(s) => s,
                Err(_) => {
                    let _ = tx.send(Ok(Err(format!("no session with id {session_id}"))));
                    return;
                }
            };
            let path = session.connection_file_path();
            let result: anyhow::Result<ShutdownOutcome> = async {
                let connection = connection_file::read(path.as_path())?;
                if probe_kernel_alive(&connection).await.is_ok() {
                    let client_id = make_client_id(None);
                    // `shutdown_kernel` sends `shutdown_request`, not an
                    // interrupt — the interrupt-mode/pid fields don't matter.
                    let mut kernel =
                        Kernel::attach(&path, &client_id, AttachOptions::default()).await?;
                    kernel.shutdown().await?;
                }
                // Kernel already dead → stop is idempotent, treat as success.
                Ok(Ok(()))
            }
            .await;
            let _ = tx.send(result);
        });
    }

    make_shutdown_poll(lua, rx)
}

fn make_shutdown_poll(
    lua: &Lua,
    rx: oneshot::Receiver<anyhow::Result<ShutdownOutcome>>,
) -> LuaResult<LuaFunction> {
    let state = RefCell::new(Some(rx));
    lua.create_function(move |lua, ()| {
        let mut borrow = state.borrow_mut();
        let Some(rx) = borrow.as_mut() else {
            return Ok(LuaValue::Nil);
        };
        match rx.try_recv() {
            Err(oneshot::error::TryRecvError::Empty) => {
                let t = lua.create_table()?;
                t.set("status", "pending")?;
                Ok(LuaValue::Table(t))
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                *borrow = None;
                Err(LuaError::external(anyhow::anyhow!(
                    "kernel shutdown task aborted before completing"
                )))
            }
            Ok(result) => {
                *borrow = None;
                let outcome = result.into_lua_err()?;
                let t = lua.create_table()?;
                t.set("status", "ready")?;
                match outcome {
                    Ok(()) => {
                        t.set("success", true)?;
                    }
                    Err(msg) => {
                        t.set("success", false)?;
                        t.set("failure_msg", msg)?;
                    }
                }
                Ok(LuaValue::Table(t))
            }
        }
    })
}

/// `jet.interrupt(client_id)`
pub fn interrupt(_lua: &Lua, client_id: String) -> LuaResult<()> {
    let handle = ClientRegistry::global().require(&client_id).into_lua_err()?;
    runtime()
        .block_on(async move { handle.lock().await.interrupt().await })
        .into_lua_err()
}

/// `jet.list_connections()` — clients open *in this process*. Keyed by `client_id`.
/// Each entry carries the bound SessionStore id (if any) so callers can correlate with
/// `jet.list_sessions()`.
pub fn list_connections(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let out = lua.create_table()?;
    for view in ClientRegistry::global().snapshot() {
        let entry = lua.create_table()?;
        if let Some(sid) = view.session_id {
            entry.set("session_id", sid)?;
        }
        entry.set("client_id", view.client_id)?;
        out.push(entry)?;
    }
    Ok(out)
}

/// `jet.list_sessions({ status?, all_dirs? }) -> poll` — Jet sessions on disk (the
/// SessionStore). Returns a poll closure the caller drives (e.g. via `vim.schedule`)
/// until it yields `{status="ready", sessions=<jet.session_info[]>}`; subsequent calls
/// return `nil`. While the background listing runs it returns `{status="pending"}`.
///
/// Returns every `session.json` jet has written, regardless of which process owns the live
/// client (or whether one is open at all). Each entry exposes the full `SessionMeta`.
///
/// Mirrors `jet list-sessions`:
/// - `status`: `"open"` (default), `"closed"`, or `"all"`.
/// - `all_dirs`: when true, return sessions for every working directory; otherwise only
///   sessions whose `working_dir` matches the current dir.
///
/// Runs the listing on the tokio runtime so the main thread doesn't block on parsing
/// `session.json` files. Uses the cached form (backed by [`ClientRegistry`]'s background
/// poller), so the future completes as soon as the runtime picks it up.
pub fn list_sessions(lua: &Lua, opts: Option<LuaTable>) -> LuaResult<LuaFunction> {
    let (status, all_dirs) = match opts {
        Some(t) => (
            t.get::<Option<String>>("status")?,
            t.get::<Option<bool>>("all_dirs")?.unwrap_or(false),
        ),
        None => (None, false),
    };
    let status: jet_core::manager::StatusFilter =
        status.as_deref().unwrap_or("open").parse().into_lua_err()?;

    let (tx, rx) = oneshot::channel::<anyhow::Result<Vec<jet_core::manager::SessionMeta>>>();
    runtime().spawn(async move {
        let result = (|| {
            // list_filtered_cached() touches ClientRegistry::global(), which
            // spawns the background liveness poller on first access — so the
            // cache is always being refreshed by the time we read from it.
            let store = SessionStore::default()?;
            store.list_filtered_cached(status, all_dirs)
        })();
        let _ = tx.send(result);
    });

    make_list_sessions_poll(lua, rx)
}

fn make_list_sessions_poll(
    lua: &Lua,
    rx: oneshot::Receiver<anyhow::Result<Vec<jet_core::manager::SessionMeta>>>,
) -> LuaResult<LuaFunction> {
    let state = RefCell::new(Some(rx));
    lua.create_function(move |lua, ()| {
        let mut borrow = state.borrow_mut();
        let Some(rx) = borrow.as_mut() else {
            return Ok(LuaValue::Nil);
        };
        match rx.try_recv() {
            Err(oneshot::error::TryRecvError::Empty) => {
                let t = lua.create_table()?;
                t.set("status", "pending")?;
                Ok(LuaValue::Table(t))
            }
            Err(oneshot::error::TryRecvError::Closed) => {
                *borrow = None;
                Err(LuaError::external(anyhow::anyhow!(
                    "list_sessions task aborted before completing"
                )))
            }
            Ok(result) => {
                *borrow = None;
                let sessions = result.into_lua_err()?;
                let list = lua.create_table()?;
                for session in sessions {
                    list.push(crate::to_lua_value(lua, &session)?)?;
                }
                let t = lua.create_table()?;
                t.set("status", "ready")?;
                t.set("sessions", list)?;
                Ok(LuaValue::Table(t))
            }
        }
    })
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

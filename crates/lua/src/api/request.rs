//! Per-request Lua API. Each function builds a Jupyter shell-channel
//! request, hands it to the session, and returns a poll closure (see
//! [`crate::poll::make_poll`]) the Lua caller drains.

use jet_core::client::ListenFilter;
use jet_core::events::Channel;
use jet_core::jupyter_protocol::{
    CommClose, CommMsg, CommOpen, CompleteRequest, DebugRequest, ExecuteRequest, HistoryRequest,
    InspectRequest, IsCompleteRequest, JupyterMessage, KernelInfoRequest,
};
use mlua::prelude::*;
use rand::Rng;
use serde_json::Value;

use crate::poll::make_poll;
use jet_core::manager::{ClientHandle, ClientRegistry, runtime};

/// Which kernel channel a request should go out on. Selects between
/// [`Client::request`] and [`Client::control_request`].
#[derive(Clone, Copy)]
enum RequestChannel {
    Shell,
    Control,
}

/// Common path: hand a message to the kernel session and wrap the
/// resulting [`RequestStream`] in a Lua poll closure. Returns the
/// message header id alongside the poll so Lua callers can correlate
/// the request with routed frames.
fn send_request(
    lua: &Lua,
    handle: &ClientHandle,
    channel: RequestChannel,
    msg: JupyterMessage,
) -> LuaResult<(LuaFunction, String)> {
    let session = handle.clone();
    let stream = runtime()
        .block_on(async move {
            let client = session.lock().await;
            match channel {
                RequestChannel::Shell => client.request(msg),
                RequestChannel::Control => client.control_request(msg),
            }
        })
        .into_lua_err()?;
    let msg_id = stream.msg_id.clone();
    let poll = make_poll(lua, stream)?;
    Ok((poll, msg_id))
}

fn shell_request(
    lua: &Lua,
    handle: &ClientHandle,
    msg: JupyterMessage,
) -> LuaResult<(LuaFunction, String)> {
    send_request(lua, handle, RequestChannel::Shell, msg)
}

fn control_request(
    lua: &Lua,
    handle: &ClientHandle,
    msg: JupyterMessage,
) -> LuaResult<(LuaFunction, String)> {
    send_request(lua, handle, RequestChannel::Control, msg)
}

/// Coerce a Lua value into a `serde_json::Map` suitable for the `data`
/// field of comm messages. Anything that doesn't deserialize to a JSON
/// object (nil, numbers, mismatched types) becomes an empty map.
fn lua_value_to_json_map(
    lua: &Lua,
    data: LuaValue,
) -> LuaResult<serde_json::Map<String, Value>> {
    Ok(match lua.from_value::<Value>(data)? {
        Value::Object(m) => m,
        _ => Default::default(),
    })
}

pub fn execute_code(
    lua: &Lua,
    (session_id, code, silent, allow_stdin, user_expressions): (
        String,
        String,
        bool,
        bool,
        LuaTable,
    ),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    // ExecuteRequest expects `HashMap<String, String>` — flatten anything
    // non-string into its serde_json string form.
    use std::collections::HashMap;
    let user_expr: Value = lua.from_value(LuaValue::Table(user_expressions))?;
    let user_expr_map: Option<HashMap<String, String>> = match user_expr {
        Value::Object(m) => Some(
            m.into_iter()
                .map(|(k, v)| {
                    let s = match v {
                        Value::String(s) => s,
                        other => other.to_string(),
                    };
                    (k, s)
                })
                .collect(),
        ),
        _ => None,
    };
    let req: JupyterMessage = ExecuteRequest {
        code,
        silent: silent,
        store_history: true,
        user_expressions: user_expr_map,
        allow_stdin: allow_stdin,
        stop_on_error: true,
    }
    .into();
    shell_request(lua, &handle, req)
}

pub fn is_complete(
    lua: &Lua,
    (session_id, code): (String, String),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let req: JupyterMessage = IsCompleteRequest { code }.into();
    shell_request(lua, &handle, req)
}

pub fn get_completions(
    lua: &Lua,
    (session_id, code, cursor_pos): (String, String, u32),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let req: JupyterMessage = CompleteRequest {
        code,
        cursor_pos: cursor_pos as usize,
    }
    .into();
    shell_request(lua, &handle, req)
}

pub fn comm_open(
    lua: &Lua,
    (session_id, target_name, data): (String, String, LuaValue),
) -> LuaResult<(LuaFunction, String, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let data_map = lua_value_to_json_map(lua, data)?;
    let comm_id = format!("{:032x}", rand::thread_rng().r#gen::<u128>());
    let req: JupyterMessage = CommOpen {
        comm_id: comm_id.clone().into(),
        target_name,
        data: data_map,
        target_module: None,
    }
    .into();
    let (poll, msg_id) = shell_request(lua, &handle, req)?;
    Ok((poll, comm_id, msg_id))
}

pub fn comm_info(
    lua: &Lua,
    (session_id, target_name): (String, Option<String>),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let session = handle.clone();
    let stream = runtime()
        .block_on(async move { session.lock().await.comm_info(target_name) })
        .into_lua_err()?;
    let msg_id = stream.msg_id.clone();
    let poll = make_poll(lua, stream)?;
    Ok((poll, msg_id))
}

/// Parse an `opts.channel` / `opts.msg_type` entry: accept either a single
/// string or a table of strings.
fn parse_string_set(v: LuaValue) -> LuaResult<Option<Vec<String>>> {
    match v {
        LuaValue::Nil => Ok(None),
        LuaValue::String(s) => Ok(Some(vec![s.to_str()?.to_string()])),
        LuaValue::Table(t) => {
            let mut out = Vec::new();
            for pair in t.sequence_values::<String>() {
                out.push(pair?);
            }
            Ok(Some(out))
        }
        other => Err(LuaError::external(format!(
            "expected string or list of strings, got {}",
            other.type_name()
        ))),
    }
}

pub fn listen(lua: &Lua, (session_id, opts): (String, Option<LuaTable>)) -> LuaResult<LuaFunction> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let mut filter = ListenFilter::default();
    if let Some(t) = opts {
        if let Some(chs) = parse_string_set(t.get("channel")?)? {
            let mut set = std::collections::HashSet::new();
            for c in chs {
                let ch = Channel::from_name(&c).ok_or_else(|| {
                    LuaError::external(format!(
                        "unknown channel {c:?}: expected one of shell, iopub, stdin, control"
                    ))
                })?;
                set.insert(ch);
            }
            filter.channels = Some(set);
        }
        if let Some(mts) = parse_string_set(t.get("msg_type")?)? {
            filter.msg_types = Some(mts.into_iter().collect());
        }
    }
    let session = handle.clone();
    let stream = runtime().block_on(async move { session.lock().await.listen(filter) });
    make_poll(lua, stream)
}

pub fn comm_listen(lua: &Lua, (session_id, comm_id): (String, String)) -> LuaResult<LuaFunction> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let session = handle.clone();
    let stream = runtime().block_on(async move { session.lock().await.comm_listen(comm_id) });
    make_poll(lua, stream)
}

pub fn comm_send(
    lua: &Lua,
    (session_id, comm_id, data): (String, String, LuaValue),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let data_map = lua_value_to_json_map(lua, data)?;
    let req: JupyterMessage = CommMsg {
        comm_id: comm_id.into(),
        data: data_map,
    }
    .into();
    shell_request(lua, &handle, req)
}

pub fn comm_close(
    lua: &Lua,
    (session_id, comm_id, data): (String, String, Option<LuaValue>),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let data_map = match data {
        Some(v) => lua_value_to_json_map(lua, v)?,
        None => Default::default(),
    };
    let req: JupyterMessage = CommClose {
        comm_id: comm_id.into(),
        data: data_map,
    }
    .into();
    shell_request(lua, &handle, req)
}

pub fn inspect(
    lua: &Lua,
    (session_id, code, cursor_pos, detail_level): (String, String, u32, Option<u32>),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let req: JupyterMessage = InspectRequest {
        code,
        cursor_pos: cursor_pos as usize,
        detail_level: detail_level.map(|d| d as usize),
    }
    .into();
    shell_request(lua, &handle, req)
}

pub fn kernel_info(lua: &Lua, session_id: String) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let req: JupyterMessage = KernelInfoRequest {}.into();
    shell_request(lua, &handle, req)
}

/// Build a [`HistoryRequest`] from a mode tag and an options table.
/// Mirrors the Jupyter tagged-enum shape: `range` / `tail` / `search`.
fn build_history_request(mode: &str, opts: LuaTable) -> LuaResult<HistoryRequest> {
    let output: bool = opts.get::<Option<bool>>("output")?.unwrap_or(false);
    let raw: bool = opts.get::<Option<bool>>("raw")?.unwrap_or(true);
    match mode {
        "range" => Ok(HistoryRequest::Range {
            session: opts.get::<Option<i32>>("session")?,
            start: opts.get::<Option<i32>>("start")?.unwrap_or(0),
            stop: opts.get::<Option<i32>>("stop")?.unwrap_or(0),
            output,
            raw,
        }),
        "tail" => Ok(HistoryRequest::Tail {
            n: opts.get::<Option<i32>>("n")?.unwrap_or(10),
            output,
            raw,
        }),
        "search" => Ok(HistoryRequest::Search {
            pattern: opts
                .get::<Option<String>>("pattern")?
                .ok_or_else(|| LuaError::external("history: `search` mode requires `pattern`"))?,
            unique: opts.get::<Option<bool>>("unique")?.unwrap_or(false),
            output,
            raw,
            n: opts.get::<Option<i32>>("n")?.unwrap_or(10),
        }),
        other => Err(LuaError::external(format!(
            "history: unknown mode {other:?}: expected \"range\", \"tail\", or \"search\""
        ))),
    }
}

pub fn history(
    lua: &Lua,
    (session_id, mode, opts): (String, String, LuaTable),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let req: JupyterMessage = build_history_request(&mode, opts)?.into();
    shell_request(lua, &handle, req)
}

pub fn debug(
    lua: &Lua,
    (session_id, content): (String, LuaValue),
) -> LuaResult<(LuaFunction, String)> {
    let handle = ClientRegistry::global().require(&session_id).into_lua_err()?;
    let content_json: Value = lua.from_value(content)?;
    let req: JupyterMessage = DebugRequest {
        content: content_json,
    }
    .into();
    control_request(lua, &handle, req)
}

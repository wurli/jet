//! Per-request Lua API. Each function builds a Jupyter shell-channel
//! request, hands it to the session, and returns a poll closure (see
//! [`crate::poll::make_poll`]) the Lua caller drains.

use jet_core::jupyter_protocol::{
    CommMsg, CommOpen, CompleteRequest, ExecuteRequest, IsCompleteRequest, JupyterMessage,
};
use mlua::prelude::*;
use rand::Rng;
use serde_json::Value;

use crate::poll::make_poll;
use crate::runtime::{KernelHandle, get, rt};

/// Common path: hand a message to the kernel session and wrap the
/// resulting [`RequestStream`] in a Lua poll closure.
fn shell_request(
    lua: &Lua,
    handle: &KernelHandle,
    msg: JupyterMessage,
) -> LuaResult<LuaFunction> {
    let session = handle.clone();
    let stream = rt()
        .block_on(async move { session.lock().await.request(msg) })
        .into_lua_err()?;
    make_poll(lua, stream)
}

pub fn execute_code(
    lua: &Lua,
    (session_id, code, user_expressions): (String, String, LuaTable),
) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
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
        silent: false,
        store_history: true,
        user_expressions: user_expr_map,
        allow_stdin: true,
        stop_on_error: true,
    }
    .into();
    shell_request(lua, &handle, req)
}

pub fn is_complete(lua: &Lua, (session_id, code): (String, String)) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
    let req: JupyterMessage = IsCompleteRequest { code }.into();
    shell_request(lua, &handle, req)
}

pub fn get_completions(
    lua: &Lua,
    (session_id, code, cursor_pos): (String, String, u32),
) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
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
) -> LuaResult<(String, LuaFunction)> {
    let handle = get(&session_id).into_lua_err()?;
    let data_json: Value = lua.from_value(data)?;
    let data_map = match data_json {
        Value::Object(m) => m,
        _ => Default::default(),
    };
    let comm_id = format!("{:032x}", rand::thread_rng().r#gen::<u128>());
    let req: JupyterMessage = CommOpen {
        comm_id: comm_id.clone().into(),
        target_name,
        data: data_map,
        target_module: None,
    }
    .into();
    let poll = shell_request(lua, &handle, req)?;
    Ok((comm_id, poll))
}

pub fn comm_send(
    lua: &Lua,
    (session_id, comm_id, data): (String, String, LuaValue),
) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
    let data_json: Value = lua.from_value(data)?;
    let data_map = match data_json {
        Value::Object(m) => m,
        _ => Default::default(),
    };
    let req: JupyterMessage = CommMsg {
        comm_id: comm_id.into(),
        data: data_map,
    }
    .into();
    shell_request(lua, &handle, req)
}

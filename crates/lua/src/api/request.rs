//! Per-request Lua API. Each function sends a Jupyter message and returns
//! a poll closure (see [`crate::poll::make_poll`]).

use jet_core::jupyter;
use mlua::prelude::*;
use serde_json::{Value, json};

use crate::poll::make_poll;
use crate::runtime::{KernelHandle, get, rt};

/// Common path: register a sender, send a frame on `shell`, return the
/// poll closure pointing at the sender's receive end.
fn shell_request(
    lua: &Lua,
    handle: &KernelHandle,
    msg_type: &str,
    content: Value,
) -> LuaResult<LuaFunction> {
    let msg_id = jupyter::new_msg_id();
    let rx = handle.router.register(msg_id.clone());
    let frame = jupyter::message("shell", &msg_id, msg_type, content);
    let channel = handle.channel.clone();
    rt()
        .block_on(async move { channel.lock().await.send(&frame).await })
        .into_lua_err()?;
    make_poll(lua, rx, msg_id, handle.router.clone())
}

/// `jet.execute_code(session_id, code, user_expressions) -> poll`
pub fn execute_code(
    lua: &Lua,
    (session_id, code, user_expressions): (String, String, LuaTable),
) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
    let user_expr: Value = lua.from_value(LuaValue::Table(user_expressions))?;
    let content = json!({
        "code": code,
        "silent": false,
        "store_history": true,
        "user_expressions": user_expr,
        "allow_stdin": true,
        "stop_on_error": true,
    });
    shell_request(lua, &handle, "execute_request", content)
}

/// `jet.is_complete(session_id, code) -> poll`
pub fn is_complete(lua: &Lua, (session_id, code): (String, String)) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
    shell_request(lua, &handle, "is_complete_request", json!({ "code": code }))
}

/// `jet.get_completions(session_id, code, cursor_pos) -> poll`
pub fn get_completions(
    lua: &Lua,
    (session_id, code, cursor_pos): (String, String, u32),
) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
    shell_request(
        lua,
        &handle,
        "complete_request",
        json!({ "code": code, "cursor_pos": cursor_pos }),
    )
}

/// `jet.comm_open(session_id, target_name, data) -> (comm_id, poll)`
pub fn comm_open(
    lua: &Lua,
    (session_id, target_name, data): (String, String, LuaValue),
) -> LuaResult<(String, LuaFunction)> {
    let handle = get(&session_id).into_lua_err()?;
    let data_json: Value = lua.from_value(data)?;
    let comm_id = format!(
        "{:032x}",
        rand::random::<u128>(),
    );
    let content = json!({
        "comm_id": comm_id,
        "target_name": target_name,
        "data": data_json,
    });
    let poll = shell_request(lua, &handle, "comm_open", content)?;
    Ok((comm_id, poll))
}

/// `jet.comm_send(session_id, comm_id, data) -> poll`
pub fn comm_send(
    lua: &Lua,
    (session_id, comm_id, data): (String, String, LuaValue),
) -> LuaResult<LuaFunction> {
    let handle = get(&session_id).into_lua_err()?;
    let data_json: Value = lua.from_value(data)?;
    let content = json!({
        "comm_id": comm_id,
        "data": data_json,
    });
    shell_request(lua, &handle, "comm_msg", content)
}

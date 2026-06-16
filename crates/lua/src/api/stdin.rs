//! `jet.provide_stdin(session_id, parent_msg_id, value)` — send the
//! `input_reply` Jupyter expects after an `input_request`.

use jet_core::jupyter;
use mlua::prelude::*;
use serde_json::json;

use crate::runtime::{get, rt};

pub fn provide_stdin(
    _lua: &Lua,
    (session_id, _parent_msg_id, value): (String, String, String),
) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    // Jupyter's input_reply doesn't need its parent_header set explicitly;
    // kallichore/the kernel pair it with the in-flight input_request by
    // proximity on the stdin channel. We accept `parent_msg_id` from Lua
    // for forward-compat / debugging but don't currently put it on the
    // wire — matches the CLI path in jet-cli/src/main.rs.
    let msg_id = jupyter::new_msg_id();
    let frame = jupyter::message("stdin", &msg_id, "input_reply", json!({ "value": value }));
    let channel = handle.channel.clone();
    rt()
        .block_on(async move { channel.lock().await.send(&frame).await })
        .into_lua_err()
}

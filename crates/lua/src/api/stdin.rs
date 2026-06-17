//! `jet.provide_stdin(session_id, parent_msg_id, value)` — send the
//! `input_reply` Jupyter expects after an `input_request`.

use jet_core::jupyter_protocol::{InputReply, JupyterMessage};
use mlua::prelude::*;

use crate::runtime::get;

pub fn provide_stdin(
    _lua: &Lua,
    (session_id, _parent_msg_id, value): (String, String, String),
) -> LuaResult<()> {
    let handle = get(&session_id).into_lua_err()?;
    // Jupyter pairs an `input_reply` with the in-flight `input_request`
    // by proximity on the stdin channel; we don't carry parent_msg_id
    // through to the wire (matches the CLI behaviour).
    let reply: JupyterMessage = InputReply {
        value,
        status: Default::default(),
        error: None,
    }
    .into();
    handle
        .stdin_tx
        .send(reply)
        .map_err(|e| anyhow::anyhow!("stdin_tx send: {e}"))
        .into_lua_err()
}

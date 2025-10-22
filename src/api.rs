use std::sync::{mpsc::Receiver, OnceLock};

use mlua::prelude::*

use crate::msg::wire::jupyter_message::Message;

static EXECUTE_CHANNEL: OnceLock<Receiver<Message>> = OnceLock::new();

fn execute_code(_: &Lua, code: String) -> LuaResult<String> {

}

fn is_complete(_lua: Lua, code) -> LuaResult<()> {

}

fn flush_streams() -> LuaResult<()> {

}

fn poll_stdin() -> LuaResult<()> {

}

fn provide_stdin() -> LuaResult<()> {
    // let x = frontend.stdin
}

//! jet_lua — mlua bindings exposing the jet wire layer to Neovim/LuaJIT.
//!
//! Loaded by Lua as `require('jet')`. Each "send a request" function
//! returns a poll closure: calling it repeatedly returns
//!   - `{status="busy", type=..., data=...}` for one incoming kernel frame,
//!   - `{status="pending"}` when nothing has arrived yet, or
//!   - `nil` when the kernel has gone idle for that request.
//!
//! Consumers (e.g. a Neovim plugin) drain via `vim.schedule(drain)` so the
//! UI thread is never blocked.

mod api;
mod poll;
mod runtime;

use mlua::SerializeOptions;
use mlua::prelude::*;
use serde::Serialize;

/// `LuaSerdeExt::to_value` with `None`/`()` mapped to Lua `nil` instead of
/// mlua's `Null` userdata sentinel. Use this everywhere we hand serde data
/// back to Lua so optional fields show up as absent keys, not userdata.
pub(crate) fn to_lua_value<T: Serialize + ?Sized>(lua: &Lua, value: &T) -> LuaResult<LuaValue> {
    let opts = SerializeOptions::new()
        .serialize_none_to_null(false)
        .serialize_unit_to_null(false);
    lua.to_value_with(value, opts)
}

use api::lifecycle::{
    attach, interrupt, list_available_kernels, list_connections, list_sessions, make_session_id,
    show_session, show_spec, shutdown_kernel, start,
};
use api::request::{
    comm_info, comm_listen, comm_open, comm_send, execute_code, get_completions, is_complete,
    listen,
};
use api::stdin::provide_stdin;

// Lifted from clap
macro_rules! crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

fn version(_lua: &Lua, _: ()) -> LuaResult<String> {
    Ok(crate_version!().to_string())
}

#[mlua::lua_module]
fn jet(lua: &Lua) -> LuaResult<LuaTable> {
    jet_core::logger::init_logger(Some(std::path::Path::new("jet-lua.log")));
    register(lua)
}

/// Build the `jet` Lua module table. The cdylib entry point [`jet`] is
/// what Neovim/LuaJIT loads; this is split out so the same registration
/// is reachable to anyone embedding mlua.
pub fn register(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("start", lua.create_function(start)?)?;
    exports.set("attach", lua.create_function(attach)?)?;
    exports.set("stop", lua.create_function(shutdown_kernel)?)?;
    exports.set("interrupt", lua.create_function(interrupt)?)?;
    exports.set("list_connections", lua.create_function(list_connections)?)?;
    exports.set("list_sessions", lua.create_function(list_sessions)?)?;
    exports.set("list_kernels", lua.create_function(list_available_kernels)?)?;
    exports.set("show_spec", lua.create_function(show_spec)?)?;
    exports.set("show_session", lua.create_function(show_session)?)?;
    exports.set("make_session_id", lua.create_function(make_session_id)?)?;
    exports.set("execute_code", lua.create_function(execute_code)?)?;
    exports.set("is_complete", lua.create_function(is_complete)?)?;
    exports.set("get_completions", lua.create_function(get_completions)?)?;
    exports.set("comm_open", lua.create_function(comm_open)?)?;
    exports.set("comm_send", lua.create_function(comm_send)?)?;
    exports.set("comm_info", lua.create_function(comm_info)?)?;
    exports.set("comm_listen", lua.create_function(comm_listen)?)?;
    exports.set("listen", lua.create_function(listen)?)?;
    exports.set("provide_stdin", lua.create_function(provide_stdin)?)?;
    exports.set("version", lua.create_function(version)?)?;
    Ok(exports)
}

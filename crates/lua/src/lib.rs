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
mod router;
mod runtime;

use mlua::prelude::*;

use api::lifecycle::{
    attach, connect, interrupt, list_available_kernels, list_running_kernels, shutdown_kernel,
};
use api::request::{comm_open, comm_send, execute_code, get_completions, is_complete};
use api::stdin::provide_stdin;

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
    exports.set("connect", lua.create_function(connect)?)?;
    exports.set("attach", lua.create_function(attach)?)?;
    exports.set("shutdown_kernel", lua.create_function(shutdown_kernel)?)?;
    exports.set("interrupt", lua.create_function(interrupt)?)?;
    exports.set(
        "list_running_kernels",
        lua.create_function(list_running_kernels)?,
    )?;
    exports.set(
        "list_available_kernels",
        lua.create_function(list_available_kernels)?,
    )?;
    exports.set("execute_code", lua.create_function(execute_code)?)?;
    exports.set("is_complete", lua.create_function(is_complete)?)?;
    exports.set("get_completions", lua.create_function(get_completions)?)?;
    exports.set("comm_open", lua.create_function(comm_open)?)?;
    exports.set("comm_send", lua.create_function(comm_send)?)?;
    exports.set("provide_stdin", lua.create_function(provide_stdin)?)?;
    Ok(exports)
}

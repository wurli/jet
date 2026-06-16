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

#[mlua::lua_module]
fn jet(lua: &Lua) -> LuaResult<LuaTable> {
    register(lua)
}

/// Build the `jet` Lua module table. The cdylib entry point [`jet`] is
/// what Neovim/LuaJIT loads; this is split out so the same registration
/// is reachable to anyone embedding mlua.
pub fn register(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("start_kernel", lua.create_function(api::lifecycle::start_kernel)?)?;
    exports.set(
        "shutdown_kernel",
        lua.create_function(api::lifecycle::shutdown_kernel)?,
    )?;
    exports.set("interrupt", lua.create_function(api::lifecycle::interrupt)?)?;
    exports.set(
        "list_running_kernels",
        lua.create_function(api::lifecycle::list_running_kernels)?,
    )?;
    exports.set(
        "list_available_kernels",
        lua.create_function(api::lifecycle::list_available_kernels)?,
    )?;
    exports.set(
        "execute_code",
        lua.create_function(api::request::execute_code)?,
    )?;
    exports.set(
        "is_complete",
        lua.create_function(api::request::is_complete)?,
    )?;
    exports.set(
        "get_completions",
        lua.create_function(api::request::get_completions)?,
    )?;
    exports.set("comm_open", lua.create_function(api::request::comm_open)?)?;
    exports.set("comm_send", lua.create_function(api::request::comm_send)?)?;
    exports.set(
        "provide_stdin",
        lua.create_function(api::stdin::provide_stdin)?,
    )?;
    Ok(exports)
}

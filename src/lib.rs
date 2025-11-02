/*
 * lib.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

pub mod api;
pub mod api_lua;
pub mod connection;
pub mod error;
pub mod kernel;
pub mod msg;
pub mod supervisor;

use mlua::prelude::*;

pub type Result<T> = std::result::Result<T, error::Error>;

#[mlua::lua_module(skip_memory_check)]
pub fn jet(lua: &Lua) -> LuaResult<LuaTable> {
    // Initialise the logger
    let log_file = String::from("jet.log");
    let target = Box::new(
        std::fs::File::create(&log_file)
            .unwrap_or_else(|_| panic!("Can't create log file at {log_file}")),
    );
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(target))
        .init();

    // Return the Lua API
    let exports = lua.create_table()?;
    exports.set("start_kernel", lua.create_function(api_lua::start_kernel)?)?;
    exports.set(
        "list_running_kernels",
        lua.create_function(api_lua::list_running_kernels)?,
    )?;
    exports.set("execute_code", lua.create_function(api_lua::execute_code)?)?;
    exports.set("is_complete", lua.create_function(api_lua::is_complete)?)?;
    exports.set(
        "get_completions",
        lua.create_function(api_lua::get_completions)?,
    )?;
    exports.set(
        "provide_stdin",
        lua.create_function(api_lua::provide_stdin)?,
    )?;
    exports.set(
        "list_available_kernels",
        lua.create_function(api_lua::list_available_kernels)?,
    )?;
    exports.set(
        "request_shutdown",
        lua.create_function(api_lua::request_shutdown)?,
    )?;
    exports.set(
        "request_restart",
        lua.create_function(api_lua::request_restart)?,
    )?;

    Ok(exports)
}

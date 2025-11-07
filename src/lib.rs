/*
 * lib.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

pub mod api;
pub mod api_lua;
pub mod callback_output;
pub mod connection;
pub mod error;
pub mod kernel;
pub mod msg;
pub mod shutdown_guard;
pub mod supervisor;

use mlua::prelude::*;

use crate::{
    api_lua::{
        execute_code, get_completions, is_complete, list_available_kernels, list_running_kernels,
        provide_stdin, request_restart, request_shutdown, start_kernel,
    },
    shutdown_guard::ShutdownGuard,
};

pub type Result<T> = std::result::Result<T, error::Error>;

macro_rules! lua_exports {
    ($lua:expr, $($func:ident),* $(,)?) => {{
        let exports = $lua.create_table()?;
        $(
            exports.set(stringify!($func), $lua.create_function($func)?)?;
        )*
        exports
    }};
}

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

    // When lua goes out of scope, i.e. when Neovim exits, ShutDownGuard will also go out of scope
    // and its Drop implementation will shut down all running kernels.
    lua.set_app_data(ShutdownGuard {});

    // Return the Lua API
    Ok(lua_exports!(
        lua,
        start_kernel,
        list_running_kernels,
        execute_code,
        is_complete,
        get_completions,
        provide_stdin,
        list_available_kernels,
        request_shutdown,
        request_restart,
    ))
}

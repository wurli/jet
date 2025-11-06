/*
 * api_lua.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use mlua::LuaSerdeExt;
use mlua::prelude::*;

use crate::api;
use crate::callback_output::CallbackOutput;
use crate::msg::wire::jupyter_message::Describe;
use crate::msg::wire::jupyter_message::Message;
use crate::msg::wire::message_id::Id;

pub fn execute_code(
    lua: &Lua,
    (kernel_id, code, user_expressions): (String, String, HashMap<String, String>),
) -> LuaResult<LuaFunction> {
    let callback = api::execute_code(Id::from(kernel_id), code, user_expressions).into_lua_err()?;

    lua.create_function_mut(move |lua, (): ()| -> LuaResult<LuaTable> {
        let result = callback();

        match result {
            CallbackOutput::Idle => {
                let table = lua.create_table().unwrap();
                let _ = table.set("status", "idle");
                Ok(table)
            }
            CallbackOutput::Busy(Some(Message::ExecuteResult(msg))) => to_lua(lua, &msg.content),
            CallbackOutput::Busy(Some(Message::ExecuteError(msg))) => to_lua(lua, &msg.content),
            CallbackOutput::Busy(Some(Message::Stream(msg))) => to_lua(lua, &msg.content),
            CallbackOutput::Busy(Some(Message::InputRequest(msg))) => to_lua(lua, &msg.content),
            CallbackOutput::Busy(Some(Message::DisplayData(msg))) => to_lua(lua, &msg.content),
            CallbackOutput::Busy(None) => {
                let table = lua.create_table().unwrap();
                let _ = table.set("status", "busy");
                Ok(table)
            }
            CallbackOutput::Busy(Some(other)) => Err(LuaError::external(format!(
                "Received unexpected message type {}",
                other.kind()
            ))),
        }
    })
}

pub fn is_complete(lua: &Lua, (kernel_id, code): (String, String)) -> LuaResult<LuaTable> {
    match api::is_complete(Id::from(kernel_id), code) {
        Ok(Message::IsCompleteReply(msg)) => to_lua(lua, &msg.content),
        Ok(msg) => Err(LuaError::external(format!(
            "Received unexpected message type {}",
            msg.kind()
        ))),
        Err(e) => Err(e.into_lua_err()),
    }
}

pub fn get_completions(
    lua: &Lua,
    (kernel_id, code, cursor_pos): (String, String, u32),
) -> LuaResult<LuaTable> {
    match api::get_completions(Id::from(kernel_id), code, cursor_pos) {
        Ok(Message::CompleteReply(msg)) => to_lua(lua, &msg.content),
        Ok(msg) => Err(LuaError::external(format!(
            "Received unexpected message type {}",
            msg.kind()
        ))),
        Err(e) => Err(e.into_lua_err()),
    }
}

fn to_lua<T: Describe + serde::Serialize>(lua: &Lua, x: &T) -> LuaResult<LuaTable> {
    let out = lua.create_table().unwrap();
    let _ = out.set("status", "busy");
    let _ = out.set("type", x.kind());
    let _ = out.set("data", lua.to_value(x).unwrap());
    Ok(out)
}

pub fn request_shutdown(_lua: &Lua, kernel_id: String) -> LuaResult<()> {
    api::request_shutdown(&Id::from(kernel_id)).into_lua_err()
}

pub fn request_restart(lua: &Lua, kernel_id: String) -> LuaResult<LuaValue> {
    let reply = api::request_restart(&Id::from(kernel_id)).into_lua_err()?;
    match reply {
        Message::ShutdownReply(msg) => lua.to_value(&msg.content),
        other => Err(LuaError::external(format!(
            "Received unexpected reply to restart request {}",
            other.describe()
        ))),
    }
}

pub fn provide_stdin(_: &Lua, (kernel_id, value): (String, String)) -> LuaResult<()> {
    api::provide_stdin(&Id::from(kernel_id), value).into_lua_err()?;
    Ok(())
}

pub fn list_available_kernels(lua: &Lua, (): ()) -> LuaResult<mlua::Table> {
    let kernels = api::list_available_kernels();

    Ok(lua.create_table_from(
        kernels
            .iter()
            .map(|(path, spec)| (path.to_string_lossy(), lua.to_value(&spec).unwrap())),
    )?)
}

pub fn start_kernel(lua: &Lua, spec_path: String) -> LuaResult<(String, LuaValue)> {
    let spec_pathbuf = PathBuf::from_str(&spec_path).into_lua_err()?;
    match api::start_kernel(spec_pathbuf) {
        Ok((kernel_id, info)) => Ok((String::from(kernel_id), lua.to_value(&info).unwrap())),
        Err(e) => Err(LuaError::external(e)),
    }
}

pub fn list_running_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let kernels = api::list_running_kernels();
    let table = lua.create_table()?;

    for (k, v) in kernels.iter() {
        table.set(String::from(k), lua.to_value(v).unwrap())?;
    }

    Ok(table)
}

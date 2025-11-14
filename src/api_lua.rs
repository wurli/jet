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
use crate::callback_output::KernelResponse;
use crate::msg::wire::jupyter_message::Message;
use crate::msg::wire::jupyter_message::ProtocolMessage;
use crate::msg::wire::message_id::Id;
use crate::supervisor::kernel_manager::KernelManager;

pub fn list_running_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let kernels = api::list_running_kernels();
    let table = lua.create_table()?;

    for (k, v) in kernels.iter() {
        table.set(String::from(k), lua.to_value(v).unwrap())?;
    }

    Ok(table)
}

pub fn list_available_kernels(lua: &Lua, (): ()) -> LuaResult<mlua::Table> {
    Ok(lua.create_table_from(
        api::list_available_kernels()
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

pub fn request_shutdown(_lua: &Lua, kernel_id: String) -> LuaResult<()> {
    api::request_shutdown(&Id::from(kernel_id)).into_lua_err()
}

pub fn request_restart(lua: &Lua, kernel_id: String) -> LuaResult<LuaValue> {
    let reply = api::request_restart(kernel_id.into()).into_lua_err()?;
    match reply {
        Message::ShutdownReply(msg) => lua.to_value(&msg.content),
        other => Err(LuaError::external(format!(
            "Received unexpected reply to restart request {}",
            other.describe()
        ))),
    }
}

pub fn comm_open(
    lua: &Lua,
    (kernel_id, target_name, data): (String, String, LuaValue),
) -> LuaResult<(String, LuaFunction)> {
    let data_json = lua.from_value::<serde_json::Value>(data).into_lua_err()?;
    let (id, callback) = api::comm_open(kernel_id.into(), target_name, data_json).into_lua_err()?;
    let lua_callback = lua.create_function_mut(move |lua, (): ()| callback().to_lua(lua))?;
    Ok((String::from(id), lua_callback))
}

pub fn comm_send(
    lua: &Lua,
    (kernel_id, comm_id, data): (String, String, LuaValue),
) -> LuaResult<LuaFunction> {
    let data_json = lua.from_value::<serde_json::Value>(data).into_lua_err()?;
    let callback = api::comm_send(kernel_id.into(), comm_id.into(), data_json).into_lua_err()?;
    lua.create_function_mut(move |lua, (): ()| callback().to_lua(lua))
}

pub fn provide_stdin(_: &Lua, (kernel_id, value): (String, String)) -> LuaResult<()> {
    api::provide_stdin(&Id::from(kernel_id), value).into_lua_err()?;
    Ok(())
}

pub fn interrupt(lua: &Lua, kernel_id: String) -> LuaResult<LuaTable> {
    let res = api::interrupt(kernel_id.into()).into_lua_err()?;
    match res {
        Some(Message::InterruptReply(msg)) => msg.content.to_lua(lua),
        Some(other) => Err(LuaError::external(format!(
            "Received unexpected reply to interrupt request {}",
            other.describe()
        ))),
        None => lua.create_table(),
    }
}

pub fn execute_code(
    lua: &Lua,
    (kernel_id, code, user_expressions): (String, String, HashMap<String, String>),
) -> LuaResult<LuaFunction> {
    let kernel = KernelManager::get(&kernel_id.into()).into_lua_err()?;
    let receivers = kernel
        .comm
        .send_execute_request(code, user_expressions)
        .into_lua_err()?;
    lua.create_function_mut(move |lua, (): ()| {
        kernel.comm.recv_execute_reply(&receivers).to_lua(lua)
    })
}

pub fn is_complete(lua: &Lua, (kernel_id, code): (String, String)) -> LuaResult<LuaFunction> {
    let kernel = KernelManager::get(&kernel_id.into()).into_lua_err()?;
    let receivers = kernel.comm.send_is_complete_request(code).into_lua_err()?;
    lua.create_function_mut(move |lua, (): ()| {
        kernel.comm.recv_is_complete_reply(&receivers).to_lua(lua)
    })
}

pub fn get_completions(
    lua: &Lua,
    (kernel_id, code, cursor_pos): (String, String, u32),
) -> LuaResult<LuaFunction> {
    let kernel = KernelManager::get(&kernel_id.into()).into_lua_err()?;
    let receivers = kernel
        .comm
        .send_completion_request(code, cursor_pos)
        .into_lua_err()?;
    lua.create_function_mut(move |lua, (): ()| {
        kernel.comm.recv_completion_reply(&receivers).to_lua(lua)
    })
}

trait ToLua {
    fn to_lua(&self, lua: &Lua) -> LuaResult<LuaTable>;
}

impl ToLua for KernelResponse {
    fn to_lua(&self, lua: &Lua) -> LuaResult<LuaTable> {
        match self {
            KernelResponse::Idle => lua_idle_sentinel(lua),
            KernelResponse::Busy(None) => lua_busy_sentinel(lua),
            KernelResponse::Busy(Some(Message::CommClose(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::CommMsg(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::CommOpen(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::CompleteReply(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::DisplayData(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::ExecuteError(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::ExecuteInput(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::ExecuteResult(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::InputRequest(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::InterruptReply(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::IsCompleteReply(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(Message::Stream(msg))) => msg.content.to_lua(lua),
            KernelResponse::Busy(Some(other)) => Err(LuaError::external(format!(
                "Received unexpected {} message {:#?}",
                other.kind(),
                other
            ))),
        }
    }
}

impl<T: ProtocolMessage> ToLua for T {
    fn to_lua(&self, lua: &Lua) -> LuaResult<LuaTable> {
        let out = lua.create_table().unwrap();
        let _ = out.set("status", "busy");
        let _ = out.set("type", self.kind());
        let _ = out.set("data", lua.to_value(self).unwrap());
        Ok(out)
    }
}

fn lua_busy_sentinel(lua: &Lua) -> LuaResult<LuaTable> {
    let out = lua.create_table().unwrap();
    let _ = out.set("status", "busy");
    Ok(out)
}

fn lua_idle_sentinel(lua: &Lua) -> LuaResult<LuaTable> {
    let out = lua.create_table().unwrap();
    let _ = out.set("status", "idle");
    Ok(out)
}

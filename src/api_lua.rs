use std::collections::HashMap;

use mlua::LuaSerdeExt;
use mlua::prelude::*;

use crate::api;
use crate::msg::wire::jupyter_message::Describe;
use crate::msg::wire::jupyter_message::Message;

pub fn execute_code(
    lua: &Lua,
    (kernel_id, code, user_expressions): (String, String, HashMap<String, String>),
) -> LuaResult<LuaFunction> {
    let callback = api::execute_code(kernel_id, code, user_expressions).into_lua_err()?;

    lua.create_function_mut(move |lua, (): ()| -> LuaResult<LuaTable> {
        let result = callback();

        match result {
            Some(Message::ExecuteResult(msg)) => to_lua(lua, &msg.content),
            Some(Message::ExecuteError(msg)) => to_lua(lua, &msg.content),
            Some(Message::Stream(msg)) => to_lua(lua, &msg.content),
            Some(Message::InputRequest(msg)) => to_lua(lua, &msg.content),
            Some(Message::DisplayData(msg)) => to_lua(lua, &msg.content),
            Some(msg) => Err(LuaError::external(format!(
                "Received unexpected message type {}",
                msg.kind()
            ))),
            _ => Ok(lua.create_table().unwrap()),
        }
    })
}

pub fn is_complete(lua: &Lua, (kernel_id, code): (String, String)) -> LuaResult<LuaTable> {
    match api::is_complete(kernel_id, code) {
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
    match api::get_completions(kernel_id, code, cursor_pos) {
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
    let _ = out.set("type", x.kind());
    let _ = out.set("data", lua.to_value(x).unwrap());
    Ok(out)
}

pub fn request_shutdown(lua: &Lua, kernel_id: String) -> LuaResult<LuaValue> {
    let reply = api::request_shutdown(kernel_id).into_lua_err()?;
    match reply {
        Message::ShutdownReply(msg) => lua.to_value(&msg.content),
        other => Err(LuaError::external(format!(
            "Received unexpected reply to shutdown request {}",
            other.describe()
        ))),
    }
}

pub fn provide_stdin(_: &Lua, (kernel_id, value): (String, String)) -> LuaResult<()> {
    api::provide_stdin(kernel_id, value).into_lua_err()?;
    Ok(())
}

pub fn discover_kernels(lua: &Lua, (): ()) -> LuaResult<mlua::Table> {
    let kernels = api::discover_kernels();

    let kernels_table = lua.create_table()?;

    for kernel in kernels {
        if let Ok(spec) = kernel.spec {
            let _ = kernels_table.set(
                kernel.path.to_string_lossy().to_string(),
                lua.to_value(&spec).unwrap(),
            );
        };
    }

    Ok(kernels_table)
}

pub fn start_kernel(lua: &Lua, spec_path: String) -> LuaResult<(String, LuaValue)> {
    match api::start_kernel(spec_path) {
        Ok((kernel_id, info)) => Ok((kernel_id, lua.to_value(&info).unwrap())),
        Err(e) => Err(LuaError::external(e)),
    }
}

pub fn list_kernels(lua: &Lua, (): ()) -> LuaResult<LuaTable> {
    let kernels = api::list_kernels();
    let table = lua.create_table()?;

    for (k, v) in kernels.iter() {
        table.set(String::from(k), lua.to_value(v).unwrap())?;
    }

    Ok(table)
}

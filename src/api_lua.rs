use mlua::LuaSerdeExt;
use mlua::prelude::*;

use crate::api;
use crate::msg::wire::jupyter_message::Message;
use crate::msg::wire::jupyter_message::MessageType;

pub fn execute_code(lua: &Lua, code: String) -> LuaResult<LuaFunction> {
    let callback = api::execute_code(code);

    lua.create_function_mut(move |lua, (): ()| -> LuaResult<LuaTable> {
        match callback() {
            Some(Message::ExecuteResult(msg)) => to_lua(&msg.content, lua),
            Some(Message::ExecuteError(msg)) => to_lua(&msg.content, lua),
            Some(Message::Stream(msg)) => to_lua(&msg.content, lua),
            Some(Message::InputRequest(msg)) => to_lua(&msg.content, lua),
            Some(msg) => Err(LuaError::external(format!(
                "Received unexpected message type {}",
                msg.kind()
            ))),
            _ => Ok(lua.create_table().unwrap()),
        }
    })
}

/// Converts a message into a Lua table like this:
/// ``` lua
/// {
///     type = "<message type>",
///     data = { <message data> }
/// }
/// ```
fn to_lua<T: MessageType + serde::Serialize>(x: &T, lua: &Lua) -> LuaResult<LuaTable> {
    let out = lua.create_table().unwrap();
    let _ = out.set("type", x.kind());
    let _ = out.set("data", lua.to_value(x).unwrap());
    Ok(out)
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

pub fn start_kernel(_lua: &Lua, spec_path: String) -> LuaResult<String> {
    match api::start_kernel(spec_path) {
        Ok(result) => Ok(result),
        Err(e) => Err(LuaError::external(e)),
    }
}

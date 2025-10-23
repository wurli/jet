use mlua::prelude::*;

use crate::api;

pub fn execute_code(_: &Lua, code: String) -> LuaResult<String> {
    match api::execute_code(code) {
        Ok(result) => Ok(result.to_string()),
        Err(e) => Err(LuaError::external(e)),
    }
}



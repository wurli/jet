use mlua::LuaSerdeExt;
use mlua::prelude::*;

use crate::api;

pub fn execute_code(_lua: &Lua, code: String) -> LuaResult<String> {
    match api::execute_code(code) {
        Ok(result) => Ok(result.to_string()),
        Err(e) => Err(LuaError::external(e)),
    }
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

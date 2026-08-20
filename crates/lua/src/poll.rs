//! Builds the Lua-callable `poll()` closure each request returns.
//!
//! Three-state response, expressed as a Lua table:
//! - `{status="ready", value=<msg>}` when a frame is ready. The `value`
//!   table is the serialized `JupyterMessage` with an added `channel`
//!   field (one of `"shell"`, `"iopub"`, `"stdin"`, `"control"`).
//! - `{status="pending"}` when nothing has arrived yet
//! - `{status="done"}` once the kernel has gone idle for this request

use crate::to_lua_value;
use jet_core::client::{RequestStream, TryRecv};
use mlua::prelude::*;
use std::cell::RefCell;

/// Wrap a per-request stream into a `LuaFunction`. The closure owns the
/// stream; pulls from it stop returning content once the kernel goes
/// idle, after which the closure keeps returning `nil`.
pub fn make_poll(lua: &Lua, stream: RequestStream) -> LuaResult<LuaFunction> {
    let cell = RefCell::new(Some(stream));
    lua.create_function(move |lua, ()| {
        let mut borrow = cell.borrow_mut();
        let Some(stream) = borrow.as_mut() else {
            let t = lua.create_table()?;
            t.set("status", "done")?;
            return Ok(t);
        };
        let t = lua.create_table()?;
        match stream.try_recv() {
            TryRecv::Frame(f) => {
                let msg = to_lua_value(lua, &f.message)?
                    .as_table()
                    .cloned()
                    .expect("JupyterMessage serializes to a table");
                msg.set("channel", lua.to_value(&f.channel)?)?;
                t.set("status", "ready")?;
                t.set("value", msg)?;
            }
            TryRecv::Empty => {
                t.set("status", "pending")?;
            }
            TryRecv::Done => {
                *borrow = None;
                t.set("status", "done")?;
            }
        }
        Ok(t)
    })
}

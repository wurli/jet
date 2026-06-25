//! Builds the Lua-callable `poll()` closure each request returns.
//!
//! Three-state response, expressed as Lua values:
//! - `{status="busy", type=<msg_type>, data=<content>}` when a frame is ready
//! - `{status="pending"}` when nothing has arrived yet
//! - `nil` once the kernel has gone idle for this request

use jet_core::client::{RequestStream, TryRecv};
use jet_core::events::raw_msg_type_and_content;
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
            return Ok(LuaValue::Nil);
        };
        match stream.try_recv() {
            TryRecv::Frame(f) => {
                let (msg_type, content) = raw_msg_type_and_content(&f.message);
                let t = lua.create_table()?;
                t.set("status", "busy")?;
                t.set("type", msg_type)?;
                t.set("data", crate::to_lua_value(lua, &content)?)?;
                Ok(LuaValue::Table(t))
            }
            TryRecv::Empty => {
                let t = lua.create_table()?;
                t.set("status", "pending")?;
                Ok(LuaValue::Table(t))
            }
            TryRecv::Done => {
                *borrow = None;
                Ok(LuaValue::Nil)
            }
        }
    })
}

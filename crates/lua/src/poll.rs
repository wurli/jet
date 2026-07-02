//! Builds the Lua-callable `poll()` closure each request returns.
//!
//! Three-state response, expressed as Lua values:
//! - `{status="busy", channel=<channel>, type=<msg_type>, data=<content>}`
//!   when a frame is ready (`channel` is one of `"shell"`, `"iopub"`,
//!   `"stdin"`, `"control"`)
//! - `{status="pending"}` when nothing has arrived yet
//! - `nil` once the kernel has gone idle for this request

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
            return Ok(LuaValue::Nil);
        };
        match stream.try_recv() {
            TryRecv::Frame(f) => {
                let msg = to_lua_value(lua, &f.message)?
                    .as_table()
                    .cloned()
                    .expect("JupyterMessage serializes to a table");
                msg.set("channel", lua.to_value(&f.channel)?)?;
                let t = lua.create_table()?;
                t.set("status", "busy")?;
                t.set("msg", msg)?;
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

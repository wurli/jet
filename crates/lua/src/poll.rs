//! Builds the Lua-callable `poll()` closure each request returns.
//!
//! Three-state response, expressed as Lua values:
//! - `{status="busy", type=<msg_type>, data=<content>}` when a frame is ready
//! - `{status="pending"}` when nothing has arrived yet
//! - `nil` once the kernel has gone idle for this request

use crossbeam_channel::{Receiver, TryRecvError};
use mlua::prelude::*;
use std::cell::Cell;
use std::sync::Arc;

use crate::router::{FrameRouter, PollItem};

/// Wrap a per-request channel into a `LuaFunction`. The closure owns the
/// receiver, the parent_msg_id (for cleanup on Disconnected), and a
/// "spent" flag so further polls after Idle keep returning `nil`.
pub fn make_poll(
    lua: &Lua,
    rx: Receiver<PollItem>,
    parent_msg_id: String,
    router: Arc<FrameRouter>,
) -> LuaResult<LuaFunction> {
    let spent = Cell::new(false);
    lua.create_function(move |lua, ()| {
        if spent.get() {
            return Ok(LuaValue::Nil);
        }
        match rx.try_recv() {
            Ok(PollItem::Idle) => {
                spent.set(true);
                Ok(LuaValue::Nil)
            }
            Ok(PollItem::Frame { msg_type, content }) => {
                let t = lua.create_table()?;
                t.set("status", "busy")?;
                t.set("type", msg_type)?;
                t.set("data", lua.to_value(&content)?)?;
                Ok(LuaValue::Table(t))
            }
            Err(TryRecvError::Empty) => {
                let t = lua.create_table()?;
                t.set("status", "pending")?;
                Ok(LuaValue::Table(t))
            }
            Err(TryRecvError::Disconnected) => {
                spent.set(true);
                router.forget(&parent_msg_id);
                Ok(LuaValue::Nil)
            }
        }
    })
}

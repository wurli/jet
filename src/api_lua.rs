use mlua::LuaSerdeExt;
use mlua::prelude::*;

use crate::{
    api::{IOPUB_BROKER, SHELL},
    frontend::frontend::ExecuteRequestOptions,
    msg::wire::{jupyter_message::Message, status::ExecutionState},
};
use std::sync::mpsc::channel;

use crate::api;

pub fn execute_code(lua: &Lua, code: String) -> LuaResult<LuaFunction> {
    log::trace!("Sending executre request `{}`", code);

    let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
    let broker = IOPUB_BROKER.get_or_init(|| unreachable!());

    // Create channels for this specific execution request
    let (tx, rx) = channel();

    // Send the execute request and get its message ID
    let request_id = shell.send_execute_request(&code, ExecuteRequestOptions::default());

    // Register this request with the broker
    broker.register_request(request_id.clone(), tx);

    let out = lua
        .create_function(move |_, ()| {
            let mut result = String::new();

            // Get the reply from shell (this blocks until all iopub replies have come through)
            // TODO: Unfortunately we can't query the shell every time the user calls this
            // function, otherwise we risk accidentally consuming replies to other requests. So we
            // need something else. E.g:
            // * We have some kind of global lookup for completed requests which we look into
            //   _before_ we try receiving here
            // * We add a broker for shell messages. This is probably the best approach since
            //   we already have the infrastructure for it.
            let _count = shell.try_recv_execute_reply(&request_id);

            while let Ok(reply) = rx.try_recv() {
                log::trace!("Receiving message {}", reply.kind());
                match reply {
                    // TODO: this won't update incrementally, so we need to change tack. I think what we
                    // need to do is return a handle which can be called from lua to get any results which
                    // may have come through.
                    Message::ExecuteResult(msg) => {
                        result = msg.content.data["text/plain"].clone().to_string();
                        break
                    }
                    Message::Stream(msg) => {
                        result = msg.content.text;
                        break
                    }
                    // Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                    //     busy = true;
                    // }
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                        broker.unregister_request(&request_id);
                        break;
                    }
                    _ => {
                        log::trace!("Dropping received message {}", reply.kind());
                        // We continue receiving until we get something to return
                        // break;
                    }
                };
            };

            Ok(result)
        })
        .unwrap();

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

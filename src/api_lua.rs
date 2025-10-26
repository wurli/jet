use mlua::LuaSerdeExt;
use mlua::prelude::*;

use crate::{
    api::{IOPUB_BROKER, SHELL, SHELL_BROKER},
    frontend::frontend::ExecuteRequestOptions,
    msg::wire::{jupyter_message::Message, status::ExecutionState},
};
use std::sync::mpsc::channel;

use crate::api;

pub fn execute_code(lua: &Lua, code: String) -> LuaResult<LuaFunction> {
    log::trace!("Sending execute request `{}`", code);

    // Send the execute request and get its message ID
    let request_id = {
        let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
        shell.send_execute_request(&code, ExecuteRequestOptions::default())
    };

    let shell_broker = SHELL_BROKER.get_or_init(|| unreachable!());
    let iopub_broker = IOPUB_BROKER.get_or_init(|| unreachable!());

    let (shell_tx, shell_rx) = channel();
    let (iopub_tx, iopub_rx) = channel();

    shell_broker.register_request(request_id.clone(), shell_tx);
    iopub_broker.register_request(request_id.clone(), iopub_tx);

    let out = lua
        .create_function(move |_, ()| {
            // First we check iopub for results. If we get something we don't want to send the user
            // we try again straight away.
            while let Ok(reply) = iopub_rx.try_recv() {
                log::trace!("Receiving message {}", reply.kind());
                match reply {
                    Message::ExecuteResult(msg) => {
                        return Ok(msg.content.data["text/plain"].clone().to_string());
                    }
                    Message::ExecuteError(msg) => return Ok(msg.content.exception.ename),
                    Message::Stream(msg) => return Ok(msg.content.text),
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                    }
                    // NB, it's possible that here we should also check if we have already received
                    // a busy status. However, I don't see any reason to confirm that the kernel is
                    // conforming to this pattern, so I'm not going to for now.
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                        IOPUB_BROKER
                            .get_or_init(|| unreachable!())
                            .unregister_request(&request_id);
                    }
                    _ => {
                        log::trace!("Dropping received message {}", reply.kind());
                        // We continue receiving until we get something to return
                        // break;
                    }
                };
            }
            let shell_broker = SHELL_BROKER.get_or_init(|| unreachable!());

            // If the request id is no longer registered as active then we've evidently already
            // received the reply and we can just return an empty result.
            if !shell_broker.is_active(&request_id) {
                return Ok(String::new());
            }

            // First let's try routing any incoming messages from the shell. In theory there should
            // be only one - the reply to this execute request. However there may be more, e.g.
            // late responses to previous requests.
            if let Ok(msg) = SHELL
                .get_or_init(|| unreachable!())
                .lock()
                .unwrap()
                .try_recv()
            {
                shell_broker.route(msg);
            };

            // Now let's check any shell replies related to this execute request. In theory there
            // should only be one, the final execute reply.
            let _ = match shell_rx.try_recv() {
                // If we get the final reply we can unregister the request since we can be confident
                // it's completed.
                Ok(Message::ExecuteReply(_)) => {
                    shell_broker.unregister_request(&request_id);
                    Ok(lua.create_table())
                }
                Ok(Message::ExecuteReplyException(msg)) => {
                    shell_broker.unregister_request(&request_id);
                    Ok(lua.create_table_from(
                        vec![msg.content.exception.evalue.to_string()]
                            .into_iter()
                            .enumerate(),
                    ))
                }
                // Any other reply is unexpected so we should error
                Ok(msg) => {
                    log::warn!("Unexpected reply received on shell: {}", msg.kind());
                    Err(mlua::Error::RuntimeError(format!(
                        "Unexpected reply received on shell: {}",
                        msg.kind()
                    )))
                }
                // If we couldn't get a reply from the shell then the request is finished
                // and we don't need to return anything.
                Err(_) => Ok(lua.create_table()),
            };

            Ok(String::from(""))
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

use crate::{
    EXECUTE_RX, SHELL,
    frontend::frontend,
    kernel::kernel_spec::KernelInfo,
    msg::wire::{jupyter_message::Message, status::ExecutionState},
};

use assert_matches::assert_matches;

pub fn discover_kernels() -> Vec<KernelInfo> {
    KernelInfo::get_all()
}

pub fn execute_code(code: String) -> anyhow::Result<String> {
    let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();

    let execute_rx = EXECUTE_RX.get_or_init(|| unreachable!()).lock().unwrap();

    shell.send_execute_request(&code, frontend::ExecuteRequestOptions::default());

    // Start with the assumption that the result is empty. Some kernels (e.g. Ark)
    // don't publish an ExecuteResult message in some cases, e.g. when the result
    // is invisible. In such cases we return an empty string for now.
    let mut result = String::from("");

    assert_matches!(execute_rx.recv().unwrap(), Message::Status(msg) => {
        assert_eq!(msg.content.execution_state, ExecutionState::Busy);
    });

    assert_matches!(execute_rx.recv().unwrap(), Message::ExecuteInput(msg) => {
        assert_eq!(code, msg.content.code);
    });

    loop {
        match execute_rx.recv().unwrap() {
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                break;
            }
            Message::ExecuteResult(msg) => {
                result = msg.content.data["text/plain"].clone().to_string();
            }
            // Message::ExecuteInput(msg) => {
            //     assert_eq!(code, msg.content.code);
            // }
            other => panic!("Expected Status(Busy), got {:#?}", other),
        };
    }

    shell.recv_execute_reply();

    Ok(result)
}

// fn is_complete(_lua: Lua, code) -> LuaResult<()> {
//
// }
//
// fn flush_streams() -> LuaResult<()> {
//
// }
//
// fn poll_stdin() -> LuaResult<()> {
//
// }
//
// fn provide_stdin() -> LuaResult<()> {
//     // let x = frontend.stdin
// }

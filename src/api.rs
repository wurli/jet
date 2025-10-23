use rand::Rng;

use crate::{
    frontend::{
        frontend::{self, Frontend},
        shell::Shell,
    },
    kernel::{
        kernel_spec::{KernelSpec, KernelSpecFull},
        startup_method::StartupMethod,
    },
    msg::wire::{
        jupyter_message::Message, kernel_info_full_reply::KernelInfoReply, status::ExecutionState,
    },
};
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, OnceLock};

use assert_matches::assert_matches;

// When we call lua functions we can only pass args from Lua. So, in order
// to access global state within these funcions, we need to use static values.
pub static KERNEL_INFO: OnceLock<(KernelSpec, KernelInfoReply)> = OnceLock::new();
pub static EXECUTE_RX: OnceLock<Mutex<Receiver<Message>>> = OnceLock::new();
pub static STREAM_CHANNEL: OnceLock<Mutex<Receiver<Message>>> = OnceLock::new();
pub static SHELL: OnceLock<Mutex<Shell>> = OnceLock::new();

pub fn discover_kernels() -> Vec<KernelSpecFull> {
    KernelSpecFull::get_all()
}

pub fn start_kernel(spec_path: String) -> anyhow::Result<String> {
    if let Some(info) = KERNEL_INFO.get() {
        return Err(anyhow::anyhow!(
            "Kernel '{}' is already running",
            info.0.display_name
        ));
    };

    let matched_spec = KernelSpecFull::get_all()
        .into_iter()
        .filter(|x| x.path.to_string_lossy() == spec_path)
        .nth(0);

    let spec_full = matched_spec.expect(&format!("No kernel found with name '{}'", spec_path));
    let spec = spec_full.spec?;

    log::info!("Using kernel '{}'", spec.display_name);

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Get the startup command
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let connection_file_path = String::from("carpo_connection_file.json");
    let kernel_cmd = spec.build_command(&connection_file_path);

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Start the frontend
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let frontend = match spec.get_startup_method() {
        StartupMethod::RegistrationFile => {
            Frontend::start_with_registration_file(kernel_cmd, connection_file_path.into())
        }
        StartupMethod::ConnectionFile => {
            Frontend::start_with_connection_file(kernel_cmd, connection_file_path.into())
        }
    };

    let kernel_info = frontend.subscribe();

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Heartbeat thread
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let heartbeat = frontend.heartbeat;

    std::thread::spawn(move || {
        loop {
            let mut rng = rand::rng();
            // We just send some random number to the kernel
            let bytes: Vec<u8> = (0..32).map(|_| rng.random_range(0..10)).collect();

            heartbeat.send(zmq::Message::from(bytes));

            // Then we (hopefully) wait to receive the same message back
            let _ = heartbeat.recv_with_timeout().expect("Heartbeat timed out");

            // TODO: check the message we receive is the one we sent
            // assert_eq!(bytes, msg.);
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Iopub thread
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let (execute_tx, execute_rx) = std::sync::mpsc::channel();
    let (stream_tx, stream_rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        loop {
            match frontend.iopub.recv() {
                msg @ Message::Stream(_) => stream_tx.send(msg).unwrap(),
                msg @ Message::Status(_) => execute_tx.send(msg).unwrap(),
                msg @ Message::ExecuteInput(_) => execute_tx.send(msg).unwrap(),
                msg @ Message::ExecuteResult(_) => execute_tx.send(msg).unwrap(),
                msg @ Message::ExecuteReply(_) => execute_tx.send(msg).unwrap(),
                _ => todo!(),
            };
        }
    });

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Initialise global state
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // log::info!("{}", kernel_info.banner);

    KERNEL_INFO.get_or_init(|| (spec, kernel_info.clone()));
    SHELL.get_or_init(|| Mutex::new(frontend.shell));
    EXECUTE_RX.get_or_init(|| Mutex::new(execute_rx));
    STREAM_CHANNEL.get_or_init(|| Mutex::new(stream_rx));

    Ok(kernel_info.banner)
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

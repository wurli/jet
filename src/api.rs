use crate::{
    frontend::{
        frontend::{ExecuteRequestOptions, Frontend},
        shell::Shell,
    },
    kernel::{
        kernel_spec::{KernelSpec, KernelSpecFull},
        startup_method::StartupMethod,
    },
    msg::wire::{
        jupyter_message::Message, kernel_info_full_reply::KernelInfoReply, status::ExecutionState,
    },
    supervisor::{
        broker::Broker,
        listeners::{listen_iopub, loop_heartbeat},
    },
};
use std::sync::{Arc, Mutex, OnceLock, mpsc::channel};

// When we call lua functions we can only pass args from Lua. So, in order
// to access global state within these funcions, we need to use static values.
pub static KERNEL_INFO: OnceLock<(KernelSpec, KernelInfoReply)> = OnceLock::new();
pub static SHELL: OnceLock<Mutex<Shell>> = OnceLock::new();
pub static IOPUB_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();
pub static SHELL_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();

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

    loop_heartbeat(frontend.heartbeat);
    let iopub_broker = Arc::new(Broker::new(String::from("IOPub")));
    let shell_broker = Arc::new(Broker::new(String::from("Shell")));

    // Start the iopub/shell processing threads
    listen_iopub(frontend.iopub, Arc::clone(&iopub_broker));
    // listen_shell(frontend.shell, Arc::clone(&shell_broker));

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Initialise global state
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // log::info!("{}", kernel_info.banner);

    KERNEL_INFO.get_or_init(|| (spec, kernel_info.clone()));
    SHELL.get_or_init(|| Mutex::new(frontend.shell));
    IOPUB_BROKER.get_or_init(|| iopub_broker);
    SHELL_BROKER.get_or_init(|| shell_broker);

    Ok(kernel_info.banner)
}

// pub fn is_complete(code: String) -> anyhow::Result<String> {
//     let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
// }

pub fn execute_code(code: String) -> anyhow::Result<String> {
    let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
    let broker = IOPUB_BROKER.get_or_init(|| unreachable!());

    // Create channels for this specific execution request
    let (tx, rx) = channel();

    // Send the execute request and get its message ID
    let request_id = shell.send_execute_request(&code, ExecuteRequestOptions::default());

    // Register this request with the broker
    broker.register_request(request_id.clone(), tx);

    // Get the reply from shell (this should block until rx has received all the iopub messages for
    // the request)
    shell.recv_execute_reply();

    let mut result = String::from("");
    let mut busy = false;

    for reply in rx.iter() {
        log::trace!("Looping through message {}", reply.kind());
        match reply {
            // TODO: this won't update incrementally, so we need to change tack. I think what we
            // need to do is return a handle which can be called from lua to get any results which
            // may have come through.
            Message::ExecuteResult(msg) => {
                result.push_str(&msg.content.data["text/plain"].clone().to_string());
            }
            Message::Stream(msg) => {
                result.push_str(&msg.content.text);
            }
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                busy = true;
            }
            Message::Status(msg) if busy && msg.content.execution_state == ExecutionState::Idle => {
                broker.unregister_request(&request_id);
                break;
            }
            _ => {
                log::trace!("Dropping received message {}", reply.kind());
            }
        }
    }

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

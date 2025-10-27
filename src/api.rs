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
        jupyter_message::{Message, MessageType},
        kernel_info_full_reply::KernelInfoReply,
        status::ExecutionState,
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

pub fn execute_code(code: String) -> impl Fn() -> Option<Message> {
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

    move || {
        // First we check iopub for results. If we get a reply without any viewable output we
        // try again straight away.
        while let Ok(reply) = iopub_rx.try_recv() {
            log::trace!("Receiving message {}", reply.kind());
            match reply {
                // These are the message types we want to surface in Lua
                Message::ExecuteResult(_) | Message::ExecuteError(_) | Message::Stream(_) => {
                    return Some(reply);
                }
                // Here we can just add a sense check to ensure the code matches what we sent
                Message::ExecuteInput(msg) => {
                    if msg.content.code != code {
                        log::warn!(
                            "Received {} with unexpected code: {}",
                            msg.content.kind(),
                            msg.content.code
                        );
                    };
                }
                // This is expected immediately after sending the execute request
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {}
                // NB, it's possible that here we should also check if we have already received
                // a busy status. However, I don't see any reason to confirm that the kernel is
                // conforming to this pattern, so I'm not going to for now.
                Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                    iopub_broker.unregister_request(&request_id);
                }
                // There shouldn't be anything else. If there is we need a warning.
                _ => {
                    log::warn!("Dropping received message {}", reply.kind());
                    // We continue receiving until we get something to return
                }
            };
        }

        // If the request id is no longer registered as active then we've evidently already
        // received the reply and we can just return an empty result.
        if !shell_broker.is_active(&request_id) {
            return None;
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
        match shell_rx.try_recv() {
            // If we get the final reply we can unregister the request since we can be confident
            // it's completed.
            Ok(Message::ExecuteReply(_)) => {
                shell_broker.unregister_request(&request_id);
            }
            // This comes through in the case that the code produced an error, but the user is
            // notified via the iopub's `ExecuteError`
            Ok(Message::ExecuteReplyException(_)) => {
                shell_broker.unregister_request(&request_id);
            }
            // Any other reply is unexpected
            Ok(msg) => {
                log::warn!("Unexpected reply received on shell: {}", msg.kind());
            }
            // If we couldn't get a reply from the shell then the request is finished
            // and we don't need to return anything.
            Err(_) => {}
        };

        None
    }
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

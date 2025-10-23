use rand::Rng;

use crate::{
    frontend::{
        frontend::{self, Frontend},
        shell::Shell,
        iopub_thread,
    },
    kernel::{
        kernel_spec::{KernelSpec, KernelSpecFull},
        startup_method::StartupMethod,
    },
    msg::{
        broker::{IopubBroker, ExecutionResult},
        wire::{
            jupyter_message::Message, kernel_info_full_reply::KernelInfoReply, status::ExecutionState,
        },
    },
};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

// When we call lua functions we can only pass args from Lua. So, in order
// to access global state within these funcions, we need to use static values.
pub static KERNEL_INFO: OnceLock<(KernelSpec, KernelInfoReply)> = OnceLock::new();
pub static SHELL: OnceLock<Mutex<Shell>> = OnceLock::new();
pub static IOPUB_BROKER: OnceLock<Arc<IopubBroker>> = OnceLock::new();

// Legacy stream channel for backward compatibility
pub static STREAM_CHANNEL: OnceLock<Mutex<Receiver<Message>>> = OnceLock::new();

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
    // IOPub thread with broker-based routing
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let broker = Arc::new(IopubBroker::new());
    
    // Set up a global stream channel for backward compatibility
    let (stream_tx, stream_rx) = std::sync::mpsc::channel();
    broker.add_global_subscriber(stream_tx);
    
    // Start the IOPub processing thread
    let broker_clone = Arc::clone(&broker);
    iopub_thread::start_iopub_thread(frontend.iopub, broker_clone);

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Initialise global state
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // log::info!("{}", kernel_info.banner);

    KERNEL_INFO.get_or_init(|| (spec, kernel_info.clone()));
    SHELL.get_or_init(|| Mutex::new(frontend.shell));
    IOPUB_BROKER.get_or_init(|| broker);
    STREAM_CHANNEL.get_or_init(|| Mutex::new(stream_rx));

    Ok(kernel_info.banner)
}

// pub fn is_complete(code: String) -> anyhow::Result<String> {
//     let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
// }

pub fn execute_code(code: String) -> anyhow::Result<String> {
    let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
    let broker = IOPUB_BROKER.get_or_init(|| unreachable!());

    // Create channels for this specific execution request
    let (channels, collector) = ExecutionResult::create_channels();

    // Send the execute request and get its message ID
    let request_id = shell.send_execute_request(&code, frontend::ExecuteRequestOptions::default());

    // Register this request with the broker
    broker.register_request(request_id.clone(), channels);

    // Start with the assumption that the result is empty. Some kernels (e.g. Ark)
    // don't publish an ExecuteResult message in some cases, e.g. when the result
    // is invisible. In such cases we return an empty string for now.
    let mut result = String::from("");

    // Wait for Busy status
    match collector.status_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Message::Status(msg)) => {
            assert_eq!(msg.content.execution_state, ExecutionState::Busy);
        }
        Ok(other) => {
            broker.unregister_request(&request_id);
            return Err(anyhow::anyhow!("Expected Status(Busy), got {:?}", other.message_type()));
        }
        Err(e) => {
            broker.unregister_request(&request_id);
            return Err(anyhow::anyhow!("Timeout waiting for Busy status: {}", e));
        }
    }

    // Wait for ExecuteInput
    match collector.execution_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Message::ExecuteInput(msg)) => {
            assert_eq!(code, msg.content.code);
        }
        Ok(other) => {
            broker.unregister_request(&request_id);
            return Err(anyhow::anyhow!("Expected ExecuteInput, got {:?}", other.message_type()));
        }
        Err(e) => {
            broker.unregister_request(&request_id);
            return Err(anyhow::anyhow!("Timeout waiting for ExecuteInput: {}", e));
        }
    }

    // Collect results until we see Idle status
    loop {
        // Try execution channel first (for results)
        match collector.execution_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Message::ExecuteResult(msg)) => {
                result = msg.content.data["text/plain"].clone().to_string();
                continue;
            }
            Ok(Message::ExecuteError(msg)) => {
                broker.unregister_request(&request_id);
                return Err(anyhow::anyhow!("Execution error: {}", msg.content.exception.evalue));
            }
            Ok(other) => {
                log::warn!("Unexpected execution message: {:?}", other.message_type());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Check status channel
            }
            Err(e) => {
                broker.unregister_request(&request_id);
                return Err(anyhow::anyhow!("Error receiving execution messages: {}", e));
            }
        }

        // Check for Idle status
        match collector.status_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Message::Status(msg)) if msg.content.execution_state == ExecutionState::Idle => {
                break;
            }
            Ok(Message::Status(_)) => {
                // Other status, continue
            }
            Ok(other) => {
                log::warn!("Unexpected status message: {:?}", other.message_type());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Continue waiting
            }
            Err(e) => {
                broker.unregister_request(&request_id);
                return Err(anyhow::anyhow!("Error receiving status messages: {}", e));
            }
        }

        // Also drain stream and display channels to avoid blocking
        let _ = collector.stream_rx.try_recv();
        let _ = collector.display_rx.try_recv();
        let _ = collector.comm_rx.try_recv();
    }

    // Get the reply from shell
    shell.recv_execute_reply();

    // Unregister the request
    broker.unregister_request(&request_id);

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

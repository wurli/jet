pub mod api;
pub mod api_lua;
pub mod frontend;
pub mod kernel;
pub mod msg;

use mlua::prelude::*;
use frontend::frontend::Frontend;
use kernel::kernel_spec::KernelInfo;
use kernel::startup_method::StartupMethod;
use msg::error;
use rand::Rng;

use crate::frontend::shell::Shell;
use crate::msg::wire::jupyter_message::Message;

use std::sync::mpsc::Receiver;
use std::sync::{Mutex, OnceLock};

// When we call lua functions we can only pass args from Lua. So, in order
// to access global state within these funcions, we need to use static values.
pub static EXECUTE_RX: OnceLock<Mutex<Receiver<Message>>> = OnceLock::new();
pub static STREAM_CHANNEL: OnceLock<Mutex<Receiver<Message>>> = OnceLock::new();
pub static SHELL: OnceLock<Mutex<Shell>> = OnceLock::new();

pub type Result<T> = std::result::Result<T, error::Error>;

#[mlua::lua_module(skip_memory_check)]
pub fn carpo(lua: &Lua) -> LuaResult<LuaTable> {
    // // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // // Parse command line options
    // // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // let mut argv = std::env::args();
    //
    // // Skip the first "argument" as it's the path/name to this executable
    // argv.next();
    //
    // let mut log_file: Option<String> = None;
    //
    // while let Some(arg) = argv.next() {
    //     match arg.as_str() {
    //         "--log" => {
    //             if let Some(file) = argv.next() {
    //                 log_file = Some(file);
    //             } else {
    //                 return Err(anyhow::anyhow!(
    //                     "A log file must be specified when using the `--log` argument."
    //                 ));
    //             }
    //         }
    //         other => {
    //             return Err(anyhow::anyhow!("Argument '{other}' unknown."));
    //         }
    //     }
    // }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Initialise the logger
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Not sure if I can pass a file when starting the lua module, so for now
    // just hardcode
    let log_file = Some(String::from("carpo.log"));

    // Initialize logging system; you can configure levels with the RUST_LOG env var
    if let Some(log_path) = log_file {
        let target = Box::new(
            std::fs::File::create(&log_path)
                .expect(&format!("Can't create log file at {log_path}")),
        );
        env_logger::Builder::from_default_env()
            .target(env_logger::Target::Pipe(target))
            .init();
    } else {
        env_logger::init();
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Get the kernel to use
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let selected_kernel_name = String::from("Ark R Kernel");
    // let selected_kernel_name = String::from("Ark R Kernel (connection file method)");
    // let selected_kernel_name = String::from("Python 3 (ipykernel)");

    let selected_kernel = KernelInfo::get_all()
        .into_iter()
        .filter_map(|x| x.spec)
        .filter(|x| x.display_name == selected_kernel_name)
        .nth(0);

    let spec = match selected_kernel {
        Some(kernel) => kernel,
        None => panic!("No kernel found with name '{}'", selected_kernel_name),
    };

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

    let _kernel_info = frontend.subscribe();

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
    SHELL.get_or_init(|| Mutex::new(frontend.shell));
    EXECUTE_RX.get_or_init(|| Mutex::new(execute_rx));
    STREAM_CHANNEL.get_or_init(|| Mutex::new(stream_rx));

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Return the Lua API
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    let exports = lua.create_table()?;
    exports.set("execute_code", lua.create_function(api_lua::execute_code)?)?;
    Ok(exports)
}

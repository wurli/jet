pub mod kernel;
pub mod msg;

use assert_matches::assert_matches;
use kernel::kernel_spec::KernelInfo;
use kernel::startup_method::StartupMethod;
use msg::error;
use msg::frontend::Frontend;
use msg::wire;

pub type Result<T> = std::result::Result<T, error::Error>;

fn main() -> anyhow::Result<()> {
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Parse command line options
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let mut argv = std::env::args();

    // Skip the first "argument" as it's the path/name to this executable
    argv.next();

    let mut log_file: Option<String> = None;

    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--log" => {
                if let Some(file) = argv.next() {
                    log_file = Some(file);
                } else {
                    return Err(anyhow::anyhow!(
                        "A log file must be specified when using the `--log` argument."
                    ));
                }
            }
            other => {
                return Err(anyhow::anyhow!("Argument '{other}' unknown."));
            }
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Initialise the logger
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
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
    // let selected_kernel_name = String::from("Ark R Kernel");
    let selected_kernel_name = String::from("Ark R Kernel (connection file method)");
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

    // Give it a brief moment for any Welcome messages to arrive
    // std::thread::sleep(std::time::Duration::from_millis(200));

    // Not all kernels implement the XPUB socket which provides the welcome message which confirms
    // the connection is established. PEP 65 recommends dealing with this by:
    // 1. Sending a kernel info request
    // 2. Checking the protocol version in the reply
    // 3. Waiting for the welcome message if the protocol supports it
    //
    // Docs: https://jupyter.org/enhancement-proposals/65-jupyter-xpub/jupyter-xpub.html#impact-on-existing-implementations
    frontend.send_shell(wire::kernel_info_request::KernelInfoRequest {});
    frontend.recv_iopub_busy();
    let reply = frontend.recv_shell();

    let kernel_info = match reply {
        wire::jupyter_message::Message::KernelInfoReply(reply) => reply,
        _ => {
            return Err(anyhow::anyhow!(
                "Expected kernel_info_reply, but got {:#?}",
                reply
            ));
        }
    };

    println!("Protocol version: {}", kernel_info.content.protocol_version);

    // Unfortunately, although JEP 65 is accepted, I can't find the version of the jupyter protocol
    // in which it becomes effective. Ark _does_ support it and is 5.4, ipython doesn't and is 5.3.
    if kernel_info.content.protocol_version >= String::from("5.4") {
        // Immediately block until we've received the IOPub welcome message from the XPUB server side
        // socket. This confirms that we've fully subscribed and avoids dropping any of the initial
        // IOPub messages that a server may send if we start to perform requests immediately (in
        // particular, busy/idle messages). https://github.com/posit-dev/ark/pull/577
        assert_matches!(frontend.recv_iopub(), wire::jupyter_message::Message::Welcome(data) => {
            assert_eq!(data.content.subscription, String::from(""));
        });
        // We also go ahead and handle the `ExecutionState::Starting` status that we know is coming
        // from the kernel right after the `Welcome` message, so tests don't have to care about this.
        assert_matches!(frontend.recv_iopub(), wire::jupyter_message::Message::Status(data) => {
            assert_eq!(data.content.execution_state, wire::status::ExecutionState::Starting);
        });
    }

    // Consume the Idle status
    frontend.recv_iopub_idle();

    let code = "1 + 1"; // R code
    // let code = "{'x': [1, 2, 3], 'y': ['a', 'b', 'c']}";  // Python code

    frontend.send_execute_request(code, msg::frontend::ExecuteRequestOptions::default());
    frontend.recv_iopub_busy();

    let input = frontend.recv_iopub_execute_input();
    let reply = frontend.recv_iopub_execute_result();
    println!("-------------------------------------------------------------");
    println!("> {}", input.code);
    println!("{}", reply);
    println!("-------------------------------------------------------------");

    frontend.recv_iopub_idle();
    frontend.recv_shell_execute_reply();

    Ok(())
}

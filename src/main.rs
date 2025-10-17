pub mod kernel;
pub mod msg;

use kernel::startup_method::StartupMethod;
use kernel::kernel_spec::KernelInfo;
use msg::error;
use msg::frontend;
use msg::frontend::Frontend;

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

    // Give it a brief moment for any Welcome messages to arrive
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Check for and consume Welcome message if present (from kernels using XPUB like Ark)
    if frontend.iopub_socket.poll_incoming(100).unwrap() {
        let msg = frontend.recv_iopub();
        if let crate::msg::wire::jupyter_message::Message::Welcome(_) = msg {
            println!("Received Welcome message from XPUB kernel");
            // After Welcome, Ark kernels send a Starting status
            let starting_msg = frontend.recv_iopub();
            if let crate::msg::wire::jupyter_message::Message::Status(ref data) = starting_msg {
                if data.content.execution_state
                    == crate::msg::wire::status::ExecutionState::Starting
                {
                    println!("Received Starting status after Welcome");
                } else {
                    panic!(
                        "Expected Starting status after Welcome, got: {:?}",
                        starting_msg
                    );
                }
            } else {
                panic!(
                    "Expected Status message after Welcome, got: {:?}",
                    starting_msg
                );
            }
        } else {
            // If it's not Welcome, we'll need to handle it below
            panic!("Expected Welcome or no message, got: {:?}", msg);
        }
    }

    // Now send a kernel_info_request to ensure the kernel is ready
    println!("Sending kernel_info_request to initialize kernel connection...");
    frontend.send_shell(crate::msg::wire::kernel_info_request::KernelInfoRequest {});

    // Consume the Busy status from kernel_info_request
    frontend.recv_iopub_busy();

    // Consume the Idle status
    frontend.recv_iopub_idle();

    // Drain the shell socket to consume the kernel_info_reply
    // (we don't parse it as it might have version-specific fields)
    if frontend.shell_socket.poll_incoming(10000).unwrap() {
        let _ = frontend.shell_socket.recv_multipart();
    }

    println!("Kernel connection initialized and ready!");

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Try receiving some info
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // This works fine
    // frontend.send_shell(crate::msg::wire::kernel_info_request::KernelInfoRequest {});
    // let res = frontend.recv_shell();
    // println!("{:#?}\n", res);

    // frontend.send_execute_request("42", frontend::ExecuteRequestOptions::default());
    // frontend.recv_iopub_busy();
    // let input = frontend.recv_iopub_execute_input();
    // println!("{:#?}\n", input)

    let code = "1 + 1"; // R code
    // let code = "{'x': [1, 2, 3], 'y': ['a', 'b', 'c']}";  // Python code

    frontend.send_execute_request(code, frontend::ExecuteRequestOptions::default());
    frontend.recv_iopub_busy();

    let input = frontend.recv_iopub_execute_input();
    let reply = frontend.recv_iopub_execute_result();
    println!("-------------------------------------------------------------");
    println!("> {}", input.code);
    println!("{}", reply);
    println!("-------------------------------------------------------------");

    frontend.recv_iopub_idle();
    frontend.recv_shell_execute_reply();

    // let code = "dplyr::tibble(x = 1)";
    // frontend.send_execute_request(code, frontend::ExecuteRequestOptions::default());
    // frontend.recv_iopub_busy();
    //
    // let input = frontend.recv_iopub_execute_input();
    // let reply = frontend.recv_iopub_execute_result();
    // println!("Input code: {}", input.code);
    // println!("Result    : {}", reply);

    // frontend.recv_iopub_idle();
    // assert_eq!(frontend.recv_shell_execute_reply(), input.execution_count);

    Ok(())
}

pub mod kernel;
pub mod msg;

use std::process::Command;

use kernel::kernel_spec::KernelSpec;
use msg::connection_file::ConnectionFile;
use msg::error;
use msg::frontend;

use crate::msg::frontend::Frontend;

pub type Result<T> = std::result::Result<T, error::Error>;

fn main() {
    // Initialize logging system; you can configure levels with the RUST_LOG env var
    env_logger::init();

    // let connection = frontend::Connection::new();
    // let (connection_file, registration_file) = connection.get_connection_files();

    let kernels = kernel::discover::discover_kernels();

    let mut paths = String::from("");

    for k in &kernels {
        paths.push_str("\n");
        paths.push_str(&k.to_string_lossy());
    }

    println!("Kernels: {}", paths);
    for k in kernels {
        match KernelSpec::from_file(k) {
            Ok(spec) => println!("{:#?}\n", spec),
            Err(e) => eprintln!("Error reading kernel spec: {}", e),
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Get the kernel to use
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    // let selected_kernel_name = String::from("Ark R Kernel");
    let selected_kernel_name = String::from("Python 3 (ipykernel)");

    let selected_kernel = KernelSpec::get_all()
        .into_iter()
        .filter(|spec| spec.display_name == selected_kernel_name)
        .nth(0);

    let spec = match selected_kernel {
        Some(kernel) => kernel,
        None => {
            println!("No kernel found with name '{}'", selected_kernel_name);
            return;
        }
    };

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Get the args to start the kernel
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let mut args = spec.argv;

    let connection_file_path = "carpo_connection_file.json";

    for arg in args.iter_mut() {
        if *arg == "{connection_file}" {
            *arg = connection_file_path.to_string()
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Start the kernel
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let mut cmd = Command::new(args.remove(0));
    cmd.args(args);

    if let Some(env_vars) = spec.env {
        println!("Setting vars {:#?}", env_vars);
        cmd.envs(env_vars);
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Start the frontend
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let frontend_opts = frontend::FrontendOptions::init();

    let mut use_registration_file = false;

    // if selected_kernel_name == String::from("Ark R Kernel") {
    //     use_registration_file = true;
    // }

    if let Some(version) = spec.kernel_protocol_version {
        if version >= String::from("5.5") {
            use_registration_file = true;
        }
    }

    let frontend: Frontend;

    if use_registration_file {
        frontend = frontend::Frontend::start_with_registration_file(
            frontend_opts,
            connection_file_path.into(),
            cmd
        );
    } else {
        frontend = frontend::Frontend::start_with_connection_file(
            frontend_opts,
            connection_file_path.into(),
            cmd
        );

        // Give it a brief moment for any Welcome messages to arrive
        std::thread::sleep(std::time::Duration::from_millis(200));

        //Check for and consume Welcome message if present (from kernels using XPUB like Ark)
        if frontend.iopub_socket.poll_incoming(100).unwrap() {
            let msg = frontend.recv_iopub();
            if let crate::msg::wire::jupyter_message::Message::Welcome(_) = msg {
                println!("Received Welcome message from XPUB kernel");
                // After Welcome, Ark kernels send a Starting status
                let starting_msg = frontend.recv_iopub();
                if let crate::msg::wire::jupyter_message::Message::Status(ref data) = starting_msg {
                    if data.content.execution_state == crate::msg::wire::status::ExecutionState::Starting {
                        println!("Received Starting status after Welcome");
                    } else {
                        panic!("Expected Starting status after Welcome, got: {:?}", starting_msg);
                    }
                } else {
                    panic!("Expected Status message after Welcome, got: {:?}", starting_msg);
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
    }

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

    let code = "1 + 1";  // R code
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
}

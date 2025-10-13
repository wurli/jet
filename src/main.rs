pub mod kernel;
pub mod msg;

use std::path::PathBuf;
use std::process::Command;

use kernel::kernel_spec::KernelSpec;
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

    let connection = frontend::Connection::new();

    let (connection_file, registration_file) = connection.get_connection_files();

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Create connection/registration files
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let connection_file_path = "carpo_connection_file.json";
    let registration_file_path = "carpo_registration_file.json";

    connection_file
        .to_file(PathBuf::from(connection_file_path))
        .expect("Could not write connection file");

    registration_file
        .to_file(PathBuf::from(registration_file_path))
        .expect("Could not write registration file");

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Get the kernel to use
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    let selected_kernel_name = String::from("Ark R Kernel");
    // let selected_kernel_name = String::from("Python 3 (ipykernel)");

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

    for arg in args.iter_mut() {
        if *arg == "{connection_file}" {
            *arg = registration_file_path.to_string()
            // *arg = connection_file_path.to_string()
        }
    }

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Start the kernel
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let _ = Command::new(args.remove(0)).args(args).spawn();

    println!("Successfully started kernel '{}'", spec.display_name);

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Try receiving some info
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let frontend = Frontend::from_connection(connection);

    // This works fine
    // frontend.send_shell(crate::msg::wire::kernel_info_request::KernelInfoRequest {});
    // let res = frontend.recv_shell();
    // println!("{:#?}\n", res);


    frontend.send_execute_request("42", frontend::ExecuteRequestOptions::default());
    frontend.recv_iopub_busy();
    let input = frontend.recv_iopub_execute_input();
    println!("{:#?}\n", input)
}

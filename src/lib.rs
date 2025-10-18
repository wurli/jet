pub mod kernel;
pub mod msg;
pub mod frontend;
// pub mod frontend;

use kernel::kernel_spec::KernelInfo;
use kernel::startup_method::StartupMethod;
use msg::error;
use frontend::Frontend;

pub type Result<T> = std::result::Result<T, error::Error>;

pub fn carpo() -> anyhow::Result<()> {
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

    let _kernel_info = frontend.subscribe();

    frontend.send_execute_request("1 + 1", frontend::ExecuteRequestOptions::default());
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

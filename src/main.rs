pub mod kernel;
pub mod msg;

use kernel::discover;
use kernel::kernel_spec;
// use msg::frontend;
use msg::error;

pub type Result<T> = std::result::Result<T, error::Error>;

fn main() {
    // Initialize logging system; you can configure levels with the RUST_LOG env var
    env_logger::init();

    // let connection = frontend::Connection::new();
    // let (connection_file, registration_file) = connection.get_connection_files();

    let kernels = discover::discover_kernels();

    let mut paths = String::from("");

    for k in &kernels {
        paths.push_str("\n");
        paths.push_str(&k.to_string_lossy());
    }


    println!("Kernels: {}", paths);
    for k in kernels {
        match kernel_spec::KernelSpec::from_file(k) {
            Ok(spec) => println!("Spec: {:#?}", spec),
            Err(e) => eprintln!("Error reading kernel spec: {}", e),
        }
    }
}



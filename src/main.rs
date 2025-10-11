pub mod kernel;
pub mod msg;

use kernel::discover;
// use msg::frontend;
use msg::error;

pub type Result<T> = std::result::Result<T, error::Error>;

fn main() {
    // Initialize logging system; you can configure levels with the RUST_LOG env var
    env_logger::init();

    // let connection = frontend::Connection::new();
    // let (connection_file, registration_file) = connection.get_connection_files();

    let mut found = String::from("");

    for k in discover::discover_kernels() {
        found.push_str("\n");
        found.push_str(&k);
    }

    println!("Kernels: {}", found)
}

use std::process::Command;
pub mod msg;

fn main() {
    let _ = Command::new("python")
        .arg("-m")
        .arg("ipykernel")
        .arg("-f=connection_file.json")
        .spawn();


}

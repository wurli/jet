use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tempfile::Builder;
use serde::{ Serialize, Deserialize };

use crate::msg::port::RandomUserPort;

/// The contents of the Connection File as listed in the Jupyter specfication;
/// directly parsed from JSON.
#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectionFile {
    /// ZeroMQ port: Control channel (kernel interrupts)
    pub control_port: u16,

    /// ZeroMQ port: Shell channel (execution, completion)
    pub shell_port: u16,

    /// ZeroMQ port: Standard input channel (prompts)
    pub stdin_port: u16,

    /// ZeroMQ port: IOPub channel (broadcasts input/output)
    pub iopub_port: u16,

    /// ZeroMQ port: Heartbeat messages (echo)
    pub hb_port: u16,

    /// The transport type to use for ZeroMQ; generally "tcp"
    pub transport: String,

    /// The signature scheme to use for messages; generally "hmac-sha256"
    pub signature_scheme: String,

    /// The IP address to bind to
    pub ip: String,

    /// The HMAC-256 signing key, or an empty string for an unauthenticated
    /// connection
    pub key: String,
}


impl ConnectionFile {
    pub fn new() -> Self {
        Self {
            control_port: RandomUserPort::find().expect("Failed to open control port"),
            shell_port: RandomUserPort::find().expect("Failed to open shell port"),
            stdin_port: RandomUserPort::find().expect("Failed to open stdin port"),
            iopub_port: RandomUserPort::find().expect("Failed to open iopub port"),
            hb_port: RandomUserPort::find().expect("Failed to open hb port"),
            transport: String::from("tcp"),
            signature_scheme: String::from("hmac-sha256"),
            ip: String::from("127.0.0.1"),
            // TODO: support keys, e.g. for using Jupyter over a network
            key: String::from(""),
        }
    }

    /// Create a `ConnectionFile` by parsing the contents of a connection file.
    pub fn from_file<P: AsRef<Path>>(connection_file: P) -> Result<ConnectionFile, Box<dyn Error>> {
        let file = File::open(connection_file)?;
        let reader = BufReader::new(file);
        let control = serde_json::from_reader(reader)?;

        Ok(control)
    }

    /// Write to an actual file
    pub fn to_file<P>(&self, file: Option<P>) -> String
    where
        P: AsRef<Path> + Into<String>,
    {
        let json = serde_json::to_string_pretty(&self).unwrap();
        log::info!("Connection file contents: {json}");

        if let Some(path) = file {
            fs::write(&path, json).expect("Unable to write connection file");
            return path.into();
        };

        // Generate a temp file if no path supplied
        let path = Builder::new()
            .prefix("ark_connection_file")
            .suffix(".json")
            .tempfile()
            .expect("Unable to generate a temporary file");

        fs::write(&path, json).expect("Unable to write connection file");

        path.path()
            .to_owned()
            .to_str()
            .expect("Unable to get connection file path")
            .to_string()
    }

    /// Given a port, return a URI-like string that can be used to connect to
    /// the port, given the other parameters in the connection file.
    ///
    /// Example: `32` => `"tcp://127.0.0.1:32"`
    pub fn endpoint(&self, port: u16) -> String {
        format!("{}://{}:{}", self.transport, self.ip, port)
    }
}

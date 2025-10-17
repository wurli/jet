use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::kernel::discover::discover_kernels;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InterruptMode {
    Signal,
    Message,
}

/// docs: https://jupyter-client.readthedocs.io/en/latest/kernels.html#kernel-specs
/// spec: https://github.com/jupyter/enhancement-proposals/blob/master/105-kernelspec-spec/kernelspec.schema.json
#[derive(Serialize, Deserialize, Debug)]
pub struct KernelSpec {
    /// A list of command line arguments used to start the kernel. The text {connection_file} in
    /// any argument will be replaced with the path to the connection file.
    pub argv: Vec<String>,

    /// The kernel’s name as it should be displayed in the UI. Unlike the kernel name used in the
    /// API, this can contain arbitrary unicode characters.
    pub display_name: String,

    /// The name of the language of the kernel. When loading notebooks, if no matching kernelspec
    /// key (may differ across machines) is found, a kernel with a matching `language` will be
    /// used. This allows a notebook written on any Python or Julia kernel to be properly
    /// associated with the user’s Python or Julia kernel, even if they aren’t listed under the
    /// same name as the author’s.
    pub language: String,

    /// (optional): May be either `signal` or `message` and specifies how a client is supposed to
    /// interrupt cell execution on this kernel, either by sending an interrupt `signal` via the
    /// operating system’s signalling facilities (e.g. `SIGINT` on POSIX systems), or by sending an
    /// `interrupt_request` message on the control channel (see Kernel interrupt). If this is not
    /// specified the client will default to `signal` mode.
    pub interrupt_mode: Option<InterruptMode>,

    /// (optional) A dictionary of environment variables to set for the kernel. These will be added
    /// to the current environment variables before the kernel is started. Existing environment
    /// variables can be referenced using `${<ENV_VAR>}` and will be substituted with the
    /// corresponding value. Administrators should note that use of `${<ENV_VAR>}` can expose
    /// sensitive variables and should use only in controlled circumstances.
    pub env: Option<HashMap<String, String>>,

    /// (optional) A dictionary of additional attributes about this kernel; used by clients to aid
    /// in kernel selection. Metadata added here should be namespaced for the tool reading and
    /// writing that metadata.
    pub metadata: Option<HashMap<String, Value>>,

    /// (optional) The version of protocol this kernel implements. If not specified, the client
    /// will assume the version is <5.5 until it can get it via the kernel_info request. The kernel
    /// protocol uses semantic versioning (SemVer).
    ///
    /// If >=5.5, the kernel supports the 'handshake' connection method, i.e. using a registration
    /// file.
    ///
    /// docs: <https://jupyter.org/enhancement-proposals/66-jupyter-handshaking/jupyter-handshaking.html#proposed-enhancement>
    pub kernel_protocol_version: Option<String>,
}

impl KernelSpec {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(BufReader::new(file))?)
    }
}

pub struct KernelInfo {
    pub path: PathBuf,
    pub spec: Option<KernelSpec>,
}

impl KernelInfo {
    pub fn get_all() -> Vec<Self> {
        discover_kernels()
            .iter()
            .map(|path| Self {
                path: path.to_path_buf(),
                spec: KernelSpec::from_file(path).ok(),
            })
            .collect()
    }
}

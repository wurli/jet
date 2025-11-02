/*
 * kernel_spec.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::fs::metadata;
use std::fs::read_dir;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use std::process::{Command, Stdio};

use crate::kernel::startup_method::StartupMethod;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum InterruptMode {
    Signal,
    Message,
}

/// docs: https://jupyter-client.readthedocs.io/en/latest/kernels.html#kernel-specs
/// spec: https://github.com/jupyter/enhancement-proposals/blob/master/105-kernelspec-spec/kernelspec.schema.json
#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub fn build_command(&self, connection_file_path: &String) -> Command {
        let mut args = self.argv.clone();

        for arg in args.iter_mut() {
            if *arg == "{connection_file}" {
                *arg = connection_file_path.clone()
            }
        }

        let mut command = Command::new(args.remove(0));
        command.args(args);
        // TODO: output from the process should probably be logged somehow rather than dropped
        // silently.
        command.stdout(Stdio::null());

        // TODO: should these be set? Kernels seem to send ANSI codes anyway so maybe not?
        // command.env("COLORTERM", "truecolor");
        // command.env("TERM", "xterm-256color");

        if let Some(env_vars) = &self.env {
            command.envs(env_vars);
        }

        command
    }

    pub fn get_connection_method(&self) -> StartupMethod {
        let mut use_registration_file = false;

        // Ark _does_ support connection through registration files, but doesn't (yet) advertise
        // this in the kernel spec
        if self.display_name == "Ark R Kernel" {
            use_registration_file = true;
        }

        if let Some(version) = &self.kernel_protocol_version {
            use_registration_file = version >= &String::from("5.5");
        }

        if use_registration_file {
            StartupMethod::RegistrationFile
        } else {
            StartupMethod::ConnectionFile
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(BufReader::new(file))?)
    }

    pub fn find_valid() -> HashMap<PathBuf, Self> {
        Self::find_all()
            .into_iter()
            .filter_map(|(path, contents)| match contents {
                Ok(spec) => Some((path, spec)),
                Err(_) => None,
            })
            .collect()
    }

    pub fn find_all() -> HashMap<PathBuf, anyhow::Result<Self>> {
        Self::discover_specs()
            .iter()
            .map(|path| (path.to_owned(), Self::from_file(path)))
            .collect()
    }

    /// Jupyter kernels should be identified through files in the following directories:
    ///
    /// System
    ///   `/usr/share/jupyter/kernels`
    ///   `/usr/local/share/jupyter/kernels`
    ///
    /// Env
    ///   `{sys.prefix}/share/jupyter/kernels`
    ///
    /// User
    ///
    ///   `~/.local/share/jupyter/kernels` (Linux)
    ///   `~/Library/Jupyter/kernels` (Mac)
    ///
    /// Other locations may also be searched if the `JUPYTER_PATH` environment variable is set.
    ///
    /// Windows exists too but I'm not supporting it yet.
    ///
    /// docs: <https://jupyter-client.readthedocs.io/en/latest/kernels.html#kernel-specs>
    pub fn discover_specs() -> Vec<PathBuf> {
        log::info!("Discovering installed kernels");
        let mut dirs: Vec<String> = Vec::new();

        // TODO: split this variable up on `:` and recurse
        if let Some(var) = std::env::var_os("JUPYTER_PATH") {
            dirs.push(format!("{}/kernels", var.to_string_lossy()));
        }

        if let Some(var) = std::env::var_os("HOME") {
            dirs.push(format!(
                "{}/.local/share/jupyter/kernels",
                var.to_string_lossy()
            ));
            dirs.push(format!("{}/Library/Jupyter/kernels", var.to_string_lossy()));
        }

        dirs.push("/usr/share/jupyter/kernels".to_string());
        dirs.push("/usr/local/share/jupyter/kernels".to_string());

        // TODO: Are there any other prefix env vars we should check?
        if let Some(var) = std::env::var_os("CONDA_PREFIX") {
            dirs.push(format!("{}/share/jupyter/kernels", var.to_string_lossy()));
        }

        dirs.into_iter()
            .filter(|dir| Self::path_exists(Path::new(dir)))
            .filter_map(|dir| read_dir(dir).ok())
            .flat_map(|entries| entries.flatten())
            .map(|entry| entry.path().join("kernel.json"))
            .filter(|path| path.exists())
            .collect()
    }

    fn path_exists(path: &Path) -> bool {
        metadata(path).is_ok()
    }
}

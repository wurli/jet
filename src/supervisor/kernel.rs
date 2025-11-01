use std::fmt::Display;
use std::process;
use std::sync::{Arc, Mutex};

use crate::connection::connection::JupyterChannels;
use crate::kernel::kernel_spec::KernelSpec;
use crate::kernel::startup_method::StartupMethod;
use crate::msg::wire::message_id::Id;
use crate::supervisor::broker::Broker;
use crate::supervisor::kernel_comm::KernelComm;
use crate::supervisor::kernel_info::KernelInfo;
use crate::supervisor::listeners::{listen_heartbeat, listen_iopub};

pub struct Kernel {
    pub id: Id,
    pub info: KernelInfo,
    pub process: process::Child,
    pub comm: KernelComm,
}

impl Display for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'{}", self.info.display_name, self.id)
    }
}

impl Kernel {
    pub fn start(spec_path: String, spec: KernelSpec) -> anyhow::Result<Self> {
        log::info!("Using kernel '{}'", spec.display_name);

        let kernel_id = Id::new();
        let cf_path = format!(
            ".connection_files/carpo_connection_file_{}.json",
            String::from(kernel_id.clone())
        );
        let kernel_cmd = spec.build_command(&cf_path);

        let (jupyter_channels, process) = match spec.get_connection_method() {
            StartupMethod::RegistrationFile => {
                JupyterChannels::init_with_registration_file(kernel_cmd, cf_path.into())?
            }
            StartupMethod::ConnectionFile => {
                JupyterChannels::init_with_connection_file(kernel_cmd, cf_path.into())?
            }
        };

        let kernel_comm = KernelComm {
            session: jupyter_channels.session.clone(),
            shell_channel: Mutex::new(jupyter_channels.shell),
            stdin_channel: Mutex::new(jupyter_channels.stdin),
            control_channel: Mutex::new(jupyter_channels.control),
            iopub_broker: Arc::new(Broker::new(format!("IOPub{}", kernel_id))),
            shell_broker: Arc::new(Broker::new(format!("Shell{}", kernel_id))),
            stdin_broker: Arc::new(Broker::new(format!("Stdin{}", kernel_id))),
            control_broker: Arc::new(Broker::new(format!("Control{}", kernel_id))),
        };

        listen_heartbeat(jupyter_channels.heartbeat);
        listen_iopub(
            jupyter_channels.iopub,
            Arc::clone(&kernel_comm.iopub_broker),
        );

        let kernel_info_reply = kernel_comm.subscribe();

        Ok(Self {
            id: kernel_id,
            comm: kernel_comm,
            process: process,
            info: KernelInfo {
                spec_path: spec_path,
                display_name: spec.display_name,
                banner: kernel_info_reply.banner,
                language: kernel_info_reply.language_info,
            },
        })
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        log::info!("Shutting down kernel '{}'", self);

        // self.comm.send_shutdown_request()?;

        match self.process.try_wait()? {
            Some(status) => {
                log::info!("Kernel '{}' exited with status {}", self, status);
            }
            None => {
                log::warn!("Kernel '{}' did not exit in time, killing", self);
                self.process.kill()?;
            }
        }

        Ok(())
    }
}

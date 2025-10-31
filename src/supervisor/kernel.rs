use std::fmt::Display;
use std::sync::{Arc, Mutex};

use crate::connection::connection::JupyterChannels;
use crate::kernel::kernel_spec::KernelSpec;
use crate::kernel::startup_method::ConnectionMethod;
use crate::msg::wire::message_id::Id;
use crate::supervisor::broker::Broker;
use crate::supervisor::kernel_comm::KernelComm;
use crate::supervisor::kernel_info::KernelInfo;
use crate::supervisor::listeners::{listen_heartbeat, listen_iopub};

pub struct Kernel {
    pub id: Id,
    pub info: KernelInfo,
    pub comm: KernelComm,
}

impl Display for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}'{}", self.info.display_name, self.id)
    }
}

impl Kernel {
    pub fn start(spec_path: String, spec: KernelSpec) -> Self {
        log::info!("Using kernel '{}'", spec.display_name);

        let kernel_id = Id::new();
        let connection_file_path = format!(
            ".connection_files/carpo_connection_file_{}.json",
            String::from(kernel_id.clone())
        );
        let kernel_cmd = spec.build_command(&connection_file_path);

        let jupyter_channels = match spec.get_connection_method() {
            ConnectionMethod::RegistrationFile => JupyterChannels::init_with_registration_file(
                kernel_cmd,
                connection_file_path.into(),
            ),
            ConnectionMethod::ConnectionFile => {
                JupyterChannels::init_with_connection_file(kernel_cmd, connection_file_path.into())
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

        Self {
            id: kernel_id,
            comm: kernel_comm,
            info: KernelInfo {
                spec_path: spec_path,
                display_name: spec.display_name,
                banner: kernel_info_reply.banner,
                language: kernel_info_reply.language_info,
            },
        }
    }
}



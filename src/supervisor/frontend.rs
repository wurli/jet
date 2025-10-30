use std::sync::{
    Arc, Mutex, OnceLock,
    mpsc::{Receiver, channel},
};

use assert_matches::assert_matches;

use crate::{
    connection::connection::Connection,
    kernel::{kernel_spec::KernelSpec, startup_method::ConnectionMethod},
    msg::wire::{
        jupyter_message::{Message, ProtocolMessage},
        kernel_info_reply::KernelInfoReply,
        kernel_info_request::KernelInfoRequest,
        status::ExecutionState,
    },
    supervisor::{
        broker::Broker,
        listeners::{listen_iopub, loop_heartbeat},
        manager::{KernelConnection, KernelId, KernelInfo, KernelManager, KernelState},
    },
};

pub static KERNEL_MANAGER: OnceLock<KernelManager> = OnceLock::new();

/// When you send a request on stdin, any replies which come back from the kernel will be routed
/// via these sockets. This allows you to handle replies _only_ related to the original request,
/// without worrying about dropping any unrelated messages.
pub struct RequestChannels {
    /// The ID of the original request message
    pub id: String,
    /// A receiver for replies to `id` on the iopub socket
    pub iopub: Receiver<Message>,
    /// A receiver for replies to `id` on the shell socket
    pub shell: Receiver<Message>,
    /// A receiver for replies to `id` on the stdin socket
    pub stdin: Receiver<Message>,
}

pub struct Frontend {}

impl Frontend {
    pub fn kernel_manager() -> &'static KernelManager {
        KERNEL_MANAGER.get_or_init(|| KernelManager::new())
    }

    pub fn start_kernel(spec: KernelSpec) -> anyhow::Result<KernelId> {
        log::info!("Using kernel '{}'", spec.display_name);

        let kernel_id = uuid::Uuid::new_v4().to_string();
        let connection_file_path = format!("carpo_connection_file_{}.json", kernel_id);
        let kernel_cmd = spec.build_command(&connection_file_path);

        let connection = match spec.get_connection_method() {
            ConnectionMethod::RegistrationFile => {
                Connection::init_with_registration_file(kernel_cmd, connection_file_path.into())
            }
            ConnectionMethod::ConnectionFile => {
                Connection::init_with_connection_file(kernel_cmd, connection_file_path.into())
            }
        };

        loop_heartbeat(connection.heartbeat);
        let iopub_broker = Arc::new(Broker::new(format!("IOPub-{}", kernel_id)));
        let shell_broker = Arc::new(Broker::new(format!("Shell-{}", kernel_id)));
        let stdin_broker = Arc::new(Broker::new(format!("Stdin-{}", kernel_id)));

        listen_iopub(connection.iopub, Arc::clone(&iopub_broker));

        let kernel_connection = KernelConnection {
            shell: Mutex::new(connection.shell),
            stdin: Mutex::new(connection.stdin),
        };

        let kernel_info = Self::subscribe(
            &kernel_connection,
            Arc::clone(&iopub_broker),
            Arc::clone(&shell_broker),
        );

        let kernel_state = KernelState {
            id: kernel_id.clone(),
            info: KernelInfo {
                spec,
                info: kernel_info.clone(),
            },
            connection: kernel_connection,
            iopub_broker,
            shell_broker,
            stdin_broker,
        };

        Self::kernel_manager().add_kernel(kernel_id.clone(), kernel_state)?;

        Ok(kernel_id)
    }

    fn subscribe(
        connection: &KernelConnection,
        iopub_broker: Arc<Broker>,
        shell_broker: Arc<Broker>,
    ) -> KernelInfoReply {
        let (welcome_tx, welcome_rx) = channel();

        iopub_broker.register_request(String::from("unparented"), welcome_tx);

        log::info!("Sending kernel info request for subscription");
        let request = Self::send_request_with_connection(
            connection,
            KernelInfoRequest {},
            Arc::clone(&iopub_broker),
            Arc::clone(&shell_broker),
            Arc::new(Broker::new(String::from("Stdin-temp"))),
        );

        Self::route_shell_reply_with_connection(connection, Arc::clone(&shell_broker), &request.id);
        let reply = request.shell.recv().unwrap();
        log::info!("Received reply on the shell");

        let kernel_info = match reply {
            Message::KernelInfoReply(reply) => reply.content,
            _ => panic!("Expected kernel_info_reply but got {:#?}", reply),
        };

        log::info!("Kernel info reply: {:#?}", kernel_info);

        if let Some(version) = &kernel_info.protocol_version {
            if version >= &String::from("5.4") {
                assert_matches!(welcome_rx.recv().unwrap(), Message::Welcome(data) => {
                    assert_eq!(data.content.subscription, String::from(""));
                    log::info!("Received the welcome message from the kernel");
                });
                assert_matches!(welcome_rx.recv().unwrap(), Message::Status(data) => {
                    assert_eq!(data.content.execution_state, ExecutionState::Starting);
                    log::info!("Received the starting message from the kernel");
                });
            }
        }

        iopub_broker.unregister_request(
            &String::from("unparented"),
            "all expected startup messages received",
        );

        log::info!("Subscription complete");
        kernel_info
    }

    pub fn get_kernel_info(kernel_id: &KernelId) -> anyhow::Result<KernelInfo> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            KernelInfo {
                spec: kernel.info.spec.clone(),
                info: kernel.info.info.clone(),
            }
        })
    }

    pub fn provide_stdin(kernel_id: &KernelId, value: String) -> anyhow::Result<()> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            kernel.connection.stdin.lock().unwrap().send_input_reply(value);
        })
    }

    pub fn send_request<T: ProtocolMessage>(
        kernel_id: &KernelId,
        message: T,
    ) -> anyhow::Result<RequestChannels> {
        let kernel = Self::kernel_manager()
            .get_kernel(kernel_id)
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", kernel_id))?;

        let request_id = kernel.connection.shell.lock().unwrap().send(message);
        let (iopub_tx, iopub_rx) = channel();
        let (stdin_tx, stdin_rx) = channel();
        let (shell_tx, shell_rx) = channel();

        kernel.iopub_broker.register_request(request_id.clone(), iopub_tx);
        kernel.stdin_broker.register_request(request_id.clone(), stdin_tx);
        kernel.shell_broker.register_request(request_id.clone(), shell_tx);

        Ok(RequestChannels {
            id: request_id,
            iopub: iopub_rx,
            shell: shell_rx,
            stdin: stdin_rx,
        })
    }

    fn send_request_with_connection<T: ProtocolMessage>(
        connection: &KernelConnection,
        message: T,
        iopub_broker: Arc<Broker>,
        shell_broker: Arc<Broker>,
        stdin_broker: Arc<Broker>,
    ) -> RequestChannels {
        let request_id = connection.shell.lock().unwrap().send(message);
        let (iopub_tx, iopub_rx) = channel();
        let (stdin_tx, stdin_rx) = channel();
        let (shell_tx, shell_rx) = channel();

        iopub_broker.register_request(request_id.clone(), iopub_tx);
        stdin_broker.register_request(request_id.clone(), stdin_tx);
        shell_broker.register_request(request_id.clone(), shell_tx);

        RequestChannels {
            id: request_id,
            iopub: iopub_rx,
            shell: shell_rx,
            stdin: stdin_rx,
        }
    }

    pub fn is_request_active(kernel_id: &KernelId, request_id: &String) -> anyhow::Result<bool> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            kernel.shell_broker.is_active(request_id)
        })
    }

    pub fn route_shell_reply(kernel_id: &KernelId, request_id: &String) -> anyhow::Result<()> {
        let kernel = Self::kernel_manager()
            .get_kernel(kernel_id)
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", kernel_id))?;

        Self::route_shell_reply_with_connection(&kernel.connection, kernel.shell_broker.clone(), request_id);
        Ok(())
    }

    fn route_shell_reply_with_connection(
        connection: &KernelConnection,
        shell_broker: Arc<Broker>,
        request_id: &String,
    ) {
        loop {
            let msg = connection.shell.lock().unwrap().recv();
            let is_reply = match msg.parent_id() {
                Some(parent_id) => *request_id == parent_id,
                None => *request_id == String::from("unparented"),
            };
            shell_broker.route(msg);
            if is_reply {
                break;
            }
        }
    }

    pub fn recv_all_incoming_shell(kernel_id: &KernelId) -> anyhow::Result<()> {
        let kernel = Self::kernel_manager()
            .get_kernel(kernel_id)
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", kernel_id))?;

        loop {
            match kernel.connection.shell.lock().unwrap().try_recv() {
                Ok(msg) => kernel.shell_broker.route(msg),
                Err(_) => break,
            }
        }
        Ok(())
    }

    pub fn recv_all_incoming_stdin(kernel_id: &KernelId) -> anyhow::Result<()> {
        let kernel = Self::kernel_manager()
            .get_kernel(kernel_id)
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", kernel_id))?;

        loop {
            match kernel.connection.stdin.lock().unwrap().try_recv() {
                Ok(msg) => kernel.stdin_broker.route(msg),
                Err(_) => break,
            }
        }
        Ok(())
    }

    pub fn get_stdin_broker(kernel_id: &KernelId) -> anyhow::Result<Arc<Broker>> {
        let kernel = Self::kernel_manager()
            .get_kernel(kernel_id)
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", kernel_id))?;
        Ok(Arc::clone(&kernel.stdin_broker))
    }
}

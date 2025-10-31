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
        shutdown_request::ShutdownRequest,
        status::ExecutionState,
    },
    supervisor::{
        broker::Broker,
        listeners::{listen_iopub, loop_heartbeat},
        manager::{InputChannels, KernelInfo, KernelManager, KernelState},
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

    pub fn start_kernel(
        spec_path: String,
        spec: KernelSpec,
    ) -> anyhow::Result<(String, KernelInfo)> {
        log::info!("Using kernel '{}'", spec.display_name);

        let kernel_id = uuid::Uuid::new_v4().to_string();
        let connection_file_path =
            format!(".connection_files/carpo_connection_file_{}.json", kernel_id);
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
        // This is only used in log messages
        let id_short = kernel_id.chars().take(8).collect::<String>();
        let iopub_broker = Arc::new(Broker::new(format!("IOPub-{}", id_short)));
        let shell_broker = Arc::new(Broker::new(format!("Shell-{}", id_short)));
        let stdin_broker = Arc::new(Broker::new(format!("Stdin-{}", id_short)));
        let control_broker = Arc::new(Broker::new(format!("Control-{}", id_short)));

        listen_iopub(connection.iopub, Arc::clone(&iopub_broker));

        let input_channels = InputChannels {
            shell: Mutex::new(connection.shell),
            stdin: Mutex::new(connection.stdin),
            control: Mutex::new(connection.control),
        };

        let kernel_info_reply = Self::subscribe(
            &input_channels,
            Arc::clone(&iopub_broker),
            Arc::clone(&shell_broker),
        );

        let info = KernelInfo {
            spec_path: spec_path,
            display_name: spec.display_name.clone(),
            banner: kernel_info_reply.banner.clone(),
            language: kernel_info_reply.language_info.clone(),
        };

        let kernel_state = KernelState {
            id: kernel_id.clone(),
            info: info.clone(),
            connection: input_channels,
            iopub_broker,
            shell_broker,
            stdin_broker,
            control_broker,
        };

        Self::kernel_manager().add_kernel(kernel_id.clone(), kernel_state)?;

        Ok((kernel_id, info))
    }

    fn subscribe(
        connection: &InputChannels,
        iopub_broker: Arc<Broker>,
        shell_broker: Arc<Broker>,
    ) -> KernelInfoReply {
        let (welcome_tx, welcome_rx) = channel();

        iopub_broker.register_request(&String::from("unparented"), welcome_tx);

        log::info!("Sending kernel info request for subscription");
        let request = Self::send_request_with_connection(
            connection,
            KernelInfoRequest {},
            Arc::clone(&iopub_broker),
            Arc::clone(&shell_broker),
            Arc::new(Broker::new(String::from("Stdin-temp"))),
        );

        Self::route_reply_impl(
            || connection.shell.lock().unwrap().recv(),
            Arc::clone(&shell_broker),
            &request.id,
        );
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

    pub fn request_shutdown(kernel_id: &String) -> anyhow::Result<Message> {
        Self::request_shutdown_impl(kernel_id, false)
    }

    pub fn request_restart(kernel_id: &String) -> anyhow::Result<Message> {
        Self::request_shutdown_impl(kernel_id, true)
    }

    /// This is a mess
    fn request_shutdown_impl(kernel_id: &String, restart: bool) -> anyhow::Result<Message> {
        Self::kernel_manager()
            .with_kernel(kernel_id, |kernel| {
                let request_id = {
                    let control = kernel.connection.control.lock().unwrap();
                    let request_id = control.send(ShutdownRequest { restart });
                    request_id
                };
                log::info!(
                    "Sent shutdown_request <{}>",
                    request_id.chars().take(8).collect::<String>()
                );
                let (control_tx, control_rx) = channel();
                let (iopub_tx, iopub_rx) = channel();
                let (stdin_tx, stdin_rx) = channel();

                kernel.iopub_broker.register_request(&request_id, iopub_tx);
                kernel.stdin_broker.register_request(&request_id, stdin_tx);
                kernel
                    .control_broker
                    .register_request(&request_id, control_tx);

                log::info!("Entering shutdown reply wait loop");
                loop {
                    match iopub_rx.try_recv() {
                        Ok(msg) => {
                            match msg {
                                Message::ShutdownReply(_) => {
                                    log::info!("Received shutdown_reply on iopub (non-standard)");
                                    return Ok(msg);
                                }
                                Message::Status(status_msg)
                                    if status_msg.content.execution_state == ExecutionState::Idle =>
                                {
                                    kernel
                                        .iopub_broker
                                        .unregister_request(&request_id, "idle status received");
                                    log::trace!("Received idle status");
                                }
                                _ => {
                                    log::trace!("Received unexpected message on iopub: {}", msg.describe())
                                }
                            }
                        }
                        Err(_) => {}
                    }

                    let _ = Self::recv_all_incoming_control(&kernel_id);

                    match stdin_rx.try_recv() {
                        Ok(msg @ Message::InputRequest(_)) => return Ok(msg),
                        Ok(msg) => log::warn!("Received unexpected reply {}", msg.describe()),
                        Err(_) => {}
                    }

                    let _ = Self::recv_all_incoming_control(&kernel_id);

                    match control_rx.try_recv() {
                        Ok(reply @ Message::ShutdownReply(_)) => {
                            log::info!("Received shutdown_reply on control (standard)");
                            kernel
                                .control_broker
                                .unregister_request(&request_id, "reply received");
                            return Ok(reply);
                        }
                        Ok(other) => {
                            log::warn!(
                                "Expected shutdown_reply but received unexpected message: {:#?}",
                                other
                            );
                            return Err(anyhow::anyhow!(
                                "Expected shutdown_reply but received unexpected message: {:#?}",
                                other
                            ));
                        }
                        Err(_) => {}
                    }

                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            })
            .unwrap()
    }

    pub fn get_kernel_info(kernel_id: &String) -> anyhow::Result<KernelInfo> {
        let kernel = Self::kernel_manager().get_kernel(kernel_id).unwrap();
        Ok(kernel.info.clone())
    }

    pub fn provide_stdin(kernel_id: &String, value: String) -> anyhow::Result<()> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            kernel
                .connection
                .stdin
                .lock()
                .unwrap()
                .send_input_reply(value);
        })
    }

    pub fn send_request<T: ProtocolMessage>(
        kernel_id: &String,
        message: T,
    ) -> anyhow::Result<RequestChannels> {
        let kernel = Self::kernel_manager()
            .get_kernel(kernel_id)
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", kernel_id))?;

        let request_id = kernel.connection.shell.lock().unwrap().send(message);
        let (iopub_tx, iopub_rx) = channel();
        let (stdin_tx, stdin_rx) = channel();
        let (shell_tx, shell_rx) = channel();

        kernel.iopub_broker.register_request(&request_id, iopub_tx);
        kernel.stdin_broker.register_request(&request_id, stdin_tx);
        kernel.shell_broker.register_request(&request_id, shell_tx);

        Ok(RequestChannels {
            id: request_id,
            iopub: iopub_rx,
            shell: shell_rx,
            stdin: stdin_rx,
        })
    }

    fn send_request_with_connection<T: ProtocolMessage>(
        connection: &InputChannels,
        message: T,
        iopub_broker: Arc<Broker>,
        shell_broker: Arc<Broker>,
        stdin_broker: Arc<Broker>,
    ) -> RequestChannels {
        let request_id = connection.shell.lock().unwrap().send(message);
        let (iopub_tx, iopub_rx) = channel();
        let (stdin_tx, stdin_rx) = channel();
        let (shell_tx, shell_rx) = channel();

        iopub_broker.register_request(&request_id, iopub_tx);
        stdin_broker.register_request(&request_id, stdin_tx);
        shell_broker.register_request(&request_id, shell_tx);

        RequestChannels {
            id: request_id,
            iopub: iopub_rx,
            shell: shell_rx,
            stdin: stdin_rx,
        }
    }

    pub fn is_request_active(kernel_id: &String, request_id: &String) -> anyhow::Result<bool> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            kernel.shell_broker.is_active(request_id)
                | kernel.iopub_broker.is_active(request_id)
                | kernel.stdin_broker.is_active(request_id)
        })
    }

    pub fn route_shell_reply(kernel_id: &String, request_id: &String) -> anyhow::Result<()> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            Self::route_reply_impl(
                || kernel.connection.shell.lock().unwrap().recv(),
                kernel.shell_broker.clone(),
                request_id,
            );
        })
    }

    pub fn route_control_reply(kernel_id: &String, request_id: &String) -> anyhow::Result<()> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            Self::route_reply_impl(
                || kernel.connection.control.lock().unwrap().recv(),
                kernel.control_broker.clone(),
                request_id,
            );
        })
    }

    /// NB, this is currently in its own function because we also use it in `subscribe()`, which
    /// needs to be run before the kernel is added to the manager.
    fn route_reply_impl(receiver: impl Fn() -> Message, broker: Arc<Broker>, request_id: &String) {
        loop {
            let msg = receiver();
            let is_reply = match msg.parent_id() {
                Some(parent_id) => *request_id == parent_id,
                None => *request_id == String::from("unparented"),
            };
            broker.route(msg);
            if is_reply {
                break;
            }
        }
    }

    pub fn recv_all_incoming_shell(kernel_id: &String) -> anyhow::Result<()> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            loop {
                match kernel.connection.shell.lock().unwrap().try_recv() {
                    Ok(msg) => kernel.shell_broker.route(msg),
                    Err(_) => break,
                }
            }
        })
    }

    pub fn recv_all_incoming_control(kernel_id: &String) -> anyhow::Result<()> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            loop {
                match kernel.connection.control.lock().unwrap().try_recv() {
                    Ok(msg) => kernel.control_broker.route(msg),
                    Err(_) => break,
                }
            }
        })
    }

    pub fn recv_all_incoming_stdin(kernel_id: &String) -> anyhow::Result<()> {
        Self::kernel_manager().with_kernel(kernel_id, |kernel| {
            loop {
                match kernel.connection.stdin.lock().unwrap().try_recv() {
                    Ok(msg) => kernel.stdin_broker.route(msg),
                    Err(_) => break,
                }
            }
        })
    }

    pub fn get_stdin_broker(kernel_id: &String) -> anyhow::Result<Arc<Broker>> {
        let kernel = Self::kernel_manager()
            .get_kernel(kernel_id)
            .ok_or_else(|| anyhow::anyhow!("Kernel '{}' not found", kernel_id))?;
        Ok(Arc::clone(&kernel.stdin_broker))
    }
}

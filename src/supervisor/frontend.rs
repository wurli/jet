use std::sync::{
    Arc, Mutex, OnceLock,
    mpsc::{Receiver, channel},
};

use assert_matches::assert_matches;

use crate::{
    connection::{connection::Connection, shell::Shell, stdin::Stdin},
    kernel::{
        kernel_spec::{KernelSpec, KernelSpecFull},
        startup_method::ConnectionMethod,
    },
    msg::wire::{
        jupyter_message::{Message, ProtocolMessage},
        kernel_info_reply::KernelInfoReply,
        kernel_info_request::KernelInfoRequest,
        status::ExecutionState,
    },
    supervisor::{
        broker::Broker,
        listeners::{listen_iopub, loop_heartbeat},
    },
};

// When we call lua functions we can only pass args from Lua. So, in order to access global state
// within these funcions, we need to use static values.
pub static KERNEL_INFO: OnceLock<(KernelSpec, KernelInfoReply)> = OnceLock::new();
pub static SHELL: OnceLock<Mutex<Shell>> = OnceLock::new();
pub static STDIN: OnceLock<Mutex<Stdin>> = OnceLock::new();
pub static IOPUB_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();
pub static SHELL_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();
pub static STDIN_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();

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
    pub fn start_kernel(spec_path: String) -> anyhow::Result<String> {
        if let Some(info) = KERNEL_INFO.get() {
            return Err(anyhow::anyhow!(
                "Kernel '{}' is already running",
                info.0.display_name
            ));
        };

        let matched_spec = KernelSpecFull::get_all()
            .into_iter()
            .filter(|x| x.path.to_string_lossy() == spec_path)
            .nth(0);

        let spec_full = matched_spec.expect(&format!("No kernel found with name '{}'", spec_path));
        let spec = spec_full.spec?;

        log::info!("Using kernel '{}'", spec.display_name);

        let connection_file_path = String::from("carpo_connection_file.json");
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
        let iopub_broker = Arc::new(Broker::new(String::from("IOPub")));
        let shell_broker = Arc::new(Broker::new(String::from("Shell")));
        let stdin_broker = Arc::new(Broker::new(String::from("Control")));

        // Start the iopub/shell processing threads
        listen_iopub(connection.iopub, Arc::clone(&iopub_broker));

        // Initialise global state
        SHELL.get_or_init(|| Mutex::new(connection.shell));
        STDIN.get_or_init(|| Mutex::new(connection.stdin));
        IOPUB_BROKER.get_or_init(|| iopub_broker);
        SHELL_BROKER.get_or_init(|| shell_broker);
        STDIN_BROKER.get_or_init(|| stdin_broker);

        // Subscribe, possibly blocking until startup messages have been received
        let kernel_info = Self::subscribe();

        KERNEL_INFO.get_or_init(|| (spec, kernel_info.clone()));

        Ok(kernel_info.banner)
    }

    fn subscribe() -> KernelInfoReply {
        // Not all kernels implement the XPUB socket which provides the welcome message which confirms
        // the connection is established. PEP 65 recommends dealing with this by:
        // 1. Sending a kernel info request
        // 2. Checking the protocol version in the reply
        // 3. Waiting for the welcome message if the protocol supports it
        //
        // Docs: https://jupyter.org/enhancement-proposals/65-jupyter-xpub/jupyter-xpub.html#impact-on-existing-implementations
        let (welcome_tx, welcome_rx) = channel();

        Self::iopub_broker().register_request(String::from("unparented"), welcome_tx);

        log::info!("Sending kernel info request for subscription");
        let request = Self::send_request(KernelInfoRequest {});

        // Block until we get the info reply
        Self::route_shell();
        let reply = request.shell.recv().unwrap();
        log::info!("Received reply on the shell");

        let kernel_info = match reply {
            Message::KernelInfoReply(reply) => reply.content,
            _ => panic!("Expected kernel_info_reply but got {:#?}", reply),
        };

        if let Some(version) = &kernel_info.protocol_version {
            log::info!("Kernel is using protocol version: {}", version);

            // Receive the Welcome message for kernels which support it
            // Unfortunately, although JEP 65 is accepted, I can't find the version of the jupyter protocol
            // in which it becomes effective. Ark _does_ support it and is 5.4, ipython doesn't and is 5.3.
            if version >= &String::from("5.4") {
                // Immediately block until we've received the IOPub welcome message from the XPUB server side
                // socket. This confirms that we've fully subscribed and avoids dropping any of the initial
                // IOPub messages that a server may send if we start to perform requests immediately (in
                // particular, busy/idle messages). https://github.com/posit-dev/ark/pull/577
                assert_matches!(welcome_rx.recv().unwrap(), Message::Welcome(data) => {
                    assert_eq!(data.content.subscription, String::from(""));
                    log::info!("Received the welcome message from the kernel");
                });
                // We also go ahead and handle the `ExecutionState::Starting` status that we know is coming
                // from the kernel right after the `Welcome` message.
                assert_matches!(welcome_rx.recv().unwrap(), Message::Status(data) => {
                    assert_eq!(data.content.execution_state, ExecutionState::Starting);
                    log::info!("Received the starting message from the kernel");
                });
            }
        }

        Self::iopub_broker().unregister_request(
            &String::from("unparented"),
            "all expected startup messages received",
        );

        log::info!("Subscription complete");
        kernel_info
    }

    pub fn lock_shell() -> std::sync::MutexGuard<'static, Shell> {
        SHELL.get_or_init(|| unreachable!()).lock().unwrap()
    }

    pub fn lock_stdin() -> std::sync::MutexGuard<'static, Stdin> {
        STDIN.get_or_init(|| unreachable!()).lock().unwrap()
    }

    pub fn iopub_broker() -> &'static Arc<Broker> {
        IOPUB_BROKER.get_or_init(|| unreachable!())
    }

    pub fn shell_broker() -> &'static Arc<Broker> {
        SHELL_BROKER.get_or_init(|| unreachable!())
    }

    pub fn stdin_broker() -> &'static Arc<Broker> {
        STDIN_BROKER.get_or_init(|| unreachable!())
    }

    pub fn provide_stdin(value: String) {
        Self::lock_stdin().send_input_reply(value);
    }

    /// Send a request on the shell, register it with the message brokers, and return channels that
    /// will receive any replies.
    pub fn send_request<T: ProtocolMessage>(message: T) -> RequestChannels {
        let request_id = Self::lock_shell().send(message);
        let (iopub_tx, iopub_rx) = channel();
        let (stdin_tx, stdin_rx) = channel();
        let (shell_tx, shell_rx) = channel();

        Self::iopub_broker().register_request(request_id.clone(), iopub_tx);
        Self::stdin_broker().register_request(request_id.clone(), stdin_tx);
        Self::shell_broker().register_request(request_id.clone(), shell_tx);

        return RequestChannels {
            id: request_id,
            iopub: iopub_rx,
            shell: shell_rx,
            stdin: stdin_rx,
        };
    }

    /// Check if a reply to `request_id` has been received yet
    pub fn is_request_active(request_id: &String) -> bool {
        Self::shell_broker().is_active(request_id)
    }

    /// Block until a message is received on the shell. This can be helpful, e.g. for long-running
    /// operations where the shell reply contains the information we want to surface to the user.
    ///
    /// TODO: if the reply isn't related to the original request, keep routing.
    pub fn route_shell() {
        let msg = Self::lock_shell().recv();
        Self::shell_broker().route(msg);
    }

    /// Drain the shell channel of all incoming messages and routes them
    pub fn recv_all_incoming_shell() {
        loop {
            match Self::lock_shell().try_recv() {
                Ok(msg) => Self::shell_broker().route(msg),
                // TODO: distinguish between no messages and error
                Err(_) => break,
            }
        }
    }

    /// Drain the stdin channel of all incoming messages and routes them
    pub fn recv_all_incoming_stdin() {
        loop {
            match Self::lock_stdin().try_recv() {
                Ok(msg) => Self::stdin_broker().route(msg),
                // TODO: distinguish between no messages and error
                Err(_) => break,
            }
        }
    }
}

use assert_matches::assert_matches;

use crate::{
    frontend::{frontend::Frontend, shell::Shell, stdin::Stdin},
    kernel::{
        kernel_spec::{KernelSpec, KernelSpecFull},
        startup_method::StartupMethod,
    },
    msg::wire::{
        complete_request::CompleteRequest,
        execute_request::ExecuteRequest,
        is_complete_request::IsCompleteRequest,
        jupyter_message::{Message, MessageType, ProtocolMessage},
        kernel_info_full_reply::KernelInfoReply,
        kernel_info_request::KernelInfoRequest,
        status::ExecutionState,
    },
    supervisor::{
        broker::Broker,
        listeners::{listen_iopub, loop_heartbeat},
    },
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        mpsc::{Receiver, channel},
    },
};

// When we call lua functions we can only pass args from Lua. So, in order
// to access global state within these funcions, we need to use static values.
pub static KERNEL_INFO: OnceLock<(KernelSpec, KernelInfoReply)> = OnceLock::new();
pub static SHELL: OnceLock<Mutex<Shell>> = OnceLock::new();
pub static STDIN: OnceLock<Mutex<Stdin>> = OnceLock::new();
pub static IOPUB_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();
pub static SHELL_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();
pub static STDIN_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();

/// When you send a request on stdin via `FrontEndMeta`, any replies which come
/// back from the kernel will be routed via these sockets. This allows you to
/// handle replies _only_ related to the original request, without worrying
/// about dropping any unrelated messages.
struct RequestChannels {
    /// The ID of the original request message
    id: String,
    /// A receiver for replies to `id` on the iopub socket
    iopub: Receiver<Message>,
    /// A receiver for replies to `id` on the shell socket
    shell: Receiver<Message>,
    /// A receiver for replies to `id` on the stdin socket
    stdin: Receiver<Message>,
}

struct FrontendMeta {}

impl FrontendMeta {
    pub fn get() -> Self {
        Self {}
    }

    pub fn lock_shell(&self) -> std::sync::MutexGuard<'_, Shell> {
        SHELL.get_or_init(|| unreachable!()).lock().unwrap()
    }

    pub fn lock_stdin(&self) -> std::sync::MutexGuard<'_, Stdin> {
        STDIN.get_or_init(|| unreachable!()).lock().unwrap()
    }

    pub fn iopub_broker(&self) -> &Arc<Broker> {
        IOPUB_BROKER.get_or_init(|| unreachable!())
    }

    pub fn shell_broker(&self) -> &Arc<Broker> {
        SHELL_BROKER.get_or_init(|| unreachable!())
    }

    pub fn stdin_broker(&self) -> &Arc<Broker> {
        STDIN_BROKER.get_or_init(|| unreachable!())
    }

    /// Send a request on the shell, register it with the message brokers, and return channels that
    /// will receive any replies.
    pub fn send_request<T: ProtocolMessage>(&self, message: T) -> RequestChannels {
        let request_id = self.lock_shell().send(message);
        let (iopub_tx, iopub_rx) = channel();
        let (stdin_tx, stdin_rx) = channel();
        let (shell_tx, shell_rx) = channel();

        self.iopub_broker()
            .register_request(request_id.clone(), iopub_tx);
        self.stdin_broker()
            .register_request(request_id.clone(), stdin_tx);
        self.shell_broker()
            .register_request(request_id.clone(), shell_tx);

        return RequestChannels {
            id: request_id,
            iopub: iopub_rx,
            shell: shell_rx,
            stdin: stdin_rx,
        };
    }

    /// Check if a reply to `request_id` has been received yet
    pub fn is_request_active(&self, request_id: &String) -> bool {
        self.shell_broker().is_active(request_id)
    }

    /// Block until a message is received on the shell. This can be helpful, e.g. for long-running
    /// operations where the shell reply contains the information we want to surface to the user.
    ///
    /// TODO: if the reply isn't related to the original request, keep routing.
    pub fn route_shell(&self) {
        let msg = self.lock_shell().recv();
        self.shell_broker().route(msg);
    }

    /// Drain the shell channel of all incoming messages and routes them
    pub fn recv_all_incoming_shell(&self) {
        loop {
            match self.lock_shell().try_recv() {
                Ok(msg) => self.shell_broker().route(msg),
                // TODO: distinguish between no messages and error
                Err(_) => break,
            }
        }
    }

    /// Drain the stdin channel of all incoming messages and routes them
    pub fn recv_all_incoming_stdin(&self) {
        loop {
            match self.lock_stdin().try_recv() {
                Ok(msg) => self.stdin_broker().route(msg),
                // TODO: distinguish between no messages and error
                Err(_) => break,
            }
        }
    }
}

pub fn discover_kernels() -> Vec<KernelSpecFull> {
    KernelSpecFull::get_all()
}

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

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Get the startup command
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let connection_file_path = String::from("carpo_connection_file.json");
    let kernel_cmd = spec.build_command(&connection_file_path);

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Start the frontend
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    let frontend = match spec.get_startup_method() {
        StartupMethod::RegistrationFile => {
            Frontend::start_with_registration_file(kernel_cmd, connection_file_path.into())
        }
        StartupMethod::ConnectionFile => {
            Frontend::start_with_connection_file(kernel_cmd, connection_file_path.into())
        }
    };

    loop_heartbeat(frontend.heartbeat);
    let iopub_broker = Arc::new(Broker::new(String::from("IOPub")));
    let shell_broker = Arc::new(Broker::new(String::from("Shell")));
    let stdin_broker = Arc::new(Broker::new(String::from("Control")));

    // Start the iopub/shell processing threads
    listen_iopub(frontend.iopub, Arc::clone(&iopub_broker));
    // listen_shell(frontend.shell, Arc::clone(&shell_broker));

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Initialise global state
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // log::info!("{}", kernel_info.banner);

    SHELL.get_or_init(|| Mutex::new(frontend.shell));
    STDIN.get_or_init(|| Mutex::new(frontend.stdin));
    IOPUB_BROKER.get_or_init(|| iopub_broker);
    SHELL_BROKER.get_or_init(|| shell_broker);
    STDIN_BROKER.get_or_init(|| stdin_broker);

    let kernel_info = subscribe();

    KERNEL_INFO.get_or_init(|| (spec, kernel_info.clone()));

    Ok(kernel_info.banner)
}

pub fn subscribe() -> KernelInfoReply {
    // Not all kernels implement the XPUB socket which provides the welcome message which confirms
    // the connection is established. PEP 65 recommends dealing with this by:
    // 1. Sending a kernel info request
    // 2. Checking the protocol version in the reply
    // 3. Waiting for the welcome message if the protocol supports it
    //
    // Docs: https://jupyter.org/enhancement-proposals/65-jupyter-xpub/jupyter-xpub.html#impact-on-existing-implementations
    let frontend = FrontendMeta::get();
    let (welcome_tx, welcome_rx) = channel();

    frontend
        .iopub_broker()
        .register_request(String::from("unparented"), welcome_tx);

    log::info!("Sending kernel info request for subscription");
    let request = frontend.send_request(KernelInfoRequest {});

    // Block until we get the info reply
    frontend.route_shell();
    let reply = request.shell.recv().unwrap();
    log::info!("Received reply on the shell");

    let kernel_info = match reply {
        Message::KernelInfoReply(reply) => reply.content,
        _ => panic!("Expected kernel_info_reply but got {:#?}", reply),
    };

    log::info!(
        "Kernel is using protocol version: {}",
        kernel_info.protocol_version
    );

    // Receive the Welcome message for kernels which support it
    // Unfortunately, although JEP 65 is accepted, I can't find the version of the jupyter protocol
    // in which it becomes effective. Ark _does_ support it and is 5.4, ipython doesn't and is 5.3.
    if kernel_info.protocol_version >= String::from("5.4") {
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

    frontend.iopub_broker().unregister_request(
        &String::from("unparented"),
        "all expected startup messages received",
    );

    log::info!("Subscription complete");
    kernel_info
}

pub fn execute_code(
    code: String,
    user_expressions: HashMap<String, String>,
) -> impl Fn() -> Option<Message> {
    log::trace!("Sending execute request `{}`", code);

    let frontend = FrontendMeta::get();

    // First let's try routing any incoming messages from the shell.
    frontend.recv_all_incoming_shell();

    let request = frontend.send_request(ExecuteRequest {
        code: code.clone(),
        silent: false,
        store_history: true,
        allow_stdin: true,
        stop_on_error: true,
        user_expressions: serde_json::to_value(user_expressions).unwrap(),
    });

    // We return a closure which can be repeatedly called as a function from Lua to get the
    // response from the kernel
    move || {
        loop {
            // --------------------------------------------------------------------------------------------------------
            // First we check if the request is still active. If not we return an empty result.
            // --------------------------------------------------------------------------------------------------------
            // If the request id is no longer registered as active then we've evidently already
            // received the reply and we can just return an empty result.
            if !frontend.is_request_active(&request.id) {
                return None;
            }

            // First let's try routing any incoming messages from the shell. In theory there should
            // be only one - the reply to this execute request. However there may be more, e.g.
            // late responses to previous requests.
            frontend.recv_all_incoming_shell();

            // --------------------------------------------------------------------------------------------------------
            // The request _is_ active, so let's see if there's anything on iopub
            // --------------------------------------------------------------------------------------------------------
            if let Ok(reply) = request.iopub.try_recv() {
                log::trace!("Receiving message from iopub: {}", reply.kind());
                match reply {
                    // These are the message types we want to surface in Lua
                    Message::ExecuteResult(_) | Message::ExecuteError(_) | Message::Stream(_) => {
                        return Some(reply);
                    }
                    // NB, it's possible that here we should also check if we have already received
                    // a busy status. However, I don't see any reason to confirm that the kernel is
                    // conforming to this pattern, so I'm not going to for now.
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                        return None;
                    }
                    // Here we can just add a sense check to ensure the code matches what we sent
                    Message::ExecuteInput(msg) => {
                        if msg.content.code != code {
                            log::warn!(
                                "Received {} with unexpected code: {}",
                                msg.content.kind(),
                                msg.content.code
                            );
                        };
                    }
                    // This is expected immediately after sending the execute request.
                    Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                    }
                    _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
                }
            }

            // --------------------------------------------------------------------------------------------------------
            // Since there was nothing on iopub, let's see if the kernel wants input from the user
            // --------------------------------------------------------------------------------------------------------
            frontend.recv_all_incoming_stdin();

            if let Ok(msg) = request.stdin.try_recv() {
                log::trace!("Received message from stdin: {}", msg.kind());
                if let Message::InputRequest(_) = msg {
                    return Some(msg);
                }
                log::warn!("Dropping unexpected stdin message {}", msg.kind());
            }

            // --------------------------------------------------------------------------------------------------------
            // Last of all we check if the request is complete. If not we loop again.
            // --------------------------------------------------------------------------------------------------------
            // Now let's check any shell replies related to this execute request. In theory there
            // should only be one, the final execute reply.
            while let Ok(msg) = request.shell.try_recv() {
                match msg {
                    Message::ExecuteReply(_) | Message::ExecuteReplyException(_) => {}
                    _ => log::warn!("Unexpected reply received on shell: {}", msg.kind()),
                }
                frontend
                    .stdin_broker()
                    .unregister_request(&request.id, "reply received");
                return None;
            }
            // If we didn't get a reply from the shell then let's try looping again
        }
    }
}

pub fn get_completions(code: String, cursor_pos: u32) -> anyhow::Result<Message> {
    log::trace!("Sending is completion request `{}`", code);

    let frontend = FrontendMeta::get();

    // First let's try routing any incoming messages from the shell.
    frontend.recv_all_incoming_shell();

    let request = frontend.send_request(CompleteRequest { code, cursor_pos });

    let mut out = Err(anyhow::anyhow!("Failed to obtain a reply from the kernel"));

    while let Ok(reply) = request.iopub.recv() {
        match reply {
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                log::trace!("Received iopub busy status for completion_request");
            }
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                log::trace!("Received iopub idle status for completion_request");
                break;
            }
            _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
        }
    }

    // First let's try routing any incoming messages from the shell. In theory there should
    // be only one - the reply to this execute request. However there may be more, e.g.
    // late responses to previous requests.
    frontend.route_shell();

    if let Ok(reply) = request.shell.recv() {
        match reply {
            Message::CompleteReply(_) => {
                log::trace!("Received completion_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.kind()),
        }
        frontend
            .stdin_broker()
            .unregister_request(&request.id, "reply received");
    } else {
        log::warn!("Failed to obtain completion_reply from the shell");
    }

    out
}

pub fn provide_stdin(value: String) {
    let stdin = STDIN.get_or_init(|| unreachable!()).lock().unwrap();
    stdin.send_input_reply(value);
}

pub fn is_complete(code: String) -> anyhow::Result<Message> {
    log::trace!("Sending is complete request `{}`", code);

    let frontend = FrontendMeta::get();

    frontend.recv_all_incoming_shell();

    let request = frontend.send_request(IsCompleteRequest { code: code.clone() });

    let mut out = Err(anyhow::anyhow!("Failed to obtain a reply from the kernel"));

    while let Ok(reply) = request.iopub.recv() {
        match reply {
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                log::trace!("Received iopub busy status for is_complete_request");
            }
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                log::trace!("Received iopub idle status for is_complete_request");
                break;
            }
            _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
        }
    }

    // First let's try routing any incoming messages from the shell. In theory there should
    // be only one - the reply to this execute request. However there may be more, e.g.
    // late responses to previous requests.
    frontend.route_shell();

    if let Ok(reply) = request.shell.recv() {
        match reply {
            Message::IsCompleteReply(_) => {
                log::trace!("Received is_complete_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.kind()),
        }
        frontend
            .stdin_broker()
            .unregister_request(&request.id, "reply received");
    } else {
        log::warn!("Failed to obtain is_complete_reply from the shell");
    }

    out
}

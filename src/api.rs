use assert_matches::assert_matches;

use crate::{
    frontend::{
        frontend::{ExecuteRequestOptions, Frontend},
        shell::Shell,
        stdin::Stdin,
    },
    kernel::{
        kernel_spec::{KernelSpec, KernelSpecFull},
        startup_method::StartupMethod,
    },
    msg::wire::{
        complete_request::CompleteRequest,
        jupyter_message::{Message, MessageType},
        kernel_info_full_reply::KernelInfoReply,
        kernel_info_request::KernelInfoRequest,
        status::ExecutionState,
    },
    supervisor::{
        broker::Broker,
        listeners::{listen_iopub, loop_heartbeat},
    },
};
use std::sync::{Arc, Mutex, OnceLock, mpsc::channel};

// When we call lua functions we can only pass args from Lua. So, in order
// to access global state within these funcions, we need to use static values.
pub static KERNEL_INFO: OnceLock<(KernelSpec, KernelInfoReply)> = OnceLock::new();
pub static SHELL: OnceLock<Mutex<Shell>> = OnceLock::new();
pub static STDIN: OnceLock<Mutex<Stdin>> = OnceLock::new();
pub static IOPUB_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();
pub static SHELL_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();
pub static STDIN_BROKER: OnceLock<Arc<Broker>> = OnceLock::new();

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

    let kernel_info = subscribe(&frontend.shell, Arc::clone(&iopub_broker));

    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // Initialise global state
    // ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    // log::info!("{}", kernel_info.banner);

    KERNEL_INFO.get_or_init(|| (spec, kernel_info.clone()));
    SHELL.get_or_init(|| Mutex::new(frontend.shell));
    STDIN.get_or_init(|| Mutex::new(frontend.stdin));
    IOPUB_BROKER.get_or_init(|| iopub_broker);
    SHELL_BROKER.get_or_init(|| shell_broker);
    STDIN_BROKER.get_or_init(|| stdin_broker);

    Ok(kernel_info.banner)
}

// TODO: write a framework for sending arbitrary requests and waiting for replies
// and use it here.
pub fn subscribe(shell: &Shell, iopub_broker: Arc<Broker>) -> KernelInfoReply {
    // Not all kernels implement the XPUB socket which provides the welcome message which confirms
    // the connection is established. PEP 65 recommends dealing with this by:
    // 1. Sending a kernel info request
    // 2. Checking the protocol version in the reply
    // 3. Waiting for the welcome message if the protocol supports it
    //
    // Docs: https://jupyter.org/enhancement-proposals/65-jupyter-xpub/jupyter-xpub.html#impact-on-existing-implementations
    let (request_tx, request_rx) = channel();
    let (unparent_tx, unparent_rx) = channel();

    iopub_broker.register_request(String::from("unparented"), unparent_tx);

    log::info!("[kernel info] Sending the startup kernel info request");
    let request_id = shell.send(KernelInfoRequest {});

    iopub_broker.register_request(request_id, request_tx);

    log::info!("[kernel info] Waiting for the busy status");
    assert_matches!(request_rx.recv().unwrap(), Message::Status(data) => {
        assert_eq!(data.content.execution_state, ExecutionState::Busy);
    });

    log::info!("[kernel info] Waiting for the reply on the shell");
    let reply = shell.recv();

    log::info!("[kernel info] Waiting for the reply on iopub");
    let kernel_info = match reply {
        Message::KernelInfoReply(reply) => reply.content,
        _ => panic!("Expected kernel_info_reply, but got {:#?}", reply),
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
        log::info!("[kernel info] Waiting for the welcome message");
        assert_matches!(unparent_rx.recv().unwrap(), Message::Welcome(data) => {
            assert_eq!(data.content.subscription, String::from(""));
        });
        // We also go ahead and handle the `ExecutionState::Starting` status that we know is coming
        // from the kernel right after the `Welcome` message.
        log::info!("[kernel info] Waiting for the starting message");
        assert_matches!(unparent_rx.recv().unwrap(), Message::Status(data) => {
            assert_eq!(data.content.execution_state, ExecutionState::Starting);
        });
    }

    iopub_broker.unregister_request(&String::from("unparented"));

    // Consume the Idle status
    log::info!("[kernel info] Waiting for the idle status");
    assert_matches!(request_rx.recv().unwrap(), Message::Status(data) => {
        assert_eq!(data.content.execution_state, ExecutionState::Idle);
    });

    log::info!("[kernel info] Subscription complete");
    kernel_info
}

pub fn execute_code(code: String) -> impl Fn() -> Option<Message> {
    log::trace!("Sending execute request `{}`", code);

    // Send the execute request and get its message ID
    let request_id = {
        let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
        shell.send_execute_request(&code, ExecuteRequestOptions::default())
    };

    let iopub_broker = IOPUB_BROKER.get_or_init(|| unreachable!());
    let shell_broker = SHELL_BROKER.get_or_init(|| unreachable!());
    let stdin_broker = STDIN_BROKER.get_or_init(|| unreachable!());

    let (iopub_tx, iopub_rx) = channel();
    let (shell_tx, shell_rx) = channel();
    let (stdin_tx, stdin_rx) = channel();

    shell_broker.register_request(request_id.clone(), shell_tx);
    iopub_broker.register_request(request_id.clone(), iopub_tx);
    stdin_broker.register_request(request_id.clone(), stdin_tx);

    // We return a closure which can be repeatedly called as a function from Lua to get the
    // response from the kernel
    move || {
        // ------------------------------------------------------------------------------------------------------------
        // First we check if the request is still active. If not we return an empty result.
        // ------------------------------------------------------------------------------------------------------------
        // If the request id is no longer registered as active then we've evidently already
        // received the reply and we can just return an empty result.
        if !shell_broker.is_active(&request_id) {
            return None;
        }

        // First let's try routing any incoming messages from the shell. In theory there should
        // be only one - the reply to this execute request. However there may be more, e.g.
        // late responses to previous requests.
        if let Ok(msg) = SHELL
            .get_or_init(|| unreachable!())
            .lock()
            .unwrap()
            .try_recv()
        {
            shell_broker.route(msg);
        };

        loop {
            // --------------------------------------------------------------------------------------------------------
            // The request _is_ active, so let's see if there's anything on iopub
            // --------------------------------------------------------------------------------------------------------
            if let Ok(reply) = iopub_rx.try_recv() {
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
                        iopub_broker.unregister_request(&request_id);
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
            while let Ok(msg) = STDIN
                .get_or_init(|| unreachable!())
                .lock()
                .unwrap()
                .try_recv()
            {
                stdin_broker.route(msg);
            }

            if let Ok(msg) = stdin_rx.try_recv() {
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
            while let Ok(msg) = shell_rx.try_recv() {
                match msg {
                    Message::ExecuteReply(_) | Message::ExecuteReplyException(_) => {}
                    _ => log::warn!("Unexpected reply received on shell: {}", msg.kind()),
                }
                shell_broker.unregister_request(&request_id);
                stdin_broker.unregister_request(&request_id);
                return None;
            }
            // If we couldn't get a reply from the shell then let's try looping again
        }
    }
}

pub fn get_completions(code: String, cursor_pos: u32) -> anyhow::Result<Message> {
    log::trace!("Sending is completion request `{}`", code);

    // Send the execute request and get its message ID
    let request_id = {
        let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
        shell.send(CompleteRequest { code, cursor_pos })
    };

    let iopub_broker = IOPUB_BROKER.get_or_init(|| unreachable!());
    let shell_broker = SHELL_BROKER.get_or_init(|| unreachable!());
    let stdin_broker = STDIN_BROKER.get_or_init(|| unreachable!());

    let (iopub_tx, iopub_rx) = channel();
    let (shell_tx, shell_rx) = channel();

    shell_broker.register_request(request_id.clone(), shell_tx);
    iopub_broker.register_request(request_id.clone(), iopub_tx);

    let mut out = Err(anyhow::anyhow!("Failed to obtain a reply from the kernel"));

    while let Ok(reply) = iopub_rx.recv() {
        match reply {
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                log::trace!("Received iopub busy status for completion_request");
            }
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                log::trace!("Received iopub idle status for completion_request");
                iopub_broker.unregister_request(&request_id);
                break;
            }
            _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
        }
    }

    // First let's try routing any incoming messages from the shell. In theory there should
    // be only one - the reply to this execute request. However there may be more, e.g.
    // late responses to previous requests.
    while let Ok(msg) = SHELL
        .get_or_init(|| unreachable!())
        .lock()
        .unwrap()
        .try_recv()
    {
        shell_broker.route(msg);
    }

    if let Ok(reply) = shell_rx.recv() {
        match reply {
            Message::CompleteReply(_) => {
                log::trace!("Received completion_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.kind()),
        }
        shell_broker.unregister_request(&request_id);
        stdin_broker.unregister_request(&request_id);
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

    // Send the execute request and get its message ID
    let request_id = {
        let shell = SHELL.get_or_init(|| unreachable!()).lock().unwrap();
        shell.send_is_complete_request(&code)
    };

    let iopub_broker = IOPUB_BROKER.get_or_init(|| unreachable!());
    let shell_broker = SHELL_BROKER.get_or_init(|| unreachable!());
    let stdin_broker = STDIN_BROKER.get_or_init(|| unreachable!());

    let (iopub_tx, iopub_rx) = channel();
    let (shell_tx, shell_rx) = channel();

    shell_broker.register_request(request_id.clone(), shell_tx);
    iopub_broker.register_request(request_id.clone(), iopub_tx);

    let mut out = Err(anyhow::anyhow!("Failed to obtain a reply from the kernel"));

    while let Ok(reply) = iopub_rx.recv() {
        match reply {
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Busy => {
                log::trace!("Received iopub busy status for is_complete_request");
            }
            Message::Status(msg) if msg.content.execution_state == ExecutionState::Idle => {
                log::trace!("Received iopub idle status for is_complete_request");
                iopub_broker.unregister_request(&request_id);
                break;
            }
            _ => log::warn!("Dropping unexpected iopub message {}", reply.kind()),
        }
    }

    // First let's try routing any incoming messages from the shell. In theory there should
    // be only one - the reply to this execute request. However there may be more, e.g.
    // late responses to previous requests.
    // TODO: we need to handle errors here. There was an error in a badly formatted message and it
    // caused the whole system to hang.
    while let Ok(msg) = SHELL
        .get_or_init(|| unreachable!())
        .lock()
        .unwrap()
        .try_recv()
    {
        shell_broker.route(msg);
    }

    if let Ok(reply) = shell_rx.recv() {
        match reply {
            Message::IsCompleteReply(_) => {
                log::trace!("Received is_complete_reply on the shell");
                out = Ok(reply);
            }
            _ => log::warn!("Unexpected reply received on shell: {}", reply.kind()),
        }
        shell_broker.unregister_request(&request_id);
        stdin_broker.unregister_request(&request_id);
    } else {
        log::warn!("Failed to obtain is_complete_reply from the shell");
    }

    out
}

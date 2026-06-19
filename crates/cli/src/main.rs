// jet — a runtimed-backed REPL for Jupyter kernels with kitty graphics.
//
// Spawns or attaches to a Jupyter kernel, drives a line-oriented REPL over
// the four ZMQ channels (shell, iopub, stdin, control), and renders PNG
// outputs inline using the kitty graphics protocol.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use jet_core::events::{Channel, InputRequest, IsCompleteReplyMsg, from_message};
use jet_core::jupyter_protocol::{
    ExecuteRequest, InputReply, IsCompleteReplyStatus, IsCompleteRequest, JupyterMessage,
    KernelInfoRequest,
};
use jet_core::kernel::Kernel;
use tokio::sync::mpsc::UnboundedReceiver;

mod cli;
mod render;

use cli::{Args, AttachArgs, Command, ConnectArgs};
use jet_core::logger::init_logger;
use render::{Renderer, SharedWriter, ansi, warn_if_passthrough_off};

enum WaitResult {
    Idle,
    Timeout,
    Closed,
    Input(InputRequest),
}

/// Run a stdin-byte watcher for the lifetime of `f`. rustyline keeps the
/// tty in raw mode (ISIG off) between readlines, so a real ^C during a
/// kernel request arrives as the byte 0x03 on stdin instead of a SIGINT.
async fn with_stdin_intr_watcher<Fut, T>(on_intr: impl Fn() + Send + Sync + 'static, f: Fut) -> T
where
    Fut: std::future::Future<Output = T>,
{
    use std::os::fd::{AsRawFd, OwnedFd};

    let (read_fd, write_fd): (OwnedFd, OwnedFd) = match nix_pipe() {
        Ok(p) => p,
        Err(_) => return f.await,
    };
    let read_fd_raw = read_fd.as_raw_fd();
    let on_intr = Arc::new(on_intr);
    let on_intr_thread = on_intr.clone();

    let handle = std::thread::spawn(move || {
        let stdin = libc::STDIN_FILENO;
        loop {
            let mut pfds = [
                libc::pollfd {
                    fd: stdin,
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: read_fd_raw,
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            let rc = unsafe { libc::poll(pfds.as_mut_ptr(), 2, -1) };
            if rc < 0 {
                continue;
            }
            if pfds[1].revents & libc::POLLIN != 0 {
                return;
            }
            if pfds[0].revents & libc::POLLIN != 0 {
                let mut b = [0u8; 1];
                let n = unsafe { libc::read(stdin, b.as_mut_ptr() as _, 1) };
                if n <= 0 {
                    continue;
                }
                if b[0] == 0x03 {
                    on_intr_thread();
                }
            }
        }
    });

    let result = f.await;

    {
        let buf = [0u8; 1];
        let _ = unsafe { libc::write(write_fd.as_raw_fd(), buf.as_ptr() as _, 1) };
    }
    let _ = handle.join();
    result
}

fn nix_pipe() -> std::io::Result<(std::os::fd::OwnedFd, std::os::fd::OwnedFd)> {
    use std::os::fd::FromRawFd;
    let mut fds = [0i32; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        Ok((
            std::os::fd::OwnedFd::from_raw_fd(fds[0]),
            std::os::fd::OwnedFd::from_raw_fd(fds[1]),
        ))
    }
}

/// Wait for the IsCompleteReply matching `target`. Returns `None` on
/// timeout or channel close — caller treats that as "execute anyway".
async fn wait_for_is_complete(
    rx: &mut UnboundedReceiver<IsCompleteReplyMsg>,
    target: &str,
    timeout: Duration,
) -> Option<IsCompleteReplyMsg> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        tokio::select! {
            _ = tokio::time::sleep(remaining) => return None,
            r = rx.recv() => match r {
                Some(reply) if reply.parent_id == target => return Some(reply),
                Some(_) => continue,
                None => return None,
            },
        }
    }
}

async fn wait_for_idle(
    idle_rx: &mut UnboundedReceiver<String>,
    input_rx: &mut UnboundedReceiver<InputRequest>,
    target: &str,
    timeout: Duration,
) -> WaitResult {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return WaitResult::Timeout;
        }
        tokio::select! {
            _ = tokio::time::sleep(remaining) => return WaitResult::Timeout,
            r = idle_rx.recv() => match r {
                Some(parent) if parent == target => return WaitResult::Idle,
                Some(_) => continue,
                None => return WaitResult::Closed,
            },
            r = input_rx.recv() => match r {
                Some(req) => return WaitResult::Input(req),
                None => return WaitResult::Closed,
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Connect(c) => run_connect(c).await,
        Command::Attach(c) => run_attach(c).await,
    }
}

/// Sender half plumbed back to the REPL loop. Dropping all of these tells
/// the reader task to stop. Kept inline because there are only two of
/// them and one type each.
struct Pipes {
    shell_tx: tokio::sync::mpsc::UnboundedSender<JupyterMessage>,
    stdin_tx: tokio::sync::mpsc::UnboundedSender<JupyterMessage>,
    /// Reader task signals "kernel exited" by notifying this.
    closed: Arc<tokio::sync::Notify>,
}

async fn run_connect(args: ConnectArgs) -> Result<()> {
    init_logger(args.global.log.as_deref());

    let spec = jet_core::kernel::KernelSpec::load(&args.kernelspec)?;
    log::info!(
        "spawning kernel (language={}, argv={:?})",
        spec.language,
        spec.argv,
    );

    let conn_path = match (args.connection_file.clone(), args.detach) {
        (Some(p), _) => Some(p),
        (None, false) => None,
        _ => unreachable!(),
    };

    let mut kernel = if let Some(ref path) = conn_path {
        Kernel::attach_or_spawn(&spec, path, args.session_name.as_deref()).await?
    } else {
        Kernel::spawn(&spec, conn_path.clone(), args.session_name.as_deref()).await?
    };

    let render_graphics = !args.no_graphics;
    drive_repl(&mut kernel, render_graphics, args.session_name).await?;

    if args.detach {
        kernel.detach();
        if let Some(p) = conn_path {
            eprintln!(
                "[jet] kernel detached. reattach with: jet attach {}",
                p.display()
            );
        }
    } else {
        let _ = kernel.shutdown().await;
    }
    Ok(())
}

async fn run_attach(args: AttachArgs) -> Result<()> {
    init_logger(args.global.log.as_deref());
    let mut kernel = Kernel::attach(&args.connection_file, args.session_name.as_deref()).await?;
    let render_graphics = !args.no_graphics;
    drive_repl(&mut kernel, render_graphics, args.session_name).await?;
    // Attach mode never kills the kernel; we just disconnect.
    Ok(())
}

/// Run the prompt → execute → render loop until the user exits or the
/// kernel dies. Borrows the kernel's channels rather than the `Kernel`
/// itself so the caller still owns the lifecycle (detach vs shutdown).
async fn drive_repl(
    kernel: &mut Kernel,
    render_graphics: bool,
    session_name: Option<String>,
) -> Result<()> {
    if render_graphics {
        warn_if_passthrough_off();
    }

    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<InputRequest>();
    let (is_complete_tx, mut is_complete_rx) =
        tokio::sync::mpsc::unbounded_channel::<IsCompleteReplyMsg>();
    let writer: SharedWriter = Arc::new(Mutex::new(std::io::stdout()));
    let renderer = Renderer::new(render_graphics, idle_tx, writer)
        .with_input_tx(input_tx)
        .with_is_complete_tx(is_complete_tx)
        .with_own_session_name(session_name.clone());

    // Channels carrying messages FROM the REPL TO the per-channel writer
    // tasks. We can't borrow &mut kernel.channels.shell across an await
    // and also use it from elsewhere, so the pattern is: take the shell /
    // stdin sockets out of the kernel for the duration of the REPL via
    // `std::mem::replace` would require Default impls we don't have.
    // Instead, run the four readers/writers as tasks owning their socket
    // halves, with mpsc back to here for sends.
    let (shell_send_tx, mut shell_send_rx) =
        tokio::sync::mpsc::unbounded_channel::<JupyterMessage>();
    let (stdin_send_tx, mut stdin_send_rx) =
        tokio::sync::mpsc::unbounded_channel::<JupyterMessage>();

    let closed = Arc::new(tokio::sync::Notify::new());
    let shutdown = Arc::new(tokio::sync::Notify::new());

    // Move the sockets out of the kernel so we can spawn tasks that own
    // them. The kernel keeps `control` for interrupt() / shutdown().
    let mut shell = kernel.channels.take_shell()?;
    let mut iopub = kernel.channels.take_iopub()?;
    let mut stdin_sock = kernel.channels.take_stdin()?;

    // Liveness watcher.
    //
    // - Spawn path (we own the child): waitpid(pid, WNOHANG) every
    //   500ms. Instant, kernel-level, gives an exit status.
    // - Attach path (no pid): heartbeat. ZMQ DEALER/SUB reads on a
    //   peer that exited don't error — they block forever — so a
    //   clean exit like R's `quit()` would otherwise hang jet
    //   indefinitely. The heartbeat REQ/REP echo is what JEP 13
    //   designed for this case.
    if kernel.child_pid().is_none() {
        let mut hb = kernel.channels.take_heartbeat()?;
        let closed_for_hb = closed.clone();
        let shutdown_for_hb = shutdown.clone();
        tokio::spawn(async move {
            let mut consecutive_timeouts = 0;
            loop {
                tokio::select! {
                    _ = shutdown_for_hb.notified() => return,
                    r = tokio::time::timeout(Duration::from_secs(5), hb.single_heartbeat()) => {
                        match r {
                            Ok(Ok(())) => {
                                consecutive_timeouts = 0;
                            }
                            Ok(Err(e)) => {
                                log::info!("heartbeat error: {e} — kernel gone");
                                closed_for_hb.notify_one();
                                return;
                            }
                            Err(_) => {
                                consecutive_timeouts += 1;
                                log::warn!("heartbeat timeout ({consecutive_timeouts})");
                                if consecutive_timeouts >= 2 {
                                    log::info!("heartbeat: kernel unresponsive, declaring dead");
                                    closed_for_hb.notify_one();
                                    return;
                                }
                            }
                        }
                    }
                }
                tokio::select! {
                    _ = shutdown_for_hb.notified() => return,
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                }
            }
        });
    }
    if let Some(pid) = kernel.child_pid() {
        let closed_for_watcher = closed.clone();
        let shutdown_for_watcher = shutdown.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_for_watcher.notified() => return,
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {
                        let mut status: libc::c_int = 0;
                        let r = unsafe {
                            libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG)
                        };
                        // r > 0  → child changed state and we reaped it
                        //          (we won't, since our wait might also race
                        //          tokio's signal handler — that's fine, the
                        //          kernel exit is the signal we care about).
                        // r == 0 → still running.
                        // r < 0  → ECHILD: tokio already reaped, child gone.
                        if r != 0 {
                            log::info!("kernel pid {pid} exited (waitpid -> {r})");
                            closed_for_watcher.notify_one();
                            return;
                        }
                    }
                }
            }
        });
    }

    // Shell driver: select between (a) outbound sends from the REPL and
    // (b) inbound replies. Replies become Events fed to the renderer.
    let renderer_shell = renderer.clone();
    let closed_shell = closed.clone();
    let shutdown_shell = shutdown.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_shell.notified() => return,
                send = shell_send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = shell.send(msg).await {
                            log::error!("shell send: {e}");
                            closed_shell.notify_one();
                            return;
                        }
                    }
                    None => return,
                },
                read = shell.read() => match read {
                    Ok(msg) => {
                        if let Err(e) = renderer_shell.handle_event(from_message(Channel::Shell, &msg)) {
                            log::warn!("renderer (shell): {e}");
                        }
                    }
                    Err(e) => {
                        log::warn!("shell recv: {e}");
                        closed_shell.notify_one();
                        return;
                    }
                },
            }
        }
    });

    // IOPub reader: read-only, pump everything to the renderer. Stop on
    // socket error (kernel went away).
    let renderer_iopub = renderer.clone();
    let closed_iopub = closed.clone();
    let shutdown_iopub = shutdown.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_iopub.notified() => return,
                read = iopub.read() => match read {
                    Ok(msg) => {
                        if let Err(e) = renderer_iopub.handle_event(from_message(Channel::IoPub, &msg)) {
                            log::warn!("renderer (iopub): {e}");
                        }
                    }
                    Err(e) => {
                        log::warn!("iopub recv: {e}");
                        closed_iopub.notify_one();
                        return;
                    }
                },
            }
        }
    });

    // Stdin driver: input_request comes IN from the kernel; input_reply
    // goes OUT.
    let renderer_stdin = renderer.clone();
    let closed_stdin = closed.clone();
    let shutdown_stdin = shutdown.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_stdin.notified() => return,
                send = stdin_send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = stdin_sock.send(msg).await {
                            log::error!("stdin send: {e}");
                            closed_stdin.notify_one();
                            return;
                        }
                    }
                    None => return,
                },
                read = stdin_sock.read() => match read {
                    Ok(msg) => {
                        if let Err(e) = renderer_stdin.handle_event(from_message(Channel::Stdin, &msg)) {
                            log::warn!("renderer (stdin): {e}");
                        }
                    }
                    Err(e) => {
                        log::warn!("stdin recv: {e}");
                        closed_stdin.notify_one();
                        return;
                    }
                },
            }
        }
    });

    let pipes = Pipes {
        shell_tx: shell_send_tx,
        stdin_tx: stdin_send_tx,
        closed: closed.clone(),
    };

    // Banner: kernel_info_request, wait for its idle. Skipped on attach
    // — the kernel's already past startup and the second client's banner
    // round-trip just produces a duplicate idle status with nothing
    // useful to render.
    if !kernel.is_attached() {
        let info_req: JupyterMessage = KernelInfoRequest {}.into();
        let info_id = info_req.header.msg_id.clone();
        let _ = pipes.shell_tx.send(info_req);
        match wait_for_idle(
            &mut idle_rx,
            &mut input_rx,
            &info_id,
            Duration::from_secs(10),
        )
        .await
        {
            WaitResult::Idle | WaitResult::Timeout => {}
            WaitResult::Closed => {
                shutdown.notify_waiters();
                return Ok(());
            }
            WaitResult::Input(_) => {}
        }
    }

    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    let mut rl = Some(rustyline::DefaultEditor::new()?);
    loop {
        // Accumulate lines until the kernel says the buffer is a
        // complete unit of code. The first prompt is `> `; continuation
        // prompts are `+ `, with the kernel-suggested indent
        // pre-filled into the editor.
        let mut buffer = String::new();
        // Continuation prompt for the next line, set from the kernel's
        // IsCompleteReply.indent. Per the Jupyter spec, that field is the
        // full continuation prompt (any leading marker plus whitespace),
        // so we render it verbatim instead of prepending our own.
        let mut next_indent: Option<String> = None;
        let code = 'accumulate: loop {
            let mut prompt_rl = rl.take().expect("editor present at top of loop");
            let prompt = match &next_indent {
                None => "> ".to_string(),
                Some(s) => s.clone(),
            };
            let read = tokio::task::spawn_blocking(move || {
                let result = prompt_rl.readline(&prompt);
                (prompt_rl, result)
            });
            let line = tokio::select! {
                _ = pipes.closed.notified() => {
                    eprintln!("{}", ansi::red("[jet] kernel exited"));
                    shutdown.notify_waiters();
                    std::process::exit(0);
                }
                joined = read => {
                    let (returned_rl, result) = joined?;
                    rl = Some(returned_rl);
                    match result {
                        Ok(l) => l,
                        Err(rustyline::error::ReadlineError::Eof) => {
                            if buffer.is_empty() {
                                shutdown.notify_waiters();
                                return Ok(());
                            }
                            // ^D inside an in-progress block: discard.
                            break 'accumulate None;
                        }
                        Err(rustyline::error::ReadlineError::Interrupted) => {
                            // ^C abandons the in-progress block.
                            break 'accumulate None;
                        }
                        Err(e) => {
                            eprintln!("[jet] readline: {e}");
                            return Ok(());
                        }
                    }
                }
            };
            if buffer.is_empty() && line.trim().is_empty() {
                continue;
            }
            if !buffer.is_empty() {
                buffer.push('\n');
            }
            buffer.push_str(&line);

            // Ask the kernel whether what we have so far is a complete
            // unit. Treat Complete / Invalid / Unknown as "go ahead and
            // execute" — for Invalid the kernel will surface the syntax
            // error, and Unknown means the kernel can't tell, in which
            // case the spec recommends executing.
            let req: JupyterMessage = IsCompleteRequest {
                code: buffer.clone(),
            }
            .into();
            let req_id = req.header.msg_id.clone();
            let _ = pipes.shell_tx.send(req);
            let reply =
                wait_for_is_complete(&mut is_complete_rx, &req_id, Duration::from_secs(5)).await;
            match reply.map(|r| (r.status, r.indent)) {
                Some((IsCompleteReplyStatus::Incomplete, indent)) => {
                    let mut p = if indent.is_empty() {
                        "+".to_string()
                    } else {
                        indent
                    };
                    if !p.ends_with(' ') {
                        p.push(' ');
                    }
                    next_indent = Some(p);
                    continue;
                }
                _ => break 'accumulate Some(buffer),
            }
        };

        let Some(code) = code else {
            continue;
        };
        let _ = rl
            .as_mut()
            .expect("editor returned from blocking task")
            .add_history_entry(&code);

        let req: JupyterMessage = ExecuteRequest {
            code,
            silent: false,
            store_history: true,
            user_expressions: None,
            allow_stdin: true,
            stop_on_error: true,
        }
        .into();
        let msg_id = req.header.msg_id.clone();
        let _ = pipes.shell_tx.send(req);

        let outcome = loop {
            let (intr_tx, mut intr_rx) = tokio::sync::mpsc::unbounded_channel::<()>();
            let r = with_stdin_intr_watcher(
                move || {
                    let _ = intr_tx.send(());
                },
                async {
                    loop {
                        tokio::select! {
                            r = wait_for_idle(&mut idle_rx, &mut input_rx, &msg_id, Duration::from_secs(300)) => return r,
                            _ = pipes.closed.notified() => return WaitResult::Closed,
                            _ = intr_rx.recv() => {
                                if let Err(e) = kernel.interrupt().await {
                                    eprintln!("{}", ansi::red(&format!("[jet] interrupt failed: {e}")));
                                }
                            }
                            _ = sigint.recv() => {
                                if let Err(e) = kernel.interrupt().await {
                                    eprintln!("{}", ansi::red(&format!("[jet] interrupt failed: {e}")));
                                }
                            }
                        }
                    }
                },
            )
            .await;

            match r {
                WaitResult::Input(req) => {
                    let prompt = req.prompt.clone();
                    let mut prompt_rl = rl.take().expect("editor present at input prompt");
                    let read = tokio::task::spawn_blocking(move || {
                        let line = if req.password {
                            prompt_rl.readline_with_initial(&prompt, ("", ""))
                        } else {
                            prompt_rl.readline(&prompt)
                        };
                        (prompt_rl, line)
                    });
                    let (returned_rl, line_result) = read.await?;
                    rl = Some(returned_rl);
                    let value = match line_result {
                        Ok(s) => s,
                        Err(rustyline::error::ReadlineError::Eof)
                        | Err(rustyline::error::ReadlineError::Interrupted) => String::new(),
                        Err(e) => {
                            eprintln!("[jet] readline (input_request): {e}");
                            String::new()
                        }
                    };
                    let reply: JupyterMessage = InputReply {
                        value,
                        status: Default::default(),
                        error: None,
                    }
                    .into();
                    let _ = pipes.stdin_tx.send(reply);
                    continue;
                }
                other => break other,
            }
        };

        match outcome {
            WaitResult::Idle => {}
            WaitResult::Input(_) => unreachable!("handled above"),
            WaitResult::Timeout => {
                log::warn!("timeout waiting for kernel idle (msg_id={msg_id})");
                eprintln!("{}", ansi::yellow("[jet] timeout waiting for kernel"));
            }
            WaitResult::Closed => {
                shutdown.notify_waiters();
                std::process::exit(0);
            }
        }
    }
}

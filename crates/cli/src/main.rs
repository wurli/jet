// jet — a kallichore-backed REPL with kitty graphics.
//
// Spawns `kcserver` with a connection file, opens a session for a Jupyter
// kernel given on the command line, connects to the per-session WebSocket,
// and drives a line-oriented REPL. PNG outputs from the kernel are rendered
// inline with the kitty graphics protocol.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use futures_util::StreamExt;
use rand::Rng;
use serde_json::json;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::Message;

mod cli;
mod render;

use cli::{Args, Command, ConnectArgs, ListSessionsArgs, StopArgs};
use jet_core::events::{Event, InputRequest, parse_event};
use jet_core::jupyter;
use jet_core::kallichore::{Channel, Client};
use render::{Renderer, SharedWriter, warn_if_passthrough_off};

enum WaitResult {
    Idle,
    Timeout,
    Closed,
    /// Kernel asked us for stdin input. The REPL must prompt the user
    /// (after dropping the stdin interrupt watcher), send `input_reply`
    /// on the `stdin` channel, then resume waiting for idle.
    Input(InputRequest),
}

/// Run a stdin-byte watcher for the lifetime of `f`. rustyline keeps the
/// tty in raw mode (ISIG off) between readlines, so a real ^C during a
/// kernel request arrives as the byte 0x03 on stdin instead of a SIGINT —
/// nobody would observe it otherwise.
///
/// We spawn a dedicated thread for the call, join it when `f` finishes, and
/// only run the watcher while we know rustyline is NOT reading from stdin
/// (so we never race rustyline for input bytes). The thread is woken
/// promptly via a self-pipe when `f` returns.
async fn with_stdin_intr_watcher<Fut, T>(on_intr: impl Fn() + Send + Sync + 'static, f: Fut) -> T
where
    Fut: std::future::Future<Output = T>,
{
    use std::os::fd::{AsRawFd, OwnedFd};

    // Self-pipe so we can interrupt the watcher's poll() when f finishes.
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
            // Wake-pipe readable means f has finished; exit before reading
            // stdin, otherwise we'd steal a byte from the next readline.
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
                // Other bytes are dropped — typing while the kernel is busy
                // is discarded, matching how most REPLs behave.
            }
        }
    });

    let result = f.await;

    // Wake the watcher thread.
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

fn init_logger(log_file: Option<&std::path::Path>) {
    // File logging because the REPL owns stdout/stderr — log lines on the
    // terminal would corrupt prompts and inline graphics. Controlled with
    // RUST_LOG (e.g. `RUST_LOG=jet=trace`).
    let Some(path) = log_file else { return };
    let Ok(file) = std::fs::File::create(path) else {
        return;
    };
    let _ = env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .try_init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Connect(c) => run_connect(c).await,
        Command::ListSessions(c) => run_list_sessions(c).await,
        Command::Stop(c) => run_stop(c).await,
    }
}

async fn run_stop(args: StopArgs) -> Result<()> {
    let client = match &args.kc.kcfile {
        Some(path) => Client::connect(path).await?,
        None => anyhow::bail!("--kcfile is required to identify the kcserver"),
    };

    if let Some(session_id) = &args.session {
        stop_session(&client, session_id).await?;
        println!("stopped session {session_id}");
        return Ok(());
    }

    let sessions = client.list_sessions().await?;
    for s in &sessions {
        if let Err(e) = stop_session(&client, &s.session_id).await {
            eprintln!("warning: stopping {} failed: {e}", s.session_id);
        }
    }
    client.shutdown_server().await?;
    println!("stopped {} session(s) and kcserver", sessions.len());
    Ok(())
}

/// Stop a single session: graceful `shutdown_request` first, escalating to
/// `kill_session` if the kernel doesn't exit within a few seconds. Once the
/// session is in `Exited`, delete it from the server.
async fn stop_session(client: &Client, session_id: &str) -> Result<()> {
    use jet_core::kallichore::api::types::Status;
    let alive = |s: Status| {
        matches!(
            s,
            Status::Starting | Status::Ready | Status::Idle | Status::Busy
        )
    };

    let sessions = client.list_sessions().await?;
    let s = sessions
        .iter()
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| anyhow::anyhow!("no session with id {session_id}"))?;

    if alive(s.status) {
        match request_graceful_shutdown(client, session_id).await {
            Ok(()) => {}
            Err(e) => {
                log::warn!("graceful shutdown of {session_id} failed: {e}; falling back to kill");
                client.kill_session(session_id).await?;
            }
        }

        // Poll until the kernel reports `Exited`. Escalate to kill if it
        // ignores the shutdown request.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if Instant::now() >= deadline {
                log::warn!("{session_id} did not exit after shutdown_request; killing");
                client.kill_session(session_id).await?;
                break;
            }
            let cur = client.list_sessions().await?;
            match cur.iter().find(|s| s.session_id == session_id) {
                None => break,
                Some(s) if !alive(s.status) => break,
                _ => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
    }

    client.delete_session(session_id).await?;
    Ok(())
}

/// Open the session's channels websocket and send a Jupyter `shutdown_request`
/// on the control channel. Returns once the request is sent — the caller
/// polls the session status to confirm exit.
async fn request_graceful_shutdown(client: &Client, session_id: &str) -> Result<()> {
    use futures_util::SinkExt;
    let ws = client.open_channels(session_id).await?;
    let (mut sink, _stream) = ws.split();
    let msg_id = jupyter::new_msg_id();
    let req = jupyter::message(
        "control",
        &msg_id,
        "shutdown_request",
        json!({ "restart": false }),
    );
    sink.send(Message::Text(req.to_string().into())).await?;
    // Send a proper close frame so kallichore doesn't log a "connection
    // reset without close handshake" error when our ws drops.
    let _ = sink.send(Message::Close(None)).await;
    let _ = sink.close().await;
    Ok(())
}

async fn run_list_sessions(args: ListSessionsArgs) -> Result<()> {
    let client = match &args.kc.kcfile {
        Some(path) => Client::connect(path).await?,
        None => {
            anyhow::bail!("--kcfile is required to identify the kcserver");
        }
    };
    let sessions = client.list_sessions().await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&sessions)?);
        return Ok(());
    }
    if sessions.is_empty() {
        println!("(no active sessions)");
        return Ok(());
    }
    for s in sessions {
        println!(
            "{:<12}  {}  {}  {:<8}  pid={:<6}  pwd={}",
            s.display_name.to_string(),
            s.started.format("%Y-%m-%d %H:%M:%S"),
            s.session_id,
            s.status.to_string(),
            s.process_id
                .map(|p| p.to_string())
                .unwrap_or_else(|| "-".into()),
            s.working_directory,
        );
    }
    Ok(())
}

async fn run_connect(args: ConnectArgs) -> Result<()> {
    init_logger(args.log.as_deref());

    let mut client = match &args.kc.kcfile {
        Some(path) => Client::connect_or_spawn(&args.kc.kcserver, path, args.persist).await?,
        None => Client::spawn(&args.kc.kcserver, None, args.persist).await?,
    };
    if args.persist {
        client.detach_server();
    }

    let session_id = format!("jet-{:x}", rand::thread_rng().gen::<u64>());
    let spec = jet_core::kernel::KernelSpec::load(&args.kernelspec)?;
    log::info!(
        "Creating session {session_id} (language={}, argv={:?})",
        spec.language,
        spec.argv,
    );
    let display_name = spec.display_name.as_deref().unwrap_or("jet");
    client
        .create_session(
            &session_id,
            display_name,
            &spec.language,
            &spec.argv,
            &spec.env,
            spec.interrupt_mode,
        )
        .await?;

    // Open the channels websocket BEFORE start so we don't miss startup messages.
    let ws = client.open_channels(&session_id).await?;
    let (ws_sink, ws_stream) = ws.split();
    let mut channel = Channel::new(ws_sink);

    log::info!("Starting session {session_id}");
    client.start_session(&session_id).await?;

    let render_graphics = !args.no_graphics;
    if render_graphics {
        warn_if_passthrough_off();
    }

    // Channel from the WS reader to the REPL: signals "kernel is idle for msg X".
    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    // Channel from the WS reader to the REPL: kernel asked for stdin input.
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<InputRequest>();
    let writer: SharedWriter = Arc::new(Mutex::new(std::io::stdout()));
    let renderer = Renderer::new(render_graphics, idle_tx, writer).with_input_tx(input_tx);

    // shutdown: REPL → reader. Set on REPL exit so the WS reader task can
    // distinguish a clean shutdown (kcserver killed by Drop → reset without
    // close handshake) from a real mid-session error worth surfacing.
    // closed: reader → REPL. Set when the websocket ends so the REPL can
    // exit immediately rather than wait for the user to press a key.
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let closed = Arc::new(tokio::sync::Notify::new());
    let reader_shutdown = shutdown.clone();
    let reader_closed = closed.clone();

    tokio::spawn(async move {
        let mut stream = ws_stream;
        loop {
            tokio::select! {
                _ = reader_shutdown.notified() => return,
                msg = stream.next() => match msg {
                    Some(Ok(Message::Text(t))) => {
                        log::trace!("ws frame: {t}");
                        let event = match parse_event(&t) {
                            Ok(e) => e,
                            Err(e) => {
                                log::warn!("parse_event failed: {e}");
                                eprintln!("\x1b[31m[jet] {e}\x1b[0m");
                                continue;
                            }
                        };
                        let exited = matches!(event, Event::KernelExited);
                        if exited {
                            log::info!("kernel exited");
                        }
                        if let Err(e) = renderer.handle_event(event) {
                            log::warn!("renderer error: {e}");
                            eprintln!("\x1b[31m[jet] {e}\x1b[0m");
                        }
                        if exited {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        log::info!("websocket closed");
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        log::error!("ws error: {e}");
                        eprintln!("\x1b[31m[jet] ws error: {e}\x1b[0m");
                        break;
                    }
                },
            }
        }
        reader_closed.notify_one();
    });

    // Ask the kernel for its banner/version info, and wait for the reply to be
    // fully rendered (kernel goes idle for this request) before drawing the
    // first prompt — otherwise rustyline races the async banner write.
    let info_id = jupyter::new_msg_id();
    let info_req = jupyter::message("shell", &info_id, "kernel_info_request", json!({}));
    channel.send(&info_req).await?;
    match wait_for_idle(&mut idle_rx, &mut input_rx, &info_id, Duration::from_secs(10)).await {
        WaitResult::Idle | WaitResult::Timeout => {}
        WaitResult::Closed => return Ok(()),
        // kernel_info_request never triggers input — drop any spurious request.
        WaitResult::Input(_) => {}
    }

    // We watch for ^C in two ways:
    //  - SIGINT from the OS (e.g. another process kill -INT). Registered
    //    eagerly so we don't race the user's first ^C against tokio's lazy
    //    registration in ctrl_c().
    //  - The literal `\x03` byte on stdin. rustyline keeps the tty in raw
    //    mode (ISIG off) between readlines, so the kernel doesn't translate
    //    ^C to SIGINT — it arrives as a byte on stdin instead. We read
    //    stdin ourselves while the kernel is busy and treat that byte as
    //    a ^C.
    // Eagerly register the SIGINT handler so we don't race the user's first
    // ^C against tokio's lazy registration in ctrl_c().
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    let mut rl = Some(rustyline::DefaultEditor::new()?);
    loop {
        // Run rustyline on a blocking thread so we can race it against
        // `closed`. If the websocket dies (kernel quit, server crash) the
        // REPL exits immediately instead of waiting for the user to press
        // a key. The blocking task can't be cancelled — we just leave it
        // pinned on stdin and drop it when the process exits.
        let mut prompt_rl = rl.take().expect("editor present at top of loop");
        let read = tokio::task::spawn_blocking(move || {
            let result = prompt_rl.readline("> ");
            (prompt_rl, result)
        });
        let line = tokio::select! {
            _ = closed.notified() => {
                eprintln!("\x1b[31m[jet] kernel exited\x1b[0m");
                shutdown.notify_one();
                channel.close().await;
                // The blocking readline task is parked on stdin.read(); the
                // tokio runtime's Drop would wait for that thread to finish
                // (it never will, until the user presses a key). Run drops
                // we care about explicitly, then exit the process.
                drop(client);
                std::process::exit(0);
            }
            joined = read => {
                let (returned_rl, result) = joined?;
                rl = Some(returned_rl);
                match result {
                    Ok(l) => l,
                    Err(rustyline::error::ReadlineError::Eof) => break,
                    // ^C at the prompt: discard the partial line and re-prompt,
                    // matching python/ipython/node REPL conventions. Exit is ^D.
                    Err(rustyline::error::ReadlineError::Interrupted) => continue,
                    Err(e) => {
                        eprintln!("[jet] readline: {e}");
                        break;
                    }
                }
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let _ = rl
            .as_mut()
            .expect("editor returned from blocking task")
            .add_history_entry(&line);

        let msg_id = jupyter::new_msg_id();
        let req = jupyter::message(
            "shell",
            &msg_id,
            "execute_request",
            json!({
                "code": line,
                "silent": false,
                "store_history": true,
                "user_expressions": {},
                "allow_stdin": true,
                "stop_on_error": true,
            }),
        );
        channel.send(&req).await?;

        // Wait for the kernel to go idle, possibly serving any number of
        // `input_request` calls along the way. Each iteration:
        //   1. Run the stdin watcher + wait_for_idle. The watcher claims
        //      stdin so it can catch ^C bytes (rustyline leaves the tty in
        //      raw mode, so the kernel doesn't translate ^C to SIGINT).
        //   2. If the kernel asks for input, drop the watcher (frees stdin
        //      for rustyline), prompt the user, send `input_reply` on the
        //      `stdin` channel, and loop back to step 1.
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
                            // Either path means the user pressed ^C: as a real
                            // SIGINT (ISIG cooked-mode tty path) or as a `\x03`
                            // byte on stdin (rustyline keeps the tty in raw
                            // mode between calls). Forward to the kernel via
                            // interrupt_session(); the tty driver already echoes
                            // `^C` for cooked-mode signals, so we don't print
                            // our own marker. Whether the kernel actually halts
                            // is up to it.
                            _ = intr_rx.recv() => {
                                if let Err(e) = client.interrupt_session(&session_id).await {
                                    eprintln!("\x1b[31m[jet] interrupt failed: {e}\x1b[0m");
                                }
                            }
                            _ = sigint.recv() => {
                                if let Err(e) = client.interrupt_session(&session_id).await {
                                    eprintln!("\x1b[31m[jet] interrupt failed: {e}\x1b[0m");
                                }
                            }
                        }
                    }
                },
            )
            .await;

            match r {
                WaitResult::Input(req) => {
                    // Watcher has been dropped; stdin is free for rustyline.
                    let prompt = if req.prompt.is_empty() {
                        "".to_string()
                    } else {
                        req.prompt.clone()
                    };
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
                        | Err(rustyline::error::ReadlineError::Interrupted) => {
                            // No input available: reply empty and let the
                            // kernel decide what to do (R returns NULL,
                            // Python raises EOFError).
                            String::new()
                        }
                        Err(e) => {
                            eprintln!("[jet] readline (input_request): {e}");
                            String::new()
                        }
                    };
                    let reply = jupyter::message(
                        "stdin",
                        &jupyter::new_msg_id(),
                        "input_reply",
                        json!({ "value": value }),
                    );
                    channel.send(&reply).await?;
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
                eprintln!("\x1b[33m[jet] timeout waiting for kernel\x1b[0m");
            }
            WaitResult::Closed => {
                // Kernel went away mid-request. The reader task either
                // already printed `[jet] kernel exited` (clean quit() /
                // exit()) or `[jet] ws error: …` (something worse). Exit
                // silently here either way.
                shutdown.notify_one();
                channel.close().await;
                drop(client);
                std::process::exit(0);
            }
        }
    }

    shutdown.notify_one();
    channel.close().await;
    // ^D is also reached only via the blocking readline thread returning,
    // so it's actually been joined. The runtime drop is fine here, but we
    // still process::exit for symmetry and to avoid waiting on the WS
    // reader's still-pending receive on a possibly-stuck socket.
    drop(client);
    std::process::exit(0);
}

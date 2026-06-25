//! The interactive REPL loop driven by `jet connect` / `jet attach`.
//!
//! Owns: rustyline prompt, is-complete polling, execute-request dispatch,
//! the kernel-liveness watchers (waitpid for spawned kernels, heartbeat
//! for attached ones), and the raw-mode SIGINT pipe that turns a tty ^C
//! into a kernel interrupt. The wire mechanics (per-channel reader and
//! writer tasks, kernel_info_request handshake, frame routing) live in
//! [`jet_core::kernel_session::KernelSession`] — this module asks for a
//! global sink that pumps every frame into the [`Renderer`], and uses
//! the renderer's mpsc channels to surface control signals (idle,
//! input_request, is_complete_reply) back into the REPL loop.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use jet_core::client::{Client, KernelStatus};
use jet_core::events::{InputRequest, IsCompleteReplyMsg, from_message};
use jet_core::jupyter_protocol::{
    ExecuteRequest, InputReply, IsCompleteReplyStatus, IsCompleteRequest, JupyterMessage,
};
use jet_core::kernel::KernelSpec;
use jet_core::manager::SessionStore;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::render::{Renderer, SharedWriter, ansi, warn_if_passthrough_off};

/// Reopen the session and flip it to Closed. Best-effort: called from
/// liveness watchers when the kernel becomes unreachable, so a missing
/// or unreadable session.json (e.g. attach by --connection-file with no
/// session id) is silently ignored.
fn mark_session_closed(session_id: &Option<String>) {
    let Some(id) = session_id else { return };
    let store = match SessionStore::default() {
        Ok(s) => s,
        Err(e) => {
            log::warn!("failed to resolve data dir to mark session {id} closed: {e}");
            return;
        }
    };
    match store.open(id) {
        Ok(mut s) => s.mark_closed(),
        Err(e) => log::warn!("failed to reopen session {id} to mark closed: {e}"),
    }
}

enum WaitResult {
    Idle,
    Timeout,
    Closed,
    Input(InputRequest),
}

/// Park until the kernel reaches `KernelStatus::Exited`. Resolves
/// immediately if it's already there.
async fn await_kernel_exited(mut rx: tokio::sync::watch::Receiver<KernelStatus>) {
    loop {
        if *rx.borrow() == KernelStatus::Exited {
            return;
        }
        if rx.changed().await.is_err() {
            return;
        }
    }
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

/// Run the prompt → execute → render loop until the user exits or the
/// kernel dies. Consumes the [`Kernel`] (wraps it in a
/// [`KernelSession`]) and returns the session so the caller can pick
/// between `.detach()` and `.shutdown()`.
/// How `drive_repl` should bring up its [`Client`]. Spawn vs Attach decides whether the
/// renderer sink suppresses the `kernel_info_reply` (so reconnects don't reprint the
/// banner the first connect already drew).
pub enum ReplTarget<'a> {
    Spawn {
        spec: &'a KernelSpec,
        connection_path: Option<PathBuf>,
    },
    Attach {
        connection_path: &'a Path,
    },
}

pub async fn drive_repl(
    target: ReplTarget<'_>,
    render_graphics: bool,
    session_name: Option<String>,
    session_id: Option<String>,
) -> Result<Client> {
    let session_id = Arc::new(session_id);
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
    let busy_state = renderer.busy_state.clone();

    // Client::*_with_sink performs the kernel_info handshake before spawning the
    // socket loop, feeding the reply through the sink as its last step. On the spawn
    // path that renders the welcome banner; on attach we suppress the
    // kernel_info_reply specifically so we don't reprint it on every reconnect (the
    // rest of the renderer's behaviour stays).
    let sink_renderer = renderer.clone();
    let is_attached = matches!(target, ReplTarget::Attach { .. });
    let sink = move |f: jet_core::client::Frame| {
        if is_attached && f.message.message_type() == "kernel_info_reply" {
            return;
        }
        if let Err(e) = sink_renderer.handle_event(from_message(f.channel, &f.message)) {
            log::warn!("renderer ({:?}): {e}", f.channel);
        }
    };
    let (mut session, _info) = match target {
        ReplTarget::Spawn {
            spec,
            connection_path,
        } => Client::spawn_with_sink(spec, connection_path, session_name.as_deref(), sink).await?,
        ReplTarget::Attach { connection_path } => {
            Client::attach_with_sink(connection_path, session_name.as_deref(), sink).await?
        }
    };
    let child_pid = session.child_pid();

    let shutdown = Arc::new(tokio::sync::Notify::new());

    // Liveness is owned by KernelSession (heartbeat for attached kernels,
    // waitpid for spawned ones, socket-loop error path for crashes). The
    // CLI's only liveness concern is flipping session.json to Closed; we
    // do that inline at the two sites where the REPL observes Exited
    // (the prompt-loop select and the wait-for-idle select), so no
    // separate bridge task is needed.
    let _ = (is_attached, child_pid);

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
            // If another session is currently executing, park before
            // drawing the prompt. Without this, every CR the user types
            // produces a fresh `> ` even though the kernel is busy with
            // someone else's request — hiding the fact that it's busy.
            // We watch for kernel-exit alongside so a crash during a
            // foreign execute still wakes us out of the park.
            while busy_state.busy.load(std::sync::atomic::Ordering::SeqCst) {
                let notified = busy_state.notify.notified();
                tokio::select! {
                    _ = notified => {}
                    _ = await_kernel_exited(session.watch_status()) => {
                        eprintln!("{}", ansi::red("Kernel exited"));
                        mark_session_closed(&session_id);
                        shutdown.notify_waiters();
                        std::process::exit(0);
                    }
                }
            }

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
                _ = await_kernel_exited(session.watch_status()) => {
                    eprintln!("{}", ansi::red("Kernel exited"));
                    mark_session_closed(&session_id);
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
                                return Ok(session);
                            }
                            // ^D inside an in-progress block: discard.
                            break 'accumulate None;
                        }
                        Err(rustyline::error::ReadlineError::Interrupted) => {
                            // ^C abandons the in-progress block.
                            break 'accumulate None;
                        }
                        Err(e) => {
                            eprintln!("Readline: {e}");
                            return Ok(session);
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
            // We don't read the per-request stream — the global sink
            // already feeds the reply through the renderer's
            // is_complete_tx channel. Drop the stream immediately;
            // RequestStream's Drop forgets its router slot for us.
            let _ = session.request(req)?;
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
        let _ = session.request(req)?;

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
                            _ = await_kernel_exited(session.watch_status()) => return WaitResult::Closed,
                            _ = intr_rx.recv() => {
                                if let Err(e) = session.interrupt().await {
                                    eprintln!("{}", ansi::red(&format!("Interrupt failed: {e}")));
                                }
                            }
                            _ = sigint.recv() => {
                                if let Err(e) = session.interrupt().await {
                                    eprintln!("{}", ansi::red(&format!("Interrupt failed: {e}")));
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
                            eprintln!("Readline (input_request): {e}");
                            String::new()
                        }
                    };
                    let reply: JupyterMessage = InputReply {
                        value,
                        status: Default::default(),
                        error: None,
                    }
                    .into();
                    let _ = session.reply_stdin(reply);
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
                eprintln!("{}", ansi::yellow("Timeout waiting for kernel"));
            }
            WaitResult::Closed => {
                mark_session_closed(&session_id);
                shutdown.notify_waiters();
                std::process::exit(0);
            }
        }
    }
}

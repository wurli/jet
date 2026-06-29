//! The interactive REPL loop driven by `jet start` / `jet attach`.
//!
//! Owns: reedline prompt, is-complete polling, execute-request dispatch,
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
use jet_core::manager::{Session, SessionStore};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedReceiver;

use reedline::{
    ExternalPrinter, IdeMenu, KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptEditMode,
    PromptHistorySearch, PromptHistorySearchStatus, Reedline, ReedlineEvent, ReedlineMenu, Signal,
};

/// Put the terminal back into cooked mode. reedline holds the tty in
/// raw mode between `read_line` calls; if we `process::exit` while a
/// `read_line` is still in flight on a blocking thread, its `Drop`
/// never runs and the shell inherits a wedged terminal. Call this
/// before any `eprintln!` that should appear on its own line — under
/// raw mode `\n` doesn't include `\r`, so the cursor stays in the same
/// column.
fn restore_terminal() {
    let _ = crossterm::terminal::disable_raw_mode();
}

fn exit_cleanly(code: i32) -> ! {
    restore_terminal();
    std::process::exit(code);
}

use crate::completer::JetCompleter;
use crate::render::{Renderer, SharedWriter, ansi, warn_if_passthrough_off};

/// Reedline prompt with a swappable left-indicator string — we set it
/// to `> ` for the first line of a cell, then to the kernel-suggested
/// continuation indent (e.g. `+ `) for subsequent lines until
/// `is_complete_reply` says we're done.
#[derive(Default, Clone)]
struct JetPrompt {
    indicator: String,
}

impl Prompt for JetPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.indicator)
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("::: ")
    }
    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!("({prefix}reverse-search: {}) ", history_search.term))
    }
}

/// Outcome of a single readline. Mirrors reedline's `Signal` but lets
/// the non-TTY pipe branch return its own EOF/line cases without
/// fabricating a reedline value.
enum LineRead {
    Line(String),
    /// User pressed ^C (reedline) — the in-progress block is abandoned.
    Interrupted,
    /// Stream ended (reedline ^D, or EOF on a piped stdin).
    Eof,
    Err(String),
}

/// Where to read REPL input from. TTY uses reedline (with completer);
/// pipe mode (e.g. `jet start < script.py`, or tests piping stdin)
/// reads bare lines via `BufRead`, since reedline requires a real
/// terminal to query cursor position.
enum LineSource {
    Tty(Reedline),
    Pipe(std::io::BufReader<std::io::Stdin>),
}

impl LineSource {
    fn read_line(&mut self, prompt: &JetPrompt) -> LineRead {
        use std::io::{BufRead, Write};
        match self {
            LineSource::Tty(rl) => match rl.read_line(prompt) {
                Ok(Signal::Success(l)) => LineRead::Line(l),
                Ok(Signal::CtrlC) => LineRead::Interrupted,
                Ok(Signal::CtrlD) => LineRead::Eof,
                Ok(_) => LineRead::Interrupted,
                Err(e) => LineRead::Err(e.to_string()),
            },
            LineSource::Pipe(stdin) => {
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(prompt.indicator.as_bytes());
                let _ = out.flush();
                let mut buf = String::new();
                match stdin.read_line(&mut buf) {
                    Ok(0) => LineRead::Eof,
                    Ok(_) => {
                        if buf.ends_with('\n') {
                            buf.pop();
                            if buf.ends_with('\r') {
                                buf.pop();
                            }
                        }
                        LineRead::Line(buf)
                    }
                    Err(e) => LineRead::Err(e.to_string()),
                }
            }
        }
    }
}

fn build_editor(completer: JetCompleter, printer: ExternalPrinter<String>) -> Reedline {
    // Drop reedline's default `| ` left-marker; the menu sits below the
    // prompt and the bar adds visual noise to every keystroke.
    let completion_menu = Box::new(
        IdeMenu::default()
            .with_name("completion_menu")
            .with_marker(""),
    );
    let mut keybindings = reedline::default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    let edit_mode = Box::new(reedline::Emacs::new(keybindings));
    Reedline::create()
        .with_completer(Box::new(completer))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode)
        .with_external_printer(printer)
}

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
/// banner the first start already drew).
pub enum ReplTarget<'a> {
    Spawn {
        spec: &'a KernelSpec,
        connection_path: Option<PathBuf>,
        /// SessionStore id this kernel is bound to (the `session.json` slug).
        session_id: Option<String>,
    },
    Attach {
        connection_path: &'a Path,
        /// SessionStore id this kernel is bound to (the `session.json` slug).
        session_id: Option<String>,
        /// Render the kernel banner on attach. Defaults to false so reconnects
        /// don't reprint the banner the original spawn already drew.
        banner: bool,
    },
}

pub async fn drive_repl(
    target: ReplTarget<'_>,
    render_graphics: bool,
    session_name: Option<String>,
    session_store_entry: Option<&mut Session>,
) -> Result<Client> {
    if render_graphics {
        warn_if_passthrough_off();
    }

    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<InputRequest>();
    let (is_complete_tx, mut is_complete_rx) =
        tokio::sync::mpsc::unbounded_channel::<IsCompleteReplyMsg>();
    let writer: SharedWriter = Arc::new(Mutex::new(std::io::stdout()));
    // The external printer is the channel reedline uses to interleave
    // foreign-session output with the active prompt. Wire it into both
    // the renderer (so foreign writes go through it) and the editor
    // builder below (so reedline polls and flushes it).
    let external_printer: ExternalPrinter<String> = ExternalPrinter::default();
    let renderer = Renderer::new(render_graphics, idle_tx, writer)
        .with_input_tx(input_tx)
        .with_is_complete_tx(is_complete_tx)
        .with_own_session_name(session_name.clone())
        .with_external_printer(external_printer.clone());
    let busy_state = renderer.busy_state.clone();

    // Client::spawn/attach perform the kernel_info handshake before returning,
    // dispatching the reply as the LAST step so it lands in `boot_stream`'s mpsc
    // (a no-filter `listen` registered before the dispatch). We then synchronously
    // pull that first frame to render the banner before drawing any prompt, so
    // banner-then-prompt ordering is preserved without a sink callback.
    let render_banner = match &target {
        ReplTarget::Spawn { .. } => true,
        ReplTarget::Attach { banner, .. } => *banner,
    };
    let (mut session, _info, mut boot_stream) = match target {
        ReplTarget::Spawn {
            spec,
            connection_path,
            session_id,
        } => {
            Client::spawn(
                spec,
                connection_path,
                session_name.as_deref(),
                session_id,
            )
            .await?
        }
        ReplTarget::Attach {
            connection_path,
            session_id,
            banner: _,
        } => Client::attach(connection_path, session_name.as_deref(), session_id).await?,
    };
    // Now that the Client exists we know our full client_id (the
    // `<name>---repl---<rand>` string the kernel will echo back as
    // parent_session on every reply); pass it into the renderer so own-
    // vs-foreign is decided on identity, not just `--session-name`.
    let renderer = renderer.with_own_client_id(session.client_id().to_string());
    // Synchronously consume the first frame — the kernel_info_reply. On spawn we
    // render it (welcome banner); on attach we drop it by default so reconnects
    // don't reprint the banner the original spawn already drew, unless --banner
    // was passed.
    if let Some(f) = boot_stream.recv().await
        && render_banner
        && let Err(e) = renderer.handle_event(from_message(f.channel, &f.message))
    {
        log::warn!("renderer (banner): {e}");
    }
    // Pump the rest of the boot stream into the renderer on a dedicated task. The
    // stream ends with a terminal idle when the kernel exits, so the task exits
    // on its own — no extra shutdown plumbing.
    {
        let renderer = renderer.clone();
        tokio::spawn(async move {
            while let Some(f) = boot_stream.recv().await {
                if let Err(e) = renderer.handle_event(from_message(f.channel, &f.message)) {
                    log::warn!("renderer ({:?}): {e}", f.channel);
                }
            }
        });
    }
    // Persist the kernel pid into session.json now — before the REPL loop starts —
    // so external readers (e.g. `jet list-sessions`, the nvim plugin) see it for the
    // whole lifetime of the kernel, not just after the user quits the REPL.
    if let (Some(pid), Some(entry)) = (session.child_pid(), session_store_entry) {
        entry.set_kernel_pid(pid);
    }
    // session.json bookkeeping (mark_session_closed on kernel exit) reads the id off
    // the Client now — kept in an Arc so it survives moves into select! branches below.
    let session_id = Arc::new(session.session_id().map(str::to_string));

    let shutdown = Arc::new(tokio::sync::Notify::new());

    // Liveness is owned by KernelSession (heartbeat for attached kernels,
    // waitpid for spawned ones, socket-loop error path for crashes). The
    // CLI's only liveness concern is flipping session.json to Closed; we
    // do that inline at the two sites where the REPL observes Exited
    // (the prompt-loop select and the wait-for-idle select), so no
    // separate bridge task is needed.

    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    use std::io::IsTerminal;
    let mut rl = Some(if std::io::stdin().is_terminal() {
        LineSource::Tty(build_editor(
            JetCompleter::new(
                session.completion_handle(),
                tokio::runtime::Handle::current(),
            ),
            external_printer.clone(),
        ))
    } else {
        LineSource::Pipe(std::io::BufReader::new(std::io::stdin()))
    });
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
                        restore_terminal();
                        eprintln!("{}", ansi::red("Kernel exited"));
                        mark_session_closed(&session_id);
                        shutdown.notify_waiters();
                        exit_cleanly(0);
                    }
                }
            }

            let mut prompt_rl = rl.take().expect("editor present at top of loop");
            let prompt = JetPrompt {
                indicator: match &next_indent {
                    None => "> ".to_string(),
                    Some(s) => s.clone(),
                },
            };
            let read = tokio::task::spawn_blocking(move || {
                let result = prompt_rl.read_line(&prompt);
                (prompt_rl, result)
            });
            let line = tokio::select! {
                _ = await_kernel_exited(session.watch_status()) => {
                    restore_terminal();
                    eprintln!("{}", ansi::red("Kernel exited"));
                    mark_session_closed(&session_id);
                    shutdown.notify_waiters();
                    exit_cleanly(0);
                }
                joined = read => {
                    let (returned_rl, result) = joined?;
                    rl = Some(returned_rl);
                    match result {
                        LineRead::Line(l) => l,
                        LineRead::Eof => {
                            if buffer.is_empty() {
                                shutdown.notify_waiters();
                                return Ok(session);
                            }
                            // ^D inside an in-progress block: discard.
                            break 'accumulate None;
                        }
                        LineRead::Interrupted => {
                            // ^C abandons the in-progress block.
                            break 'accumulate None;
                        }
                        LineRead::Err(e) => {
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
        if let Some(LineSource::Tty(ed)) = rl.as_mut() {
            let item = reedline::HistoryItem::from_command_line(&code);
            let _ = ed.history_mut().save(item);
        }

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
            // reedline 0.48 enables raw mode at the start of each read_line
            // and disables it on exit (engine.rs:766/774), so the TTY is in
            // cooked mode here, between read_lines, and ISIG is on. A real
            // ^C therefore generates SIGINT directly, which `sigint.recv()`
            // catches below. No stdin-byte watcher: that races whatever is
            // piping in input (nvim chansend, `jet start < script.py`) and
            // silently swallows non-^C bytes destined for the next read.
            let r: WaitResult = async {
                loop {
                    tokio::select! {
                        r = wait_for_idle(&mut idle_rx, &mut input_rx, &msg_id, Duration::from_secs(300)) => return r,
                        _ = await_kernel_exited(session.watch_status()) => return WaitResult::Closed,
                        _ = sigint.recv() => {
                            if let Err(e) = session.interrupt().await {
                                eprintln!("{}", ansi::red(&format!("Interrupt failed: {e}")));
                            }
                        }
                    }
                }
            }
            .await;

            match r {
                WaitResult::Input(req) => {
                    let prompt = JetPrompt {
                        indicator: req.prompt.clone(),
                    };
                    let mut prompt_rl = rl.take().expect("editor present at input prompt");
                    let read = tokio::task::spawn_blocking(move || {
                        let line = prompt_rl.read_line(&prompt);
                        (prompt_rl, line)
                    });
                    let (returned_rl, line_result) = read.await?;
                    rl = Some(returned_rl);
                    let value = match line_result {
                        LineRead::Line(s) => s,
                        LineRead::Eof | LineRead::Interrupted => String::new(),
                        LineRead::Err(e) => {
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
                exit_cleanly(0);
            }
        }
    }
}

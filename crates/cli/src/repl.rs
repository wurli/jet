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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use jet_core::client::{Client, KernelStatus};
use jet_core::events::{InputRequest, IsCompleteReplyMsg, from_message};
use jet_core::jupyter_protocol::{
    ExecuteRequest, InputReply, IsCompleteReplyStatus, IsCompleteRequest, JupyterMessage,
};
use jet_core::kernel::{AttachOptions, KernelSpec};
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
use crate::render::{ExternalClientStyle, Renderer, SharedWriter, ansi, warn_if_passthrough_off};

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
        Cow::Borrowed("+ ")
    }
    fn get_prompt_multiline_color(&self) -> nu_ansi_term::Color {
        // Match the single-line prompt's default (Color::Green); reedline's
        // default multiline color is LightBlue, which clashes with the
        // green `>` we use for the primary prompt.
        nu_ansi_term::Color::Green
    }
    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!(
            "({prefix}reverse-search: {}) ",
            history_search.term
        ))
    }
}

/// Outcome of a single readline. Mirrors reedline's `Signal` but lets
/// the non-TTY pipe branch return its own EOF/line cases without
/// fabricating a reedline value.
/// Host-command sentinel: "the user pressed Backspace; check whether to
/// delete a char or merge back into the prior accumulator line."
const HOSTCMD_BACKSPACE: &str = "jet:backspace";

enum LineRead {
    Line(String),
    /// User pressed ^C (reedline) — the in-progress block is abandoned.
    Interrupted,
    /// Stream ended (reedline ^D, or EOF on a piped stdin).
    Eof,
    /// Reedline emitted an `ExecuteHostCommand` signal. The editor is
    /// suspended (buffer state preserved); we handle the command and
    /// re-enter `read_line` to resume.
    HostCommand(String),
    /// Reedline returned from `read_line` because the `break_signal` was
    /// flipped by another thread (a foreign session's `Busy` event). The
    /// inner string is the user's in-progress buffer; we wait for the
    /// foreign Idle, then re-enter `read_line` with the buffer pre-filled.
    ExternalBreak(String),
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
                Ok(Signal::HostCommand(c)) => LineRead::HostCommand(c),
                Ok(Signal::ExternalBreak(buf)) => LineRead::ExternalBreak(buf),
                Ok(_) => LineRead::Interrupted,
                Err(e) => LineRead::Err(e.to_string()),
            },
            LineSource::Pipe(stdin) => {
                {
                    let mut out = std::io::stdout().lock();
                    let _ = out.write_all(prompt.indicator.as_bytes());
                    let _ = out.flush();
                }
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

fn build_editor(
    completer: JetCompleter,
    printer: ExternalPrinter<String>,
    break_signal: Arc<AtomicBool>,
) -> Reedline {
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
    // Backspace goes through us so we can detect "empty buffer + Backspace"
    // mid-block and merge the prior line back into the editor. The REPL
    // loop checks `current_buffer_contents()`; if non-empty it forwards the
    // event as a normal `EditCommand::Backspace`.
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Backspace,
        ReedlineEvent::ExecuteHostCommand(HOSTCMD_BACKSPACE.to_string()),
    );
    let edit_mode = Box::new(reedline::Emacs::new(keybindings));
    Reedline::create()
        .with_completer(Box::new(completer))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode)
        .with_external_printer(printer)
        // When a foreign session goes Busy, the renderer flips this flag.
        // reedline returns Signal::ExternalBreak from read_line with the
        // user's in-progress buffer, so the REPL loop can write foreign
        // output in cooked mode without reedline's repaint fighting it.
        .with_break_signal(break_signal)
        // Honor bracketed-paste: a multi-line block wrapped in
        // \x1b[200~ ... \x1b[201~ goes into the buffer as one unit
        // and waits for a separate Enter to submit. Matches what real
        // terminals send on Cmd/Ctrl+V and what editor/REPL integrations
        // (nvim chansend, etc.) emit to submit a cell.
        .use_bracketed_paste(true)
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
) -> WaitResult {
    loop {
        tokio::select! {
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
        /// Kernelspec-derived interrupt mode + kernel pid, when the caller
        /// could recover them from the session store. The connection file
        /// alone doesn't carry either, so without this ^C forwarding is a
        /// no-op after `jet attach`.
        attach_opts: AttachOptions,
        /// Render the kernel banner on attach. Defaults to false so reconnects
        /// don't reprint the banner the original spawn already drew.
        banner: bool,
    },
}

pub async fn drive_repl(
    target: ReplTarget<'_>,
    render_graphics: bool,
    no_indent: bool,
    session_name: Option<String>,
    external_client_style: ExternalClientStyle,
    session_store_entry: Option<&mut Session>,
) -> Result<Client> {
    if render_graphics {
        warn_if_passthrough_off();
    }

    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<InputRequest>();
    let (is_complete_tx, mut is_complete_rx) =
        tokio::sync::mpsc::unbounded_channel::<IsCompleteReplyMsg>();
    let (execute_reply_tx, mut execute_reply_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let writer: SharedWriter = Arc::new(Mutex::new(std::io::stdout()));
    // Reedline needs an ExternalPrinter to build its editor even though
    // we don't route any output through it — foreign writes go directly
    // to the shared writer (with \n→\r\n translation for raw mode).
    let external_printer: ExternalPrinter<String> = ExternalPrinter::default();
    let renderer = Renderer::new(render_graphics, idle_tx, writer)
        .with_input_tx(input_tx)
        .with_is_complete_tx(is_complete_tx)
        .with_execute_reply_tx(execute_reply_tx)
        .with_own_session_name(session_name.clone())
        .with_external_client_style(external_client_style);
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
        } => Client::spawn(spec, connection_path, session_name.as_deref(), session_id).await?,
        ReplTarget::Attach {
            connection_path,
            session_id,
            attach_opts,
            banner: _,
        } => {
            Client::attach(
                connection_path,
                session_name.as_deref(),
                session_id,
                attach_opts,
            )
            .await?
        }
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
            busy_state.break_signal.clone(),
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
        let mut next_initial: Option<String> = None;
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
            if let (Some(s), LineSource::Tty(ed)) = (&next_initial, &mut prompt_rl)
                && !s.is_empty()
            {
                ed.run_edit_commands(&[reedline::EditCommand::InsertString(s.clone())]);
            }
            next_initial = None;
            let read_line_active = busy_state.read_line_active.clone();
            read_line_active.store(true, Ordering::SeqCst);
            let read = tokio::task::spawn_blocking(move || {
                let result = prompt_rl.read_line(&prompt);
                read_line_active.store(false, Ordering::SeqCst);
                (prompt_rl, result)
            });
            let line = tokio::select! {
                _ = await_kernel_exited(session.watch_status()) => {
                    // Kernel died while we were waiting for the user's next input. The
                    // overwhelming common case is an in-band exit (ipykernel `quit()`, R
                    // `quit()`) — the user asked to leave, no error to surface. Even for
                    // genuine crashes there's nothing the user can do at this prompt, so
                    // exit silently rather than printing a red warning that looks like a
                    // jet bug. The `printnl!()` moves the cursor off the prompt row so
                    // the parent shell doesn't paint its no-newline marker (`%` in zsh,
                    // `⏎` in fish).
                    restore_terminal();
                    println!();
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
                        LineRead::HostCommand(cmd) if cmd == HOSTCMD_BACKSPACE => {
                            // Plain Backspace was routed through us so we
                            // could decide: normal char-delete, or merge
                            // the prior accumulator line back into the
                            // editor when the visible line is empty.
                            if let Some(LineSource::Tty(ed)) = rl.as_mut() {
                                if ed.current_buffer_contents().is_empty() {
                                    // Empty visible line: pop the last
                                    // line off the accumulator and pre-
                                    // fill the editor with it. If the
                                    // accumulator is also empty, ignore.
                                    if !buffer.is_empty() {
                                        let prev = match buffer.rfind('\n') {
                                            Some(i) => buffer.split_off(i + 1),
                                            None => std::mem::take(&mut buffer),
                                        };
                                        if buffer.ends_with('\n') {
                                            buffer.pop();
                                        }
                                        ed.run_edit_commands(&[
                                            reedline::EditCommand::InsertString(prev),
                                        ]);
                                        // Erase the prior prompt row (the
                                        // one we're rejoining) and the
                                        // current empty continuation row,
                                        // then put the cursor at the
                                        // start of the prior row. The
                                        // next `read_line` sees a cursor
                                        // outside the suspended prompt
                                        // range and draws a fresh prompt
                                        // there.
                                        use std::io::Write;
                                        let mut out = std::io::stdout().lock();
                                        let _ = out.write_all(b"\x1b[A\r\x1b[J");
                                        let _ = out.flush();
                                        // We're back to editing what was
                                        // the prior line; reset the
                                        // continuation-prompt state so
                                        // the next prompt matches.
                                        next_indent = if buffer.is_empty() {
                                            None
                                        } else {
                                            Some("+ ".to_string())
                                        };
                                        next_initial = None;
                                    }
                                } else {
                                    // Non-empty: behave like a normal
                                    // backspace.
                                    ed.run_edit_commands(&[
                                        reedline::EditCommand::Backspace,
                                    ]);
                                }
                            }
                            continue;
                        }
                        LineRead::HostCommand(_) => continue,
                        LineRead::ExternalBreak(_buf) => {
                            // A foreign session went Busy while we were in
                            // `read_line`. reedline keeps the in-progress
                            // buffer in the editor's state across the
                            // suspend/resume cycle, so we don't need to
                            // re-insert it — the next `read_line` will
                            // redraw the prompt with the buffer already
                            // present. The busy-park gate at the top waits
                            // for Idle before drawing the new prompt.
                            continue;
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
                    // Per Jupyter spec, `indent` is the full continuation
                    // prompt — typically a marker (e.g. `...`) followed by
                    // whitespace to align with the previous line. We treat
                    // only the leading non-whitespace as the prompt marker;
                    // any trailing whitespace becomes editable text in the
                    // input buffer so the user can backspace through it.
                    let marker_end = indent
                        .find(|c: char| c.is_whitespace())
                        .unwrap_or(indent.len());
                    let (marker, ws) = indent.split_at(marker_end);
                    let mut p = if marker.is_empty() {
                        "+".to_string()
                    } else {
                        marker.to_string()
                    };
                    if !p.ends_with(' ') {
                        p.push(' ');
                    }
                    next_indent = Some(p);
                    next_initial = if ws.is_empty() | no_indent {
                        None
                    } else {
                        Some(ws.to_string())
                    };
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
                        r = wait_for_idle(&mut idle_rx, &mut input_rx, &msg_id) => return r,
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
                    // For stdin prompts there's no accumulator to merge
                    // into, so a HostCommand Backspace just becomes a
                    // plain Backspace edit; we loop until the user
                    // actually submits or aborts.
                    let value = loop {
                        let mut prompt_rl = rl.take().expect("editor present at input prompt");
                        let prompt_for_read = prompt.clone();
                        let read_line_active = busy_state.read_line_active.clone();
                        read_line_active.store(true, Ordering::SeqCst);
                        let read = tokio::task::spawn_blocking(move || {
                            let line = prompt_rl.read_line(&prompt_for_read);
                            read_line_active.store(false, Ordering::SeqCst);
                            (prompt_rl, line)
                        });
                        let (returned_rl, line_result) = read.await?;
                        rl = Some(returned_rl);
                        match line_result {
                            LineRead::Line(s) => break s,
                            LineRead::Eof | LineRead::Interrupted => break String::new(),
                            LineRead::HostCommand(cmd) if cmd == HOSTCMD_BACKSPACE => {
                                if let Some(LineSource::Tty(ed)) = rl.as_mut() {
                                    ed.run_edit_commands(&[reedline::EditCommand::Backspace]);
                                }
                                continue;
                            }
                            LineRead::HostCommand(_) => continue,
                            LineRead::ExternalBreak(_) => {
                                // Foreign Busy during a stdin input prompt:
                                // just retry. Stdin prompts don't have an
                                // accumulator to merge into.
                                continue;
                            }
                            LineRead::Err(e) => {
                                eprintln!("Readline (input_request): {e}");
                                break String::new();
                            }
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
            WaitResult::Closed => {
                mark_session_closed(&session_id);
                shutdown.notify_waiters();
                exit_cleanly(0);
            }
        }

        // "Execute is fully done" requires both the terminal `status: idle` on iopub
        // AND the `execute_reply` on shell for this msg_id. Kallichore and
        // jupyter_console both spell this out: the two arrive on different sockets
        // and may reorder, so gating on Idle alone is wrong. Concretely: on
        // ipykernel's `quit()`, iopub Idle arrives, then shell execute_reply, then
        // the kernel exits — if we redraw on Idle we paint a `> ` that is
        // immediately followed by "Kernel exited". Waiting for both keeps this
        // spec-correct and kernel-agnostic (works for R's `q()`, Julia, etc.
        // without any per-language sniffing).
        //
        // If the kernel dies while we're waiting for the reply, `watch_status`
        // trips to Exited — exit cleanly without drawing another prompt.
        let watch = session.watch_status();
        tokio::select! {
            r = execute_reply_rx.recv() => match r {
                Some(parent) if parent == msg_id => {}
                // Non-matching parents (foreign execute_replies leaking through)
                // are ignored via a re-drain loop below.
                Some(_) => {
                    loop {
                        tokio::select! {
                            r = execute_reply_rx.recv() => match r {
                                Some(p) if p == msg_id => break,
                                Some(_) => continue,
                                None => break,
                            },
                            _ = await_kernel_exited(watch.clone()) => {
                                mark_session_closed(&session_id);
                                shutdown.notify_waiters();
                                exit_cleanly(0);
                            }
                        }
                    }
                }
                None => {}
            },
            _ = await_kernel_exited(watch.clone()) => {
                mark_session_closed(&session_id);
                shutdown.notify_waiters();
                exit_cleanly(0);
            }
        }

        // Both signals in hand. If the kernel is dying (an in-band exit like
        // `quit()` / `q()`), Exited hasn't quite transitioned yet — the shell
        // reply commonly beats the process-death socket-read-error by a few
        // milliseconds. Wait a short window for the next `watch_status`
        // change; if it goes to Exited, bail cleanly without redrawing. On a
        // healthy kernel `watch_status` won't change (the kernel just went
        // back to Idle), so the timer fires and we draw the next prompt.
        // 30ms is short enough to be invisible under keystroke latency but
        // comfortably longer than the socket-loop's post-exit read error.
        if *watch.borrow() == KernelStatus::Exited {
            mark_session_closed(&session_id);
            shutdown.notify_waiters();
            exit_cleanly(0);
        }
    }
}

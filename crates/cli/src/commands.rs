//! Subcommand handlers for `jet` (start, attach, list-sessions, list-kernels).

use anyhow::Result;

use jet_core::client::{Client, make_client_id};
use jet_core::kernel::{AttachOptions, Kernel, KernelSpec};
use jet_core::manager::{SessionStatus, SessionStore};

use crate::cli::{
    AttachArgs, ExecuteArgs, ListKernelsArgs, ListSessionsArgs, SendArgs, ShowArgs, StartArgs,
    StatusFilter, StopArgs,
};
use crate::pickers::{pick_kernelspec, pick_session, pick_sessions_multi};
use crate::repl::{ReplTarget, drive_repl};

/// Best-effort: dig the kernelspec's `interrupt_mode` and the session-tracked
/// kernel pid out of the session store, so `Kernel::attach` can forward `^C`
/// correctly. Falls back to `AttachOptions::default()` (signal mode, no pid)
/// when the connection file isn't part of a tracked session, or when the
/// kernelspec on disk has since been removed — matching the pre-change
/// behavior in those edge cases.
fn recover_attach_options(conn_path: &std::path::Path) -> AttachOptions {
    let Some(store) = SessionStore::default().ok() else {
        return AttachOptions::default();
    };
    let Some(session) = store.find_by_connection_file(conn_path).ok().flatten() else {
        return AttachOptions::default();
    };
    let meta = session.meta();
    let interrupt_mode = KernelSpec::load(&meta.kernelspec_path)
        .map(|s| s.interrupt_mode)
        .unwrap_or_default();
    AttachOptions {
        interrupt_mode,
        pid: meta.kernel_pid,
    }
}

pub fn run_list_kernels(args: ListKernelsArgs) -> Result<()> {
    let paths: Vec<_> = jet_core::kernel_spec::KernelSpec::find_valid()
        .into_iter()
        .map(|(p, _)| p)
        .collect();

    if args.json {
        let objs: Vec<_> = paths
            .iter()
            .map(|p| {
                let spec = std::fs::read(p)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .unwrap_or(serde_json::Value::Null);
                serde_json::json!({ "path": p, "spec": spec })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&objs)?);
    } else {
        for path in &paths {
            println!("{}", path.display());
        }
    }
    Ok(())
}

pub fn run_skill() -> Result<()> {
    print!("{}", include_str!("skill.md"));
    Ok(())
}

pub fn run_show(args: ShowArgs) -> Result<()> {
    let view = jet_core::manager::show_session(&args.session_id)?;
    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}

pub async fn run_list_sessions(args: ListSessionsArgs) -> Result<()> {
    let store = SessionStore::default()?;
    let sessions = store
        .list_filtered(args.status.into(), args.all_dirs)
        .await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&sessions)?);
        return Ok(());
    }

    let show_status = matches!(args.status, StatusFilter::All);
    let id_w = sessions
        .iter()
        .map(|s| s.session_id.len())
        .max()
        .unwrap_or(0);
    let name_w = sessions
        .iter()
        .map(|s| s.display_name.len())
        .max()
        .unwrap_or(0);
    let created_w = sessions
        .iter()
        .map(|s| s.created_at.len())
        .max()
        .unwrap_or(0);
    for s in &sessions {
        if show_status {
            let st = match s.status {
                SessionStatus::Open => "open",
                SessionStatus::Closed => "closed",
            };
            println!(
                "{:<id_w$}  {:<name_w$}  {:<created_w$}  {}",
                s.session_id, s.display_name, s.created_at, st,
            );
        } else {
            println!(
                "{:<id_w$}  {:<name_w$}  {}",
                s.session_id, s.display_name, s.created_at,
            );
        }
    }
    Ok(())
}

pub async fn run_connect(args: StartArgs) -> Result<()> {
    let kernelspec = match args.kernelspec {
        Some(p) => p,
        None => match pick_kernelspec().await? {
            Some(p) => p,
            None => return Ok(()),
        },
    };

    let spec = jet_core::kernel::KernelSpec::load(&kernelspec)?;
    log::info!(
        "spawning kernel (language={}, argv={:?})",
        spec.language,
        spec.argv,
    );

    let cwd = std::env::current_dir()?;
    // `--connection-file` is an escape hatch for callers managing the
    // file themselves (e.g. a parent process that wants to pin the
    // path). In that mode we don't create a session.json — the session
    // store only tracks kernels jet owns end-to-end.
    let mut session = match args.connection_file.clone() {
        None => {
            let store = SessionStore::default()?;
            let name = spec.display_name.clone().unwrap_or_default();
            let session = match args.session_id.as_deref() {
                Some(id) => store.create_with_id(id, &spec.language, &name, &kernelspec, &cwd)?,
                None => store.create(&spec.language, &name, &kernelspec, &cwd)?,
            };
            Some(session)
        }
        Some(_) => None,
    };

    let conn_path = args
        .connection_file
        .clone()
        .unwrap_or_else(|| session.as_ref().unwrap().connection_file_path());

    if conn_path.exists() {
        let store = SessionStore::default()?;
        let reattach = match store.find_by_connection_file(&conn_path)? {
            Some(s) => format!("jet attach {}", s.meta().session_id),
            None => format!("jet attach --connection-file {}", conn_path.display()),
        };
        anyhow::bail!(
            "connection file already exists at {}: remove it or run `{reattach}` to reconnect",
            conn_path.display(),
        );
    }

    let render_graphics = !args.no_graphics;
    let session_id = session.as_ref().map(|s| s.meta().session_id.clone());
    let mut kernel_session = drive_repl(
        ReplTarget::Spawn {
            spec: &spec,
            connection_path: Some(conn_path.clone()),
            session_id,
        },
        render_graphics,
        args.no_indent,
        args.session_name,
        args.external_client_style.into(),
        session.as_mut(),
    )
    .await?;

    if args.persist {
        kernel_session.detach();
    } else {
        let _ = kernel_session.shutdown().await;
        if let Some(s) = session.as_mut() {
            s.mark_closed();
        }
    }
    Ok(())
}

pub async fn run_attach(args: AttachArgs) -> Result<()> {
    let (conn_path, session_id) = match (args.session_id, args.connection_file) {
        (Some(id), None) => {
            let path = SessionStore::default()?.open(&id)?.connection_file_path();
            (path, Some(id))
        }
        (None, Some(path)) => {
            // Best-effort: if the path lives inside a tracked session, recover the id so
            // `mark_session_closed` works on kernel death.
            // NOTE: right now jet's awareness of a session only influences whether the kernel
            // is marked as closed on exit - but we have recovery even if the marker isn't set.
            // However I expect we'll have more behaviour in the future which depends on
            // session.json. Anyway this is an extreme edge-case, so probs not one to worry about
            // much right now.
            let id = SessionStore::default()
                .ok()
                .and_then(|s| s.find_by_connection_file(&path).ok().flatten())
                .map(|s| s.meta().session_id.clone());
            (path, id)
        }
        (None, None) => {
            let Some(id) = pick_session("Connect to an existing session:").await? else {
                // No session selected
                return Ok(());
            };
            let path = SessionStore::default()?.open(&id)?.connection_file_path();
            (path, Some(id))
        }
        (Some(_), Some(_)) => {
            unreachable!("clap ArgGroup forbids passing both session_id and --connection-file")
        }
    };
    let render_graphics = !args.no_graphics;
    let attach_opts = recover_attach_options(&conn_path);
    drive_repl(
        ReplTarget::Attach {
            connection_path: &conn_path,
            session_id,
            attach_opts,
            banner: args.banner,
        },
        render_graphics,
        args.no_indent,
        args.session_name,
        args.external_client_style.into(),
        None,
    )
    .await?;
    // Attach mode never kills the kernel; we just disconnect.
    Ok(())
}

/// Where a code-running subcommand (`execute`, `send`) should get its kernel from.
enum KernelTarget {
    /// Attach to an already-running kernel.
    Attach(std::path::PathBuf),
    /// Spawn a fresh kernel and shut it down when done.
    Spawn {
        kernelspec: std::path::PathBuf,
        conn_path: Option<std::path::PathBuf>,
    },
}

/// Resolve the target and the code-to-run from the four argument slots
/// shared by `jet execute` and `jet send`.
///
/// Clap can't disambiguate positionals when one of them (`session_id`)
/// is excluded by a flag (`--kernelspec`): with `--kernelspec K 'code'`
/// clap fills `session_id="code"`. We handle that shift here. The valid
/// shapes are:
///
///   - `session_id [code]`                            → Attach
///   - `--connection-file P [code]`                   → Attach
///   - `session_id --connection-file P` (code)        → Attach, positional was code
///   - `--kernelspec K [--connection-file P] [code]`  → Spawn
///   - `--kernelspec K session_id` (code)             → Spawn, positional was code
///
/// `subcommand` is the name the conflict errors should attribute to.
fn resolve_kernel_target(
    subcommand: &str,
    session_id: Option<String>,
    connection_file: Option<std::path::PathBuf>,
    kernelspec: Option<std::path::PathBuf>,
    code: Option<String>,
) -> Result<(KernelTarget, Option<String>)> {
    if let Some(kernelspec) = kernelspec {
        // Spawn mode. `session_id` is never valid here — but if the user
        // also passed `code`, treat session_id as a real conflict; if
        // not, clap just shifted code into session_id.
        let code = match (session_id, code) {
            (Some(_), Some(_)) => crate::cli::conflict_exit(
                subcommand,
                "the argument '[SESSION_ID]' cannot be used with '--kernelspec <KERNELSPEC>'",
            ),
            (shifted, code) => code.or(shifted),
        };
        return Ok((
            KernelTarget::Spawn {
                kernelspec,
                conn_path: connection_file,
            },
            code,
        ));
    }

    // Attach mode.
    match (session_id, connection_file, code) {
        (Some(id), None, code) => Ok((
            KernelTarget::Attach(SessionStore::default()?.open(&id)?.connection_file_path()),
            code,
        )),
        (None, Some(path), code) => Ok((KernelTarget::Attach(path), code)),
        // `--connection-file P shifted` → the positional was code, not session_id.
        (Some(shifted), Some(path), None) => Ok((KernelTarget::Attach(path), Some(shifted))),
        (Some(_), Some(_), Some(_)) => crate::cli::conflict_exit(
            subcommand,
            "the argument '[SESSION_ID]' cannot be used with '--connection-file <CONNECTION_FILE>'",
        ),
        (None, None, _) => crate::cli::conflict_exit(
            subcommand,
            "must provide one of '[SESSION_ID]', '--connection-file <CONNECTION_FILE>', \
             or '--kernelspec <KERNELSPEC>'",
        ),
    }
}

/// Read code from stdin if the caller didn't pass it as an argument.
/// Errors instead of blocking when stdin is a tty — `jet e <session>` with
/// no arg would otherwise hang forever waiting on EOF.
fn code_or_stdin(code: Option<String>) -> Result<String> {
    use std::io::{IsTerminal, Read};
    if let Some(c) = code {
        return Ok(c);
    }
    if std::io::stdin().is_terminal() {
        anyhow::bail!("no code given; pass it as an argument or pipe via stdin");
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

/// Open a client for one of the code-running subcommands. Returns the
/// client, the kernel_info reply, and a flag indicating whether we spawned
/// the kernel (and so should shut it down on the way out).
async fn open_target_client(
    target: KernelTarget,
    session_name: Option<&str>,
) -> Result<(Client, serde_json::Value, bool)> {
    match target {
        KernelTarget::Attach(conn_path) => {
            // Recover the SessionStore id if the path lives inside a tracked session,
            // matching `run_attach` so `client.session_id()` is populated wherever possible.
            let session_id = SessionStore::default()
                .ok()
                .and_then(|s| s.find_by_connection_file(&conn_path).ok().flatten())
                .map(|s| s.meta().session_id.clone());
            let attach_opts = recover_attach_options(&conn_path);
            let (client, info, _stream) =
                Client::attach(&conn_path, session_name, session_id, attach_opts).await?;
            Ok((client, info, false))
        }
        KernelTarget::Spawn {
            kernelspec,
            conn_path,
        } => {
            if let Some(p) = &conn_path
                && p.exists()
            {
                anyhow::bail!(
                    "connection file already exists at {}: remove it or attach to it instead",
                    p.display(),
                );
            }
            let spec = jet_core::kernel::KernelSpec::load(&kernelspec)?;
            log::info!(
                "spawning kernel (language={}, argv={:?})",
                spec.language,
                spec.argv,
            );
            // execute/send don't create SessionStore entries — they're one-shot.
            let (client, info, _stream) =
                Client::spawn(&spec, conn_path, session_name, None).await?;
            Ok((client, info, true))
        }
    }
}

pub async fn run_execute(args: ExecuteArgs) -> Result<()> {
    use std::sync::{Arc, Mutex};

    use jet_core::jupyter_protocol::ExecuteRequest;

    let ExecuteArgs {
        session_id,
        connection_file,
        kernelspec,
        code,
        silent,
        no_graphics,
        session_name,
        ..
    } = args;
    let (target, code_arg) =
        resolve_kernel_target("execute", session_id, connection_file, kernelspec, code)?;
    let code = code_or_stdin(code_arg)?;
    // Client::spawn / Client::attach perform a kernel_info handshake — that's
    // the fast-fail probe that confirms the kernel is answering. We don't install
    // a sink, so the banner isn't rendered for execute.
    let (mut session, _info, spawned) = open_target_client(target, session_name.as_deref()).await?;

    let render_graphics = !no_graphics;
    if render_graphics {
        crate::render::warn_if_passthrough_off();
    }
    let (idle_tx, _idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let writer: crate::render::SharedWriter = Arc::new(Mutex::new(std::io::stdout()));
    let renderer = crate::render::Renderer::new(render_graphics, idle_tx, writer)
        .with_own_session_name(session_name);

    session
        .request(
            ExecuteRequest {
                code,
                silent,
                store_history: false,
                user_expressions: None,
                allow_stdin: false,
                stop_on_error: true,
            }
            .into(),
        )?
        .drain_to_idle(|f| {
            renderer.handle_event(jet_core::events::from_message(f.channel, &f.message))
        })
        .await?;

    if spawned {
        let _ = session.shutdown().await;
    }

    Ok(())
}

pub async fn run_send(args: SendArgs) -> Result<()> {
    use jet_core::jupyter_protocol::ExecuteRequest;

    let SendArgs {
        session_id,
        connection_file,
        kernelspec,
        code,
        silent,
        session_name,
        ..
    } = args;
    let (target, code_arg) =
        resolve_kernel_target("send", session_id, connection_file, kernelspec, code)?;
    let code = code_or_stdin(code_arg)?;
    let (mut session, _info, spawned) = open_target_client(target, session_name.as_deref()).await?;

    let mut stream = session.request(
        ExecuteRequest {
            code,
            silent,
            store_history: false,
            user_expressions: None,
            allow_stdin: false,
            stop_on_error: true,
        }
        .into(),
    )?;

    if spawned {
        // We own this kernel — wait for the cell to finish, then shut
        // down. Otherwise we'd race ZMQ teardown against the kernel
        // mid-execution and lose the work the user just asked for.
        while stream.recv().await.is_some() {}
        let _ = session.shutdown().await;
    } else {
        // Fire-and-forget against a kernel we don't own. Wait only for
        // the first routed frame (typically `status: busy` on iopub for
        // our msg_id) — that confirms the kernel has the request off
        // shell. After that, dropping the sockets is safe; the kernel
        // keeps running the cell on its own.
        let _ = stream.recv().await;
    }

    Ok(())
}

pub async fn run_stop(args: StopArgs) -> Result<()> {
    let conn_paths: Vec<std::path::PathBuf> = match (args.session_id, args.connection_file) {
        (Some(id), None) => vec![SessionStore::default()?.open(&id)?.connection_file_path()],
        (None, Some(path)) => vec![path],
        (None, None) => {
            let ids = pick_sessions_multi("Shutdown running kernels (space to toggle):").await?;
            if ids.is_empty() {
                return Ok(());
            }
            let store = SessionStore::default()?;
            ids.iter()
                .map(|id| Ok(store.open(id)?.connection_file_path()))
                .collect::<Result<Vec<_>>>()?
        }
        (Some(_), Some(_)) => {
            unreachable!("clap ArgGroup forbids passing both session_id and --connection-file")
        }
    };

    let mut last_err: Option<anyhow::Error> = None;
    for path in conn_paths {
        let client_id = make_client_id(args.session_name.as_deref());
        // `run_stop` doesn't send `^C` — only a `shutdown_request` on control —
        // so interrupt-mode/pid don't matter here.
        match Kernel::attach(&path, &client_id, AttachOptions::default()).await {
            Ok(mut kernel) => {
                if let Err(e) = kernel.shutdown().await {
                    // TODO: show session id instead of client_id
                    eprintln!("shutdown failed for {}: {e}", client_id);
                    last_err = Some(e);
                } else {
                    // TODO: show session id instead of client_id
                    println!("Shut down kernel {}", client_id);
                }
            }
            Err(e) => {
                last_err = Some(e);
            }
        }
    }
    match last_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

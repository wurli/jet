//! Subcommand handlers for `jet` (connect, attach, list-sessions, list-kernels).

use anyhow::Result;

use jet_core::kernel::Kernel;
use jet_core::manager::{SessionStatus, SessionStore};

use crate::cli::{
    AttachArgs, ConnectArgs, ExecuteArgs, ListArgs, ListKernelsArgs, StatusFilter, StopArgs,
};
use crate::pickers::{pick_kernelspec, pick_session, pick_sessions_multi};
use crate::repl::drive_repl;

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

pub async fn run_list(args: ListArgs) -> Result<()> {
    // Flip any Open sessions whose kernel has gone away to Closed before
    // we read the list. Otherwise a kernel that exited while no jet
    // process was attached (or crashed) stays falsely Open on disk.
    let store = SessionStore::default()?;
    store.probe_open().await?;

    let cwd = std::env::current_dir()?;
    let sessions: Vec<_> = store
        .list()?
        .into_iter()
        .filter(|s| match args.status {
            StatusFilter::Open => s.status == SessionStatus::Open,
            StatusFilter::Closed => s.status == SessionStatus::Closed,
            StatusFilter::All => true,
        })
        .filter(|s| args.all_dirs || s.working_dir == cwd)
        .collect();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&sessions)?);
        return Ok(());
    }

    let show_status = matches!(args.status, StatusFilter::All);
    let id_w = sessions.iter().map(|s| s.id.len()).max().unwrap_or(0);
    let name_w = sessions.iter().map(|s| s.name.len()).max().unwrap_or(0);
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
                s.id, s.name, s.created_at, st,
            );
        } else {
            println!("{:<id_w$}  {:<name_w$}  {}", s.id, s.name, s.created_at,);
        }
    }
    Ok(())
}

pub async fn run_connect(args: ConnectArgs) -> Result<()> {
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
        None => Some(SessionStore::default()?.create(
            &spec.language,
            &spec.display_name.clone().unwrap_or_default(),
            &kernelspec,
            &cwd,
        )?),
        Some(_) => None,
    };

    let conn_path = args
        .connection_file
        .clone()
        .unwrap_or_else(|| session.as_ref().unwrap().connection_file_path());

    if conn_path.exists() {
        let store = SessionStore::default()?;
        let reattach = match store.find_by_connection_file(&conn_path)? {
            Some(s) => format!("jet attach {}", s.meta().id),
            None => format!("jet attach --connection-file {}", conn_path.display()),
        };
        anyhow::bail!(
            "connection file already exists at {}: remove it or run `{reattach}` to reconnect",
            conn_path.display(),
        );
    }

    let kernel =
        Kernel::spawn(&spec, Some(conn_path.clone()), args.session_name.as_deref()).await?;
    if let (Some(pid), Some(s)) = (kernel.child_pid(), session.as_mut()) {
        s.set_kernel_pid(pid);
    }

    let render_graphics = !args.no_graphics;
    let session_id = session.as_ref().map(|s| s.meta().id.clone());
    let mut kernel_session =
        drive_repl(kernel, render_graphics, args.session_name, session_id).await?;

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
                .map(|s| s.meta().id.clone());
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
    let kernel = Kernel::attach(&conn_path, args.session_name.as_deref()).await?;
    let render_graphics = !args.no_graphics;
    drive_repl(kernel, render_graphics, args.session_name, session_id).await?;
    // Attach mode never kills the kernel; we just disconnect.
    Ok(())
}

pub async fn run_execute(args: ExecuteArgs) -> Result<()> {
    use std::io::Read;
    use std::sync::{Arc, Mutex};

    use jet_core::jupyter_protocol::{ExecuteRequest, JupyterMessage};

    // When --connection-file is given, clap may have shifted positionals
    // (filling session_id with what the user meant as code). Detect that
    // and re-interpret: the lone positional is code.
    let (conn_path, code_arg) = match (args.session_id, args.connection_file, args.code) {
        (Some(id), None, code) => (
            SessionStore::default()?.open(&id)?.connection_file_path(),
            code,
        ),
        (Some(first), Some(path), None) => (path, Some(first)),
        (None, Some(path), code) => (path, code),
        (Some(_), Some(_), Some(_)) => {
            anyhow::bail!(
                "cannot pass both a session id and --connection-file together with code; \
                 pick one target"
            )
        }
        (None, None, _) => {
            anyhow::bail!("must provide a session id or --connection-file")
        }
    };

    let code = match code_arg {
        Some(c) => c,
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    let kernel = Kernel::attach(&conn_path, args.session_name.as_deref()).await?;

    let render_graphics = !args.no_graphics;
    if render_graphics {
        crate::render::warn_if_passthrough_off();
    }
    let (idle_tx, _idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let writer: crate::render::SharedWriter = Arc::new(Mutex::new(std::io::stdout()));
    let renderer = crate::render::Renderer::new(render_graphics, idle_tx, writer)
        .with_own_session_name(args.session_name.clone());

    // KernelSession::start performs a kernel_info handshake — that's
    // the fast-fail probe that confirms the kernel is answering. We
    // don't install a sink, so the banner isn't rendered for execute.
    let (session, _info) = jet_core::client::Client::start(kernel).await?;

    let req: JupyterMessage = ExecuteRequest {
        code,
        silent: false,
        store_history: false,
        user_expressions: None,
        allow_stdin: false,
        stop_on_error: true,
    }
    .into();
    session
        .request(req)?
        .drain_to_idle(|f| {
            renderer.handle_event(jet_core::events::from_message(f.channel, &f.message))
        })
        .await?;
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
        match Kernel::attach(&path, args.session_name.as_deref()).await {
            Ok(mut kernel) => {
                if let Err(e) = kernel.shutdown().await {
                    eprintln!("shutdown failed for {}: {e}", kernel.session_id);
                    last_err = Some(e);
                } else {
                    println!("Shut down kernel {}", kernel.session_id);
                }
            }
            Err(e) => {
                // eprintln!("attach failed for {}: {e}", kernel.session_id);
                last_err = Some(e);
            }
        }
    }
    match last_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

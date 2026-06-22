//! Subcommand handlers for `jet` (connect, attach, list-sessions, list-kernels).

use anyhow::Result;

use jet_core::kernel::Kernel;
use jet_core::logger::init_logger;
use jet_core::session::{SessionStatus, SessionStore};

use crate::cli::{AttachArgs, ConnectArgs, ListArgs, ListKernelsArgs, StatusFilter};
use crate::pickers::{pick_kernelspec, pick_session};
use crate::repl::drive_repl;

pub fn run_list_kernels(args: ListKernelsArgs) -> Result<()> {
    init_logger(args.global.log.as_deref());

    let paths: Vec<_> = jet_core::kernel_spec::KernelSpec::find_valid()
        .into_iter()
        .map(|(p, _)| p)
        .collect();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&paths)?);
    } else {
        for path in &paths {
            println!("{}", path.display());
        }
    }
    Ok(())
}

pub async fn run_list(args: ListArgs) -> Result<()> {
    init_logger(args.global.log.as_deref());

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
    let created_w = sessions.iter().map(|s| s.created_at.len()).max().unwrap_or(0);
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
            println!(
                "{:<id_w$}  {:<name_w$}  {}",
                s.id, s.name, s.created_at,
            );
        }
    }
    Ok(())
}

pub async fn run_connect(args: ConnectArgs) -> Result<()> {
    init_logger(args.global.log.as_deref());

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
    let name = spec.display_name.clone().unwrap_or_default();
    let mut session =
        SessionStore::default()?.create(&spec.language, &name, &kernelspec, &cwd)?;

    // --connection-file overrides where the connection file is written;
    // otherwise it lives inside the session dir.
    let conn_path = args
        .connection_file
        .clone()
        .unwrap_or_else(|| session.connection_file_path());

    let mut kernel =
        Kernel::attach_or_spawn(&spec, &conn_path, args.session_name.as_deref()).await?;
    if let Some(pid) = kernel.child_pid() {
        session.set_kernel_pid(pid);
    }

    let render_graphics = !args.no_graphics;
    let session_id = session.meta().id.clone();
    drive_repl(
        &mut kernel,
        render_graphics,
        args.session_name,
        Some(session_id),
    )
    .await?;

    if args.persist {
        kernel.detach();
    } else {
        let _ = kernel.shutdown().await;
        session.mark_closed();
    }
    Ok(())
}

pub async fn run_attach(args: AttachArgs) -> Result<()> {
    init_logger(args.global.log.as_deref());
    let (conn_path, session_id) = match (args.session_id, args.connection_file) {
        (Some(id), None) => {
            let path = SessionStore::default()?.open(&id)?.connection_file_path();
            (path, Some(id))
        }
        (None, Some(path)) => (path, None),
        (None, None) => {
            let Some(id) = pick_session().await? else {
                return Ok(());
            };
            let path = SessionStore::default()?.open(&id)?.connection_file_path();
            (path, Some(id))
        }
        (Some(_), Some(_)) => {
            unreachable!("clap ArgGroup forbids passing both session_id and --connection-file")
        }
    };
    let mut kernel = Kernel::attach(&conn_path, args.session_name.as_deref()).await?;
    let render_graphics = !args.no_graphics;
    drive_repl(&mut kernel, render_graphics, args.session_name, session_id).await?;
    // Attach mode never kills the kernel; we just disconnect.
    Ok(())
}

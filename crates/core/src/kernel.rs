//! Jupyter kernel: spawn or attach, send/recv on shell+stdin+control+iopub.
//!
//! Replaces what was previously the kallichore `Client` + `Channel` plumbing.
//! `jet` owns the kernel process directly; runtimed's `jupyter-zmq-client`
//! handles the wire protocol and HMAC signing.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, anyhow, bail};
use jupyter_protocol::{ConnectionInfo, JupyterMessage};
use jupyter_zmq_client::{
    ClientControlConnection, ClientHeartbeatConnection, ClientIoPubConnection,
    ClientShellConnection, ClientStdinConnection, create_client_control_connection,
    create_client_heartbeat_connection, create_client_iopub_connection,
    create_client_shell_connection_with_identity, create_client_stdin_connection_with_identity,
    peer_identity_for_session,
};
use rand::Rng;
use tokio::process::{Child, Command};

use crate::connection_file;
pub use crate::kernel_spec::{InterruptMode, KernelSpec};

/// RAII guard for the kernel process. Drop kills + waits unless `detach`
/// has been called. Matches the old `kallichore::server::ChildGuard`
/// pattern, but for `tokio::process::Child`.
pub struct ChildGuard {
    child: Option<Child>,
    detached: bool,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self {
            child: Some(child),
            detached: false,
        }
    }

    /// Leave the kernel running when this guard drops.
    pub fn detach(&mut self) {
        self.detached = true;
    }

    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().and_then(Child::id)
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.detached {
            return;
        }
        if let Some(mut c) = self.child.take() {
            // start_kill is non-blocking; the OS reaps after we exit.
            let _ = c.start_kill();
        }
    }
}

/// Out-of-process reaper that kills the kernel pgrp if jet dies without
/// running drop glue (SIGKILL, terminal close → SIGHUP with no handler,
/// segfault). Implementation: fork() a tiny peer that blocks reading
/// the read end of a pipe; jet holds the write end. However jet's
/// process-table entry disappears, the write end closes, the peer
/// wakes with EOF and SIGTERMs (then SIGKILLs) the kernel group.
///
/// On any path where jet's Drop chain *does* run — graceful exit or
/// [`Kernel::detach`] — Drop here SIGKILLs the watchdog before it sees
/// EOF, so it doesn't redundantly (or, worse, racily against a recycled
/// pid) kill the kernel itself.
#[cfg(unix)]
struct Watchdog {
    pid: libc::pid_t,
    /// Write end of the EOF pipe. Holding it open keeps the peer parked
    /// in read(); closing it is what wakes the peer on jet's death.
    _write_fd: std::os::fd::OwnedFd,
}

#[cfg(unix)]
impl Watchdog {
    /// Fork a watchdog that will kill `kernel_pgid` (negated to address
    /// the whole group) on EOF over its inherited pipe.
    ///
    /// Safety: fork() in a multithreaded program is allowed, but the
    /// child must use only async-signal-safe libc calls until exec or
    /// _exit. We confine the child to `close`/`read`/`nanosleep`/
    /// `kill`/`_exit`, all of which are signal-safe.
    fn spawn(kernel_pgid: libc::pid_t) -> std::io::Result<Self> {
        use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

        let mut fds = [0i32; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let read_fd = unsafe { OwnedFd::from_raw_fd(fds[0]) };
        let write_fd = unsafe { OwnedFd::from_raw_fd(fds[1]) };

        match unsafe { libc::fork() } {
            -1 => Err(std::io::Error::last_os_error()),
            0 => unsafe { watchdog_child(read_fd.as_raw_fd(), write_fd.as_raw_fd(), kernel_pgid) },
            pid => Ok(Self {
                pid,
                _write_fd: write_fd,
            }),
        }
    }
}

/// Forked watchdog body — runs only in the child after fork(). Returns
/// `!` because every exit path goes through `_exit`. Uses only async-
/// signal-safe libc calls.
#[cfg(unix)]
unsafe fn watchdog_child(read_fd: i32, write_fd: i32, kernel_pgid: libc::pid_t) -> ! {
    unsafe {
        libc::close(write_fd);
        let mut buf = [0u8; 1];
        loop {
            let n = libc::read(read_fd, buf.as_mut_ptr() as *mut _, 1);
            if n == 0 {
                // EOF: parent gone. SIGTERM, brief grace, SIGKILL.
                libc::kill(-kernel_pgid, libc::SIGTERM);
                let ts = libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 500_000_000,
                };
                libc::nanosleep(&ts, std::ptr::null_mut());
                libc::kill(-kernel_pgid, libc::SIGKILL);
                libc::_exit(0);
            }
            // n < 0 → EINTR or a real error; we can't read errno
            // portably from inside a fork()ed child without pulling
            // in OS-specific symbols. Loop until EOF. n > 0 is
            // unused — nobody writes to the pipe.
        }
    }
}

#[cfg(unix)]
impl Drop for Watchdog {
    fn drop(&mut self) {
        // SIGKILL + reap. Reached only on paths where jet's Drop chain
        // runs — graceful exit (ChildGuard has already killed the
        // kernel) or detach (the kernel should outlive us). Either
        // way, we want the watchdog gone before its pipe-EOF fires a
        // now-redundant kill.
        unsafe {
            libc::kill(self.pid, libc::SIGKILL);
            let mut status = 0;
            let _ = libc::waitpid(self.pid, &mut status, 0);
        }
    }
}

/// Where the connection file lives — temp (cleaned up on drop) or a
/// caller-chosen persistent path (left in place so a later `attach` can
/// find it).
enum ConnectionPath {
    OwnedTemp(PathBuf),
    Persistent(PathBuf),
}

impl ConnectionPath {
    fn as_path(&self) -> &Path {
        match self {
            ConnectionPath::OwnedTemp(p) | ConnectionPath::Persistent(p) => p,
        }
    }
}

impl Drop for ConnectionPath {
    fn drop(&mut self) {
        if let ConnectionPath::OwnedTemp(p) = self {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Sibling log file path for a given connection file: `foo.json` →
/// `foo.json.log`. Persistent across detach so a later `attach` can tail
/// it.
pub fn log_path_for(connection_path: &Path) -> PathBuf {
    let mut s = connection_path.as_os_str().to_owned();
    s.push(".log");
    PathBuf::from(s)
}

/// The five ZMQ client connections. `Client::start_with_sink` moves shell/iopub/stdin
/// into background tasks (and, for attached kernels, heartbeat into a liveness watcher);
/// control stays on the `Kernel` so `interrupt()` and `shutdown()` can use it. The slots
/// are `Option`s so the owning Client can `.take()` them; for spawned kernels heartbeat
/// is never taken (waitpid is used instead) and just drops with the kernel.
#[derive(Default)]
pub struct Channels {
    pub shell: Option<ClientShellConnection>,
    pub iopub: Option<ClientIoPubConnection>,
    pub stdin: Option<ClientStdinConnection>,
    pub control: Option<ClientControlConnection>,
    pub heartbeat: Option<ClientHeartbeatConnection>,
}

pub struct Kernel {
    /// Some when we spawned the kernel ourselves; None when we attached.
    child: Option<ChildGuard>,
    /// Connection file path. Tempfiles get cleaned up on drop.
    _connection_path: ConnectionPath,
    pub interrupt_mode: InterruptMode,
    pub channels: Channels,
    /// Path to the on-disk log file capturing the kernel's stderr.
    /// `Some` whenever the connection file lives on a persistent path
    /// (so it survives detach for later inspection / a future attach);
    /// `None` for temp-path spawns. Cleaned up on graceful shutdown.
    pub log_file_path: Option<PathBuf>,
    /// Out-of-process reaper armed to kill the kernel pgrp if jet dies
    /// without running drop glue (terminal closed, SIGKILL, crash).
    /// Disarmed on [`Kernel::detach`]; replaced with the no-op
    /// `Watchdog::Drop` on normal shutdown.
    #[cfg(unix)]
    watchdog: Option<Watchdog>,
}

impl Kernel {
    /// Spawn a kernel from the spec, generate a connection file, and bring
    /// up all four ZMQ client sockets.
    ///
    /// `connection_path` chooses where the file lives. `None` → a tempfile
    /// scoped to this kernel's lifetime. `Some(path)` → that exact path,
    /// preserved when the kernel is later detached or attached to.
    pub async fn spawn(
        spec: &KernelSpec,
        connection_path: Option<PathBuf>,
        client_id: &str,
    ) -> Result<Self> {
        let conn_path = match connection_path {
            Some(p) => ConnectionPath::Persistent(p),
            None => ConnectionPath::OwnedTemp(default_temp_path()),
        };
        let info = connection_file::generate(conn_path.as_path())?;

        // Persistent connection paths get a sibling log file so a later
        // `jet attach` can tail the kernel's stderr. Temp/owned paths
        // keep stderr in-process: nothing else will ever attach to them.
        let log_file_path = match &conn_path {
            ConnectionPath::Persistent(p) => Some(log_path_for(p)),
            ConnectionPath::OwnedTemp(_) => None,
        };

        let mut command = build_kernel_command(spec, conn_path.as_path())?;
        if let Some(p) = &log_file_path {
            let f = std::fs::File::create(p)
                .with_context(|| format!("creating kernel log file {}", p.display()))?;
            command.stderr(Stdio::from(f));
        }
        // Put the kernel in its own process group so a ^C at the tty
        // (cooked-mode SIGINT to the foreground pgrp) doesn't reach it
        // until we explicitly forward via interrupt().
        #[cfg(unix)]
        {
            command.process_group(0);
        }
        log::info!("spawning kernel: {:?}", spec.argv);
        let child = command.spawn().with_context(|| {
            format!(
                "running startup command given by kernelspec `{}`",
                spec.argv.join(" ")
            )
        })?;
        let guard = ChildGuard::new(child);

        let channels = match connect_channels(&info, client_id).await {
            Ok(c) => c,
            Err(e) => {
                // The most common cause of channel-start failure is
                // the kernel exiting before opening its ports. Give
                // the OS a beat to mark the child dead so child_alive
                // reports honestly, then enrich.
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let alive = match guard.id() {
                    Some(pid) => unsafe { libc::kill(pid as libc::pid_t, 0) == 0 },
                    None => true,
                };
                return Err(enrich_startup_error(
                    e,
                    guard.id(),
                    alive,
                    log_file_path.as_deref(),
                ));
            }
        };

        // Arm an out-of-process watchdog that kills the kernel pgrp if
        // jet's process dies without running destructors (terminal
        // closed → SIGHUP, SIGKILL, segfault, etc.). Best-effort: if
        // fork fails we log and continue — leaking a kernel on crash
        // is worse than dropping this safety net, but failing to start
        // because of it would be far worse.
        #[cfg(unix)]
        let watchdog = match guard.id() {
            Some(pid) => match Watchdog::spawn(pid as libc::pid_t) {
                Ok(w) => Some(w),
                Err(e) => {
                    log::warn!("failed to start kernel watchdog: {e}");
                    None
                }
            },
            None => None,
        };

        Ok(Self {
            child: Some(guard),
            _connection_path: conn_path,
            interrupt_mode: spec.interrupt_mode,
            channels,
            log_file_path,
            #[cfg(unix)]
            watchdog,
        })
    }

    /// Build a degenerate [`Kernel`] for tests — no ZMQ channels, no
    /// child process, with a caller-supplied log path. Used by
    /// `kernel_session::tests` to exercise the startup-error
    /// enrichment path without paying zeromq-rs's 30s start
    /// timeout against a non-listening peer.
    #[cfg(test)]
    pub fn synthetic_for_test(log_file_path: Option<PathBuf>) -> Self {
        Self {
            child: None,
            _connection_path: ConnectionPath::OwnedTemp(default_temp_path()),
            interrupt_mode: InterruptMode::Signal,
            channels: Channels::default(),
            log_file_path,
            #[cfg(unix)]
            watchdog: None,
        }
    }

    /// Attach to an already-running kernel via its connection file. We do
    /// not own the child process; dropping this `Kernel` does not stop
    /// the kernel.
    pub async fn attach(connection_path: &Path, client_id: &str) -> Result<Self> {
        let info = connection_file::read(connection_path)?;
        // ZMQ DEALER/SUB sockets start to dead endpoints without
        // complaint and just queue forever, so probe the shell port
        // with a plain TCP start first to fail fast when the kernel
        // recorded in the connection file is no longer alive.
        probe_kernel_alive(&info).await?;
        let channels = connect_channels(&info, client_id).await?;
        let log_path = log_path_for(connection_path);
        let log_file_path = log_path.exists().then_some(log_path);
        Ok(Self {
            child: None,
            _connection_path: ConnectionPath::Persistent(connection_path.to_path_buf()),
            // No kernelspec on attach — assume signal-mode so ^C goes to
            // the kernel pgid. Override via a dedicated method if a use
            // case appears.
            interrupt_mode: InterruptMode::Signal,
            channels,
            log_file_path,
            #[cfg(unix)]
            watchdog: None,
        })
    }

    /// Stop killing the child on drop. Use before exiting `jet` when the
    /// caller wants the kernel to outlive the process. No-op for attached
    /// kernels.
    pub fn detach(&mut self) {
        if let Some(g) = self.child.as_mut() {
            g.detach();
        }
        // Drop the watchdog so the kernel survives jet's exit. Drop
        // SIGKILLs + reaps before the peer can read EOF and fire.
        #[cfg(unix)]
        {
            self.watchdog.take();
        }
    }

    /// PID of the spawned child, if any.
    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(ChildGuard::id)
    }

    /// `true` if the spawned child still exists. Sends signal 0 with
    /// `kill(pid, 0)` — non-destructive liveness probe. Returns `true`
    /// for attached kernels (we can't tell from this side; rely on
    /// socket error to surface the death).
    pub fn child_alive(&self) -> bool {
        let Some(pid) = self.child_pid() else {
            return true;
        };
        #[cfg(unix)]
        unsafe {
            libc::kill(pid as libc::pid_t, 0) == 0
        }
        #[cfg(not(unix))]
        true
    }

    /// `true` if we own a child kernel that we're keeping alive.
    pub fn is_attached(&self) -> bool {
        self.child.is_none()
    }

    /// Forward a ^C-equivalent to the kernel.
    ///
    /// Spec-driven: `signal` mode kernels (the default) want SIGINT;
    /// `message` mode kernels want an `interrupt_request` on control.
    pub async fn interrupt(&mut self) -> Result<()> {
        match self.interrupt_mode {
            InterruptMode::Signal => self.interrupt_signal(),
            InterruptMode::Message => {
                let msg: JupyterMessage = jupyter_protocol::InterruptRequest::default().into();
                let control = self
                    .channels
                    .control
                    .as_mut()
                    .ok_or_else(|| anyhow!("control channel taken — cannot send interrupt"))?;
                control
                    .send(msg)
                    .await
                    .map_err(|e| anyhow!("control.send: {e}"))?;
                Ok(())
            }
        }
    }

    fn interrupt_signal(&self) -> Result<()> {
        let Some(pid) = self.child_pid() else {
            // Attached or already gone — nothing to signal.
            return Ok(());
        };
        // We launched the kernel via setsid(), so it's the leader of its
        // own session. Send SIGINT to that process group.
        #[cfg(unix)]
        unsafe {
            // Negate the pgid to address the whole group.
            let pgid: libc::pid_t = pid as libc::pid_t;
            if libc::kill(-pgid, libc::SIGINT) != 0 {
                let err = std::io::Error::last_os_error();
                return Err(anyhow!("kill -INT {pgid}: {err}"));
            }
        }
        Ok(())
    }

    /// Best-effort graceful shutdown: send `shutdown_request` on control,
    /// give the kernel a moment to react. The caller should drop the
    /// `Kernel` after this returns (or call [`Kernel::detach`] first to
    /// keep the kernel running).
    pub async fn shutdown(&mut self) -> Result<()> {
        log::debug!("Sending shutdown_request to kernel");
        let req = jupyter_protocol::ShutdownRequest { restart: false };
        let msg: JupyterMessage = req.into();
        if let Some(control) = self.channels.control.as_mut()
            && let Err(e) = control.send(msg).await
        {
            log::warn!("shutdown_request send failed: {e}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        // Clean up the on-disk stderr log on graceful shutdown. Detach
        // skips this path, so detached kernels leave the log in place
        // for a future `attach` to tail.
        if let Some(p) = self.log_file_path.take() {
            let _ = std::fs::remove_file(p);
        }
        Ok(())
    }
}

/// Decorate a startup-time failure with extra context the bare error
/// can't carry: whether the spawned child has already exited, and a
/// tail of the kernel's stderr log (when one was created).
///
/// "Connection refused" on a fresh attach, "Connect timed out after
/// 30s" from zeromq-rs's start, or "timed out waiting for
/// kernel_info_reply" from the handshake are all useless on their
/// own. Most real-world startup failures show up in the kernel's
/// stderr — a Python ImportError, a missing R library, an
/// interpreter that can't find its prefix. Surface that here so the
/// user doesn't have to know about the log file.
pub fn enrich_startup_error(
    err: anyhow::Error,
    child_pid: Option<u32>,
    child_alive: bool,
    log_path: Option<&Path>,
) -> anyhow::Error {
    let mut parts = vec![err.to_string()];

    if let Some(pid) = child_pid
        && !child_alive
    {
        parts.push(format!("kernel process (pid {pid}) has already exited"));
    }

    if let Some(path) = log_path {
        match std::fs::read_to_string(path) {
            Ok(s) if !s.trim().is_empty() => {
                let tail: Vec<&str> = s.lines().rev().take(20).collect();
                let tail = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
                parts.push(format!("kernel stderr (tail):\n{tail}"));
            }
            _ => {}
        }
    }

    anyhow!(parts.join("\n\n"))
}

/// Quick liveness check: TCP-start to the shell port with a short
/// timeout. Returns `Err` if the kernel's no longer listening, so
/// external probers (session list self-heal) can check liveness
/// without constructing a full `Kernel`.
pub async fn probe_kernel_alive(info: &ConnectionInfo) -> Result<()> {
    use jupyter_protocol::Transport;
    if !matches!(info.transport, Transport::TCP) {
        return Ok(());
    }
    let addr = format!("{}:{}", info.ip, info.shell_port);
    let start = tokio::net::TcpStream::connect(&addr);
    match tokio::time::timeout(std::time::Duration::from_millis(200), start).await {
        Ok(Ok(_stream)) => Ok(()),
        Ok(Err(e)) => Err(anyhow!("kernel not reachable at {addr}: {e}")),
        Err(_) => Err(anyhow!("kernel probe timed out at {addr}")),
    }
}

async fn connect_channels(info: &ConnectionInfo, session_id: &str) -> Result<Channels> {
    let identity =
        peer_identity_for_session(session_id).map_err(|e| anyhow!("peer_identity: {e}"))?;
    let shell = create_client_shell_connection_with_identity(info, session_id, identity.clone())
        .await
        .map_err(|e| anyhow!("shell start: {e}"))?;
    // Empty topic: subscribe to all iopub messages.
    let iopub = create_client_iopub_connection(info, "", session_id)
        .await
        .map_err(|e| anyhow!("iopub start: {e}"))?;
    let stdin = create_client_stdin_connection_with_identity(info, session_id, identity)
        .await
        .map_err(|e| anyhow!("stdin start: {e}"))?;
    let control = create_client_control_connection(info, session_id)
        .await
        .map_err(|e| anyhow!("control start: {e}"))?;
    let heartbeat = create_client_heartbeat_connection(info)
        .await
        .map_err(|e| anyhow!("heartbeat start: {e}"))?;
    Ok(Channels {
        shell: Some(shell),
        iopub: Some(iopub),
        stdin: Some(stdin),
        control: Some(control),
        heartbeat: Some(heartbeat),
    })
}

fn build_kernel_command(spec: &KernelSpec, connection_path: &Path) -> Result<Command> {
    if spec.argv.is_empty() {
        bail!("kernelspec argv is empty");
    }
    let mut cmd = Command::new(&spec.argv[0]);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for arg in &spec.argv[1..] {
        if arg == "{connection_file}" {
            cmd.arg(connection_path.as_os_str());
        } else {
            cmd.arg(OsStr::new(arg));
        }
    }
    // tokio::Command inherits the parent env by default; spec entries
    // are layered on top and win on conflict.
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    Ok(cmd)
}

fn default_temp_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "jet-conn-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::path::Path;

    use super::{InterruptMode, KernelSpec, build_kernel_command};

    fn spec_with_env(env: &[(&str, &str)]) -> KernelSpec {
        KernelSpec {
            argv: vec!["/bin/true".to_string(), "{connection_file}".to_string()],
            language: "python".to_string(),
            display_name: None,
            interrupt_mode: InterruptMode::Signal,
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            metadata: HashMap::new(),
            kernel_protocol_version: None,
        }
    }

    fn cmd_env(cmd: &tokio::process::Command) -> HashMap<OsString, OsString> {
        cmd.as_std()
            .get_envs()
            .filter_map(|(k, v)| v.map(|v| (k.to_os_string(), v.to_os_string())))
            .collect()
    }

    #[test]
    fn spec_env_overrides_parent_env_on_conflict() {
        // SAFETY: tests in the same process share env; the keys we set are
        // unique to this test so they won't collide with other tests.
        unsafe {
            // Same key is also in the spec — spec must win.
            std::env::set_var("JET_TEST_OVERRIDE_PROBE", "from-parent");
        }
        let spec = spec_with_env(&[
            ("JET_TEST_SPEC_ONLY", "from-spec"),
            ("JET_TEST_OVERRIDE_PROBE", "from-spec"),
        ]);
        let cmd = build_kernel_command(&spec, Path::new("/tmp/conn.json")).unwrap();
        let env = cmd_env(&cmd);

        assert_eq!(
            env.get(OsString::from("JET_TEST_SPEC_ONLY").as_os_str()),
            Some(&OsString::from("from-spec")),
            "spec-only key should be present",
        );
        assert_eq!(
            env.get(OsString::from("JET_TEST_OVERRIDE_PROBE").as_os_str()),
            Some(&OsString::from("from-spec")),
            "spec must override parent on conflict",
        );
    }
}

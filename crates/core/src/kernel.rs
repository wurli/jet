//! Jupyter kernel: spawn or attach, send/recv on shell+stdin+control+iopub.
//!
//! Replaces what was previously the kallichore `Client` + `Channel` plumbing.
//! `jet` owns the kernel process directly; runtimed's `jupyter-zmq-client`
//! handles the wire protocol and HMAC signing.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, anyhow, bail};
use jupyter_protocol::{ConnectionInfo, JupyterMessage};
use jupyter_zmq_client::{
    ClientControlConnection, ClientIoPubConnection, ClientShellConnection, ClientStdinConnection,
    create_client_control_connection, create_client_iopub_connection,
    create_client_shell_connection_with_identity, create_client_stdin_connection_with_identity,
    peer_identity_for_session,
};
use rand::Rng;
use serde::Deserialize;
use tokio::process::{Child, Command};

use crate::connection_file;

/// Per the Jupyter kernelspec: how the kernel expects to be interrupted.
/// `Signal` (default) means SIGINT; `Message` means an `interrupt_request`
/// on the control channel.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InterruptMode {
    #[default]
    Signal,
    Message,
}

/// A parsed Jupyter `kernel.json` kernelspec.
#[derive(Debug, Deserialize)]
pub struct KernelSpec {
    pub argv: Vec<String>,
    pub language: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub interrupt_mode: InterruptMode,
}

impl KernelSpec {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading kernelspec at {}", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing kernelspec at {}", path.display()))
    }
}

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

/// The four ZMQ client connections. Stored as `Option`s so callers can
/// `take_*()` ownership of one socket for a long-running task without
/// borrowing the whole `Kernel`. Once taken, the slot stays `None`; the
/// `Kernel` is the source of truth for which channels are still in-house.
#[derive(Default)]
pub struct Channels {
    pub shell: Option<ClientShellConnection>,
    pub iopub: Option<ClientIoPubConnection>,
    pub stdin: Option<ClientStdinConnection>,
    pub control: Option<ClientControlConnection>,
}

impl Channels {
    pub fn take_shell(&mut self) -> Result<ClientShellConnection> {
        self.shell.take().ok_or_else(|| anyhow!("shell channel already taken"))
    }
    pub fn take_iopub(&mut self) -> Result<ClientIoPubConnection> {
        self.iopub.take().ok_or_else(|| anyhow!("iopub channel already taken"))
    }
    pub fn take_stdin(&mut self) -> Result<ClientStdinConnection> {
        self.stdin.take().ok_or_else(|| anyhow!("stdin channel already taken"))
    }
}

pub struct Kernel {
    /// Some when we spawned the kernel ourselves; None when we attached.
    child: Option<ChildGuard>,
    /// Connection file path. Tempfiles get cleaned up on drop.
    _connection_path: ConnectionPath,
    pub session_id: String,
    pub interrupt_mode: InterruptMode,
    pub channels: Channels,
}

impl Kernel {
    /// Spawn a kernel from the spec, generate a connection file, and bring
    /// up all four ZMQ client sockets.
    ///
    /// `connection_path` chooses where the file lives. `None` → a tempfile
    /// scoped to this kernel's lifetime. `Some(path)` → that exact path,
    /// preserved when the kernel is later detached or attached to.
    pub async fn spawn(spec: &KernelSpec, connection_path: Option<PathBuf>) -> Result<Self> {
        let conn_path = match connection_path {
            Some(p) => ConnectionPath::Persistent(p),
            None => ConnectionPath::OwnedTemp(default_temp_path()),
        };
        let info = connection_file::generate(conn_path.as_path())?;

        let mut command = build_kernel_command(spec, conn_path.as_path())?;
        // Put the kernel in its own process group so a ^C at the tty
        // (cooked-mode SIGINT to the foreground pgrp) doesn't reach it
        // until we explicitly forward via interrupt().
        #[cfg(unix)]
        {
            command.process_group(0);
        }
        log::info!("spawning kernel: {:?}", spec.argv);
        let child = command
            .spawn()
            .with_context(|| format!("spawning kernel {:?}", spec.argv.first()))?;
        let guard = ChildGuard::new(child);

        let session_id = format!("jet-{:x}", rand::thread_rng().gen::<u64>());
        let channels = connect_channels(&info, &session_id).await?;

        Ok(Self {
            child: Some(guard),
            _connection_path: conn_path,
            session_id,
            interrupt_mode: spec.interrupt_mode,
            channels,
        })
    }

    /// Attach to an already-running kernel via its connection file. We do
    /// not own the child process; dropping this `Kernel` does not stop
    /// the kernel.
    pub async fn attach(connection_path: &Path) -> Result<Self> {
        let info = connection_file::read(connection_path)?;
        let session_id = format!("jet-{:x}", rand::thread_rng().gen::<u64>());
        let channels = connect_channels(&info, &session_id).await?;
        Ok(Self {
            child: None,
            _connection_path: ConnectionPath::Persistent(connection_path.to_path_buf()),
            session_id,
            // No kernelspec on attach — assume signal-mode so ^C goes to
            // the kernel pgid. Override via a dedicated method if a use
            // case appears.
            interrupt_mode: InterruptMode::Signal,
            channels,
        })
    }

    /// Stop killing the child on drop. Use before exiting `jet` when the
    /// caller wants the kernel to outlive the process. No-op for attached
    /// kernels.
    pub fn detach(&mut self) {
        if let Some(g) = self.child.as_mut() {
            g.detach();
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
                let msg: JupyterMessage =
                    jupyter_protocol::InterruptRequest::default().into();
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
        let req = jupyter_protocol::ShutdownRequest { restart: false };
        let msg: JupyterMessage = req.into();
        if let Some(control) = self.channels.control.as_mut() {
            if let Err(e) = control.send(msg).await {
                log::warn!("shutdown_request send failed: {e}");
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        Ok(())
    }
}

async fn connect_channels(info: &ConnectionInfo, session_id: &str) -> Result<Channels> {
    let identity =
        peer_identity_for_session(session_id).map_err(|e| anyhow!("peer_identity: {e}"))?;
    let shell = create_client_shell_connection_with_identity(info, session_id, identity.clone())
        .await
        .map_err(|e| anyhow!("shell connect: {e}"))?;
    // Empty topic: subscribe to all iopub messages.
    let iopub = create_client_iopub_connection(info, "", session_id)
        .await
        .map_err(|e| anyhow!("iopub connect: {e}"))?;
    let stdin = create_client_stdin_connection_with_identity(info, session_id, identity)
        .await
        .map_err(|e| anyhow!("stdin connect: {e}"))?;
    let control = create_client_control_connection(info, session_id)
        .await
        .map_err(|e| anyhow!("control connect: {e}"))?;
    Ok(Channels {
        shell: Some(shell),
        iopub: Some(iopub),
        stdin: Some(stdin),
        control: Some(control),
    })
}

fn build_kernel_command(spec: &KernelSpec, connection_path: &Path) -> Result<Command> {
    if spec.argv.is_empty() {
        bail!("kernelspec argv is empty");
    }
    let mut cmd = Command::new(&spec.argv[0]);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    for arg in &spec.argv[1..] {
        if arg == "{connection_file}" {
            cmd.arg(connection_path.as_os_str());
        } else {
            cmd.arg(OsStr::new(arg));
        }
    }
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    Ok(cmd)
}

fn default_temp_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "jet-conn-{:x}.json",
        rand::thread_rng().gen::<u64>()
    ))
}

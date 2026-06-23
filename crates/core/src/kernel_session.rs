//! Single-kernel session: owns one [`Kernel`], spawns the long-lived
//! reader/writer tasks for shell/iopub/stdin, and demuxes incoming
//! frames by `parent_msg_id` so callers can fire many concurrent
//! requests and only see frames for their own.
//!
//! This is the shared core of what used to be open-coded in two places:
//! the Lua binding's `boot_kernel` + `FrameRouter`, and the CLI REPL's
//! three `tokio::spawn` blocks. Both collapse to a [`KernelSession`]:
//! the Lua side wraps it in `Arc<Mutex<_>>` because its sync callers
//! need shared access; the CLI owns it by value.
//!
//! `KernelSession` is single-session by design — one kernel, one
//! session. Multi-session bookkeeping (e.g. a session-id → session
//! registry) is the consumer's problem.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use crossbeam_channel::{Receiver as XReceiver, Sender as XSender, TryRecvError, unbounded};
use jupyter_protocol::{JupyterMessage, KernelInfoRequest};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{Notify, mpsc::UnboundedReceiver, mpsc::UnboundedSender, mpsc::unbounded_channel};

use crate::events::{Channel, EventData, from_message};
use crate::kernel::Kernel;
use crate::jupyter_zmq_client::{
    ClientIoPubConnection, ClientShellConnection, ClientStdinConnection,
};

/// One routed frame for a particular in-flight request, tagged with
/// the ZMQ channel it arrived on (so consumers can reconstruct typed
/// [`crate::events::Event`]s — `JupyterMessage::channel` is `None` for
/// ZMQ transports).
///
/// `Idle` is the terminal item — once delivered, the per-request channel
/// is torn down and further pulls from the stream return `None`.
pub struct Frame {
    pub channel: Channel,
    pub message: JupyterMessage,
}

enum RoutedFrame {
    Frame(Frame),
    Idle,
}

/// Demuxes kernel frames by `parent_msg_id`. Each in-flight request gets
/// one slot; the reader tasks dispatch into it until they see a matching
/// `status: idle`, which closes the slot.
struct FrameRouter {
    by_parent: Mutex<HashMap<String, XSender<RoutedFrame>>>,
}

impl FrameRouter {
    fn new() -> Self {
        Self {
            by_parent: Mutex::new(HashMap::new()),
        }
    }

    fn register(&self, parent_msg_id: String) -> XReceiver<RoutedFrame> {
        let (tx, rx) = unbounded();
        self.by_parent.lock().unwrap().insert(parent_msg_id, tx);
        rx
    }

    fn forget(&self, parent_msg_id: &str) {
        self.by_parent.lock().unwrap().remove(parent_msg_id);
    }

    /// Route one parsed message. Idle for a registered parent closes the
    /// slot; content for an unregistered parent (e.g. kernel-initiated
    /// `comm_msg`) is dropped — no consumer for it at this layer.
    fn dispatch(&self, channel: Channel, msg: JupyterMessage) {
        let parent_id = msg.parent_header.as_ref().map(|h| h.msg_id.clone());

        // Idle is special: it's the per-request terminator.
        if let EventData::Idle { parent_id: idle_parent } = from_message(channel, &msg).data {
            if let Some(pid) = idle_parent {
                if let Some(tx) = self.by_parent.lock().unwrap().remove(&pid) {
                    let _ = tx.send(RoutedFrame::Idle);
                }
            }
            return;
        }

        let Some(pid) = parent_id else {
            return;
        };
        let sender = self.by_parent.lock().unwrap().get(&pid).cloned();
        if let Some(tx) = sender {
            let _ = tx.send(RoutedFrame::Frame(Frame { channel, message: msg }));
        }
    }
}

/// A long-lived session over a single [`Kernel`].
///
/// After [`KernelSession::start`] returns, the shell/iopub/stdin sockets
/// have been moved into background tasks; the [`Kernel`] retains only
/// `control` + `heartbeat` + (for spawned kernels) the child process
/// guard, which is what [`KernelSession::interrupt`] /
/// [`KernelSession::shutdown`] need.
pub struct KernelSession {
    kernel: Kernel,
    shell_tx: UnboundedSender<JupyterMessage>,
    stdin_tx: UnboundedSender<JupyterMessage>,
    router: Arc<FrameRouter>,
    /// Notified once when any of the reader/writer tasks observes a
    /// fatal socket error or task shutdown. Lets consumers (REPL,
    /// liveness watchers) react to a kernel that has gone away.
    closed: Arc<Notify>,
}

impl KernelSession {
    /// Take the shell/iopub/stdin channels out of the kernel, send a
    /// `kernel_info_request` synchronously, and spawn the long-running
    /// reader/writer tasks. Returns the session and the kernel-info
    /// reply content as JSON (the runtimed reply struct, serialized).
    pub async fn start(mut kernel: Kernel) -> Result<(Self, Value)> {
        let mut shell = kernel.channels.take_shell()?;
        let iopub = kernel.channels.take_iopub()?;
        let stdin_sock = kernel.channels.take_stdin()?;

        let info_req: JupyterMessage = KernelInfoRequest {}.into();
        let info_id = info_req.header.msg_id.clone();
        shell.send(info_req).await
            .map_err(|e| anyhow!("shell.send: {e}"))
            .context("sending kernel_info_request")?;
        let info = match tokio::time::timeout(
            Duration::from_secs(10),
            await_kernel_info(&mut shell, &info_id),
        )
        .await
        {
            Ok(r) => r?,
            Err(_) => anyhow::bail!("timed out waiting for kernel_info_reply"),
        };

        let (shell_tx, shell_rx) = unbounded_channel::<JupyterMessage>();
        let (stdin_tx, stdin_rx) = unbounded_channel::<JupyterMessage>();
        let router = Arc::new(FrameRouter::new());
        let closed = Arc::new(Notify::new());

        spawn_shell_loop(shell, shell_rx, router.clone(), closed.clone());
        spawn_iopub_reader(iopub, router.clone(), closed.clone());
        spawn_stdin_loop(stdin_sock, stdin_rx, router.clone(), closed.clone());

        Ok((
            Self {
                kernel,
                shell_tx,
                stdin_tx,
                router,
                closed,
            },
            info,
        ))
    }

    /// Send a shell-channel request and return a stream of its routed
    /// frames. The stream ends when the kernel reports `status: idle`
    /// matching this request's `msg_id`.
    pub fn request(&self, msg: JupyterMessage) -> Result<RequestStream> {
        let msg_id = msg.header.msg_id.clone();
        let rx = self.router.register(msg_id.clone());
        self.shell_tx
            .send(msg)
            .map_err(|e| anyhow!("shell_tx send: {e}"))?;
        Ok(RequestStream {
            msg_id,
            rx: Some(rx),
            router: self.router.clone(),
        })
    }

    /// Send an `input_reply` (or other stdin-channel message). Jupyter
    /// pairs replies with the in-flight `input_request` by proximity on
    /// the stdin channel, so no msg_id routing is involved.
    pub fn reply_stdin(&self, msg: JupyterMessage) -> Result<()> {
        self.stdin_tx
            .send(msg)
            .map_err(|e| anyhow!("stdin_tx send: {e}"))?;
        Ok(())
    }

    /// Notified once when the session loses contact with the kernel
    /// (socket error in any reader/writer task). The CLI REPL selects on
    /// this alongside its prompt; the Lua binding ignores it and lets
    /// individual request streams close naturally.
    pub fn closed(&self) -> Arc<Notify> {
        self.closed.clone()
    }

    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    pub fn kernel_mut(&mut self) -> &mut Kernel {
        &mut self.kernel
    }

    /// Forward a ^C-equivalent to the kernel.
    pub async fn interrupt(&mut self) -> Result<()> {
        self.kernel.interrupt().await
    }

    /// Shutdown the kernel (best-effort). Consumes the session because
    /// the reader/writer tasks will tear down once their sockets close.
    pub async fn shutdown(mut self) -> Result<()> {
        self.kernel.shutdown().await
    }
}

/// Stream of frames for one in-flight request.
///
/// Two pull surfaces:
/// - [`RequestStream::try_recv`] for sync callers (the Lua poll closure)
/// - [`RequestStream::recv`] for async callers (the CLI)
///
/// Both return `None` after the kernel goes idle for this request. Once
/// drained, the per-request slot is removed from the router; further
/// calls keep returning `None`.
pub struct RequestStream {
    pub msg_id: String,
    rx: Option<XReceiver<RoutedFrame>>,
    router: Arc<FrameRouter>,
}

pub enum TryRecv {
    Frame(Frame),
    Empty,
    Done,
}

impl RequestStream {
    pub fn try_recv(&mut self) -> TryRecv {
        let Some(rx) = self.rx.as_ref() else {
            return TryRecv::Done;
        };
        match rx.try_recv() {
            Ok(RoutedFrame::Frame(f)) => TryRecv::Frame(f),
            Ok(RoutedFrame::Idle) => {
                self.rx = None;
                TryRecv::Done
            }
            Err(TryRecvError::Empty) => TryRecv::Empty,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                self.router.forget(&self.msg_id);
                TryRecv::Done
            }
        }
    }

    /// Await the next frame, blocking the current task until one
    /// arrives. Returns `None` after `Idle` for this request.
    pub async fn recv(&mut self) -> Option<Frame> {
        loop {
            match self.try_recv() {
                TryRecv::Frame(f) => return Some(f),
                TryRecv::Done => return None,
                TryRecv::Empty => {
                    // crossbeam channel has no async API; the reader
                    // tasks deliver into it from a different thread, so
                    // a short yield + sleep is fine here. Production
                    // CLI callers want low overhead but not millisecond
                    // latency on every frame.
                    tokio::task::yield_now().await;
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        }
    }

    /// Drain the stream until idle, invoking `on_frame` for every routed
    /// frame. Used by `jet execute` to pump events to a renderer.
    pub async fn drain_to_idle<F>(mut self, mut on_frame: F) -> Result<()>
    where
        F: FnMut(&Frame) -> Result<()>,
    {
        while let Some(f) = self.recv().await {
            on_frame(&f)?;
        }
        Ok(())
    }
}

impl Drop for RequestStream {
    fn drop(&mut self) {
        // Drop early (consumer abandoned the stream): remove the
        // router slot so the reader tasks don't keep accumulating
        // frames for a parent_id nobody's listening to.
        if self.rx.is_some() {
            self.router.forget(&self.msg_id);
        }
    }
}

async fn await_kernel_info(
    shell: &mut ClientShellConnection,
    expected_id: &str,
) -> Result<Value> {
    loop {
        let msg = shell.read().await.map_err(|e| anyhow!("shell.read: {e}"))?;
        let parent = msg
            .parent_header
            .as_ref()
            .map(|h| h.msg_id.as_str())
            .unwrap_or("");
        if parent == expected_id && msg.message_type() == "kernel_info_reply" {
            return Ok(serde_json::to_value(&msg.content)?);
        }
        // Boot-time chatter we don't surface.
    }
}

fn spawn_shell_loop(
    mut shell: ClientShellConnection,
    mut send_rx: UnboundedReceiver<JupyterMessage>,
    router: Arc<FrameRouter>,
    closed: Arc<Notify>,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                send = send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = shell.send(msg).await {
                            log::warn!("shell.send: {e}");
                            closed.notify_waiters();
                            return;
                        }
                    }
                    None => return,
                },
                read = shell.read() => match read {
                    Ok(msg) => router.dispatch(Channel::Shell, msg),
                    Err(e) => {
                        log::warn!("shell.read: {e}");
                        closed.notify_waiters();
                        return;
                    }
                },
            }
        }
    });
}

fn spawn_iopub_reader(
    mut iopub: ClientIoPubConnection,
    router: Arc<FrameRouter>,
    closed: Arc<Notify>,
) {
    tokio::spawn(async move {
        loop {
            match iopub.read().await {
                Ok(msg) => router.dispatch(Channel::IoPub, msg),
                Err(e) => {
                    log::warn!("iopub.read: {e}");
                    closed.notify_waiters();
                    return;
                }
            }
        }
    });
}

fn spawn_stdin_loop(
    mut stdin_sock: ClientStdinConnection,
    mut send_rx: UnboundedReceiver<JupyterMessage>,
    router: Arc<FrameRouter>,
    closed: Arc<Notify>,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                send = send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = stdin_sock.send(msg).await {
                            log::warn!("stdin.send: {e}");
                            closed.notify_waiters();
                            return;
                        }
                    }
                    None => return,
                },
                read = stdin_sock.read() => match read {
                    Ok(msg) => router.dispatch(Channel::Stdin, msg),
                    Err(e) => {
                        log::warn!("stdin.read: {e}");
                        closed.notify_waiters();
                        return;
                    }
                },
            }
        }
    });
}

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

/// Out-of-band frame sink — anything not consumed by a registered
/// request slot is handed here. Boxed so callers (e.g. the REPL) can
/// thread their renderer in without exposing its type to core.
type Sink = Arc<dyn Fn(Frame) + Send + Sync + 'static>;

/// Demuxes kernel frames by `parent_msg_id`. Each in-flight request gets
/// one slot; the reader tasks dispatch into it until they see a matching
/// `status: idle`, which closes the slot. Frames with no matching slot
/// fall through to the [`Sink`] passed at session start.
struct FrameRouter {
    by_parent: Mutex<HashMap<String, XSender<RoutedFrame>>>,
    sink: Sink,
}

impl FrameRouter {
    fn new(sink: Sink) -> Self {
        Self {
            by_parent: Mutex::new(HashMap::new()),
            sink,
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

    /// Route one parsed message:
    /// - Idle status with a matching registered parent closes that slot
    ///   (terminal item for the request stream).
    /// - Content frames with a matching registered parent go to the
    ///   per-request slot.
    /// - Everything (including Idle and per-request content) also goes
    ///   to the global [`Sink`] — out-of-band consumers (the REPL's
    ///   renderer) need to see every frame, not just unrouted ones.
    fn dispatch(&self, channel: Channel, msg: JupyterMessage) {
        let parent_id = msg.parent_header.as_ref().map(|h| h.msg_id.clone());
        let is_idle = matches!(
            from_message(channel, &msg).data,
            EventData::Idle { .. },
        );

        // Close out a matching per-request slot on Idle. We use the
        // parent_header msg_id here rather than the EventData's
        // already-extracted parent_id — they're the same field.
        if is_idle {
            if let Some(pid) = parent_id.as_deref() {
                if let Some(tx) = self.by_parent.lock().unwrap().remove(pid) {
                    let _ = tx.send(RoutedFrame::Idle);
                }
            }
        } else if let Some(pid) = parent_id.as_deref() {
            // Content frame: clone into the per-request slot if any.
            // The Frame still also goes to the global sink below.
            let sender = self.by_parent.lock().unwrap().get(pid).cloned();
            if let Some(tx) = sender {
                let _ = tx.send(RoutedFrame::Frame(Frame {
                    channel,
                    message: msg.clone(),
                }));
            }
        }

        (self.sink)(Frame { channel, message: msg });
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
    ///
    /// Out-of-band frames (anything not consumed by a [`RequestStream`])
    /// are dropped. Use [`KernelSession::start_with_sink`] to surface
    /// them — the CLI REPL does this to feed every frame to its
    /// renderer.
    pub async fn start(kernel: Kernel) -> Result<(Self, Value)> {
        Self::start_with_sink(kernel, |_| {}).await
    }

    /// Like [`KernelSession::start`] but every routed frame is also
    /// handed to `sink`, including those that match a registered
    /// per-request slot. Use when the consumer has a global view (a
    /// renderer, a logger) that needs every frame regardless of which
    /// request — if any — it belongs to.
    pub async fn start_with_sink<F>(mut kernel: Kernel, sink: F) -> Result<(Self, Value)>
    where
        F: Fn(Frame) + Send + Sync + 'static,
    {
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
        let sink: Sink = Arc::new(sink);
        let router = Arc::new(FrameRouter::new(sink));
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

    /// Take the heartbeat connection out of the kernel. Used by the
    /// REPL's attach-path liveness watcher; once taken, the session no
    /// longer carries heartbeat, so callers must not call this twice.
    pub fn take_heartbeat(&mut self) -> Result<crate::jupyter_zmq_client::ClientHeartbeatConnection> {
        self.kernel.channels.take_heartbeat()
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

    /// Mark the underlying kernel as detached — i.e. don't kill the
    /// child when the session drops. Used by `--persist`.
    pub fn detach(&mut self) {
        self.kernel.detach();
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

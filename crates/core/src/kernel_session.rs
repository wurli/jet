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

use anyhow::{Result, anyhow};
use jupyter_protocol::{JupyterMessage, KernelInfoRequest};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{
    Notify,
    mpsc::{
        UnboundedReceiver, UnboundedSender,
        error::TryRecvError,
        unbounded_channel,
    },
};

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
    by_parent: Mutex<HashMap<String, UnboundedSender<RoutedFrame>>>,
    sink: Sink,
}

impl FrameRouter {
    fn new(sink: Sink) -> Self {
        Self {
            by_parent: Mutex::new(HashMap::new()),
            sink,
        }
    }

    fn register(&self, parent_msg_id: String) -> UnboundedReceiver<RoutedFrame> {
        let (tx, rx) = unbounded_channel();
        self.by_parent.lock().unwrap().insert(parent_msg_id, tx);
        rx
    }

    fn forget(&self, parent_msg_id: &str) {
        self.by_parent.lock().unwrap().remove(parent_msg_id);
    }

    /// Route one parsed message:
    /// - Idle status with a matching registered parent closes that
    ///   slot (terminal item for the request stream).
    /// - Content frames with a matching registered parent go to the
    ///   per-request slot.
    /// - Everything (including Idle and per-request content) also
    ///   goes to the global [`Sink`] — out-of-band consumers (the
    ///   REPL's renderer) need to see every frame, not just unrouted
    ///   ones.
    ///
    /// Caveat: idle and the reply for a request arrive on different
    /// sockets (iopub vs shell). The kernel sends reply-then-idle in
    /// time order, but the socket driver task can observe them in
    /// either order, so closing the slot on idle can lose the reply
    /// to the slot. Consumers that care about that ordering should
    /// not use the slot as their synchronisation point — use the
    /// sink (or, for the REPL, the renderer's idle_tx signal which is
    /// emitted after the renderer has processed every prior frame).
    fn dispatch(&self, channel: Channel, msg: JupyterMessage) {
        let parent_id = msg.parent_header.as_ref().map(|h| h.msg_id.clone());
        let is_idle = matches!(
            from_message(channel, &msg).data,
            EventData::Idle { .. },
        );

        if is_idle {
            if let Some(pid) = parent_id.as_deref() {
                if let Some(tx) = self.by_parent.lock().unwrap().remove(pid) {
                    let _ = tx.send(RoutedFrame::Idle);
                }
            }
        } else if let Some(pid) = parent_id.as_deref() {
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
    /// Take the shell/iopub/stdin channels out of the kernel and spawn
    /// the long-running reader/writer tasks. Out-of-band frames
    /// (anything not consumed by a [`RequestStream`]) are dropped.
    pub async fn start(kernel: Kernel) -> Result<(Self, Value)> {
        Self::start_with_sink(kernel, |_| {}).await
    }

    /// Like [`KernelSession::start`] but every routed frame is also
    /// handed to `sink`. Use when the consumer has a global view (a
    /// renderer, a logger) that needs every frame regardless of
    /// which request — if any — it belongs to.
    ///
    /// Performs a synchronous `kernel_info` handshake before spawning
    /// the socket loop. The reply is fed through the sink (so a
    /// renderer sink draws the banner) and returned as JSON. iopub
    /// `status: busy`/`idle` for our own handshake request are
    /// dropped — the client didn't initiate the request, so surfacing
    /// idle for it would mislead any "wait for idle" consumer. Other
    /// iopub frames (kernel-side startup prints) are forwarded to the
    /// sink in order.
    ///
    /// The handshake doubles as the "is the kernel actually
    /// answering" probe. Owning both sockets exclusively here means
    /// no concurrent socket-loop dispatch can race the banner write.
    pub async fn start_with_sink<F>(mut kernel: Kernel, sink: F) -> Result<(Self, Value)>
    where
        F: Fn(Frame) + Send + Sync + 'static,
    {
        let mut shell = kernel.channels.take_shell()?;
        let mut iopub = kernel.channels.take_iopub()?;
        let stdin_sock = kernel.channels.take_stdin()?;

        let sink: Sink = Arc::new(sink);
        let (reply, info) = handshake(&mut shell, &mut iopub, &sink).await?;

        // Feed the reply through the sink as the last step of the
        // handshake. By the time start_with_sink returns, the sink
        // has finished writing the banner (synchronous call), so the
        // caller can draw a prompt next without racing.
        sink(Frame {
            channel: Channel::Shell,
            message: reply,
        });

        let (shell_tx, shell_rx) = unbounded_channel::<JupyterMessage>();
        let (stdin_tx, stdin_rx) = unbounded_channel::<JupyterMessage>();
        let router = Arc::new(FrameRouter::new(sink));
        let closed = Arc::new(Notify::new());

        spawn_socket_loop(
            shell,
            iopub,
            stdin_sock,
            shell_rx,
            stdin_rx,
            router.clone(),
            closed.clone(),
        );

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
    rx: Option<UnboundedReceiver<RoutedFrame>>,
    router: Arc<FrameRouter>,
}

pub enum TryRecv {
    Frame(Frame),
    Empty,
    Done,
}

impl RequestStream {
    pub fn try_recv(&mut self) -> TryRecv {
        let Some(rx) = self.rx.as_mut() else {
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

    /// Await the next frame, parking until one arrives. Returns `None`
    /// after `Idle` for this request.
    pub async fn recv(&mut self) -> Option<Frame> {
        let rx = self.rx.as_mut()?;
        match rx.recv().await {
            Some(RoutedFrame::Frame(f)) => Some(f),
            Some(RoutedFrame::Idle) | None => {
                self.rx = None;
                None
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

/// One tokio task that drives all three ZMQ sockets via `tokio::select!`.
///
/// Splitting these across three tasks was the cause of a real race: the
/// kernel sends `kernel_info_reply` on shell and `status: idle` on
/// iopub, and consumers wait for the idle to know "the request is
/// done." If shell-reader and iopub-reader run on separate tasks, the
/// iopub task can dispatch the idle (which fires the consumer's
/// renderer + signals the main loop) before the shell task has
/// dispatched its reply (which would render the banner). The user
/// then sees `> Python ... > ` because the prompt drew before the
/// banner write reached stdout.
///
/// Combining the readers serialises dispatch: at any moment we're
/// processing exactly one message, and the order in which the kernel
/// sent them (busy → reply → idle, per Jupyter spec) is preserved
/// through the router/sink. A single task is enough — `tokio::select!`
/// polls all sockets concurrently; we're not actually blocking on any
/// one read.
/// Synchronous `kernel_info` handshake: send `kernel_info_request`
/// on shell, wait for the matching reply, drain iopub concurrently so
/// the kernel can't block on a full iopub HWM.
///
/// During the handshake:
/// - `status: busy`/`idle` for our own request are silently dropped —
///   the consumer didn't initiate this request, so signalling idle
///   for it would mislead any later "wait for idle" logic.
/// - Other iopub frames (startup prints, comm_open from the kernel,
///   etc.) flow through the sink in arrival order.
///
/// Returns the reply message AND its content serialised to JSON.
/// Reply is fed through the sink by the caller as the LAST step (so
/// it's the last write before start_with_sink returns, ensuring
/// banner-then-prompt ordering for renderer sinks).
async fn handshake(
    shell: &mut ClientShellConnection,
    iopub: &mut ClientIoPubConnection,
    sink: &Sink,
) -> Result<(JupyterMessage, Value)> {
    let req: JupyterMessage = KernelInfoRequest {}.into();
    let info_id = req.header.msg_id.clone();
    shell
        .send(req)
        .await
        .map_err(|e| anyhow!("shell.send (kernel_info): {e}"))?;

    let wait = async {
        loop {
            tokio::select! {
                biased;
                read = shell.read() => {
                    let msg = read.map_err(|e| anyhow!("shell.read: {e}"))?;
                    let parent = msg.parent_header.as_ref().map(|h| h.msg_id.as_str()).unwrap_or("");
                    if parent == info_id && msg.message_type() == "kernel_info_reply" {
                        let content = serde_json::to_value(&msg.content)?;
                        return Ok::<_, anyhow::Error>((msg, content));
                    }
                    // Other shell traffic this early is unexpected; let
                    // the sink see it so logs/renderer can surface it.
                    sink(Frame { channel: Channel::Shell, message: msg });
                }
                read = iopub.read() => {
                    let msg = read.map_err(|e| anyhow!("iopub.read: {e}"))?;
                    let parent = msg.parent_header.as_ref().map(|h| h.msg_id.as_str()).unwrap_or("");
                    // Drop status frames belonging to our own
                    // kernel_info_request — consumer didn't ask for it.
                    if parent == info_id && msg.message_type() == "status" {
                        continue;
                    }
                    sink(Frame { channel: Channel::IoPub, message: msg });
                }
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), wait)
        .await
        .map_err(|_| anyhow!("timed out waiting for kernel_info_reply"))?
}

fn spawn_socket_loop(
    mut shell: ClientShellConnection,
    mut iopub: ClientIoPubConnection,
    mut stdin_sock: ClientStdinConnection,
    mut shell_send_rx: UnboundedReceiver<JupyterMessage>,
    mut stdin_send_rx: UnboundedReceiver<JupyterMessage>,
    router: Arc<FrameRouter>,
    closed: Arc<Notify>,
) {
    tokio::spawn(async move {
        loop {
            // `biased;` polls the arms in declaration order. Reads
            // come before sends so a backlog of inbound frames can't
            // be starved by a tight loop of outbound sends; shell read
            // comes before iopub read so on the rare iteration where
            // a reply and the matching idle are both ready we dispatch
            // the reply (banner) first. The Jupyter spec puts
            // busy(iopub) → reply(shell) → idle(iopub) in order, but
            // they arrive on different sockets, so without ordering
            // help the consumer's "wait for idle" can win the race
            // against the renderer's "draw banner."
            tokio::select! {
                biased;
                read = shell.read() => match read {
                    Ok(msg) => router.dispatch(Channel::Shell, msg),
                    Err(e) => {
                        log::warn!("shell.read: {e}");
                        closed.notify_waiters();
                        return;
                    }
                },
                read = iopub.read() => match read {
                    Ok(msg) => router.dispatch(Channel::IoPub, msg),
                    Err(e) => {
                        log::warn!("iopub.read: {e}");
                        closed.notify_waiters();
                        return;
                    }
                },
                read = stdin_sock.read() => match read {
                    Ok(msg) => router.dispatch(Channel::Stdin, msg),
                    Err(e) => {
                        log::warn!("stdin.read: {e}");
                        closed.notify_waiters();
                        return;
                    }
                },
                send = shell_send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = shell.send(msg).await {
                            log::warn!("shell.send: {e}");
                            closed.notify_waiters();
                            return;
                        }
                    }
                    None => return,
                },
                send = stdin_send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = stdin_sock.send(msg).await {
                            log::warn!("stdin.send: {e}");
                            closed.notify_waiters();
                            return;
                        }
                    }
                    None => return,
                },
            }
        }
    });
}

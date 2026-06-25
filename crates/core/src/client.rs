//! Single-kernel session: owns one [`Kernel`], spawns the long-lived
//! reader/writer tasks for shell/iopub/stdin, and demuxes incoming
//! frames by `parent_msg_id` so callers can fire many concurrent
//! requests and only see frames for their own.
//!
//! This is the shared core of what used to be open-coded in two places:
//! the Lua binding's `boot_kernel` + `FrameRouter`, and the CLI REPL's
//! three `tokio::spawn` blocks. Both collapse to a [`Client`]:
//! the Lua side wraps it in `Arc<Mutex<_>>` because its sync callers
//! need shared access; the CLI owns it by value.
//!
//! `Client` is single-session by design — one kernel, one
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
    mpsc::{UnboundedReceiver, UnboundedSender, error::TryRecvError, unbounded_channel},
    watch,
};

/// High-level kernel liveness state. The session funnels every liveness
/// signal (iopub status frames, socket errors, heartbeat timeouts, child
/// exit) into a single `tokio::sync::watch` channel of this type, so
/// consumers don't have to wire up four separate watchers.
///
/// `Exited` is terminal: once a session reaches it, no further transition
/// is allowed. A late `status: idle` arriving from a kernel that quit
/// cleanly (ark's `quit()` does this) can't resurrect the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelStatus {
    /// Before the kernel_info handshake completes. Transient.
    Starting,
    /// Kernel reachable, not running a cell for us.
    Idle,
    /// Kernel processing one of our requests.
    Busy,
    /// Kernel gone. Terminal — only reset by a fresh Client.
    Exited,
}

/// Update the watch channel to `next`, unless we've already reached the
/// terminal `Exited` state. Returns whether the value changed.
fn transition(tx: &watch::Sender<KernelStatus>, next: KernelStatus) -> bool {
    let mut changed = false;
    tx.send_if_modified(|cur| {
        if *cur == KernelStatus::Exited {
            return false;
        }
        if *cur == next {
            return false;
        }
        *cur = next;
        changed = true;
        true
    });
    changed
}

use crate::events::{Channel, EventData, from_message};
use crate::jupyter_zmq_client::{
    ClientIoPubConnection, ClientShellConnection, ClientStdinConnection,
};
use crate::kernel::Kernel;

/// Generate a client id of the form `<name>---repl---<rand>`. Kernels report this back in
/// the parent header so we can see which client triggered a message. `'jet'` is a special
/// value which won't be printed in the CLI; other values get surfaced to show when another
/// client (e.g. an agent) is interacting with the kernel.
pub fn make_client_id(name: Option<&str>) -> String {
    use rand::Rng;
    log::info!("Generated new client id: {:?}", name);
    format!(
        "{}---repl---{:x}",
        name.unwrap_or("jet"),
        rand::thread_rng().r#gen::<u64>()
    )
}

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
    status_tx: Arc<watch::Sender<KernelStatus>>,
}

impl FrameRouter {
    fn new(sink: Sink, status_tx: Arc<watch::Sender<KernelStatus>>) -> Self {
        Self {
            by_parent: Mutex::new(HashMap::new()),
            sink,
            status_tx,
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
        let event_data = from_message(channel, &msg).data;
        let is_idle = matches!(event_data, EventData::Idle { .. });

        // Drive the KernelStatus state machine from iopub status frames.
        // Guarded inside `transition` so a trailing idle after Exited
        // can't resurrect the session.
        match event_data {
            EventData::Busy { .. } => {
                transition(&self.status_tx, KernelStatus::Busy);
            }
            EventData::Idle { .. } => {
                transition(&self.status_tx, KernelStatus::Idle);
            }
            _ => {}
        }

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

        (self.sink)(Frame {
            channel,
            message: msg,
        });
    }
}

/// A long-lived session over a single [`Kernel`].
///
/// After construction (via [`Client::spawn`] / [`Client::attach`]) the shell/iopub/stdin
/// sockets have been moved into background tasks; the [`Kernel`] retains only `control` +
/// `heartbeat` + (for spawned kernels) the child process guard, which is what
/// [`Client::interrupt`] / [`Client::shutdown`] need.
pub struct Client {
    kernel: Kernel,
    /// Like `<name>---repl---<rand>`. Kernels report this back in the parent header so we can
    /// see which client triggered a message.
    client_id: String,
    shell_tx: UnboundedSender<JupyterMessage>,
    stdin_tx: UnboundedSender<JupyterMessage>,
    router: Arc<FrameRouter>,
    status_tx: Arc<watch::Sender<KernelStatus>>,
    /// Background liveness watchers (heartbeat for attached kernels,
    /// waitpid for spawned ones). Aborted on Drop.
    watchers: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for Client {
    fn drop(&mut self) {
        for w in self.watchers.drain(..) {
            w.abort();
        }
    }
}

impl Client {
    /// Spawn a kernel and bring up a fully-handshaked client over it. The session name
    /// is mixed into a fresh client id (see [`make_client_id`]); `'jet'` keeps the id
    /// invisible to the CLI's renderer, anything else surfaces as a foreign-client tag.
    /// Pass `|_| {}` for `sink` when you don't need a global frame view.
    pub async fn spawn<F>(
        spec: &crate::kernel::KernelSpec,
        connection_path: Option<std::path::PathBuf>,
        session_name: Option<&str>,
        sink: F,
    ) -> Result<(Self, Value)>
    where
        F: Fn(Frame) + Send + Sync + 'static,
    {
        let client_id = make_client_id(session_name);
        let kernel = Kernel::spawn(spec, connection_path, &client_id).await?;
        Self::start_with_sink(kernel, client_id, sink).await
    }

    /// Attach to a running kernel and bring up a fully-handshaked client over it.
    /// Pass `|_| {}` for `sink` when you don't need a global frame view.
    pub async fn attach<F>(
        connection_path: &std::path::Path,
        session_name: Option<&str>,
        sink: F,
    ) -> Result<(Self, Value)>
    where
        F: Fn(Frame) + Send + Sync + 'static,
    {
        let client_id = make_client_id(session_name);
        let kernel = Kernel::attach(connection_path, &client_id).await?;
        Self::start_with_sink(kernel, client_id, sink).await
    }

    /// Take the shell/iopub/stdin channels out of the kernel, perform the
    /// `kernel_info` handshake (fast-fail probe that the kernel is answering), and spawn
    /// the long-running reader/writer tasks. Every routed frame is also handed to `sink`,
    /// which a renderer/logger uses for its global view.
    ///
    /// The reply is fed through the sink as the last step of the handshake (so a renderer
    /// sink draws the banner before [`Client::spawn`] returns) and returned as
    /// JSON. iopub `status: busy`/`idle` for our own handshake request are dropped — the
    /// consumer didn't initiate the request, so signalling idle for it would mislead any
    /// later "wait for idle" logic. Other iopub frames (kernel-side startup prints) flow
    /// through the sink in arrival order.
    async fn start_with_sink<F>(
        mut kernel: Kernel,
        client_id: String,
        sink: F,
    ) -> Result<(Self, Value)>
    where
        F: Fn(Frame) + Send + Sync + 'static,
    {
        // shell/iopub/stdin always present immediately after connect; the Options exist
        // for the post-take state (control stays on kernel; heartbeat moves out below for
        // attached kernels only).
        let mut shell = kernel.channels.shell.take().expect("shell channel");
        let mut iopub = kernel.channels.iopub.take().expect("iopub channel");
        let stdin_sock = kernel.channels.stdin.take().expect("stdin channel");

        let (status_tx, _status_rx) = watch::channel(KernelStatus::Starting);
        let status_tx = Arc::new(status_tx);

        let sink: Sink = Arc::new(sink);
        let (reply, info) = match handshake(&mut shell, &mut iopub, &sink).await {
            Ok(v) => v,
            Err(e) => {
                return Err(crate::kernel::enrich_startup_error(
                    e,
                    kernel.child_pid(),
                    kernel.child_alive(),
                    kernel.log_file_path.as_deref(),
                ));
            }
        };

        // Feed the reply through the sink as the last step of the
        // handshake. By the time start_with_sink returns, the sink
        // has finished writing the banner (synchronous call), so the
        // caller can draw a prompt next without racing.
        sink(Frame {
            channel: Channel::Shell,
            message: reply,
        });

        // Handshake succeeded — we're talking to a live kernel.
        transition(&status_tx, KernelStatus::Idle);

        let (shell_tx, shell_rx) = unbounded_channel::<JupyterMessage>();
        let (stdin_tx, stdin_rx) = unbounded_channel::<JupyterMessage>();
        let router = Arc::new(FrameRouter::new(sink, status_tx.clone()));

        spawn_socket_loop(
            shell,
            iopub,
            stdin_sock,
            shell_rx,
            stdin_rx,
            router.clone(),
            status_tx.clone(),
        );

        // Liveness watchers:
        // - Attach path (no owned pid): heartbeat. ZMQ DEALER/SUB reads
        //   on a kernel that has exited cleanly don't error — they block
        //   forever — so the heartbeat REQ/REP is the only way to catch
        //   a clean exit like R's `quit()`.
        // - Spawn path (we own the child): waitpid(WNOHANG) every 500ms.
        //   Instant, kernel-level, gives an exit status.
        // The socket loop also flips status to Exited on any read/send
        // error, so a crash is caught even if neither watcher polls in
        // time.
        let mut watchers = Vec::new();
        if kernel.is_attached() {
            let hb = kernel.channels.heartbeat.take().expect("heartbeat channel");
            watchers.push(spawn_heartbeat_watcher(hb, status_tx.clone()));
        }
        if let Some(pid) = kernel.child_pid() {
            watchers.push(spawn_waitpid_watcher(pid, status_tx.clone()));
        }

        Ok((
            Self {
                kernel,
                client_id,
                shell_tx,
                stdin_tx,
                router,
                status_tx,
                watchers,
            },
            info,
        ))
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Watch handle for kernel liveness/execution state. Latest-value channel: callers can
    /// `borrow()` for the current state or `changed().await` to park until it moves.
    pub fn watch_status(&self) -> watch::Receiver<KernelStatus> {
        self.status_tx.subscribe()
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

    /// PID of the underlying spawned kernel, if any. `None` for attached kernels.
    pub fn child_pid(&self) -> Option<u32> {
        self.kernel.child_pid()
    }

    /// Forward a ^C-equivalent to the kernel.
    pub async fn interrupt(&mut self) -> Result<()> {
        self.kernel.interrupt().await
    }

    /// Shutdown the kernel (best-effort). Drop the [`Client`] afterwards to tear down the
    /// reader/writer tasks; if you want the kernel to outlive this process, call
    /// [`Client::detach`] before dropping instead.
    pub async fn shutdown(&mut self) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::Kernel;
    use jupyter_protocol::{JupyterMessage, Status};

    fn with_parent(mut m: JupyterMessage, parent_id: &str) -> JupyterMessage {
        let mut header = m.header.clone();
        header.msg_id = parent_id.to_string();
        m.parent_header = Some(header);
        m
    }

    #[test]
    fn router_drives_status_busy_and_idle() {
        let (tx, _rx) = watch::channel(KernelStatus::Idle);
        let tx = Arc::new(tx);
        let router = FrameRouter::new(Arc::new(|_| {}), tx.clone());

        let busy: JupyterMessage = with_parent(Status::busy().into(), "exec-1");
        router.dispatch(Channel::IoPub, busy);
        assert_eq!(*tx.borrow(), KernelStatus::Busy);

        let idle: JupyterMessage = with_parent(Status::idle().into(), "exec-1");
        router.dispatch(Channel::IoPub, idle);
        assert_eq!(*tx.borrow(), KernelStatus::Idle);
    }

    #[test]
    fn exited_is_terminal_against_trailing_idle() {
        let (tx, _rx) = watch::channel(KernelStatus::Busy);
        let tx = Arc::new(tx);
        // Simulate the socket loop flipping the kernel to Exited.
        transition(&tx, KernelStatus::Exited);
        assert_eq!(*tx.borrow(), KernelStatus::Exited);

        let router = FrameRouter::new(Arc::new(|_| {}), tx.clone());
        // Trailing iopub idle from a kernel that just quit cleanly
        // must not resurrect the session.
        let idle: JupyterMessage = with_parent(Status::idle().into(), "exec-1");
        router.dispatch(Channel::IoPub, idle);
        assert_eq!(*tx.borrow(), KernelStatus::Exited);

        // Same for a trailing busy.
        let busy: JupyterMessage = with_parent(Status::busy().into(), "exec-1");
        router.dispatch(Channel::IoPub, busy);
        assert_eq!(*tx.borrow(), KernelStatus::Exited);
    }

    /// Unit-test the enrichment function directly with a synthetic
    /// log file and a fake kernel handle. We don't drive a full
    /// handshake here — zeromq-rs's 30s connect timeout against a
    /// non-listening peer would dominate the test, and the
    /// `enrich_startup_error` logic is what we actually want to
    /// guard against regressing.
    #[test]
    fn enrich_includes_log_tail() {
        let dir =
            std::env::temp_dir().join(format!("jet-enrich-unit-{:x}", rand::random::<u64>(),));
        std::fs::create_dir_all(&dir).unwrap();
        let log_path = dir.join("conn.json.log");
        std::fs::write(
            &log_path,
            "line one\nline two\nBROKEN_KERNEL_MARKER_xyz\nlast line\n",
        )
        .unwrap();

        let kernel = Kernel::synthetic_for_test(Some(log_path));
        let base = anyhow!("timed out waiting for kernel_info_reply");
        let err = crate::kernel::enrich_startup_error(
            base,
            kernel.child_pid(),
            kernel.child_alive(),
            kernel.log_file_path.as_deref(),
        );
        let msg = format!("{err:#}");

        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            msg.contains("timed out waiting for kernel_info_reply"),
            "original error preserved; got: {msg}",
        );
        assert!(
            msg.contains("BROKEN_KERNEL_MARKER_xyz"),
            "stderr tail included; got: {msg}",
        );
        assert!(
            msg.contains("kernel stderr"),
            "stderr section header present; got: {msg}",
        );
    }
}

/// Poll the heartbeat REQ/REP echo every 2 seconds with a 5s timeout.
/// After two consecutive timeouts, or any send/recv error, declare the
/// kernel dead by transitioning status to `Exited`. Returns when the
/// kernel is declared dead (or the watcher is aborted).
fn spawn_heartbeat_watcher(
    mut hb: crate::jupyter_zmq_client::ClientHeartbeatConnection,
    status_tx: Arc<watch::Sender<KernelStatus>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut consecutive_timeouts = 0;
        loop {
            match tokio::time::timeout(Duration::from_secs(5), hb.single_heartbeat()).await {
                Ok(Ok(())) => {
                    consecutive_timeouts = 0;
                }
                Ok(Err(e)) => {
                    log::info!("heartbeat error: {e} — kernel gone");
                    transition(&status_tx, KernelStatus::Exited);
                    return;
                }
                Err(_) => {
                    consecutive_timeouts += 1;
                    log::warn!("heartbeat timeout ({consecutive_timeouts})");
                    if consecutive_timeouts >= 2 {
                        log::info!("heartbeat: kernel unresponsive, declaring dead");
                        transition(&status_tx, KernelStatus::Exited);
                        return;
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    })
}

/// Poll `waitpid(pid, WNOHANG)` every 500ms; the moment the child
/// changes state, transition status to `Exited`. Used for kernels we
/// spawned ourselves (where we own the pid and tokio doesn't reap it
/// for us until much later).
fn spawn_waitpid_watcher(
    pid: u32,
    status_tx: Arc<watch::Sender<KernelStatus>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut status: libc::c_int = 0;
            let r = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG) };
            // r > 0  → child changed state and we reaped it.
            // r == 0 → still running.
            // r < 0  → ECHILD: tokio already reaped, child gone.
            if r != 0 {
                log::info!("kernel pid {pid} exited (waitpid -> {r})");
                transition(&status_tx, KernelStatus::Exited);
                return;
            }
        }
    })
}

fn spawn_socket_loop(
    mut shell: ClientShellConnection,
    mut iopub: ClientIoPubConnection,
    mut stdin_sock: ClientStdinConnection,
    mut shell_send_rx: UnboundedReceiver<JupyterMessage>,
    mut stdin_send_rx: UnboundedReceiver<JupyterMessage>,
    router: Arc<FrameRouter>,
    status_tx: Arc<watch::Sender<KernelStatus>>,
) {
    tokio::spawn(async move {
        let mark_exited = |reason: &str, e: Option<&dyn std::fmt::Display>| {
            match e {
                Some(e) => log::warn!("{reason}: {e}"),
                None => log::warn!("{reason}"),
            }
            transition(&status_tx, KernelStatus::Exited);
        };
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
                    Err(e) => { mark_exited("shell.read", Some(&e)); return; }
                },
                read = iopub.read() => match read {
                    Ok(msg) => router.dispatch(Channel::IoPub, msg),
                    Err(e) => { mark_exited("iopub.read", Some(&e)); return; }
                },
                read = stdin_sock.read() => match read {
                    Ok(msg) => router.dispatch(Channel::Stdin, msg),
                    Err(e) => { mark_exited("stdin.read", Some(&e)); return; }
                },
                send = shell_send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = shell.send(msg).await {
                            mark_exited("shell.send", Some(&e));
                            return;
                        }
                    }
                    None => return,
                },
                send = stdin_send_rx.recv() => match send {
                    Some(msg) => {
                        if let Err(e) = stdin_sock.send(msg).await {
                            mark_exited("stdin.send", Some(&e));
                            return;
                        }
                    }
                    None => return,
                },
            }
        }
    });
}

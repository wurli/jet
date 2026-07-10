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

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use jupyter_protocol::{
    CommInfoRequest, CompleteReply, CompleteRequest, JupyterMessage, JupyterMessageContent,
    KernelInfoRequest,
};

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
/// the parent header so we can see which client triggered a message. When `name` is `None`
/// the prefix is empty (`---repl---<rand>`), and the CLI will not display a tag for that
/// client. Pass a non-empty name (via `--session-name`) to surface foreign-client attribution.
pub fn make_client_id(name: Option<&str>) -> String {
    use rand::Rng;
    log::info!("Generated new client id: {:?}", name);
    format!(
        "{}---repl---{:x}",
        name.unwrap_or(""),
        rand::thread_rng().r#gen::<u64>()
    )
}

/// Render a kernel message as a single-line JSON object combining
/// `header_id`, `parent_id`, and the spread of `content`. Falls back to
/// `Debug` if serialization fails. Used by `log::debug!` traces of every
/// message in and out of the kernel.
fn fmt_msg(msg: &JupyterMessage) -> String {
    // `JupyterMessageContent` is `#[serde(untagged)]` over struct variants,
    // so it serializes to a JSON object. Spread its fields after our
    // identifying header fields. Relies on `serde_json/preserve_order` for
    // stable field order.
    let mut obj = serde_json::Map::new();
    obj.insert(
        "msg_type".to_string(),
        Value::String(msg.header.msg_type.clone()),
    );
    obj.insert(
        "header_id".to_string(),
        Value::String(msg.header.msg_id.clone()),
    );
    obj.insert(
        "parent_id".to_string(),
        match msg.parent_header.as_ref() {
            Some(h) => Value::String(h.msg_id.clone()),
            None => Value::Null,
        },
    );
    match serde_json::to_value(&msg.content) {
        Ok(Value::Object(map)) => {
            for (k, v) in map {
                obj.insert(k, v);
            }
        }
        Ok(other) => {
            obj.insert("content".to_string(), other);
        }
        Err(_) => return format!("{:?}", msg),
    }
    serde_json::to_string(&Value::Object(obj)).unwrap_or_else(|_| format!("{:?}", msg))
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

// Frame is ~472 bytes (JupyterMessage). Idle is the terminal item and rare per request;
// boxing the hot variant just to balance an enum we move once per message isn't worth it.
#[allow(clippy::large_enum_variant)]
enum RoutedFrame {
    Frame(Frame),
    Idle,
}

/// Filter for a [`Listener`]. `None` for a field means "accept any value
/// in that field"; `Some(set)` restricts to the named values. A `ListenFilter`
/// with both fields `None` is the firehose used by `start`/`attach`'s
/// returned `stream`.
#[derive(Debug, Default, Clone)]
pub struct ListenFilter {
    pub channels: Option<HashSet<Channel>>,
    pub msg_types: Option<HashSet<String>>,
}

impl ListenFilter {
    fn matches(&self, channel: Channel, msg_type: &str) -> bool {
        if let Some(ch) = &self.channels
            && !ch.contains(&channel)
        {
            return false;
        }
        if let Some(mt) = &self.msg_types
            && !mt.contains(msg_type)
        {
            return false;
        }
        true
    }
}

struct Listener {
    filter: ListenFilter,
    tx: UnboundedSender<RoutedFrame>,
}

/// Demuxes kernel frames to three destinations:
///
/// - `by_parent` — per-request slots keyed by the request's `msg_id`.
///   Each in-flight shell request gets one slot; frames whose parent
///   matches dispatch there until the matching `status: idle`, which
///   closes the slot.
/// - `by_comm` — per-comm subscribers keyed by `comm_id`. Fed every
///   `comm_msg`/`comm_close` for the comm regardless of parent_msg_id;
///   closed on `comm_close`.
/// - `listeners` — open-ended subscribers from [`Client::listen`].
///   Each receives every matching frame; closed when the kernel
///   transitions to `Exited`.
///
/// All three destinations see each frame; routing is fan-out, not pick-one.
/// Consumers that want a global frame view (REPL renderer, logger, etc.)
/// register a no-filter listener — there is no separate "sink" hook.
struct FrameRouter {
    by_parent: Mutex<HashMap<String, UnboundedSender<RoutedFrame>>>,
    /// Per-comm subscribers — fed every `comm_msg` / `comm_close` for that
    /// comm_id regardless of parent_msg_id. Closed (slot removed and
    /// `RoutedFrame::Idle` sent) when we see a `comm_close` for the comm.
    by_comm: Mutex<HashMap<String, UnboundedSender<RoutedFrame>>>,
    /// Open-ended subscribers from [`Client::listen`]. Each receives every
    /// matching frame from registration until either the consumer drops
    /// the stream or the kernel transitions to `Exited` (terminal `Idle`
    /// is sent then so the stream ends naturally).
    listeners: Mutex<HashMap<u64, Listener>>,
    next_listener_id: AtomicU64,
    status_tx: Arc<watch::Sender<KernelStatus>>,
}

impl FrameRouter {
    fn new(status_tx: Arc<watch::Sender<KernelStatus>>) -> Self {
        Self {
            by_parent: Mutex::new(HashMap::new()),
            by_comm: Mutex::new(HashMap::new()),
            listeners: Mutex::new(HashMap::new()),
            next_listener_id: AtomicU64::new(1),
            status_tx,
        }
    }

    fn register_listener(&self, filter: ListenFilter) -> (u64, UnboundedReceiver<RoutedFrame>) {
        let id = self.next_listener_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = unbounded_channel();
        self.listeners
            .lock()
            .unwrap()
            .insert(id, Listener { filter, tx });
        (id, rx)
    }

    fn forget_listener(&self, id: u64) {
        self.listeners.lock().unwrap().remove(&id);
    }

    /// Drain every listener and send `Idle` on each. Called when the
    /// kernel transitions to `Exited` so every open `listen()` stream
    /// terminates naturally instead of leaking.
    fn close_listeners(&self) {
        let drained: Vec<_> = self.listeners.lock().unwrap().drain().collect();
        for (_, l) in drained {
            let _ = l.tx.send(RoutedFrame::Idle);
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

    fn register_comm(&self, comm_id: String) -> UnboundedReceiver<RoutedFrame> {
        let (tx, rx) = unbounded_channel();
        self.by_comm.lock().unwrap().insert(comm_id, tx);
        rx
    }

    fn forget_comm(&self, comm_id: &str) {
        self.by_comm.lock().unwrap().remove(comm_id);
    }

    /// Route one parsed message to every destination that wants it:
    /// per-request slot (if `parent_msg_id` matches a registered
    /// request), per-comm subscriber (for `comm_msg`/`comm_close`),
    /// and every matching `listen()` subscriber.
    ///
    /// An `iopub` `status: idle` whose parent matches a registered
    /// request closes that request's slot (terminal item for the
    /// stream); a `comm_close` does the same for the comm subscriber.
    ///
    /// Caveat: idle and the reply for a shell request arrive on
    /// different sockets (iopub vs shell). The kernel sends
    /// reply-then-idle in time order, but the socket driver task can
    /// observe them in either order, so closing the request slot on
    /// idle can lose the reply to that slot. Consumers that care about
    /// the ordering should not use the slot as their synchronisation
    /// point — use a `listen()` subscriber or (in the REPL) the
    /// renderer's `idle_tx` signal which is emitted after the renderer
    /// has processed every prior frame.
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
            if let Some(pid) = parent_id.as_deref()
                && let Some(tx) = self.by_parent.lock().unwrap().remove(pid)
            {
                let _ = tx.send(RoutedFrame::Idle);
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

        // Fan out comm traffic to per-comm subscribers. comm_msg/comm_close
        // from the kernel don't share a parent_msg_id with our outbound
        // comm_open, so the by_parent path can't deliver them — route by
        // comm_id instead. comm_close also tears the subscriber down.
        match &msg.content {
            JupyterMessageContent::CommMsg(m) => {
                let sender = self.by_comm.lock().unwrap().get(&m.comm_id.0).cloned();
                if let Some(tx) = sender {
                    let _ = tx.send(RoutedFrame::Frame(Frame {
                        channel,
                        message: msg.clone(),
                    }));
                }
            }
            JupyterMessageContent::CommClose(c) => {
                let tx = self.by_comm.lock().unwrap().remove(&c.comm_id.0);
                if let Some(tx) = tx {
                    let _ = tx.send(RoutedFrame::Frame(Frame {
                        channel,
                        message: msg.clone(),
                    }));
                    let _ = tx.send(RoutedFrame::Idle);
                }
            }
            _ => {}
        }

        // Fan out to filtered listeners. We hold the lock only for the
        // send loop; senders are cheap clones and the listener slot is
        // small. A `tx.send` failure just means the receiver was dropped
        // without unregistering — leave it; the next forget_listener or
        // close_listeners call will reap it.
        let msg_type = msg.message_type();
        let listeners = self.listeners.lock().unwrap();
        for l in listeners.values() {
            if l.filter.matches(channel, msg_type) {
                let _ = l.tx.send(RoutedFrame::Frame(Frame {
                    channel,
                    message: msg.clone(),
                }));
            }
        }
    }
}

/// A long-lived session over a single [`Kernel`].
///
/// After construction (via [`Client::spawn`] / [`Client::attach`]) the shell/iopub/stdin/control
/// sockets have all been moved into background tasks; the [`Kernel`] retains only `heartbeat`
/// (for attached kernels) + (for spawned kernels) the child process guard. Outbound
/// control messages go through `control_tx` — the socket loop owns the socket so
/// `shutdown_reply` frames flow through the router without waiting behind shell traffic
/// (per the Jupyter spec's guidance to service control on a separate thread).
pub struct Client {
    kernel: Kernel,
    /// Like `<name>---repl---<rand>`. Kernels report this back in the parent header so we can
    /// see which client triggered a message.
    client_id: String,
    /// SessionStore id (the `session.json` slug). `Some` when this Client was started
    /// against a tracked Jet session — either spawning into one (`jet start` without an
    /// explicit `--connection-file`) or attaching to a connection file that lives inside
    /// a tracked session dir. `None` when the caller manages the connection file directly
    /// (`jet start --connection-file P` / `jet attach --connection-file P` where P isn't
    /// tracked) — there's no session.json to write back to.
    session_id: Option<String>,
    /// PID of the spawned kernel child, captured at spawn time. `None` for attached
    /// kernels. Cached here so it remains readable after the OS-level child has been
    /// reaped (by `spawn_waitpid_watcher`), at which point `kernel.child_pid()` would
    /// otherwise return `None`.
    child_pid: Option<u32>,
    /// Whether the kernel wants SIGINT or an `interrupt_request` on control for ^C.
    /// Cached off the spec at bring-up so [`Client::interrupt`] can branch without
    /// reaching into `Kernel`.
    interrupt_mode: crate::kernel_spec::InterruptMode,
    shell_tx: UnboundedSender<JupyterMessage>,
    stdin_tx: UnboundedSender<JupyterMessage>,
    control_tx: UnboundedSender<JupyterMessage>,
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
    /// is mixed into a fresh client id (see [`make_client_id`]); `None` produces an unnamed
    /// client whose output is never tagged in the CLI renderer.
    ///
    /// Returns `(client, kernel_info, boot_stream)`. `boot_stream` is a no-filter
    /// [`listen`] subscriber registered before the handshake reply is dispatched, so a
    /// consumer that immediately pulls from it sees every frame from kernel boot onward
    /// (the handshake reply included — that's the welcome banner). Pull synchronously
    /// before drawing your first prompt if you need banner-before-prompt ordering.
    pub async fn spawn(
        spec: &crate::kernel::KernelSpec,
        connection_path: Option<std::path::PathBuf>,
        session_name: Option<&str>,
        session_id: Option<String>,
    ) -> Result<(Self, Value, RequestStream)> {
        let client_id = make_client_id(session_name);
        let kernel = Kernel::spawn(spec, connection_path, &client_id).await?;
        Self::bring_up(kernel, client_id, session_id).await
    }

    /// Attach to a running kernel and bring up a fully-handshaked client over it.
    /// See [`Client::spawn`] for the return shape. `opts` carries the kernelspec-
    /// derived interrupt mode and (when known) the kernel pid, so `^C` after
    /// `jet attach` can be forwarded correctly instead of being silently swallowed.
    pub async fn attach(
        connection_path: &std::path::Path,
        session_name: Option<&str>,
        session_id: Option<String>,
        opts: crate::kernel::AttachOptions,
    ) -> Result<(Self, Value, RequestStream)> {
        let client_id = make_client_id(session_name);
        let kernel = Kernel::attach(connection_path, &client_id, opts).await?;
        Self::bring_up(kernel, client_id, session_id).await
    }

    /// Take the shell/iopub/stdin channels out of the kernel, perform the
    /// `kernel_info` handshake (fast-fail probe that the kernel is answering), spawn
    /// the long-running reader/writer tasks, and return a boot-time listener stream.
    ///
    /// The router is built before the handshake so handshake-time frames go through the
    /// same dispatch path as everything else. iopub `status: busy`/`idle` for our own
    /// handshake request are still dropped (the consumer didn't initiate it); other
    /// shell/iopub frames flow through the router as normal.
    ///
    /// The handshake reply itself is dispatched as the LAST step before returning, so a
    /// consumer that synchronously pulls one frame from the boot stream gets the banner
    /// before [`Client::spawn`] returns — preserving banner-before-prompt ordering.
    async fn bring_up(
        mut kernel: Kernel,
        client_id: String,
        session_id: Option<String>,
    ) -> Result<(Self, Value, RequestStream)> {
        // shell/iopub/stdin/control always present immediately after start; the Options
        // exist for the post-take state (heartbeat moves out below for attached kernels
        // only). Control moves into the socket loop so `shutdown_reply` and other
        // control-channel frames flow through the router — per Jupyter spec, control
        // is meant to run on a separate thread from shell so shutdown/interrupt aren't
        // queued behind execute traffic.
        let mut shell = kernel.channels.shell.take().expect("shell channel");
        let mut iopub = kernel.channels.iopub.take().expect("iopub channel");
        let stdin_sock = kernel.channels.stdin.take().expect("stdin channel");
        let control = kernel.channels.control.take().expect("control channel");
        let interrupt_mode = kernel.interrupt_mode;

        let (status_tx, _status_rx) = watch::channel(KernelStatus::Starting);
        let status_tx = Arc::new(status_tx);
        let router = Arc::new(FrameRouter::new(status_tx.clone()));

        // Register the boot listener BEFORE handshake so callers see every
        // frame from kernel boot — including the handshake reply we
        // dispatch below.
        let boot_stream = {
            let (id, rx) = router.register_listener(ListenFilter::default());
            RequestStream {
                msg_id: String::new(),
                kind: SlotKind::Listener(id),
                rx: Some(rx),
                router: router.clone(),
            }
        };

        let (reply, info) = match handshake(&mut shell, &mut iopub, &router).await {
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

        // Dispatch the reply as the LAST step before returning. The router
        // delivers it synchronously to the boot listener's mpsc, so a
        // consumer that pulls one frame before drawing its prompt sees
        // the banner first.
        log::debug!("kernel to client shell (handshake): {}", fmt_msg(&reply));
        router.dispatch(Channel::Shell, reply);

        // Handshake succeeded — we're talking to a live kernel.
        transition(&status_tx, KernelStatus::Idle);

        let (shell_tx, shell_rx) = unbounded_channel::<JupyterMessage>();
        let (stdin_tx, stdin_rx) = unbounded_channel::<JupyterMessage>();
        let (control_tx, control_rx) = unbounded_channel::<JupyterMessage>();

        spawn_socket_loop(
            shell,
            iopub,
            stdin_sock,
            control,
            shell_rx,
            stdin_rx,
            control_rx,
            router.clone(),
            status_tx.clone(),
        );

        // Liveness watchers:
        // - Attach path (no owned pid): heartbeat. ZMQ DEALER/SUB reads
        //   on a kernel that has exited cleanly don't error — they block
        //   forever — so the heartbeat REQ/REP is the only way to catch
        //   a clean exit like R's `quit()`.
        // - Spawn path (we own the child): move the `Child` handle into a
        //   task and `wait().await` it. Fully event-driven — SIGCHLD wakes
        //   the wait the instant the kernel dies, so we detect in-band
        //   exits (ipykernel `quit()`, R `q()`) fast enough that the REPL
        //   can suppress the trailing prompt redraw without a poll loop.
        // The socket loop also flips status to Exited on any read/send
        // error, so a crash is caught even if neither watcher polls in
        // time.
        let mut watchers = Vec::new();
        if kernel.is_attached() {
            let hb = kernel.channels.heartbeat.take().expect("heartbeat channel");
            watchers.push(spawn_heartbeat_watcher(hb, status_tx.clone()));
        }
        let child_pid = kernel.child_pid();
        if let Some(child) = kernel.take_child() {
            watchers.push(spawn_child_wait_watcher(child, status_tx.clone()));
        }
        // Tear down open `listen()` streams the moment the kernel is
        // declared dead, regardless of which watcher detected it. The
        // listeners receive a terminal Idle so consumers' iterator loops
        // exit naturally instead of hanging.
        watchers.push(spawn_listener_closer(status_tx.clone(), router.clone()));

        Ok((
            Self {
                kernel,
                client_id,
                session_id,
                child_pid,
                interrupt_mode,
                shell_tx,
                stdin_tx,
                control_tx,
                router,
                status_tx,
                watchers,
            },
            info,
            boot_stream,
        ))
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// SessionStore id this Client is bound to, if any. See the `session_id` field.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
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
            kind: SlotKind::Parent,
            rx: Some(rx),
            router: self.router.clone(),
        })
    }

    /// A cheap clonable handle that can issue `complete_request`s without
    /// holding `&Client`. Used by the rustyline completer, which runs on
    /// the blocking thread pool and can't borrow the REPL-owned `Client`.
    pub fn completion_handle(&self) -> CompletionHandle {
        CompletionHandle {
            shell_tx: self.shell_tx.clone(),
            router: self.router.clone(),
        }
    }

    /// Send a `comm_info_request` and return a stream of routed frames.
    /// Optionally filter by `target_name`; passing `None` asks the kernel
    /// to list all open comms.
    pub fn comm_info(&self, target_name: Option<String>) -> Result<RequestStream> {
        let req: JupyterMessage = CommInfoRequest { target_name }.into();
        self.request(req)
    }

    /// Subscribe to every frame the kernel sends, optionally filtered by
    /// channel and/or message type. The stream emits frames in dispatch
    /// order and ends with a terminal idle when the kernel transitions to
    /// `Exited`. Drop the stream early to unsubscribe — the kernel keeps
    /// running.
    ///
    /// History semantics: subscribers only see frames that arrive AFTER
    /// `listen` returns. If you need every frame from boot, call
    /// `listen` before triggering any work on this client.
    pub fn listen(&self, filter: ListenFilter) -> RequestStream {
        let (id, rx) = self.router.register_listener(filter);
        RequestStream {
            // RequestStream's msg_id is used for cleanup-by-string; the
            // listener slot is keyed by u64, carried in SlotKind::Listener.
            // Keep msg_id empty here so accidental use doesn't alias a
            // real msg_id.
            msg_id: String::new(),
            kind: SlotKind::Listener(id),
            rx: Some(rx),
            router: self.router.clone(),
        }
    }

    /// Subscribe to kernel→frontend traffic for one open comm. The stream
    /// yields every `comm_msg` (and the terminating `comm_close`) the
    /// kernel sends bearing this `comm_id`, regardless of which client
    /// request — if any — they're parented to. The stream ends after the
    /// `comm_close` for that comm.
    ///
    /// Dropping the stream before close just unsubscribes; the comm stays
    /// open as far as the kernel is concerned.
    pub fn comm_listen(&self, comm_id: String) -> RequestStream {
        let rx = self.router.register_comm(comm_id.clone());
        RequestStream {
            msg_id: comm_id,
            kind: SlotKind::Comm,
            rx: Some(rx),
            router: self.router.clone(),
        }
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
    /// Stays `Some` for the lifetime of the `Client` even after the kernel exits —
    /// we cache the pid at spawn time so the OS-level reap by the waitpid watcher
    /// doesn't make it disappear.
    pub fn child_pid(&self) -> Option<u32> {
        self.child_pid
    }

    /// Forward a ^C-equivalent to the kernel. `Signal`-mode kernels get a real SIGINT to
    /// their process group (handled inside `Kernel` — needs the pid); `Message`-mode
    /// kernels get an `interrupt_request` pushed onto the control-send channel so it
    /// races through the socket loop without waiting behind shell traffic.
    pub async fn interrupt(&mut self) -> Result<()> {
        match self.interrupt_mode {
            crate::kernel_spec::InterruptMode::Signal => self.kernel.interrupt().await,
            crate::kernel_spec::InterruptMode::Message => {
                let msg: JupyterMessage = jupyter_protocol::InterruptRequest::default().into();
                self.control_tx
                    .send(msg)
                    .map_err(|e| anyhow!("control_tx.send (interrupt): {e}"))?;
                Ok(())
            }
        }
    }

    /// Shutdown the kernel (best-effort). Sends `shutdown_request` on control and gives
    /// the kernel a moment to react before returning. Drop the [`Client`] afterwards to
    /// tear down the reader/writer tasks; if you want the kernel to outlive this process,
    /// call [`Client::detach`] before dropping instead.
    pub async fn shutdown(&mut self) -> Result<()> {
        log::debug!("Sending shutdown_request to kernel");
        let req = jupyter_protocol::ShutdownRequest { restart: false };
        let msg: JupyterMessage = req.into();
        if let Err(e) = self.control_tx.send(msg) {
            log::warn!("shutdown_request send failed: {e}");
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Clean up the on-disk stderr log on graceful shutdown. Detach skips this path,
        // so detached kernels leave the log in place for a future `attach` to tail.
        if let Some(p) = self.kernel.log_file_path.take() {
            let _ = std::fs::remove_file(p);
        }
        Ok(())
    }

    /// Mark the underlying kernel as detached — i.e. don't kill the
    /// child when the session drops. Used by `--persist`.
    pub fn detach(&mut self) {
        self.kernel.detach();
    }
}

/// Cheap clonable handle for issuing one-shot requests (currently just
/// `complete_request`) off the main REPL task — e.g. from rustyline's
/// sync `Completer::complete`, which runs on the blocking thread pool.
///
/// Holds clones of the shell sender and the frame router, so a dropped
/// `CompletionHandle` doesn't affect the owning [`Client`].
#[derive(Clone)]
pub struct CompletionHandle {
    shell_tx: UnboundedSender<JupyterMessage>,
    router: Arc<FrameRouter>,
}

impl CompletionHandle {
    /// Send a `complete_request` and await the matching `complete_reply`.
    /// Returns `None` if the request stream ends without a reply — treat
    /// that as "no completions" rather than an error so a flaky completion
    /// path doesn't disrupt the prompt.
    pub async fn complete(&self, code: String, cursor_pos: usize) -> Result<Option<CompleteReply>> {
        let req: JupyterMessage = CompleteRequest { code, cursor_pos }.into();
        let msg_id = req.header.msg_id.clone();
        let rx = self.router.register(msg_id.clone());
        self.shell_tx
            .send(req)
            .map_err(|e| anyhow!("shell_tx send: {e}"))?;
        let mut stream = RequestStream {
            msg_id,
            kind: SlotKind::Parent,
            rx: Some(rx),
            router: self.router.clone(),
        };
        while let Some(frame) = stream.recv().await {
            if let JupyterMessageContent::CompleteReply(reply) = frame.message.content {
                return Ok(Some(reply));
            }
        }
        Ok(None)
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
/// Which router map a [`RequestStream`] is registered in — by `msg_id`
/// for shell requests, by `comm_id` for [`Client::comm_listen`] —
/// so [`RequestStream::Drop`] knows which map to clean from.
#[derive(Clone, Copy, Debug)]
enum SlotKind {
    Parent,
    Comm,
    Listener(u64),
}

pub struct RequestStream {
    /// Key into the router map indicated by `kind`: `msg_id` for shell
    /// requests, `comm_id` for [`Client::comm_listen`].
    pub msg_id: String,
    kind: SlotKind,
    rx: Option<UnboundedReceiver<RoutedFrame>>,
    router: Arc<FrameRouter>,
}

#[allow(clippy::large_enum_variant)] // see RoutedFrame
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
                self.forget_slot();
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

    fn forget_slot(&self) {
        match self.kind {
            SlotKind::Parent => self.router.forget(&self.msg_id),
            SlotKind::Comm => self.router.forget_comm(&self.msg_id),
            SlotKind::Listener(id) => self.router.forget_listener(id),
        }
    }
}

impl Drop for RequestStream {
    fn drop(&mut self) {
        // Drop early (consumer abandoned the stream): remove the
        // router slot so the reader tasks don't keep accumulating
        // frames for a parent_id nobody's listening to.
        if self.rx.is_some() {
            self.forget_slot();
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
/// through the router. A single task is enough — `tokio::select!`
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
///   etc.) flow through the router (and any listeners on it) in
///   arrival order.
///
/// Returns the reply message AND its content serialised to JSON.
/// Reply is dispatched by the caller as the LAST step (so it's the last
/// write before [`Client::bring_up`] returns, ensuring
/// banner-then-prompt ordering for renderer consumers).
async fn handshake(
    shell: &mut ClientShellConnection,
    iopub: &mut ClientIoPubConnection,
    router: &FrameRouter,
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
                    // Other shell traffic this early is unexpected; dispatch
                    // it so logs/renderer can surface it.
                    router.dispatch(Channel::Shell, msg);
                }
                read = iopub.read() => {
                    let msg = read.map_err(|e| anyhow!("iopub.read: {e}"))?;
                    let parent = msg.parent_header.as_ref().map(|h| h.msg_id.as_str()).unwrap_or("");
                    // Drop status frames belonging to our own
                    // kernel_info_request — consumer didn't ask for it.
                    if parent == info_id && msg.message_type() == "status" {
                        continue;
                    }
                    router.dispatch(Channel::IoPub, msg);
                }
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), wait)
        .await
        .map_err(|_| anyhow!("timed out waiting for kernel_info_reply"))?
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

/// `wait().await` on a `tokio::process::Child` and transition status to
/// `Exited` as soon as it returns. Event-driven: SIGCHLD wakes the wait
/// the moment the kernel dies, so in-band exits (ipykernel `quit()`,
/// R `q()`) surface within microseconds — fast enough that the REPL's
/// post-execute check can suppress the trailing prompt redraw. Matches
/// kallichore's `run_child` pattern.
fn spawn_child_wait_watcher(
    mut child: tokio::process::Child,
    status_tx: Arc<watch::Sender<KernelStatus>>,
) -> tokio::task::JoinHandle<()> {
    let pid = child.id();
    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => log::info!("kernel pid {pid:?} exited (status: {status})"),
            Err(e) => log::warn!("kernel pid {pid:?} wait failed: {e}"),
        }
        transition(&status_tx, KernelStatus::Exited);
    })
}

/// Wait for the kernel to transition to `Exited`, then drain every open
/// `listen()` subscriber so their streams end with a terminal `Idle`
/// instead of hanging on the consumer's iterator.
fn spawn_listener_closer(
    status_tx: Arc<watch::Sender<KernelStatus>>,
    router: Arc<FrameRouter>,
) -> tokio::task::JoinHandle<()> {
    let mut rx = status_tx.subscribe();
    tokio::spawn(async move {
        loop {
            if *rx.borrow_and_update() == KernelStatus::Exited {
                router.close_listeners();
                return;
            }
            if rx.changed().await.is_err() {
                // Sender dropped — Client is gone, listeners with it.
                return;
            }
        }
    })
}

fn spawn_socket_loop(
    mut shell: ClientShellConnection,
    mut iopub: ClientIoPubConnection,
    mut stdin_sock: ClientStdinConnection,
    mut control: crate::jupyter_zmq_client::ClientControlConnection,
    mut shell_send_rx: UnboundedReceiver<JupyterMessage>,
    mut stdin_send_rx: UnboundedReceiver<JupyterMessage>,
    mut control_send_rx: UnboundedReceiver<JupyterMessage>,
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
            // `biased;` polls the arms in declaration order. Reads come before
            // sends so a backlog of inbound frames can't be starved by a tight
            // loop of outbound sends. Control read comes first so shutdown/
            // interrupt replies never queue behind shell traffic (the Jupyter
            // spec explicitly recommends servicing control on a separate
            // thread for that reason); shell read then comes before iopub read
            // so on the rare iteration where a reply and the matching idle are
            // both ready we dispatch the reply (banner) first.
            tokio::select! {
                biased;
                read = control.read() => match read {
                    Ok(msg) => {
                        log::debug!("kernel to client control: {}", fmt_msg(&msg));
                        router.dispatch(Channel::Control, msg)
                    },
                    Err(e) => { mark_exited("control.read", Some(&e)); return; }
                },
                read = shell.read() => match read {
                    Ok(msg) => {
                        log::debug!("kernel to client shell: {}", fmt_msg(&msg));
                        router.dispatch(Channel::Shell, msg)
                    },
                    Err(e) => { mark_exited("shell.read", Some(&e)); return; }
                },
                read = iopub.read() => match read {
                    Ok(msg) => {
                        log::debug!("kernel to client iopub: {}", fmt_msg(&msg));
                        router.dispatch(Channel::IoPub, msg)
                    },
                    Err(e) => { mark_exited("iopub.read", Some(&e)); return; }
                },
                read = stdin_sock.read() => match read {
                    Ok(msg) => {
                        log::debug!("kernel to client stdin: {}", fmt_msg(&msg));
                        router.dispatch(Channel::Stdin, msg)
                    },
                    Err(e) => { mark_exited("stdin.read", Some(&e)); return; }
                },
                send = shell_send_rx.recv() => match send {
                    Some(msg) => {
                        log::debug!("client to kernel shell: {}", fmt_msg(&msg));
                        if let Err(e) = shell.send(msg).await {
                            mark_exited("shell.send", Some(&e));
                            return;
                        }
                    }
                    None => return,
                },
                send = stdin_send_rx.recv() => match send {
                    Some(msg) => {
                        log::debug!("client to kernel stdin: {}", fmt_msg(&msg));
                        if let Err(e) = stdin_sock.send(msg).await {
                            mark_exited("stdin.send", Some(&e));
                            return;
                        }
                    }
                    None => return,
                },
                send = control_send_rx.recv() => match send {
                    Some(msg) => {
                        log::debug!("client to kernel control: {}", fmt_msg(&msg));
                        if let Err(e) = control.send(msg).await {
                            mark_exited("control.send", Some(&e));
                            return;
                        }
                    }
                    None => return,
                },
            }
        }
    });
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
        let router = FrameRouter::new(tx.clone());

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

        let router = FrameRouter::new(tx.clone());
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
    /// handshake here — zeromq-rs's 30s start timeout against a
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

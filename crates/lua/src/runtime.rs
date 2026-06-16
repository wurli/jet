//! Process-global tokio runtime + kernel registry.
//!
//! Lua callers are sync; we own a multi-threaded tokio runtime once and
//! `block_on` all kallichore HTTP/WS calls through it. The runtime is also
//! where per-kernel frame-reader tasks are spawned.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::{Builder, Runtime};

use crate::router::FrameRouter;
use jet_core::kallichore::{Channel, Client};

/// Lazily-built, process-global runtime. Two worker threads is enough for
/// the kallichore client + a handful of WebSocket reader tasks.
pub fn rt() -> &'static Runtime {
    static RT: Lazy<Runtime> = Lazy::new(|| {
        Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime")
    });
    &RT
}

pub struct KernelHandle {
    /// Owning the client keeps the kcserver process alive (for clients we
    /// spawned) and the HTTP/auth context for subsequent calls.
    pub client: Arc<Client>,
    /// Sink half of the per-session WebSocket. Wrapped in a Mutex because
    /// Lua callers hand off frames from arbitrary threads.
    pub channel: Arc<tokio::sync::Mutex<Channel>>,
    pub router: Arc<FrameRouter>,
    /// kallichore session_id, kept here so `provide_stdin` etc. can stamp
    /// the frame correctly without re-deriving it.
    pub session_id: String,
}

pub static KERNELS: Lazy<Mutex<HashMap<String, Arc<KernelHandle>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Look up a kernel by session id; error if it isn't registered.
pub fn get(session_id: &str) -> anyhow::Result<Arc<KernelHandle>> {
    KERNELS
        .lock()
        .unwrap()
        .get(session_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no kernel with session id {session_id}"))
}

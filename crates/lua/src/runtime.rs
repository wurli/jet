//! Process-global tokio runtime + kernel registry.
//!
//! Lua callers are sync; we own a multi-threaded tokio runtime once and
//! `block_on` jet-core calls through it. Each kernel gets a long-lived
//! pair of reader tasks (one per ZMQ channel that flows kernel→client)
//! plus mpsc senders for the two channels that flow client→kernel
//! (shell + stdin). Lifecycle calls (interrupt, shutdown) own the
//! `Kernel` directly under a tokio mutex.

use jet_core::jupyter_protocol::JupyterMessage;
use jet_core::kernel::Kernel;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc::UnboundedSender;

use crate::router::FrameRouter;

/// Lazily-built, process-global runtime.
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
    /// The kernel itself. Held under a tokio mutex because lifecycle
    /// methods (interrupt, shutdown) need `&mut self` and we want one at
    /// a time.
    pub kernel: Arc<tokio::sync::Mutex<Kernel>>,
    /// Outbound shell sends: `execute_request`, `complete_request`, etc.
    pub shell_tx: UnboundedSender<JupyterMessage>,
    /// Outbound stdin sends: `input_reply`.
    pub stdin_tx: UnboundedSender<JupyterMessage>,
    pub router: Arc<FrameRouter>,
    /// Carried for parity with the kallichore shape and for future
    /// debugging surfaces; not currently read from outside the lifecycle
    /// module.
    #[allow(dead_code)]
    pub session_id: String,
}

pub static KERNELS: Lazy<Mutex<HashMap<String, Arc<KernelHandle>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn get(session_id: &str) -> anyhow::Result<Arc<KernelHandle>> {
    KERNELS
        .lock()
        .unwrap()
        .get(session_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no kernel with session id {session_id}"))
}

//! Process-global tokio runtime + kernel registry.
//!
//! Lua callers are sync; we own a multi-threaded tokio runtime once and
//! `block_on` jet-core calls through it. Each kernel is wrapped in an
//! [`Arc<tokio::sync::Mutex<KernelSession>>`] so the lua-side registry
//! can hand out shared handles — the lock keeps per-kernel state safe
//! when sync lua callers race on lifecycle methods (`interrupt`,
//! `shutdown`) and per-request sends.

use jet_core::client::Client;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::{Builder, Runtime};

/// Lazily-built, process-global runtime.
pub fn runtime() -> &'static Runtime {
    static RT: Lazy<Runtime> = Lazy::new(|| {
        Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime")
    });
    &RT
}

pub type KernelHandle = Arc<tokio::sync::Mutex<Client>>;

pub static KERNELS: Lazy<Mutex<HashMap<String, KernelHandle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn get(session_id: &str) -> anyhow::Result<KernelHandle> {
    KERNELS
        .lock()
        .unwrap()
        .get(session_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no kernel with session id {session_id}"))
}

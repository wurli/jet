//! Spawn an LSP server bound to `127.0.0.1:0` and hand back the assigned
//! port. Each accepted TCP connection gets its own tower-lsp session
//! sharing the same [`LspBackend`] — external clients see the same
//! document state and the same kernel.

use std::io;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower_lsp::{LspService, Server};

use super::backend::LspBackend;
use super::server::LspServer;

pub struct LspTcpHandle {
    pub port: u16,
    task: JoinHandle<()>,
}

impl LspTcpHandle {
    /// Terminate the accept loop. Existing client sessions get dropped
    /// on the next await point.
    pub fn abort(&self) {
        self.task.abort();
    }
}

impl Drop for LspTcpHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Bind an ephemeral port on the loopback and start accepting LSP
/// clients. The returned `port` is what should be written into
/// `session.json` for external discovery.
pub async fn spawn_tcp(backend: Arc<LspBackend>) -> io::Result<LspTcpHandle> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    log::info!("jet-lsp: listening on 127.0.0.1:{port}");

    let task = tokio::spawn(async move {
        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("jet-lsp accept: {e}");
                    // Transient accept failures on loopback are unusual;
                    // yield and try again rather than tearing the whole
                    // listener down.
                    tokio::task::yield_now().await;
                    continue;
                }
            };
            log::info!("jet-lsp: client connected from {peer}");
            let backend = backend.clone();
            tokio::spawn(async move {
                let (read, write) = tokio::io::split(stream);
                let (service, socket) =
                    LspService::new(|client| LspServer::new(backend, client));
                Server::new(read, write, socket).serve(service).await;
                log::info!("jet-lsp: client {peer} disconnected");
            });
        }
    });

    Ok(LspTcpHandle { port, task })
}

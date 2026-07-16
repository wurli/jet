//! LSP server wrapping a Jupyter kernel's `complete_request` /
//! `complete_reply` shell exchange behind the standard textDocument
//! surface. Runs alongside [`crate::client::Client`]:
//! - `spawn_tcp` binds an ephemeral loopback port for external editors;
//! - internal consumers (the CLI completer, the Lua binding) hold an
//!   `Arc<LspBackend>` and call its plain methods in-process, so every
//!   caller — external or internal — exercises the same code path.

mod backend;
mod completion;
mod documents;
mod position;
mod serve;
mod server;

pub use backend::{COMPLETE_TIMEOUT, LspBackend};
pub use serve::{LspTcpHandle, spawn_tcp};

/// Re-export of `tower-lsp` so downstream crates can build LSP requests
/// to feed the in-process backend without pulling in tower-lsp as a
/// direct dependency.
pub use tower_lsp::{self, lsp_types};

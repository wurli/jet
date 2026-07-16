//! reedline `Completer` that drives Tab completion through jet's own LSP
//! backend, which in turn calls the kernel's `complete_request`
//! shell-channel exchange.
//!
//! `reedline::Completer::complete` is sync and runs from inside
//! `read_line`, which we already wrap in `tokio::task::spawn_blocking`,
//! so we use `tokio::runtime::Handle::block_on` to drive the async
//! backend call from the worker thread. The handle is cloned from the
//! ambient `#[tokio::main]` runtime once at editor construction.

use std::sync::Arc;

use jet_core::lsp::LspBackend;
use reedline::{Completer, Span, Suggestion};

pub struct JetCompleter {
    backend: Arc<LspBackend>,
    rt: tokio::runtime::Handle,
}

impl JetCompleter {
    pub fn new(backend: Arc<LspBackend>, rt: tokio::runtime::Handle) -> Self {
        Self { backend, rt }
    }
}

impl Completer for JetCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let backend = self.backend.clone();
        let line = line.to_string();
        // We're on the blocking thread pool (spawn_blocking in repl.rs);
        // block_on here is safe because this thread isn't a tokio worker.
        let matches = self
            .rt
            .block_on(async move { backend.complete_line(&line, pos).await });

        matches
            .into_iter()
            .map(|m| Suggestion {
                value: m.value,
                description: None,
                style: None,
                extra: None,
                span: Span::new(m.replace.start, m.replace.end),
                append_whitespace: false,
                display_override: None,
                match_indices: None,
            })
            .collect()
    }
}

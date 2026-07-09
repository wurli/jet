//! reedline `Completer` that drives Tab completion through the kernel's
//! `complete_request` / `complete_reply` shell-channel exchange.
//!
//! reedline's `Completer::complete` is sync and runs from inside
//! `read_line`, which we already wrap in `tokio::task::spawn_blocking`,
//! so we use `tokio::runtime::Handle::block_on` to drive the async
//! kernel request from the worker thread. The handle is cloned from the
//! ambient `#[tokio::main]` runtime once at editor construction.

use std::time::Duration;

use jet_core::client::CompletionHandle;
use reedline::{Completer, Span, Suggestion};

/// Timeout for one completion round-trip. Kept short so a slow or
/// unresponsive kernel doesn't freeze the prompt — on timeout we return
/// empty matches, which reedline renders as "no completions."
const COMPLETE_TIMEOUT: Duration = Duration::from_secs(2);

pub struct JetCompleter {
    handle: CompletionHandle,
    rt: tokio::runtime::Handle,
}

impl JetCompleter {
    pub fn new(handle: CompletionHandle, rt: tokio::runtime::Handle) -> Self {
        Self { handle, rt }
    }
}

impl Completer for JetCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let code = line.to_string();
        let handle = self.handle.clone();
        // We're on the blocking thread pool (spawn_blocking in repl.rs);
        // block_on here is safe because this thread isn't a tokio worker.
        let reply = self.rt.block_on(async move {
            tokio::time::timeout(COMPLETE_TIMEOUT, handle.complete(code, pos)).await
        });
        let reply = match reply {
            Ok(Ok(Some(r))) => r,
            Ok(Ok(None)) => return Vec::new(),
            Ok(Err(e)) => {
                log::warn!("complete_request failed: {e}");
                return Vec::new();
            }
            Err(_) => {
                log::warn!("complete_request timed out");
                return Vec::new();
            }
        };
        // Jupyter's cursor_start/cursor_end are byte offsets into `code`
        // marking the span the match should replace.
        let span = Span::new(reply.cursor_start, reply.cursor_end);
        reply
            .matches
            .into_iter()
            .map(|m| Suggestion {
                value: m,
                description: None,
                style: None,
                extra: None,
                span,
                append_whitespace: false,
                display_override: None,
                match_indices: None,
            })
            .collect()
    }
}

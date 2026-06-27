//! rustyline `Helper` that drives Tab completion through the kernel's
//! `complete_request` / `complete_reply` shell-channel exchange.
//!
//! rustyline's `Completer::complete` is sync and runs on the blocking
//! thread (we already wrap `readline` in `tokio::task::spawn_blocking`),
//! so we use `tokio::runtime::Handle::block_on` to drive the async
//! kernel request from the worker thread. The handle is cloned from the
//! ambient `#[tokio::main]` runtime once at editor construction.

use std::time::Duration;

use jet_core::client::CompletionHandle;
use rustyline::completion::{Completer, Pair};
use rustyline_derive::{Helper, Highlighter, Hinter, Validator};

/// Timeout for one completion round-trip. Kept short so a slow or
/// unresponsive kernel doesn't freeze the prompt — on timeout we return
/// empty matches, which rustyline renders as "no completions."
const COMPLETE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Helper, Hinter, Highlighter, Validator)]
pub struct JetHelper {
    handle: CompletionHandle,
    rt: tokio::runtime::Handle,
}

impl JetHelper {
    pub fn new(handle: CompletionHandle, rt: tokio::runtime::Handle) -> Self {
        Self { handle, rt }
    }
}

impl Completer for JetHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let code = line.to_string();
        let handle = self.handle.clone();
        // We're on the blocking thread pool (spawn_blocking in repl.rs);
        // block_on here is safe because this thread isn't a tokio worker.
        let reply = self.rt.block_on(async move {
            tokio::time::timeout(COMPLETE_TIMEOUT, handle.complete(code, pos)).await
        });
        let reply = match reply {
            Ok(Ok(Some(r))) => r,
            Ok(Ok(None)) => return Ok((pos, Vec::new())),
            Ok(Err(e)) => {
                log::warn!("complete_request failed: {e}");
                return Ok((pos, Vec::new()));
            }
            Err(_) => {
                log::warn!("complete_request timed out");
                return Ok((pos, Vec::new()));
            }
        };
        // Jupyter's cursor_start/cursor_end are byte offsets into `code`
        // marking the span the match should replace. rustyline wants the
        // start byte index plus the candidate text; the kernel's
        // `matches` are full replacements for [cursor_start, cursor_end).
        let pairs = reply
            .matches
            .into_iter()
            .map(|m| Pair {
                display: m.clone(),
                replacement: m,
            })
            .collect();
        Ok((reply.cursor_start, pairs))
    }
}


//! [`LspBackend`] — the LSP data plane, decoupled from tower-lsp's
//! transport. Holds the document store and the kernel completion handle.
//!
//! Two callers share this type:
//! - the tower-lsp [`super::server::LspServer`] adapter, which routes
//!   JSON-RPC handlers through the plain methods below;
//! - internal Rust consumers (the CLI completer, the Lua binding), which
//!   call [`LspBackend::complete_line`] directly with a plain
//!   line + byte cursor and skip the LSP position dance.
//!
//! Both paths hit the same kernel round-trip.

use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use tower_lsp::lsp_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
};

use crate::client::CompletionHandle;

use super::completion::reply_to_response;
use super::documents::Documents;
use super::position::position_to_byte;

/// Ceiling on a single `complete_request` round trip. A slow or
/// unresponsive kernel shouldn't freeze either the LSP or the REPL prompt.
pub const COMPLETE_TIMEOUT: Duration = Duration::from_secs(2);

/// One completion match with the byte span it should replace in the
/// original buffer. Returned by [`LspBackend::complete_line`].
#[derive(Debug, Clone)]
pub struct LineMatch {
    pub value: String,
    pub replace: Range<usize>,
}

#[derive(Clone)]
pub struct LspBackend {
    documents: Documents,
    completion: CompletionHandle,
}

impl LspBackend {
    pub fn new(completion: CompletionHandle) -> Arc<Self> {
        Arc::new(Self {
            documents: Documents::default(),
            completion,
        })
    }

    pub fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents
            .open(params.text_document.uri, &params.text_document.text);
    }

    pub fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if !self.documents.apply_changes(&uri, &params.content_changes) {
            log::warn!("didChange for unknown document: {uri}");
        }
    }

    pub fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.close(&params.text_document.uri);
    }

    /// Serve a `textDocument/completion` request. Returns `None` when
    /// the URI is unknown or the kernel yields nothing within
    /// [`COMPLETE_TIMEOUT`].
    pub async fn completion(&self, params: CompletionParams) -> Option<CompletionResponse> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let (code, cursor_byte) = {
            let rope = self.documents.get(uri)?;
            let cursor = position_to_byte(pos, rope.value());
            (rope.value().to_string(), cursor)
        };

        let reply = self.request_complete(code, cursor_byte).await?;

        // Reacquire the rope for byte->Position mapping. The client may
        // have edited between the initial snapshot and the reply; clamp
        // against the current state. If the doc was closed under us,
        // drop the reply — the client won't apply it anyway.
        let rope = self.documents.get(uri)?;
        Some(reply_to_response(reply, rope.value()))
    }

    /// Complete `line` at byte offset `cursor` and return each match
    /// with the byte span in `line` it should replace.
    ///
    /// Convenience for internal Rust callers (the CLI completer, the Lua
    /// binding) that already have a plain buffer and byte cursor and
    /// don't want to marshal an LSP `Position` just to have the backend
    /// convert it back. Same kernel round-trip as [`Self::completion`],
    /// same timeout, no shared document state.
    pub async fn complete_line(&self, line: &str, cursor: usize) -> Vec<LineMatch> {
        let cursor = cursor.min(line.len());
        let Some(reply) = self.request_complete(line.to_string(), cursor).await else {
            return Vec::new();
        };
        // The kernel returns byte offsets into the code string we sent,
        // which for one line == the offsets we hand back. Clamp for
        // safety in case a kernel returns something odd.
        let start = reply.cursor_start.min(line.len());
        let end = reply.cursor_end.min(line.len()).max(start);
        reply
            .matches
            .into_iter()
            .map(|value| LineMatch {
                value,
                replace: start..end,
            })
            .collect()
    }

    /// Shared kernel round-trip. Returns `None` on timeout, transport
    /// error, or an empty reply — every caller treats those as "no
    /// completions."
    async fn request_complete(
        &self,
        code: String,
        cursor: usize,
    ) -> Option<jupyter_protocol::CompleteReply> {
        let fut = self.completion.complete(code, cursor);
        match tokio::time::timeout(COMPLETE_TIMEOUT, fut).await {
            Ok(Ok(Some(r))) => Some(r),
            Ok(Ok(None)) => None,
            Ok(Err(e)) => {
                log::warn!("complete_request failed: {e}");
                None
            }
            Err(_) => {
                log::warn!("complete_request timed out");
                None
            }
        }
    }
}


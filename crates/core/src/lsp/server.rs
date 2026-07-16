//! tower-lsp [`LanguageServer`] adapter over an [`LspBackend`].
//!
//! Kept intentionally thin: every handler delegates to a plain method on
//! `LspBackend`, so the in-process CLI path and the JSON-RPC path hit
//! identical code.

use std::sync::Arc;

use tower_lsp::jsonrpc::Result as RpcResult;
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    InitializedParams, PositionEncodingKind, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp::{Client as LspClient, LanguageServer};

use super::backend::LspBackend;

pub struct LspServer {
    backend: Arc<LspBackend>,
    /// tower-lsp's client handle. Kept around so we can push
    /// `window/logMessage` etc. if we grow diagnostics later.
    #[allow(dead_code)]
    client: LspClient,
}

impl LspServer {
    pub fn new(backend: Arc<LspBackend>, client: LspClient) -> Self {
        Self { backend, client }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LspServer {
    async fn initialize(&self, _params: InitializeParams) -> RpcResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "jet-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF16),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    // Trigger on `.` at minimum; the kernel decides
                    // what's actually completable from `cursor_start`.
                    trigger_characters: Some(vec![".".into()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("jet-lsp: client initialized");
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.backend.did_open(params);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.backend.did_change(params);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.backend.did_close(params);
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> RpcResult<Option<CompletionResponse>> {
        Ok(self.backend.completion(params).await)
    }
}

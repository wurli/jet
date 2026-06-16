//! kallichore HTTP/WebSocket client.
//!
//! `Client` owns the kcserver process (when we spawned it), the auth token,
//! and the base URL. Drop the client and the server dies with it.

pub mod api;

mod server;
mod session;

use std::{path::PathBuf, time::Duration};

pub use api::types::ActiveSession;
pub use server::ConnectionFile;
use server::{ChildGuard, probe_status, spawn_kcserver, wait_for_status};

use anyhow::{Result, anyhow};
use futures_util::stream::SplitSink;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsSink = SplitSink<WsStream, Message>;

/// Write-half of the channels websocket. Owns the sink so callers can
/// `send(&msg)` without re-importing `SinkExt` or the tungstenite types.
pub struct Channel {
    sink: WsSink,
}

impl Channel {
    pub fn new(sink: WsSink) -> Self {
        Self { sink }
    }

    pub async fn send(&mut self, msg: &Value) -> Result<()> {
        use futures_util::SinkExt;
        log::trace!("ws send: {msg}");
        self.sink
            .send(Message::Text(msg.to_string().into()))
            .await?;
        Ok(())
    }
}

/// Bits we need for the WebSocket upgrade — the auto-generated `api::Client`
/// handles bearer auth for HTTP, but the WS path connects directly via
/// tungstenite and needs the headers re-applied.
struct WsAuth {
    base: String,
    bearer: String,
}

pub struct Client {
    api: api::Client,
    ws_auth: WsAuth,
    /// Server we spawned; kept alive (and killed on drop) for the lifetime
    /// of the client. `None` when connecting to an existing server.
    _server: Option<ChildGuard>,
}

impl Client {
    /// Spawn a fresh `kcserver` and connect to it.
    pub async fn spawn(bin: &str, connection_file: Option<PathBuf>) -> Result<Self> {
        log::info!("Spawning kcserver: {bin}");
        let (conn, server) = spawn_kcserver(bin, connection_file).await?;
        let (api, ws_auth) = Self::build_api(conn)?;
        wait_for_status(&api, Duration::from_secs(3)).await?;
        log::info!("kcserver ready at {}", ws_auth.base);
        Ok(Self {
            api,
            ws_auth,
            _server: Some(server),
        })
    }

    /// Connect to an already-running `kcserver` via its connection file.
    pub async fn connect(connection_file: &std::path::Path) -> Result<Self> {
        log::info!("Connecting to kcserver via existing {connection_file:?}");
        let conn = ConnectionFile::read(connection_file)?;
        let (api, ws_auth) = Self::build_api(conn)?;
        probe_status(&api).await?;
        log::info!("kcserver ready at {}", ws_auth.base);
        Ok(Self {
            api,
            ws_auth,
            _server: None,
        })
    }

    /// Join a running `kcserver` if possible, otherwise spawn a new one.
    pub async fn connect_or_spawn(bin: &str, connection_file: &std::path::Path) -> Result<Self> {
        log::info!("Attempting to connect or spawn a new kcserver with {connection_file:?}");
        match Self::connect(connection_file).await {
            Err(e) => log::warn!(
                "Failed to connect with {connection_file:?}: {e}, will spawn a new kcserver"
            ),
            Ok(client) => return Ok(client),
        }

        Self::spawn(bin, Some(connection_file.to_path_buf())).await
    }

    fn build_api(conn: ConnectionFile) -> Result<(api::Client, WsAuth)> {
        let base = conn
            .base_path
            .clone()
            .or_else(|| conn.port.map(|p| format!("http://127.0.0.1:{p}")))
            .ok_or_else(|| anyhow!("connection file has neither base_path nor port"))?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", conn.bearer_token).parse()?,
        );
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        let api = api::Client::new_with_client(&base, http);
        let ws_auth = WsAuth {
            base,
            bearer: conn.bearer_token,
        };
        Ok((api, ws_auth))
    }

    pub fn base(&self) -> &str {
        &self.ws_auth.base
    }

    /// If this client spawned the `kcserver`, leave it running on drop.
    /// No-op for clients that connected to an existing server.
    pub fn detach_server(&mut self) {
        if let Some(server) = self._server.as_mut() {
            server.detach();
        }
    }

    /// `PUT /sessions` — create a new session.
    pub async fn create_session(
        &self,
        session_id: &str,
        language: &str,
        argv: &[String],
    ) -> Result<()> {
        session::create(&self.api, session_id, language, argv).await
    }

    /// `POST /sessions/{id}/start` — start the kernel for an existing session.
    pub async fn start_session(&self, session_id: &str) -> Result<()> {
        session::start(&self.api, session_id).await
    }

    /// `GET /sessions` — list active sessions on the server.
    pub async fn list_sessions(&self) -> Result<Vec<ActiveSession>> {
        session::list(&self.api).await
    }

    /// Open the channels websocket for a session. The websocket is
    /// returned **before** the session is started; this lets the caller
    /// avoid a race where startup messages arrive before they're listening.
    pub async fn open_channels(&self, session_id: &str) -> Result<WsStream> {
        session::open_channels(&self.ws_auth.base, &self.ws_auth.bearer, session_id).await
    }
}

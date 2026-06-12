//! kallichore HTTP/WebSocket client.
//!
//! `Client` owns the kcserver process (when we spawned it), the auth token,
//! and the base URL. Drop the client and the server dies with it.

pub mod api;
mod server;
mod session;

pub use server::ConnectionFile;
use server::{spawn_kcserver, wait_for_status, ChildGuard};

use anyhow::{anyhow, Result};
use futures_util::stream::SplitSink;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type WsSink = SplitSink<WsStream, Message>;

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
    pub async fn spawn(bin: &str) -> Result<Self> {
        let (conn, server) = spawn_kcserver(bin).await?;
        Self::from_conn(conn, Some(server)).await
    }

    /// Connect to an already-running `kcserver` via its connection file.
    pub async fn connect(connection_file: &std::path::Path) -> Result<Self> {
        let conn = ConnectionFile::read(connection_file)?;
        Self::from_conn(conn, None).await
    }

    async fn from_conn(conn: ConnectionFile, server: Option<ChildGuard>) -> Result<Self> {
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

        wait_for_status(&api).await?;
        Ok(Self {
            api,
            ws_auth: WsAuth {
                base,
                bearer: conn.bearer_token,
            },
            _server: server,
        })
    }

    pub fn base(&self) -> &str {
        &self.ws_auth.base
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

    /// Open the channels websocket for a session. The websocket is
    /// returned **before** the session is started; this lets the caller
    /// avoid a race where startup messages arrive before they're listening.
    pub async fn open_channels(&self, session_id: &str) -> Result<WsStream> {
        session::open_channels(&self.ws_auth.base, &self.ws_auth.bearer, session_id).await
    }
}

/// Convenience: send a Jupyter message as a tungstenite Text frame.
pub async fn send(sink: &mut WsSink, msg: &Value) -> Result<()> {
    use futures_util::SinkExt;
    sink.send(Message::Text(msg.to_string().into())).await?;
    Ok(())
}

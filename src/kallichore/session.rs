//! Session HTTP endpoints + channels websocket upgrade.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::json;

use super::WsStream;

pub async fn create(
    http: &reqwest::Client,
    base: &str,
    session_id: &str,
    language: &str,
    argv: &[String],
) -> Result<()> {
    let body = json!({
        "session_id": session_id,
        "display_name": "jet",
        "language": language,
        "username": whoami::username(),
        "input_prompt": ">>> ",
        "continuation_prompt": "... ",
        "argv": argv,
        "session_mode": "console",
        "working_directory": std::env::current_dir()?.to_string_lossy(),
        "env": [],
        "interrupt_mode": "signal",
        "startup_environment": "none",
    });
    let r = http
        .put(format!("{base}/sessions"))
        .json(&body)
        .send()
        .await?;
    if !r.status().is_success() {
        bail!(
            "PUT /sessions failed: {} — {}",
            r.status(),
            r.text().await.unwrap_or_default()
        );
    }
    Ok(())
}

pub async fn start(http: &reqwest::Client, base: &str, session_id: &str) -> Result<()> {
    let r = http
        .post(format!("{base}/sessions/{session_id}/start"))
        .send()
        .await?;
    if !r.status().is_success() {
        bail!(
            "POST /sessions/{session_id}/start failed: {} — {}",
            r.status(),
            r.text().await.unwrap_or_default()
        );
    }
    Ok(())
}

pub async fn open_channels(base: &str, bearer: &str, session_id: &str) -> Result<WsStream> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let url = ws_url(base, session_id)?;
    let mut req = url.as_str().into_client_request()?;
    req.headers_mut()
        .insert("Authorization", format!("Bearer {bearer}").parse()?);
    let (ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .with_context(|| format!("websocket connect failed: {url}"))?;
    Ok(ws)
}

pub fn ws_url(base: &str, session_id: &str) -> Result<url::Url> {
    let mut u = url::Url::parse(base)?;
    let scheme = match u.scheme() {
        "https" => "wss",
        _ => "ws",
    };
    u.set_scheme(scheme)
        .map_err(|_| anyhow!("set_scheme failed"))?;
    u.set_path(&format!("/sessions/{session_id}/channels"));
    Ok(u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_url_swaps_http_for_ws() {
        let u = ws_url("http://127.0.0.1:8080", "abc").unwrap();
        assert_eq!(u.scheme(), "ws");
        assert_eq!(u.path(), "/sessions/abc/channels");
        let u = ws_url("https://example.com:9000", "xyz").unwrap();
        assert_eq!(u.scheme(), "wss");
    }
}

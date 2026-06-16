//! Session HTTP endpoints + channels websocket upgrade.

use anyhow::{Context, Result, anyhow};

use super::WsStream;
use super::api::{self, types};

pub async fn create(
    api: &api::Client,
    session_id: &str,
    display_name: &str,
    language: &str,
    argv: &[String],
    env: &std::collections::HashMap<String, String>,
    interrupt_mode: types::InterruptMode,
) -> Result<()> {
    let env_actions = env
        .iter()
        .map(|(name, value)| types::VarAction {
            action: types::VarActionType::Replace,
            name: name.clone(),
            value: value.clone(),
        })
        .collect();
    let body = types::NewSession {
        session_id: session_id.to_string(),
        display_name: display_name.to_string(),
        language: language.to_string(),
        username: whoami::username(),
        input_prompt: ">>> ".into(),
        continuation_prompt: "... ".into(),
        argv: argv.to_vec(),
        session_mode: types::SessionMode::Console,
        working_directory: std::env::current_dir()?.to_string_lossy().into_owned(),
        env: types::EnvVarActions(env_actions),
        interrupt_mode,
        startup_environment: types::StartupEnvironment::None,
        // Defaults from the spec; expressed explicitly so we don't depend on
        // serde defaults firing at the right layer.
        connection_timeout: 30,
        protocol_version: "5.3".into(),
        notebook_uri: None,
        startup_environment_arg: None,
    };
    api.new_session(&body)
        .await
        .map_err(|e| anyhow!("PUT /sessions failed: {e}"))?;
    Ok(())
}

pub async fn start(api: &api::Client, session_id: &str) -> Result<()> {
    api.start_session(session_id)
        .await
        .map_err(|e| anyhow!("POST /sessions/{session_id}/start failed: {e}"))?;
    Ok(())
}

pub async fn kill(api: &api::Client, session_id: &str) -> Result<()> {
    api.kill_session(session_id)
        .await
        .map_err(|e| anyhow!("POST /sessions/{session_id}/kill failed: {e}"))?;
    Ok(())
}

pub async fn interrupt(api: &api::Client, session_id: &str) -> Result<()> {
    api.interrupt_session(session_id)
        .await
        .map_err(|e| anyhow!("POST /sessions/{session_id}/interrupt failed: {e}"))?;
    Ok(())
}

pub async fn delete(api: &api::Client, session_id: &str) -> Result<()> {
    api.delete_session(session_id)
        .await
        .map_err(|e| anyhow!("DELETE /sessions/{session_id} failed: {e}"))?;
    Ok(())
}

pub async fn shutdown_server(api: &api::Client) -> Result<()> {
    api.shutdown_server()
        .await
        .map_err(|e| anyhow!("POST /shutdown failed: {e}"))?;
    Ok(())
}

pub async fn list(api: &api::Client) -> Result<Vec<types::ActiveSession>> {
    let resp = api
        .list_sessions()
        .await
        .map_err(|e| anyhow!("GET /sessions failed: {e}"))?;
    Ok(resp.into_inner().sessions)
}

pub async fn open_channels(base: &str, bearer: &str, session_id: &str) -> Result<WsStream> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let url = ws_url(base, session_id)?;
    log::debug!("opening channels websocket: {url}");
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

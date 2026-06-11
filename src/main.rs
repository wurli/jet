// jet — a kallichore-backed REPL with kitty graphics.
//
// Spawns `kcserver` with a connection file, opens a session for a Jupyter
// kernel (default: ipython), connects to the per-session WebSocket, and
// drives a line-oriented REPL. PNG outputs from the kernel are rendered
// inline with the kitty graphics protocol.

use std::io::{Read, Write as IoWrite};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;

#[derive(Parser, Debug)]
#[command(name = "jet", about = "kallichore-backed REPL with kitty graphics")]
struct Args {
    /// Path to the kcserver binary.
    #[arg(long, default_value = "kcserver")]
    kcserver: String,

    /// Connect to an already-running kcserver instead of spawning one.
    /// Pass the path to its connection file.
    #[arg(long)]
    connect: Option<PathBuf>,

    /// Kernel argv. Pass after `--`. Use `{connection_file}` as the
    /// placeholder kallichore replaces with the generated connection file.
    /// Default starts an ipython kernel.
    /// Example: jet --language r -- /path/to/ark --connection_file {connection_file} --session-mode console
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    kernel: Vec<String>,

    /// Language label for the session.
    #[arg(long, default_value = "python")]
    language: String,

    /// Disable kitty graphics; PNGs are reported as `[image/png NxN bytes]`.
    #[arg(long)]
    no_graphics: bool,
}

#[derive(Debug, Deserialize)]
struct ConnectionFile {
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    base_path: Option<String>,
    bearer_token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let (conn, _server) = match &args.connect {
        Some(path) => (read_conn_file(path)?, None),
        None => spawn_server(&args.kcserver).await?,
    };

    let base = conn
        .base_path
        .clone()
        .or_else(|| conn.port.map(|p| format!("http://127.0.0.1:{p}")))
        .ok_or_else(|| anyhow!("connection file has neither base_path nor port"))?;

    let http = reqwest::Client::builder()
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", conn.bearer_token).parse()?,
            );
            h
        })
        .build()?;

    wait_for_status(&http, &base).await?;

    let session_id = format!("jet-{:x}", rand::thread_rng().gen::<u64>());
    let kernel_argv = build_kernel_argv(&args.kernel);
    create_session(&http, &base, &session_id, &args.language, &kernel_argv).await?;

    // Open the channels websocket BEFORE start so we don't miss startup messages.
    let ws_url = ws_url_from_base(&base, &session_id)?;
    let mut req = ws_url.clone().into_client_request()?;
    req.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", conn.bearer_token).parse()?,
    );
    let (ws, _) = tokio_tungstenite::connect_async(req)
        .await
        .with_context(|| format!("websocket connect failed: {ws_url}"))?;
    let (ws_sink, ws_stream) = ws.split();
    let ws_sink = Arc::new(Mutex::new(ws_sink));

    start_session(&http, &base, &session_id).await?;

    // Channel from the WS reader to the REPL: signals "kernel is idle for msg X".
    let (idle_tx, mut idle_rx) = mpsc::unbounded_channel::<String>();

    // Spawn the websocket reader. It prints kernel output as it arrives.
    let render_graphics = !args.no_graphics;
    if render_graphics && std::env::var_os("TMUX").is_some() {
        warn_if_tmux_passthrough_off();
    }
    tokio::spawn(async move {
        let mut stream = ws_stream;
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    if let Err(e) = handle_ws_text(&t, render_graphics, &idle_tx) {
                        eprintln!("\x1b[31m[jet] {e}\x1b[0m");
                    }
                }
                Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(e) => {
                    eprintln!("\x1b[31m[jet] ws error: {e}\x1b[0m");
                    break;
                }
            }
        }
    });

    // REPL.
    let mut rl = rustyline::DefaultEditor::new()?;
    println!("jet — connected to session {session_id}. ^D to quit.");
    loop {
        let line = match rl.readline(">>> ") {
            Ok(l) => l,
            Err(rustyline::error::ReadlineError::Eof)
            | Err(rustyline::error::ReadlineError::Interrupted) => break,
            Err(e) => {
                eprintln!("[jet] readline: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(&line);

        let msg_id = new_msg_id();
        let req = jupyter_message(
            "shell",
            &msg_id,
            "execute_request",
            json!({
                "code": line,
                "silent": false,
                "store_history": true,
                "user_expressions": {},
                "allow_stdin": false,
                "stop_on_error": true,
            }),
        );
        ws_sink
            .lock()
            .await
            .send(Message::Text(req.to_string()))
            .await?;

        // Wait for the kernel to report idle for our request, with a timeout.
        let deadline = Instant::now() + Duration::from_secs(300);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                eprintln!("\x1b[33m[jet] timeout waiting for kernel\x1b[0m");
                break;
            }
            match tokio::time::timeout(remaining, idle_rx.recv()).await {
                Ok(Some(parent)) if parent == msg_id => break,
                Ok(Some(_)) => continue,
                Ok(None) => {
                    eprintln!("\x1b[31m[jet] websocket closed\x1b[0m");
                    return Ok(());
                }
                Err(_) => {
                    eprintln!("\x1b[33m[jet] timeout waiting for kernel\x1b[0m");
                    break;
                }
            }
        }
    }

    Ok(())
}

// ---------- kcserver lifecycle ----------

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn spawn_server(bin: &str) -> Result<(ConnectionFile, Option<ChildGuard>)> {
    let conn_path = std::env::temp_dir().join(format!(
        "jet-kc-{:x}.json",
        rand::thread_rng().gen::<u64>()
    ));
    // Make sure stale file doesn't trick us.
    let _ = std::fs::remove_file(&conn_path);

    let child = Command::new(bin)
        .arg("--connection-file")
        .arg(&conn_path)
        .arg("--transport")
        .arg("tcp")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {bin}"))?;
    let guard = ChildGuard(child);

    // Poll for the connection file.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if conn_path.exists() {
            // Give the server a moment to finish writing.
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Ok(c) = read_conn_file(&conn_path) {
                return Ok((c, Some(guard)));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("timed out waiting for kcserver connection file at {conn_path:?}");
}

fn read_conn_file(path: &std::path::Path) -> Result<ConnectionFile> {
    let mut s = String::new();
    std::fs::File::open(path)
        .with_context(|| format!("opening {path:?}"))?
        .read_to_string(&mut s)?;
    Ok(serde_json::from_str(&s)?)
}

async fn wait_for_status(http: &reqwest::Client, base: &str) -> Result<()> {
    let url = format!("{base}/status");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(r) = http.get(&url).send().await {
            if r.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("kcserver /status never became ready at {url}");
}

// ---------- session creation ----------

fn build_kernel_argv(custom: &[String]) -> Vec<String> {
    if !custom.is_empty() {
        return custom.to_vec();
    }
    // Default: ipython kernel. {connection_file} is replaced by kallichore.
    // Resolve `python3`/`python` to an absolute path — kallichore requires it.
    let python = which_python().unwrap_or_else(|| "python3".into());
    vec![
        python,
        "-m".into(),
        "ipykernel_launcher".into(),
        "-f".into(),
        "{connection_file}".into(),
    ]
}

fn which_python() -> Option<String> {
    for name in ["python3", "python"] {
        if let Ok(out) = Command::new("which").arg(name).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}

async fn create_session(
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
        bail!("PUT /sessions failed: {} — {}", r.status(), r.text().await.unwrap_or_default());
    }
    Ok(())
}

async fn start_session(http: &reqwest::Client, base: &str, session_id: &str) -> Result<()> {
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

fn ws_url_from_base(base: &str, session_id: &str) -> Result<url::Url> {
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

trait IntoClientRequest {
    fn into_client_request(self) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request>;
}
impl IntoClientRequest for url::Url {
    fn into_client_request(self) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest as TIcr;
        Ok(self.as_str().into_client_request()?)
    }
}

// ---------- jupyter / websocket framing ----------

fn new_msg_id() -> String {
    format!("{:032x}", rand::thread_rng().gen::<u128>())
}

#[derive(Serialize)]
struct JupyterHeader {
    msg_id: String,
    msg_type: String,
    username: String,
    session: String,
    date: String,
    version: String,
}

fn jupyter_message(channel: &str, msg_id: &str, msg_type: &str, content: Value) -> Value {
    let header = JupyterHeader {
        msg_id: msg_id.to_string(),
        msg_type: msg_type.to_string(),
        username: whoami::username(),
        session: "jet".into(),
        date: chrono_like_now(),
        version: "5.3".into(),
    };
    json!({
        "channel": channel,
        "header": header,
        "parent_header": null,
        "metadata": {},
        "content": content,
        "buffers": [],
    })
}

// Avoid pulling in `chrono` for one timestamp. Format ISO-8601 in UTC.
fn chrono_like_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let nanos = now.subsec_nanos();
    // Days from epoch to civil date — Howard Hinnant's algorithm.
    let z = secs.div_euclid(86_400) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    let sod = secs.rem_euclid(86_400) as u64;
    let h = sod / 3600;
    let mi = (sod % 3600) / 60;
    let s = sod % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
        y, m, d, h, mi, s, nanos / 1000
    )
}

fn handle_ws_text(
    text: &str,
    render_graphics: bool,
    idle_tx: &mpsc::UnboundedSender<String>,
) -> Result<()> {
    let v: Value = serde_json::from_str(text)?;
    let channel = v.get("channel").and_then(|s| s.as_str()).unwrap_or("");
    let msg_type = v
        .pointer("/header/msg_type")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let parent_id = v
        .pointer("/parent_header/msg_id")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let content = v.get("content").cloned().unwrap_or(Value::Null);

    match (channel, msg_type) {
        ("iopub", "stream") => {
            let name = content.get("name").and_then(|s| s.as_str()).unwrap_or("stdout");
            let txt = content.get("text").and_then(|s| s.as_str()).unwrap_or("");
            let mut out = std::io::stdout();
            if name == "stderr" {
                let _ = write!(out, "\x1b[31m{txt}\x1b[0m");
            } else {
                let _ = write!(out, "{txt}");
            }
            let _ = out.flush();
        }
        ("iopub", "execute_result") | ("iopub", "display_data") => {
            render_data(&content, render_graphics);
        }
        ("iopub", "error") => {
            let traceback = content
                .get("traceback")
                .and_then(|t| t.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            println!("\x1b[31m{traceback}\x1b[0m");
        }
        ("iopub", "status") => {
            let state = content
                .get("execution_state")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            if state == "idle" && !parent_id.is_empty() {
                let _ = idle_tx.send(parent_id);
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_data(content: &Value, render_graphics: bool) {
    let data = match content.get("data") {
        Some(Value::Object(_)) => content.get("data").unwrap(),
        _ => return,
    };
    if render_graphics {
        if let Some(b64) = data.get("image/png").and_then(|s| s.as_str()) {
            if let Err(e) = emit_kitty_png(b64) {
                eprintln!("\x1b[33m[jet] kitty render failed: {e}\x1b[0m");
            } else {
                return;
            }
        }
    } else if let Some(b64) = data.get("image/png").and_then(|s| s.as_str()) {
        let len = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map(|b| b.len())
            .unwrap_or(0);
        println!("[image/png {len} bytes]");
        return;
    }
    if let Some(t) = data.get("text/plain").and_then(|s| s.as_str()) {
        println!("{t}");
    }
}

// Warn (once, at startup) if we're inside tmux and `allow-passthrough` is
// off — the kitty graphics escapes will be silently swallowed.
fn warn_if_tmux_passthrough_off() {
    // Check pane-scope first (snacks.nvim and friends set it per-pane), then
    // fall back to global.
    for scope in ["-pv", "-gv"] {
        let out = match Command::new("tmux")
            .args(["show-options", scope, "allow-passthrough"])
            .output()
        {
            Ok(o) if o.status.success() => o.stdout,
            _ => continue,
        };
        let val = String::from_utf8_lossy(&out).trim().to_lowercase();
        if val == "on" || val == "all" {
            return;
        }
    }
    eprintln!(
        "\x1b[33m[jet] warning: tmux `allow-passthrough` is off. \
         Kitty graphics will not render inline.\n\
         Enable it in this pane:    tmux set -p allow-passthrough all\n\
         Or globally in your config: set -g allow-passthrough on\x1b[0m"
    );
}

// ---------- kitty graphics protocol ----------
//
// Format: \x1b_G<keys>;<payload>\x1b\\
// We use a=T (transmit & display), f=100 (PNG), m=1 for chunked,
// q=2 (suppress responses). Final chunk has m=0.
//
// Inside tmux, the kitty escape is wrapped in a DCS passthrough envelope
// so tmux forwards it to the outer terminal: \x1bPtmux;<payload>\x1b\\,
// with every \x1b inside <payload> doubled. tmux requires
// `set -g allow-passthrough on` for this to work; recent tmux defaults to
// allowing it, but older versions may need the option set.
fn emit_kitty_png(b64_png: &str) -> Result<()> {
    // Strip whitespace (some kernels insert line breaks) and ensure the
    // base64 is `=`-padded — ark/R omits trailing padding, but kitty's
    // graphics decoder rejects unpadded base64 and silently drops the image.
    let mut payload: String = b64_png.chars().filter(|c| !c.is_whitespace()).collect();
    let pad = (4 - payload.len() % 4) % 4;
    for _ in 0..pad {
        payload.push('=');
    }

    // Decode just enough of the PNG to learn its pixel height so we can
    // advance the cursor past the image. The IHDR chunk is at a fixed
    // offset: 8-byte signature, 4-byte length, 4-byte "IHDR", 4-byte width,
    // 4-byte height. We only need the first ~24 base64 chars.
    let img_px_height = png_height_from_b64_prefix(&payload).unwrap_or(0);
    let cell_px = std::env::var("JET_CELL_PX_HEIGHT")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(20);
    let rows = if img_px_height > 0 {
        img_px_height.div_ceil(cell_px).max(1)
    } else {
        1
    };

    let bytes = payload.as_bytes();
    const CHUNK: usize = 4096;

    let mut raw = Vec::with_capacity(bytes.len() + 64);
    if bytes.len() <= CHUNK {
        write!(raw, "\x1b_Ga=T,f=100,q=2;{}\x1b\\", payload)?;
    } else {
        let mut i = 0;
        let mut first = true;
        while i < bytes.len() {
            let end = (i + CHUNK).min(bytes.len());
            let more = if end < bytes.len() { 1 } else { 0 };
            let slice = &bytes[i..end];
            if first {
                write!(raw, "\x1b_Ga=T,f=100,q=2,m={};", more)?;
                first = false;
            } else {
                write!(raw, "\x1b_Gm={};", more)?;
            }
            raw.extend_from_slice(slice);
            raw.extend_from_slice(b"\x1b\\");
            i = end;
        }
    }

    let mut out = std::io::stdout().lock();
    if std::env::var_os("TMUX").is_some() {
        // Wrap in tmux DCS passthrough; double every ESC inside the payload.
        out.write_all(b"\x1bPtmux;")?;
        for &b in &raw {
            if b == 0x1b {
                out.write_all(b"\x1b\x1b")?;
            } else {
                out.write_all(&[b])?;
            }
        }
        out.write_all(b"\x1b\\")?;
    } else {
        out.write_all(&raw)?;
    }
    // Advance the cursor past the image so the next prompt doesn't draw on
    // top of it. The kitty `a=T` action does not move the cursor on most
    // terminals, so we emit one newline per estimated text row of image
    // height.
    for _ in 0..rows {
        out.write_all(b"\n")?;
    }
    out.flush()?;
    Ok(())
}

// Parse just the PNG IHDR to get pixel height. PNG layout:
//   8 bytes  signature   89 50 4E 47 0D 0A 1A 0A
//   4 bytes  IHDR length (always 13)
//   4 bytes  "IHDR"
//   4 bytes  width  (big-endian)
//   4 bytes  height (big-endian)
// We only need the first 24 PNG bytes — i.e. the first 32 base64 chars.
fn png_height_from_b64_prefix(b64: &str) -> Option<u32> {
    let prefix: String = b64.chars().take(36).collect();
    // Re-pad so base64 decode works on the prefix.
    let mut p = prefix;
    while p.len() % 4 != 0 {
        p.push('=');
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(p).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    Some(u32::from_be_bytes(bytes[20..24].try_into().ok()?))
}

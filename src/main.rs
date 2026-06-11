// jet — a kallichore-backed REPL with kitty graphics.
//
// Spawns `kcserver` with a connection file, opens a session for a Jupyter
// kernel (default: ipython), connects to the per-session WebSocket, and
// drives a line-oriented REPL. PNG outputs from the kernel are rendered
// inline with the kitty graphics protocol.

use std::fmt::Write as FmtWrite;
use std::io::{Read, Write as IoWrite};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use std::os::fd::{AsRawFd, RawFd};
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

// ---------- kitty graphics protocol (unicode placeholder mode) ----------
//
// Why placeholder mode: kitty graphics drawn directly in tmux are not tracked
// by tmux — they linger across pane switches, scrolling, and redraws. With
// "unicode placeholder" mode, the image is uploaded once with an `id`, then
// "placed" by writing real text cells (U+10EEEE) that tmux can move, scroll,
// and clear like any other character. ghostty/kitty repaints the image
// wherever those placeholder cells are visible.
//
// Steps:
//   1. Transmit the PNG with `a=T,U=1,i=<id>,f=100,q=2`. `U=1` says the
//      image will be referenced from text cells, so the terminal does not
//      draw it immediately.
//   2. Write `rows × cols` of placeholder text. Each cell:
//        SGR fg = i  (low 8 bits of image id encoded as 256-color)
//        U+10EEEE  + row_diacritic + col_diacritic
//      Image-id MSB encoded via underline color SGR (skipped here — id ≤ 255).
//
// Cell pixel size defaults: 10 px wide × 20 px tall. Override with
// JET_CELL_PX_WIDTH / JET_CELL_PX_HEIGHT.

static NEXT_IMG_ID: AtomicU32 = AtomicU32::new(1);

fn emit_kitty_png(b64_png: &str) -> Result<()> {
    let mut payload: String = b64_png.chars().filter(|c| !c.is_whitespace()).collect();
    let pad = (4 - payload.len() % 4) % 4;
    for _ in 0..pad {
        payload.push('=');
    }

    let (img_w, img_h) = png_dims_from_b64_prefix(&payload).unwrap_or((0, 0));
    // Cell dimensions: env overrides win; otherwise query the terminal once
    // and cache. Falls back to typical 9×18 if the query fails.
    let (queried_w, queried_h) = cell_pixel_size().unwrap_or((9, 18));
    let cell_w = env_u32("JET_CELL_PX_WIDTH", queried_w);
    let cell_h = env_u32("JET_CELL_PX_HEIGHT", queried_h);

    // Use floor for rows so we don't reserve a blank bottom row; ceil for
    // columns so the right edge isn't clipped.
    let cols = if img_w > 0 { img_w.div_ceil(cell_w).max(1) } else { 40 };
    let rows = if img_h > 0 { (img_h / cell_h).max(1) } else { 10 };

    // Image ids are 1..=255 (low byte). We wrap; the terminal recognizes
    // the most-recent transmission for that id.
    let id = (NEXT_IMG_ID.fetch_add(1, Ordering::Relaxed) % 255) + 1;

    let bytes = payload.as_bytes();
    const CHUNK: usize = 4096;
    let mut raw = Vec::with_capacity(bytes.len() + 128);
    let mut i = 0;
    let mut first = true;
    while i < bytes.len() {
        let end = (i + CHUNK).min(bytes.len());
        let more = if end < bytes.len() { 1 } else { 0 };
        if first {
            write!(
                raw,
                "\x1b_Ga=T,U=1,i={},f=100,q=2,m={};",
                id, more
            )?;
            first = false;
        } else {
            write!(raw, "\x1b_Gm={};", more)?;
        }
        raw.extend_from_slice(&bytes[i..end]);
        raw.extend_from_slice(b"\x1b\\");
        i = end;
    }

    let mut out = std::io::stdout().lock();
    write_passthrough(&mut out, &raw)?;

    // Build the placeholder grid — `rows` lines, each `cols` cells wide.
    // Each cell: U+10EEEE then a row diacritic then a column diacritic.
    let mut grid = String::with_capacity((rows as usize) * (cols as usize) * 16);
    for r in 0..rows.min(ROW_COL_DIACRITICS.len() as u32) {
        // SGR foreground = image id (256-color)
        write!(&mut grid, "\x1b[38;5;{}m", id).unwrap();
        let row_d = ROW_COL_DIACRITICS[r as usize];
        for c in 0..cols.min(ROW_COL_DIACRITICS.len() as u32) {
            let col_d = ROW_COL_DIACRITICS[c as usize];
            grid.push('\u{10EEEE}');
            grid.push(char::from_u32(row_d).unwrap());
            grid.push(char::from_u32(col_d).unwrap());
        }
        grid.push_str("\x1b[39m\n");
    }
    out.write_all(grid.as_bytes())?;
    out.flush()?;
    Ok(())
}

fn write_passthrough<W: std::io::Write>(out: &mut W, raw: &[u8]) -> std::io::Result<()> {
    if std::env::var_os("TMUX").is_some() {
        out.write_all(b"\x1bPtmux;")?;
        for &b in raw {
            if b == 0x1b {
                out.write_all(b"\x1b\x1b")?;
            } else {
                out.write_all(&[b])?;
            }
        }
        out.write_all(b"\x1b\\")?;
    } else {
        out.write_all(raw)?;
    }
    Ok(())
}

fn env_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(default)
}

// Cached cell pixel size, queried once from the terminal via `CSI 16 t`.
// Returns (width, height) or None if the terminal didn't reply or we're
// not on a tty.
fn cell_pixel_size() -> Option<(u32, u32)> {
    static CACHE: OnceLock<Option<(u32, u32)>> = OnceLock::new();
    *CACHE.get_or_init(query_cell_pixel_size)
}

fn query_cell_pixel_size() -> Option<(u32, u32)> {
    // Must have a controlling tty for both directions.
    let in_fd: RawFd = std::io::stdin().as_raw_fd();
    let out_fd: RawFd = std::io::stdout().as_raw_fd();
    if unsafe { libc::isatty(in_fd) } == 0 || unsafe { libc::isatty(out_fd) } == 0 {
        return None;
    }

    // Save termios, switch to raw, send the query, read the reply, restore.
    let mut saved: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(in_fd, &mut saved) } != 0 {
        return None;
    }
    let mut raw = saved;
    unsafe { libc::cfmakeraw(&mut raw) };
    raw.c_cc[libc::VMIN] = 0;
    raw.c_cc[libc::VTIME] = 1; // 100 ms inter-byte timeout
    if unsafe { libc::tcsetattr(in_fd, libc::TCSANOW, &raw) } != 0 {
        return None;
    }
    // Always restore termios on exit from this scope.
    struct Restore(RawFd, libc::termios);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe { libc::tcsetattr(self.0, libc::TCSANOW, &self.1) };
        }
    }
    let _restore = Restore(in_fd, saved);

    // Send `CSI 16 t` — request cell size in pixels. Reply: `CSI 6 ; H ; W t`.
    let query = b"\x1b[16t";
    if unsafe { libc::write(out_fd, query.as_ptr() as *const _, query.len()) } < 0 {
        return None;
    }

    // Wait briefly for any response before reading.
    let mut pfd = libc::pollfd {
        fd: in_fd,
        events: libc::POLLIN,
        revents: 0,
    };
    if unsafe { libc::poll(&mut pfd, 1, 150) } <= 0 {
        return None;
    }

    // Read up to 64 bytes (reply is < 20 bytes); stop after the terminating `t`.
    let mut buf = [0u8; 64];
    let mut filled = 0usize;
    while filled < buf.len() {
        let n = unsafe {
            libc::read(
                in_fd,
                buf.as_mut_ptr().add(filled) as *mut _,
                buf.len() - filled,
            )
        };
        if n <= 0 {
            break;
        }
        filled += n as usize;
        if buf[..filled].iter().any(|&b| b == b't') {
            break;
        }
    }

    parse_cell_size_reply(&buf[..filled])
}

// Reply format: ESC [ 6 ; <height> ; <width> t
fn parse_cell_size_reply(b: &[u8]) -> Option<(u32, u32)> {
    let s = std::str::from_utf8(b).ok()?;
    // Find "[6;" then split on ';' until 't'.
    let start = s.find("[6;")?;
    let after = &s[start + 3..];
    let end = after.find('t')?;
    let mut parts = after[..end].split(';');
    let h: u32 = parts.next()?.parse().ok()?;
    let w: u32 = parts.next()?.parse().ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    Some((w, h))
}

// PNG IHDR layout (after 8-byte signature, 4-byte length, 4 bytes "IHDR"):
// width: u32 BE, height: u32 BE.
fn png_dims_from_b64_prefix(b64: &str) -> Option<(u32, u32)> {
    let mut p: String = b64.chars().take(36).collect();
    while p.len() % 4 != 0 {
        p.push('=');
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(p).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

// Diacritic codepoints used to encode row/column indices in unicode
// placeholder cells. Index N → diacritic for row/column N.
// Source: kitty rowcolumn-diacritics.txt.
#[rustfmt::skip]
const ROW_COL_DIACRITICS: &[u32] = &[
    0x0305, 0x030D, 0x030E, 0x0310, 0x0312, 0x033D, 0x033E, 0x033F, 0x0346, 0x034A,
    0x034B, 0x034C, 0x0350, 0x0351, 0x0352, 0x0357, 0x035B, 0x0363, 0x0364, 0x0365,
    0x0366, 0x0367, 0x0368, 0x0369, 0x036A, 0x036B, 0x036C, 0x036D, 0x036E, 0x036F,
    0x0483, 0x0484, 0x0485, 0x0486, 0x0487, 0x0592, 0x0593, 0x0594, 0x0595, 0x0597,
    0x0598, 0x0599, 0x059C, 0x059D, 0x059E, 0x059F, 0x05A0, 0x05A1, 0x05A8, 0x05A9,
    0x05AB, 0x05AC, 0x05AF, 0x05C4, 0x0610, 0x0611, 0x0612, 0x0613, 0x0614, 0x0615,
    0x0616, 0x0617, 0x0657, 0x0658, 0x0659, 0x065A, 0x065B, 0x065D, 0x065E, 0x06D6,
    0x06D7, 0x06D8, 0x06D9, 0x06DA, 0x06DB, 0x06DC, 0x06DF, 0x06E0, 0x06E1, 0x06E2,
    0x06E4, 0x06E7, 0x06E8, 0x06EB, 0x06EC, 0x0730, 0x0732, 0x0733, 0x0735, 0x0736,
    0x073A, 0x073D, 0x073F, 0x0740, 0x0741, 0x0743, 0x0745, 0x0747, 0x0749, 0x074A,
    0x07EB, 0x07EC, 0x07ED, 0x07EE, 0x07EF, 0x07F0, 0x07F1, 0x07F3, 0x0816, 0x0817,
    0x0818, 0x0819, 0x081B, 0x081C, 0x081D, 0x081E, 0x081F, 0x0820, 0x0821, 0x0822,
    0x0823, 0x0825, 0x0826, 0x0827, 0x0829, 0x082A, 0x082B, 0x082C, 0x082D, 0x0951,
    0x0953, 0x0954, 0x0F82, 0x0F83, 0x0F86, 0x0F87, 0x135D, 0x135E, 0x135F, 0x17DD,
    0x193A, 0x1A17, 0x1A75, 0x1A76, 0x1A77, 0x1A78, 0x1A79, 0x1A7A, 0x1A7B, 0x1A7C,
    0x1B6B, 0x1B6D, 0x1B6E, 0x1B6F, 0x1B70, 0x1B71, 0x1B72, 0x1B73, 0x1CD0, 0x1CD1,
    0x1CD2, 0x1CDA, 0x1CDB, 0x1CE0, 0x1DC0, 0x1DC1, 0x1DC3, 0x1DC4, 0x1DC5, 0x1DC6,
    0x1DC7, 0x1DC8, 0x1DC9, 0x1DCB, 0x1DCC, 0x1DD1, 0x1DD2, 0x1DD3, 0x1DD4, 0x1DD5,
    0x1DD6, 0x1DD7, 0x1DD8, 0x1DD9, 0x1DDA, 0x1DDB, 0x1DDC, 0x1DDD, 0x1DDE, 0x1DDF,
    0x1DE0, 0x1DE1, 0x1DE2, 0x1DE3, 0x1DE4, 0x1DE5, 0x1DE6, 0x1DFE, 0x20D0, 0x20D1,
    0x20D4, 0x20D5, 0x20D6, 0x20D7, 0x20DB, 0x20DC, 0x20E1, 0x20E7, 0x20E9, 0x20F0,
    0x2CEF, 0x2CF0, 0x2CF1, 0x2DE0, 0x2DE1, 0x2DE2, 0x2DE3, 0x2DE4, 0x2DE5, 0x2DE6,
    0x2DE7, 0x2DE8, 0x2DE9, 0x2DEA, 0x2DEB, 0x2DEC, 0x2DED, 0x2DEE, 0x2DEF, 0x2DF0,
    0x2DF1, 0x2DF2, 0x2DF3, 0x2DF4, 0x2DF5, 0x2DF6, 0x2DF7, 0x2DF8, 0x2DF9, 0x2DFA,
    0x2DFB, 0x2DFC, 0x2DFD, 0x2DFE, 0x2DFF, 0xA66F, 0xA67C, 0xA67D, 0xA6F0, 0xA6F1,
    0xA8E0, 0xA8E1, 0xA8E2, 0xA8E3, 0xA8E4, 0xA8E5, 0xA8E6, 0xA8E7, 0xA8E8, 0xA8E9,
    0xA8EA, 0xA8EB, 0xA8EC, 0xA8ED, 0xA8EE, 0xA8EF, 0xA8F0, 0xA8F1, 0xAAB0, 0xAAB2,
    0xAAB3, 0xAAB7, 0xAAB8, 0xAABE, 0xAABF, 0xAAC1, 0xFE20, 0xFE21, 0xFE22, 0xFE23,
    0xFE24, 0xFE25, 0xFE26, 0x10A0F, 0x10A38, 0x1D185, 0x1D186, 0x1D187, 0x1D188,
    0x1D189, 0x1D1AA, 0x1D1AB, 0x1D1AC, 0x1D1AD, 0x1D242, 0x1D243, 0x1D244,
];


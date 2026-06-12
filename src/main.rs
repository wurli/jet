// jet — a kallichore-backed REPL with kitty graphics.
//
// Spawns `kcserver` with a connection file, opens a session for a Jupyter
// kernel (default: ipython), connects to the per-session WebSocket, and
// drives a line-oriented REPL. PNG outputs from the kernel are rendered
// inline with the kitty graphics protocol.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use futures_util::StreamExt;
use rand::Rng;
use serde_json::json;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::Message;

use jet::cli::Args;
use jet::jupyter;
use jet::kallichore::{Channel, Client};
use jet::render::{warn_if_passthrough_off, Renderer, SharedWriter};

enum WaitResult {
    Idle,
    Timeout,
    Closed,
}

async fn wait_for_idle(
    rx: &mut UnboundedReceiver<String>,
    target: &str,
    timeout: Duration,
) -> WaitResult {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return WaitResult::Timeout;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(parent)) if parent == target => return WaitResult::Idle,
            Ok(Some(_)) => continue,
            Ok(None) => return WaitResult::Closed,
            Err(_) => return WaitResult::Timeout,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = match &args.connect {
        Some(path) => Client::connect(path).await?,
        None => Client::spawn(&args.kcserver).await?,
    };

    let session_id = format!("jet-{:x}", rand::thread_rng().gen::<u64>());
    let spec = args.kernel_spec();
    client
        .create_session(&session_id, &spec.language, &spec.argv)
        .await?;

    // Open the channels websocket BEFORE start so we don't miss startup messages.
    let ws = client.open_channels(&session_id).await?;
    let (ws_sink, ws_stream) = ws.split();
    let mut channel = Channel::new(ws_sink);

    client.start_session(&session_id).await?;

    let render_graphics = !args.no_graphics;
    if render_graphics {
        warn_if_passthrough_off();
    }

    // Channel from the WS reader to the REPL: signals "kernel is idle for msg X".
    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let writer: SharedWriter = Arc::new(Mutex::new(std::io::stdout()));
    let renderer = Renderer::new(render_graphics, idle_tx, writer);

    // Set on REPL exit so the WS reader task can distinguish a clean
    // shutdown (kcserver killed by Drop → reset without close handshake)
    // from a real mid-session error worth surfacing to the user.
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let reader_shutdown = shutdown.clone();

    tokio::spawn(async move {
        let mut stream = ws_stream;
        loop {
            tokio::select! {
                _ = reader_shutdown.notified() => break,
                msg = stream.next() => match msg {
                    Some(Ok(Message::Text(t))) => {
                        if let Err(e) = renderer.handle_text(&t) {
                            eprintln!("\x1b[31m[jet] {e}\x1b[0m");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        eprintln!("\x1b[31m[jet] ws error: {e}\x1b[0m");
                        break;
                    }
                },
            }
        }
    });

    // Ask the kernel for its banner/version info, and wait for the reply to be
    // fully rendered (kernel goes idle for this request) before drawing the
    // first prompt — otherwise rustyline races the async banner write.
    let info_id = jupyter::new_msg_id();
    let info_req = jupyter::message("shell", &info_id, "kernel_info_request", json!({}));
    channel.send(&info_req).await?;
    match wait_for_idle(&mut idle_rx, &info_id, Duration::from_secs(10)).await {
        WaitResult::Idle | WaitResult::Timeout => {}
        WaitResult::Closed => {
            eprintln!("\x1b[31m[jet] websocket closed\x1b[0m");
            return Ok(());
        }
    }

    let mut rl = rustyline::DefaultEditor::new()?;
    println!("jet — connected to session {session_id}. ^D to quit.");
    loop {
        let line = match rl.readline("> ") {
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

        let msg_id = jupyter::new_msg_id();
        let req = jupyter::message(
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
        channel.send(&req).await?;

        match wait_for_idle(&mut idle_rx, &msg_id, Duration::from_secs(300)).await {
            WaitResult::Idle => {}
            WaitResult::Timeout => eprintln!("\x1b[33m[jet] timeout waiting for kernel\x1b[0m"),
            WaitResult::Closed => {
                eprintln!("\x1b[31m[jet] websocket closed\x1b[0m");
                shutdown.notify_one();
                return Ok(());
            }
        }
    }

    shutdown.notify_one();
    Ok(())
}

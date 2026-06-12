// jet — a kallichore-backed REPL with kitty graphics.
//
// Spawns `kcserver` with a connection file, opens a session for a Jupyter
// kernel (default: ipython), connects to the per-session WebSocket, and
// drives a line-oriented REPL. PNG outputs from the kernel are rendered
// inline with the kitty graphics protocol.

use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use futures_util::StreamExt;
use rand::Rng;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

use jet::cli::Args;
use jet::jupyter;
use jet::kallichore::Client;
use jet::kernel;
use jet::render::{warn_if_passthrough_off, Renderer};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = match &args.connect {
        Some(path) => Client::connect(path).await?,
        None => Client::spawn(&args.kcserver).await?,
    };

    let session_id = format!("jet-{:x}", rand::thread_rng().gen::<u64>());
    let kernel_argv = kernel::build_argv(&args.kernel);
    client
        .create_session(&session_id, &args.language, &kernel_argv)
        .await?;

    // Open the channels websocket BEFORE start so we don't miss startup messages.
    let ws = client.open_channels(&session_id).await?;
    let (mut ws_sink, ws_stream) = ws.split();

    client.start_session(&session_id).await?;

    let render_graphics = !args.no_graphics;
    if render_graphics {
        warn_if_passthrough_off();
    }

    // Channel from the WS reader to the REPL: signals "kernel is idle for msg X".
    let (idle_tx, mut idle_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let renderer = Renderer::new(render_graphics, idle_tx);

    tokio::spawn(async move {
        let mut stream = ws_stream;
        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(t)) => {
                    if let Err(e) = renderer.handle_text(&t) {
                        eprintln!("\x1b[31m[jet] {e}\x1b[0m");
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(e) => {
                    eprintln!("\x1b[31m[jet] ws error: {e}\x1b[0m");
                    break;
                }
            }
        }
    });

    // Ask the kernel for its banner/version info, and wait for the reply to be
    // fully rendered (kernel goes idle for this request) before drawing the
    // first prompt — otherwise rustyline races the async banner write.
    let info_id = jupyter::new_msg_id();
    let info_req = jupyter::message("shell", &info_id, "kernel_info_request", json!({}));
    jet::kallichore::send(&mut ws_sink, &info_req).await?;
    let banner_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = banner_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, idle_rx.recv()).await {
            Ok(Some(parent)) if parent == info_id => break,
            Ok(Some(_)) => continue,
            Ok(None) => {
                eprintln!("\x1b[31m[jet] websocket closed\x1b[0m");
                return Ok(());
            }
            Err(_) => break,
        }
    }

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
        jet::kallichore::send(&mut ws_sink, &req).await?;

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

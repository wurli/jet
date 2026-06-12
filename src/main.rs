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
use jet::render::{parse_event, warn_if_passthrough_off, Event, Renderer, SharedWriter};

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

    // shutdown: REPL → reader. Set on REPL exit so the WS reader task can
    // distinguish a clean shutdown (kcserver killed by Drop → reset without
    // close handshake) from a real mid-session error worth surfacing.
    // closed: reader → REPL. Set when the websocket ends so the REPL can
    // exit immediately rather than wait for the user to press a key.
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let closed = Arc::new(tokio::sync::Notify::new());
    let reader_shutdown = shutdown.clone();
    let reader_closed = closed.clone();

    tokio::spawn(async move {
        let mut stream = ws_stream;
        loop {
            tokio::select! {
                _ = reader_shutdown.notified() => return,
                msg = stream.next() => match msg {
                    Some(Ok(Message::Text(t))) => {
                        let event = match parse_event(&t) {
                            Ok(e) => e,
                            Err(e) => {
                                eprintln!("\x1b[31m[jet] {e}\x1b[0m");
                                continue;
                            }
                        };
                        let exited = matches!(event, Event::KernelExited);
                        if let Err(e) = renderer.handle_event(event) {
                            eprintln!("\x1b[31m[jet] {e}\x1b[0m");
                        }
                        if exited {
                            break;
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
        reader_closed.notify_one();
    });

    // Ask the kernel for its banner/version info, and wait for the reply to be
    // fully rendered (kernel goes idle for this request) before drawing the
    // first prompt — otherwise rustyline races the async banner write.
    let info_id = jupyter::new_msg_id();
    let info_req = jupyter::message("shell", &info_id, "kernel_info_request", json!({}));
    channel.send(&info_req).await?;
    match wait_for_idle(&mut idle_rx, &info_id, Duration::from_secs(10)).await {
        WaitResult::Idle | WaitResult::Timeout => {}
        WaitResult::Closed => return Ok(()),
    }

    let mut rl = Some(rustyline::DefaultEditor::new()?);
    println!("jet — connected to session {session_id}. ^D to quit.");
    loop {
        // Run rustyline on a blocking thread so we can race it against
        // `closed`. If the websocket dies (kernel quit, server crash) the
        // REPL exits immediately instead of waiting for the user to press
        // a key. The blocking task can't be cancelled — we just leave it
        // pinned on stdin and drop it when the process exits.
        let mut prompt_rl = rl.take().expect("editor present at top of loop");
        let read = tokio::task::spawn_blocking(move || {
            let result = prompt_rl.readline("> ");
            (prompt_rl, result)
        });
        let line = tokio::select! {
            _ = closed.notified() => {
                eprintln!("\x1b[31m[jet] kernel exited\x1b[0m");
                shutdown.notify_one();
                // The blocking readline task is parked on stdin.read(); the
                // tokio runtime's Drop would wait for that thread to finish
                // (it never will, until the user presses a key). Run drops
                // we care about explicitly, then exit the process.
                drop(client);
                std::process::exit(0);
            }
            joined = read => {
                let (returned_rl, result) = joined?;
                rl = Some(returned_rl);
                match result {
                    Ok(l) => l,
                    Err(rustyline::error::ReadlineError::Eof)
                    | Err(rustyline::error::ReadlineError::Interrupted) => break,
                    Err(e) => {
                        eprintln!("[jet] readline: {e}");
                        break;
                    }
                }
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let _ = rl.as_mut().expect("editor returned from blocking task").add_history_entry(&line);

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
                // Kernel went away mid-request. The reader task either
                // already printed `[jet] kernel exited` (clean quit() /
                // exit()) or `[jet] ws error: …` (something worse). Exit
                // silently here either way.
                shutdown.notify_one();
                drop(client);
                std::process::exit(0);
            }
        }
    }

    shutdown.notify_one();
    // ^D is also reached only via the blocking readline thread returning,
    // so it's actually been joined. The runtime drop is fine here, but we
    // still process::exit for symmetry and to avoid waiting on the WS
    // reader's still-pending receive on a possibly-stuck socket.
    drop(client);
    std::process::exit(0);
}

//! Snapshot-style integration tests for jet.
//!
//! Each test spawns the `jet` binary under a real PTY, parses every byte
//! the binary writes through a [`vt100::Parser`] (which models a virtual
//! terminal with scrollback), and then snapshots the resulting screen and
//! optional scrollback via `insta::assert_snapshot!`.
//!
//! ## Design notes
//!
//! - The PTY reader thread feeds bytes into `vt100` AND answers cursor-
//!   position (DSR) queries with the *actual* tracked cursor row/column,
//!   not a hard-coded fake. Reedline's repaint logic branches on the
//!   cursor row — a lying DSR responder makes the test diverge from
//!   real-terminal behaviour.
//! - Tests synchronise on observable state (e.g. "the iopub Idle echo
//!   landed in the byte stream") instead of `sleep`. A small post-Idle
//!   quiescence wait drains any trailing repaint.
//! - We `JET_CELL_PX_WIDTH=9 JET_CELL_PX_HEIGHT=18` so jet doesn't issue
//!   the kitty `CSI 16t` query (vt100 ignores it but the timing varies).
//! - Per-test isolation: `XDG_DATA_HOME` and connection-file paths are
//!   unique. Children are killed and the kernel `pkill`'d on tear-down.
//!
//! Tests skip (printed as `SKIP: …`) when `python -m ipykernel` is
//! missing, like the existing `repl.rs` suite.

#![allow(clippy::zombie_processes)]

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use rand::Rng;
use serde_json::json;

// ─────────────────────────────────────────────────────────────────────
// Test gates and isolation helpers (duplicated from repl.rs because
// integration test files don't share state via cfg(test) modules).
// ─────────────────────────────────────────────────────────────────────

fn which(name: &str) -> Option<String> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn skip(reason: &str) {
    eprintln!("SKIP: {reason}");
}

fn ipykernel_available() -> bool {
    Command::new("python3")
        .args(["-c", "import ipykernel"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn ensure_python_kernelspec() -> Result<std::path::PathBuf> {
    let user = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/python3/kernel.json");
    if user.exists() {
        return Ok(user);
    }
    let python = which("python3").ok_or_else(|| anyhow::anyhow!("python3 not on PATH"))?;
    let dir = std::env::temp_dir().join(format!(
        "jet-test-kernelspec-{:x}",
        rand::thread_rng().r#gen::<u64>()
    ));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("kernel.json");
    let spec = json!({
        "argv": [python, "-m", "ipykernel_launcher", "-f", "{connection_file}"],
        "display_name": "Python (jet test)",
        "language": "python",
        "interrupt_mode": "signal",
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&spec)?)?;
    Ok(path)
}

/// Collapse the volatile parts of the IPython/Python banner so snapshots
/// survive python/ipykernel/clang version bumps. Matches anything that
/// starts with `Python ` or `IPython ` and replaces the line with a
/// stable placeholder; leaves all other lines untouched.
fn normalise_banner(line: &str) -> String {
    if line.starts_with("Python ") {
        "Python <version>".to_string()
    } else if line.starts_with("IPython ") {
        "IPython <version>".to_string()
    } else if line.starts_with("Type 'copyright', 'credits' or 'license'") {
        // ipykernel's middle banner line — collapse for stability across
        // CPython builds.
        "Type 'copyright', 'credits' or 'license' for more information".to_string()
    } else {
        line.to_string()
    }
}

fn scratch_xdg_dir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "jet-xdg-test-{:x}",
        rand::thread_rng().r#gen::<u64>()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

// ─────────────────────────────────────────────────────────────────────
// PTY harness
// ─────────────────────────────────────────────────────────────────────

/// A `Write` adapter so the PTY reader thread (which needs to answer DSR
/// queries) and the test caller (which writes user input) can share the
/// master's writer half safely.
struct SharedWriter(Arc<Mutex<Box<dyn Write + Send>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// One spawned `jet` process plumbed through a PTY with a vt100 parser
/// modelling everything jet writes. `writer` lets the test send user
/// input; `parser` and `output` are shared with the reader thread.
struct Harness {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Kept so it's not dropped (closing the master ends the reader).
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    parser: Arc<Mutex<vt100::Parser>>,
    /// Raw byte stream from jet's stdout — useful for substring sync.
    output: Arc<Mutex<String>>,
    reader: Option<std::thread::JoinHandle<()>>,
    /// Unique connection-file path for `pkill -f` teardown.
    conn_str: String,
}

impl Harness {
    fn spawn(args: &[&str], xdg: &std::path::Path, conn_str: &str) -> Result<Self> {
        Self::spawn_with_size(args, xdg, conn_str, 24, 120)
    }

    fn spawn_with_size(
        args: &[&str],
        xdg: &std::path::Path,
        conn_str: &str,
        rows: u16,
        cols: u16,
    ) -> Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows,
            cols,
            ..Default::default()
        })?;
        let bin = env!("CARGO_BIN_EXE_jet");
        let mut cmd = CommandBuilder::new(bin);
        for a in args {
            cmd.arg(a);
        }
        cmd.env("XDG_DATA_HOME", xdg);
        // Skip jet's CSI 16t cell-size query: it's not relevant for our
        // tests, vt100 doesn't answer it, and the fallback timing adds
        // noise.
        cmd.env("JET_CELL_PX_WIDTH", "9");
        cmd.env("JET_CELL_PX_HEIGHT", "18");
        // Make logs collectable but don't spam stderr during normal runs.
        cmd.env("RUST_LOG", "warn");
        cmd.cwd(std::env::current_dir()?);
        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let (writer, parser, output, reader) = spawn_reader(&*pair.master, rows, cols);
        Ok(Self {
            child,
            master: pair.master,
            writer,
            parser,
            output,
            reader: Some(reader),
            conn_str: conn_str.to_string(),
        })
    }

    /// Wait up to `timeout` for the raw output stream to contain `needle`.
    /// Returns true on success, false on timeout.
    fn wait_for(&self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.output.lock().unwrap().contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Send bytes as if the user typed them.
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    /// Wait until no new bytes have arrived for `quiet_for`, up to `cap`
    /// total. Used to let jet finish a render cycle before snapshotting.
    fn settle(&self, quiet_for: Duration, cap: Duration) {
        let deadline = Instant::now() + cap;
        let mut last_len = self.output.lock().unwrap().len();
        let mut last_change = Instant::now();
        while Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
            let len = self.output.lock().unwrap().len();
            if len != last_len {
                last_len = len;
                last_change = Instant::now();
            } else if last_change.elapsed() >= quiet_for {
                return;
            }
        }
    }

    /// Render the current visible screen as plain text (trailing spaces
    /// stripped per row, no ANSI codes). One row per line. Version-strings
    /// from the Python/IPython banner are normalised so snapshots survive
    /// kernel/python upgrades.
    fn snapshot_screen(&self) -> String {
        let parser = self.parser.lock().unwrap();
        let contents = parser.screen().contents();
        let mut out = String::new();
        for line in contents.lines() {
            let line = line.trim_end_matches(' ');
            out.push_str(&normalise_banner(line));
            out.push('\n');
        }
        while out.ends_with("\n\n") {
            out.pop();
        }
        out
    }

    /// Tear down: kill the child, close the master, join the reader, and
    /// best-effort `pkill -f <conn_str>` so the persisted kernel exits.
    fn shutdown(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // Master drop closes the slave fd; reader sees EOF.
        drop(self.master);
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        if !self.conn_str.is_empty() {
            let _ = Command::new("pkill")
                .args(["-9", "-f", &self.conn_str])
                .status();
            let _ = std::fs::remove_file(&self.conn_str);
        }
    }
}

/// Spawn a reader thread that:
/// - mirrors every byte from the PTY into both `output` (a String) and
///   `vt100::Parser` (a virtual terminal model),
/// - answers `ESC [ 6 n` (DSR) queries by replying with the *actual*
///   tracked cursor row/column from vt100 (1-indexed),
/// - returns the user-facing writer half plus the parser + output handles
///   and the join handle.
fn spawn_reader(
    master: &dyn MasterPty,
    rows: u16,
    cols: u16,
) -> (
    Box<dyn Write + Send>,
    Arc<Mutex<vt100::Parser>>,
    Arc<Mutex<String>>,
    std::thread::JoinHandle<()>,
) {
    let mut reader = master.try_clone_reader().expect("clone reader");
    let raw_writer = master.take_writer().expect("take writer");
    let shared_writer = Arc::new(Mutex::new(raw_writer));
    let writer_for_reader = shared_writer.clone();
    let writer: Box<dyn Write + Send> = Box::new(SharedWriter(shared_writer));

    let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 1000)));
    let parser_for_reader = parser.clone();
    let output = Arc::new(Mutex::new(String::new()));
    let output_for_reader = output.clone();

    let handle = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let chunk = &buf[..n];
                    // Feed bytes into vt100 BEFORE answering DSR so the
                    // cursor model is up to date.
                    parser_for_reader.lock().unwrap().process(chunk);
                    if chunk.windows(4).any(|w| w == b"\x1b[6n") {
                        let (r, c) = parser_for_reader
                            .lock()
                            .unwrap()
                            .screen()
                            .cursor_position();
                        // vt100 returns 0-indexed; DSR is 1-indexed.
                        let reply = format!("\x1b[{};{}R", r + 1, c + 1);
                        let mut w = writer_for_reader.lock().unwrap();
                        let _ = w.write_all(reply.as_bytes());
                        let _ = w.flush();
                    }
                    output_for_reader
                        .lock()
                        .unwrap()
                        .push_str(&String::from_utf8_lossy(chunk));
                }
            }
        }
    });
    (writer, parser, output, handle)
}

// ─────────────────────────────────────────────────────────────────────
// Snapshot tests
// ─────────────────────────────────────────────────────────────────────

/// Single client: spawn jet, wait for the banner-and-prompt to settle,
/// execute one print, snapshot the resulting screen.
#[test]
fn single_client_executes_print() {
    if !ipykernel_available() {
        skip("ipykernel not installed");
        return;
    }
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare kernelspec: {e}"));
            return;
        }
    };

    let xdg = scratch_xdg_dir();
    let conn = std::env::temp_dir().join(format!(
        "jet-snap-single-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    let mut h = Harness::spawn(
        &[
            "start",
            "--connection-file",
            &conn_str,
            kernel_json.to_str().unwrap(),
        ],
        &xdg,
        &conn_str,
    )
    .expect("spawn");

    assert!(
        h.wait_for("Python", Duration::from_secs(20)),
        "kernel banner never landed"
    );
    h.settle(Duration::from_millis(300), Duration::from_secs(2));

    h.send(b"print(\"hello, jet\")\n").unwrap();
    assert!(
        h.wait_for("hello, jet", Duration::from_secs(10)),
        "print output never landed"
    );
    h.settle(Duration::from_millis(300), Duration::from_secs(2));

    insta::assert_snapshot!("single_client_executes_print", h.snapshot_screen());

    h.shutdown();
}

/// Two clients sharing a kernel. `start` is the observer; `attach` runs
/// `print("HELLO_FROM_FOREIGN")`. Snapshot the observer's screen to
/// confirm the foreign output appears tagged `[jet]` and that the local
/// prompt is preserved.
#[test]
fn two_clients_foreign_print_is_tagged_and_visible() {
    if !ipykernel_available() {
        skip("ipykernel not installed");
        return;
    }
    let kernel_json = match ensure_python_kernelspec() {
        Ok(p) => p,
        Err(e) => {
            skip(&format!("could not prepare kernelspec: {e}"));
            return;
        }
    };

    let xdg = scratch_xdg_dir();
    let conn = std::env::temp_dir().join(format!(
        "jet-snap-foreign-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ));
    let conn_str = conn.to_string_lossy().to_string();

    let t1 = Harness::spawn(
        &[
            "start",
            "--persist",
            "--connection-file",
            &conn_str,
            kernel_json.to_str().unwrap(),
        ],
        &xdg,
        &conn_str,
    )
    .expect("spawn t1");
    assert!(
        t1.wait_for("Python", Duration::from_secs(20)),
        "t1 banner never landed"
    );
    t1.settle(Duration::from_millis(300), Duration::from_secs(2));

    let mut t2 = Harness::spawn(
        &["attach", "--connection-file", &conn_str],
        &xdg,
        // t2 doesn't own the persisted kernel; leave conn_str empty so
        // its teardown doesn't pkill (t1's teardown handles it).
        "",
    )
    .expect("spawn t2");
    // t2's prompt is ready when it has any local "> " row drawn — but
    // since reedline's prompt uses ANSI we wait on `"\x1b[38;5;10m> "`
    // which is the bright-green prompt indicator. Easier: just wait a
    // beat after spawn.
    t2.settle(Duration::from_millis(500), Duration::from_secs(3));

    t2.send(b"print(\"HELLO_FROM_FOREIGN\")\n").unwrap();
    assert!(
        t1.wait_for("HELLO_FROM_FOREIGN", Duration::from_secs(10)),
        "t1 never received foreign output"
    );
    t1.settle(Duration::from_millis(500), Duration::from_secs(3));

    insta::assert_snapshot!(
        "two_clients_foreign_print_is_tagged_and_visible__observer_screen",
        t1.snapshot_screen()
    );

    t2.shutdown();
    t1.shutdown();
}

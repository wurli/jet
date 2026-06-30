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
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use rand::Rng;

mod common;
use common::*;

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

/// Macro replacement for `if !ipykernel_available() { skip(...); return }` +
/// kernelspec prep. Returns the kernelspec path or `None` (logging SKIP)
/// when the test should be silently skipped.
fn python_kernelspec_or_skip() -> Option<std::path::PathBuf> {
    if !ipykernel_available() {
        skip("ipykernel not installed");
        return None;
    }
    match ensure_python_kernelspec() {
        Ok(p) => Some(p),
        Err(e) => {
            skip(&format!("could not prepare kernelspec: {e}"));
            None
        }
    }
}

/// Generate a unique connection-file path under tmpdir.
fn temp_conn_file(slug: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "jet-snap-{slug}-{:x}.json",
        rand::thread_rng().r#gen::<u64>()
    ))
}

/// Spawn a `jet start` harness against a python kernel, wait for the
/// banner, and settle. Returns the harness ready for input.
fn start_jet(
    kernel_json: &std::path::Path,
    xdg: &std::path::Path,
    conn_str: &str,
    extra_args: &[&str],
) -> Result<Harness> {
    let kernel_arg = kernel_json.to_str().unwrap().to_string();
    let mut args: Vec<&str> = vec!["start", "--connection-file", conn_str];
    args.extend_from_slice(extra_args);
    args.push(&kernel_arg);
    let h = Harness::spawn(&args, xdg, conn_str)?;
    assert!(
        h.wait_for("Python", Duration::from_secs(20)),
        "kernel banner never landed"
    );
    h.settle(Duration::from_millis(300), Duration::from_secs(2));
    Ok(h)
}

/// Spawn a `--persist` `jet start` plus a `jet attach` against the same
/// kernel. Each side gets its own `--session-name` if provided so foreign
/// output is tagged distinctly. Returns `(start, attach)`. Tear down by
/// calling `attach.shutdown()` then `start.shutdown()`.
fn start_pair(
    kernel_json: &std::path::Path,
    xdg: &std::path::Path,
    conn_str: &str,
    start_name: Option<&str>,
    attach_name: Option<&str>,
) -> Result<(Harness, Harness)> {
    let mut start_args: Vec<&str> = vec!["--persist"];
    if let Some(n) = start_name {
        start_args.extend_from_slice(&["--session-name", n]);
    }
    let t1 = start_jet(kernel_json, xdg, conn_str, &start_args)?;

    let mut attach_args: Vec<&str> = vec!["attach", "--connection-file", conn_str];
    if let Some(n) = attach_name {
        attach_args.extend_from_slice(&["--session-name", n]);
    }
    // t2 doesn't own the persisted kernel; pass empty conn_str so its
    // teardown doesn't pkill (t1's teardown handles it).
    let t2 = Harness::spawn(&attach_args, xdg, "")?;
    t2.settle(Duration::from_millis(500), Duration::from_secs(3));
    Ok((t1, t2))
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
    /// Returns true on success, false on timeout. Use this when the needle
    /// is plain ASCII unlikely to be split across escape sequences (banner
    /// substrings, markers within `print()` output, etc.). For "the
    /// rendered screen shows X" use [`Harness::wait_for_screen`].
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

    /// Wait until the rendered visible screen (via vt100) contains
    /// `needle`. More robust than `wait_for` when the needle would be
    /// interrupted by ANSI escapes in the raw stream (e.g. matching
    /// across a line break or after a bracketed-paste sequence).
    fn wait_for_screen(&self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self
                .parser
                .lock()
                .unwrap()
                .screen()
                .contents()
                .contains(needle)
            {
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
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("single");
    let conn_str = conn.to_string_lossy().to_string();

    let mut h = start_jet(&kernel_json, &xdg, &conn_str, &[]).expect("spawn");
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
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("foreign");
    let conn_str = conn.to_string_lossy().to_string();
    let (t1, mut t2) =
        start_pair(&kernel_json, &xdg, &conn_str, None, None).expect("spawn pair");

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

/// The kernel banner is rendered before the first `> ` prompt, not
/// sandwiched between two prompts. Replaces the old assertion-based
/// `spawn_emits_kernel_banner` test.
#[test]
fn banner_renders_before_first_prompt() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("banner");
    let conn_str = conn.to_string_lossy().to_string();

    let h = start_jet(&kernel_json, &xdg, &conn_str, &[]).expect("spawn");
    // Banner must precede the first prompt in the raw byte stream.
    let captured = h.output.lock().unwrap().clone();
    let banner_idx = captured.find("Python ").expect("banner present");
    let prompt_idx = captured.find("> ").expect("prompt present");
    assert!(
        banner_idx < prompt_idx,
        "banner must precede the first '> ' prompt; got:\n{captured}",
    );

    insta::assert_snapshot!("banner_renders_before_first_prompt", h.snapshot_screen());
    h.shutdown();
}

/// A complete one-liner executes without showing a continuation prompt.
/// Snapshot captures the value (`2`) appearing directly under `> 1+1`,
/// with no `+ ` in between.
#[test]
fn complete_one_liner_executes() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("oneliner");
    let conn_str = conn.to_string_lossy().to_string();

    let mut h = start_jet(&kernel_json, &xdg, &conn_str, &[]).expect("spawn");
    // Use print so the result is a stdout stream — not an ipykernel
    // `Out[1]:` echo. snapshot captures the value directly under the
    // typed line with no `+ ` continuation in between.
    h.send(b"print(1+1)\n").unwrap();
    assert!(
        h.wait_for_screen("\n2\n", Duration::from_secs(15)),
        "did not see '2' result"
    );
    h.settle(Duration::from_millis(300), Duration::from_secs(2));
    insta::assert_snapshot!("complete_one_liner_executes", h.snapshot_screen());
    h.shutdown();
}

/// Multi-line function definition: `def f():` is incomplete so jet
/// should show a `+ ` continuation prompt with the kernel-suggested
/// indent; an empty line closes the block; `f()` then runs and returns
/// `42`. Snapshot captures the whole sequence on screen.
#[test]
fn multi_line_function_definition_executes() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("multiline");
    let conn_str = conn.to_string_lossy().to_string();

    let mut h = start_jet(&kernel_json, &xdg, &conn_str, &[]).expect("spawn");
    h.send(b"def f():\n").unwrap();
    h.settle(Duration::from_millis(400), Duration::from_secs(2));
    h.send(b"    return 42\n").unwrap();
    h.settle(Duration::from_millis(200), Duration::from_secs(2));
    // Empty line closes the def-block, then call f().
    h.send(b"\n").unwrap();
    h.settle(Duration::from_millis(200), Duration::from_secs(2));
    h.send(b"print(f())\n").unwrap();
    assert!(
        h.wait_for_screen("42", Duration::from_secs(15)),
        "did not see '42' from f()"
    );
    h.settle(Duration::from_millis(400), Duration::from_secs(2));
    insta::assert_snapshot!(
        "multi_line_function_definition_executes",
        h.snapshot_screen()
    );
    h.shutdown();
}

/// Backspace at an empty continuation line merges the prior accumulator
/// line back into the editor. We submit `x = (`, which ipykernel reports
/// Incomplete (waiting for `)`). With merge-back working we can erase
/// the broken line via Backspace and submit a clean `print(...)` that
/// the kernel actually executes. Without merge-back the unclosed paren
/// keeps the buffer Incomplete forever and the marker never appears.
#[test]
fn backspace_merges_continuation_back_into_editor() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("backspace");
    let conn_str = conn.to_string_lossy().to_string();

    let mut h = start_jet(&kernel_json, &xdg, &conn_str, &[]).expect("spawn");
    h.send(b"x = (\n").unwrap();
    h.settle(Duration::from_millis(400), Duration::from_secs(2));
    // 1 merge-back + 5 chars to clear + 2 spare = 8 DELs. Pacing the
    // bytes (15ms) gives reedline time to route each one separately.
    for _ in 0..8 {
        h.send(b"\x7f").unwrap();
        std::thread::sleep(Duration::from_millis(15));
    }
    h.settle(Duration::from_millis(200), Duration::from_secs(1));
    // Assemble the marker at runtime so the typed echo doesn't itself
    // contain it — only kernel-executed `print` output will.
    let marker = "merged-backspace-9f31";
    h.send(b"print(chr(109) + \"erged-backspace-9f31\")\n").unwrap();
    assert!(
        h.wait_for(marker, Duration::from_secs(10)),
        "marker {marker:?} never appeared — merge-back regressed"
    );
    h.settle(Duration::from_millis(300), Duration::from_secs(2));
    insta::assert_snapshot!(
        "backspace_merges_continuation_back_into_editor",
        h.snapshot_screen()
    );
    h.shutdown();
}

/// `input(prompt)` from the kernel surfaces the prompt and accepts the
/// user's reply. Replaces the assertion-based input_request test.
#[test]
fn input_request_displays_and_accepts_value() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("input");
    let conn_str = conn.to_string_lossy().to_string();

    let mut h = start_jet(&kernel_json, &xdg, &conn_str, &[]).expect("spawn");
    // Raw mode: \r is the Enter keycode (\n becomes a literal char).
    h.send(b"v = input('ASK> '); print('GOT:' + v)\r").unwrap();
    assert!(
        h.wait_for("ASK> ", Duration::from_secs(15)),
        "input prompt 'ASK> ' never appeared"
    );
    h.settle(Duration::from_millis(300), Duration::from_secs(2));
    h.send(b"hello-input\r").unwrap();
    assert!(
        h.wait_for("GOT:hello-input", Duration::from_secs(10)),
        "kernel did not echo input value back"
    );
    h.settle(Duration::from_millis(300), Duration::from_secs(2));
    insta::assert_snapshot!(
        "input_request_displays_and_accepts_value",
        h.snapshot_screen()
    );
    h.shutdown();
}

/// When `jet attach --session-name alpha` runs code, the observer's
/// foreign-output prefix should be `[alpha]` rather than the default
/// `[jet]`. Captures the prefix end-to-end from CLI flag to terminal
/// rendering.
#[test]
fn foreign_attach_session_name_appears_as_prefix() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("name-attach");
    let conn_str = conn.to_string_lossy().to_string();
    let (t1, mut t2) = start_pair(&kernel_json, &xdg, &conn_str, None, Some("alpha"))
        .expect("spawn pair");

    t2.send(b"print(\"x\")\n").unwrap();
    assert!(
        t1.wait_for_screen("[alpha]", Duration::from_secs(15)),
        "t1 never saw `[alpha]` prefix"
    );
    t1.settle(Duration::from_millis(700), Duration::from_secs(5));

    insta::assert_snapshot!(
        "foreign_attach_session_name_appears_as_prefix",
        t1.snapshot_screen()
    );
    t2.shutdown();
    t1.shutdown();
}

/// And the reverse direction: when `jet start --session-name beta` runs
/// code, the attached observer's prefix should be `[beta]`.
#[test]
fn foreign_start_session_name_appears_as_prefix() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("name-start");
    let conn_str = conn.to_string_lossy().to_string();
    let (mut t1, t2) = start_pair(&kernel_json, &xdg, &conn_str, Some("beta"), None)
        .expect("spawn pair");
    // t2 is the observer here; give it longer to land its own prompt and
    // settle before t1 starts firing iopub at it. (The default
    // start_pair settle is 500ms; double it for the slower path.)
    t2.settle(Duration::from_millis(500), Duration::from_secs(3));

    t1.send(b"print(\"y\")\n").unwrap();
    assert!(
        t2.wait_for_screen("[beta]", Duration::from_secs(15)),
        "t2 never saw `[beta]` prefix"
    );
    t2.settle(Duration::from_millis(700), Duration::from_secs(5));

    insta::assert_snapshot!(
        "foreign_start_session_name_appears_as_prefix",
        t2.snapshot_screen()
    );
    t2.shutdown();
    t1.shutdown();
}

/// Foreign multi-line cell: t2 submits a `def f():` block (continuation
/// prompt territory). The kernel echoes `ExecuteInput` once with the
/// embedded `\n`, so the observer should see the first line tagged with
/// `> ` and each continuation line tagged with `+ `.
#[test]
fn foreign_multi_line_cell_renders_with_continuation_prefix() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("foreign-multi");
    let conn_str = conn.to_string_lossy().to_string();
    let (t1, mut t2) =
        start_pair(&kernel_json, &xdg, &conn_str, None, None).expect("spawn pair");

    // Drive t2 through a continuation prompt. Each \n submits a line;
    // ipykernel's is_complete returns Incomplete until the def-block is
    // closed by a blank line.
    t2.send(b"def f():\n").unwrap();
    t2.settle(Duration::from_millis(400), Duration::from_secs(2));
    t2.send(b"    return 99\n").unwrap();
    t2.settle(Duration::from_millis(200), Duration::from_secs(2));
    t2.send(b"\n").unwrap();
    t2.settle(Duration::from_millis(400), Duration::from_secs(2));
    t2.send(b"print(f())\n").unwrap();
    assert!(
        t1.wait_for_screen("99", Duration::from_secs(15)),
        "t1 never saw foreign `print(f())` output"
    );
    t1.settle(Duration::from_millis(700), Duration::from_secs(3));

    insta::assert_snapshot!(
        "foreign_multi_line_cell_renders_with_continuation_prefix",
        t1.snapshot_screen()
    );
    t2.shutdown();
    t1.shutdown();
}

/// Foreign error: t2 raises `ValueError`. The observer should see the
/// `ExecuteInput` tagged with `[jet] > ` and each line of the traceback
/// tagged with `[jet] ` too — no untagged kernel output should leak past
/// the prefix logic.
#[test]
fn foreign_traceback_lines_are_tagged() {
    let Some(kernel_json) = python_kernelspec_or_skip() else {
        return;
    };
    let xdg = scratch_xdg_dir();
    let conn = temp_conn_file("foreign-error");
    let conn_str = conn.to_string_lossy().to_string();
    let (t1, mut t2) =
        start_pair(&kernel_json, &xdg, &conn_str, None, None).expect("spawn pair");

    t2.send(b"raise ValueError(\"oh no\")\n").unwrap();
    // Wait for the error to land in the rendered screen — the
    // traceback's final line will contain `ValueError`.
    assert!(
        t1.wait_for_screen("ValueError", Duration::from_secs(10)),
        "t1 never saw foreign ValueError traceback"
    );
    t1.settle(Duration::from_millis(700), Duration::from_secs(3));

    insta::assert_snapshot!(
        "foreign_traceback_lines_are_tagged",
        t1.snapshot_screen()
    );
    t2.shutdown();
    t1.shutdown();
}

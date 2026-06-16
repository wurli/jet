//! Drives `crates/lua/tests/scripts/*.lua` through real luajit. Skipped
//! when luajit, kcserver, or ipykernel are missing.
//!
//! The harness ensures the cdylib is built, copies it to `jet.so` under
//! the workspace target dir so `require('jet')` resolves, and shells out
//! to luajit with the right `package.cpath` and env vars.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn skip(reason: &str) {
    eprintln!("SKIP: {reason}");
}

fn which(name: &str) -> Option<String> {
    let out = Command::new("which").arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn locate_kcserver() -> Option<String> {
    if let Ok(p) = std::env::var("JET_KCSERVER") {
        if Path::new(&p).exists() {
            return Some(p);
        }
    }
    for p in ["/tmp/kc/kcserver"] {
        if Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    which("kcserver")
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

/// Locate or generate a Python kernelspec. We can't rely on a system one
/// in CI, so fall back to a synthesized spec in the OS tempdir.
fn ensure_python_kernelspec() -> Option<PathBuf> {
    let user = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/python3/kernel.json");
    if user.exists() {
        return Some(user);
    }
    let python = which("python3")?;
    let dir = std::env::temp_dir().join("jet-lua-test-kernelspec");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("kernel.json");
    let spec = serde_json::json!({
        "argv": [python, "-m", "ipykernel_launcher", "-f", "{connection_file}"],
        "display_name": "Python (jet-lua test)",
        "language": "python",
        "interrupt_mode": "signal",
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&spec).ok()?).ok()?;
    Some(path)
}

/// Build the `jet_lua` cdylib and return its on-disk path. We invoke
/// `cargo build` from the test so the binary is always fresh — relying on
/// implicit ordering between test crates and lib crates is fragile.
fn build_cdylib() -> PathBuf {
    let status = Command::new(env!("CARGO"))
        .args(["build", "-p", "jet_lua"])
        .status()
        .expect("cargo build");
    assert!(status.success(), "cargo build -p jet_lua failed");

    // CARGO_MANIFEST_DIR is .../crates/lua. Walk up to workspace root.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest.parent().unwrap().parent().unwrap();

    let candidates = [
        workspace.join("target/debug/libjet_lua.dylib"),
        workspace.join("target/debug/libjet_lua.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("cdylib not found; checked {candidates:?}");
}

/// Stage the cdylib as `jet.so` in a per-test temp dir so `require('jet')`
/// finds it. luajit's package loader matches against `?.so`, so the file
/// extension must be `.so` even on macOS — luajit's loader doesn't care
/// what kind of dylib it actually is.
fn stage_module(dylib: &Path) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jet-lua-stage-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir stage");
    let staged = dir.join("jet.so");
    std::fs::copy(dylib, &staged).expect("copy cdylib");
    dir
}

fn run_lua_test(script_name: &str) {
    let Some(luajit) = which("luajit") else {
        skip("luajit not on PATH");
        return;
    };
    let Some(kc) = locate_kcserver() else {
        skip("kcserver not found (set JET_KCSERVER=/path/to/kcserver)");
        return;
    };
    if !ipykernel_available() {
        skip("ipykernel not installed (`pip install ipykernel`)");
        return;
    }
    let Some(kernelspec) = ensure_python_kernelspec() else {
        skip("could not prepare a python kernelspec");
        return;
    };

    let dylib = build_cdylib();
    let stage = stage_module(&dylib);
    let cpath = format!("{}/?.so", stage.display());

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = manifest.join("tests/scripts").join(script_name);
    assert!(script.exists(), "missing test script {script:?}");

    let status = Command::new(&luajit)
        .arg(&script)
        .env("LUA_CPATH", &cpath)
        .env("JET_KCSERVER", &kc)
        .env("JET_TEST_KERNEL", &kernelspec)
        .status()
        .expect("spawn luajit");
    assert!(status.success(), "{} failed", script_name);
}

#[test]
fn execute_smoke() {
    run_lua_test("execute_smoke.lua");
}

#[test]
fn input_request_smoke() {
    run_lua_test("input_request_smoke.lua");
}

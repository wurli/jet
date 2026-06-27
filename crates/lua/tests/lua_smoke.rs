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

/// Path to the prebuilt `jet_lua` cdylib. `cargo test` only links the rlib
/// into the test binary and doesn't produce the cdylib that luajit loads,
/// so callers must run `cargo build -p jet_lua` first. Tests skip when
/// the artifact is missing.
fn find_cdylib() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug")
        .join(format!("libjet_lua{}", std::env::consts::DLL_SUFFIX));
    path.exists().then_some(path)
}

/// True if any `.rs` file under crates/lua/src is newer than the dylib.
/// Catches the footgun of forgetting to rebuild before re-running tests.
fn dylib_is_stale(dylib: &Path) -> bool {
    let Ok(dylib_mtime) = dylib.metadata().and_then(|m| m.modified()) else {
        return false;
    };
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    walkdir(&src).any(|p| {
        p.metadata()
            .and_then(|m| m.modified())
            .map(|t| t > dylib_mtime)
            .unwrap_or(false)
    })
}

fn walkdir(root: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
    let Ok(rd) = std::fs::read_dir(root) else {
        return Box::new(std::iter::empty());
    };
    Box::new(rd.flatten().flat_map(|e| {
        let p = e.path();
        if p.is_dir() {
            walkdir(&p)
        } else {
            Box::new(std::iter::once(p))
        }
    }))
}

/// Locate the user's ark kernelspec, if installed. Unlike ipykernel we
/// don't synthesize one — ark is a real binary, not a `python -m` invocation.
fn find_ark_kernelspec() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Library/Jupyter/kernels/ark/kernel.json");
    p.exists().then_some(p)
}

enum TestKernel {
    Python,
    Ark,
}

fn run_lua_test(script_name: &str) {
    run_lua_test_with(script_name, TestKernel::Python);
}

fn run_lua_test_with(script_name: &str, which_kernel: TestKernel) {
    let Some(luajit) = which("luajit") else {
        skip("luajit not on PATH");
        return;
    };
    let kernelspec = match which_kernel {
        TestKernel::Python => {
            if !ipykernel_available() {
                skip("ipykernel not installed (`pip install ipykernel`)");
                return;
            }
            let Some(p) = ensure_python_kernelspec() else {
                skip("could not prepare a python kernelspec");
                return;
            };
            p
        }
        TestKernel::Ark => {
            let Some(p) = find_ark_kernelspec() else {
                skip("ark kernelspec not installed at ~/Library/Jupyter/kernels/ark");
                return;
            };
            p
        }
    };

    let Some(dylib) = find_cdylib() else {
        skip("cdylib not built; run `cargo build -p jet_lua` first");
        return;
    };
    if dylib_is_stale(&dylib) {
        skip("cdylib older than src/; run `cargo build -p jet_lua` first");
        return;
    }
    // LUA_CPATH patterns are tried verbatim — no `?` substitution needed.
    // luajit happily loads `libjet_lua.dylib` as the `jet` module.
    let cpath = dylib.display().to_string();

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = manifest.join("tests/scripts").join(script_name);
    assert!(script.exists(), "missing test script {script:?}");

    let status = Command::new(&luajit)
        .arg(&script)
        .env("LUA_CPATH", &cpath)
        .env("JET_TEST_KERNEL", &kernelspec)
        .status()
        .expect("spawn luajit");
    assert!(status.success(), "{} failed", script_name);
}

#[test]
fn execute_smoke() {
    run_lua_test("execute.lua");
}

#[test]
fn input_request_smoke() {
    run_lua_test("input.lua");
}

#[test]
fn comm_lsp_smoke() {
    run_lua_test_with("comm_lsp.lua", TestKernel::Ark);
}

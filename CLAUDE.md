# jet

A small CLI REPL for Jupyter kernels. Spawns the kernel directly over ZMQ
using [`runtimed`](https://github.com/runtimed/runtimed)'s
`jupyter-zmq-client` and renders kernel output — including PNG plots —
inline in the terminal using the kitty graphics protocol.

## What it does

- Spawns or attaches to a Jupyter kernel.
- Kernel is identified by a path to a Jupyter `kernel.json` kernelspec
  (e.g. `~/Library/Jupyter/kernels/ark/kernel.json`). `argv`, `language`,
  `env`, and `interrupt_mode` come from the spec. We substitute
  `{connection_file}` in `argv` ourselves before launching the kernel.
- For each new connection, picks 5 free TCP ports (bind-and-drop), generates
  a 16-byte hex HMAC key, writes the connection file, then opens four ZMQ
  client sockets (shell DEALER, iopub SUB, stdin DEALER, control DEALER).
- Reads input with reedline; builds `ExecuteRequest` and sends it on shell.
- Streams iopub messages to stdout as they arrive (text, errors, banners,
  plots).
- Renders `image/png` outputs via kitty graphics in unicode-placeholder
  mode, so images survive tmux pane switches and scrolling.
- Supports `--persist` (keep kernel running after jet exits) and
  `jet attach <connection-file>` (reconnect to a previously-detached
  kernel from a fresh process).

## Architecture

This is a Cargo workspace.

```
crates/
├── core/                       library: connection layer, no terminal deps
│   └── src/
│       ├── lib.rs              module declarations + re-exports
│       ├── connection_file.rs  port pick + HMAC key + write/read JSON
│       ├── kernel.rs           Kernel: spawn/attach/send/recv/interrupt/shutdown
│       └── events.rs           JupyterMessage → typed Event for the renderer
├── cli/                        binary: `jet`
│   └── src/
│       ├── main.rs             arg parse + REPL loop
│       ├── cli.rs              clap Args (Connect, Attach)
│       └── render/             kernel output → terminal
│           ├── mod.rs          Renderer (consumes Event)
│           ├── tmux.rs         passthrough warning + DCS wrapping
│           └── kitty/          kitty graphics protocol
└── lua/                        cdylib: Neovim/LuaJIT binding
    └── src/
        ├── lib.rs              mlua module registration
        ├── runtime.rs          process-global tokio runtime + KernelHandle
        ├── router.rs           per-msg_id frame demux
        ├── poll.rs             Lua-callable poll closure
        └── api/                lifecycle.rs / request.rs / stdin.rs
```

`Kernel` owns the spawned child via a `ChildGuard` — drop the kernel and
the child dies, unless `kernel.detach()` was called first. Attached
kernels never own a child; their `Kernel` is purely a client handle and
dropping it just closes the sockets.

## Why runtimed?

Previously jet shelled out to a [`kallichore`](https://github.com/posit-dev/kallichore)
supervisor process and talked to it over HTTP+WebSocket. That layer added
a multi-session abstraction we never used. Switching to runtimed's
`jupyter-zmq-client` + `jupyter-protocol` lets jet talk ZMQ directly to
the kernel — fewer moving parts, no external `kcserver` to ship, and the
JEP 66 handshake / advanced features are now patches we can land in our
own fork.

## Running

```bash
# Python (ipykernel)
cargo run -- start ~/Library/Jupyter/kernels/python3/kernel.json

# R (ark)
cargo run -- start ~/Library/Jupyter/kernels/ark/kernel.json

# Persist + reattach. --connection-file is optional; without it,
# the connection file is written to the session dir under $XDG_DATA_HOME/jet.
cargo run -- start --persist ~/Library/Jupyter/kernels/python3/kernel.json
# (jet prints the connection file path on exit; pass that to attach)
cargo run -- attach <printed-path>
```

## Tests

```bash
cargo test --workspace          # unit + integration
```

Integration tests under `crates/cli/tests/repl.rs` and
`crates/lua/tests/lua_smoke.rs` skip with `SKIP: …` when the relevant
kernel (`python -m ipykernel`, ark) isn't installed — they pass rather
than fail.

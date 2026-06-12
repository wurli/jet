# jet

A small CLI REPL for Jupyter kernels. Wraps a [`kallichore`](https://github.com/posit-dev/kallichore)
supervisor process and renders kernel output — including PNG plots — inline in
the terminal using the kitty graphics protocol.

## What it does

- Spawns (or connects to) `kcserver` and creates a Jupyter session.
- Defaults to ipython; pass any kernel argv after `--` (e.g. ark for R).
- Reads input with rustyline; sends `execute_request` over the per-session
  websocket.
- Streams `iopub` output to stdout as it arrives (text, errors, banners, plots).
- Renders `image/png` outputs via kitty graphics in unicode-placeholder mode,
  so images survive tmux pane switches and scrolling.

## Architecture

```
src/
├── main.rs                 binary: arg parse + REPL loop
├── lib.rs                  module declarations
├── cli.rs                  clap Args
├── jupyter.rs              wire-format helpers + ISO timestamp
├── kernel.rs               build kernel argv (default ipython)
├── kallichore/             HTTP + WebSocket client for kcserver
│   ├── mod.rs              Client (owns http, base, bearer, spawned server)
│   ├── server.rs           ConnectionFile, ChildGuard, spawn, status poll
│   └── session.rs          create / start / open_channels / ws_url
└── render/                 kernel output → terminal
    ├── mod.rs              Renderer dispatching on (channel, msg_type)
    ├── tmux.rs             passthrough warning + DCS wrapping
    └── kitty/              kitty graphics protocol
        ├── mod.rs          emit_png + transmission/grid builders
        ├── cell_size.rs    CSI 16t query for cell pixel size, cached
        └── diacritics.rs   297-codepoint table for placeholder cells

tests/
└── kcserver.rs             end-to-end against real kcserver + ipython
```

`Client` owns the spawned `kcserver` process via a `ChildGuard` — drop the
client and the server dies with it.

## Running

```bash
# Python (default kernel)
cargo run -- --kcserver /path/to/kcserver

# R (ark)
cargo run -- --kcserver /path/to/kcserver --language r -- \
  /path/to/ark --connection_file '{connection_file}' --session-mode console
```

`{connection_file}` is the placeholder kallichore substitutes with the kernel's
connection-file path.

## Tests

```bash
cargo test                                          # everything
JET_KCSERVER=/path/to/kcserver cargo test           # with kcserver integration
```

Integration tests under `tests/kcserver.rs` skip with `SKIP: …` when prerequisites
(`kcserver`, `ipykernel`) are missing — they pass rather than fail.

## Notable design choices

- **kallichore frame format**: flat `{channel, header, parent_header, metadata,
  content, buffers}` — *not* `{channel, msg: {...}}`. This was the first
  source of confusion when bringing the project up.
- **kitty graphics in tmux**: requires `set -g allow-passthrough on`. We wrap
  transmissions in a tmux DCS envelope (`\x1bPtmux;…\x1b\\` with interior ESCs
  doubled) and warn at startup if passthrough is off. Placeholder cells (the
  visible part of the image) go through normally as wide unicode chars.
- **Base64 padding**: ark's R kernel emits unpadded base64 PNGs. kitty's
  decoder rejects unpadded base64 silently, so we pad to a multiple of 4
  before transmission.
- **Cell pixel size**: queried via `CSI 16t` and cached in a `OnceLock`.
  Falls back to 9×18 if the query fails. Override with `JET_CELL_PX_WIDTH` /
  `JET_CELL_PX_HEIGHT`.
- **Banner ordering**: we send `kernel_info_request` and wait for its idle
  reply before drawing the first prompt, so rustyline doesn't race the
  async banner write.

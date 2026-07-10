# Building jet

Standard workspace build:

```bash
cargo build --workspace
cargo test  --workspace
```

Tests need Jupyter kernels under `test-kernels/`. Populate them with:

```bash
./scripts/install-dev-kernels.sh
```

Or:
```
make install-dev-kernels
```

## Linux dev container on a Mac

An Ubuntu dev container you can `exec` into to run `cargo build`,
`cargo test`, `nvim`, or anything else against a Linux userland — useful
for reproducing CI-only failures and sanity-checking Linux-specific code
paths (kernel process groups, tty handling, kitty passthrough) without
leaving your Mac.

The container is meant to stay running in the background; you open shells
into it as needed. The Containerfile bakes in rust, R (4.5.0), and a few
dev tools (git, neovim, tmux, ripgrep, fd). Everything jet-specific —
kernels, ipykernel venv, cargo build artifacts — lives in the mounted
repo and named volumes, so nothing is lost when you rebuild the image.

Requirements:

- macOS 15+ with Apple's `container` installed (`brew install --cask container`).
- **VPN disconnected** during builds. Zurich's corporate VPN kills the
  container VM's outbound NAT — `apt-get update` will hang on
  `ports.ubuntu.com`. You can reconnect once the image is built; the
  test run itself only needs the bind-mounted repo.

Build the image (once, ~5 minutes for R + rust toolchain):

```bash
container build -f Containerfile -t jet-test --platform linux/arm64 .
```

Create named volumes so cargo's `target/` and registry cache survive
container restarts (once per host):

```bash
container volume create jet-cargo-target
container volume create jet-cargo-home
```

Start a long-running dev container with the repo bind-mounted. `-m 8g`
matters — linking `jet_lua` needs more than the 1 GB default and `ld`
gets OOM-killed otherwise:

```bash
container run -d --name jet-dev \
  -m 8g -c 4 \
  -v "$PWD:/jet" \
  -v jet-cargo-target:/target \
  -v jet-cargo-home:/root/.cargo \
  -e CARGO_TARGET_DIR=/target \
  jet-test
```

Open a shell into it — reuse this as often as you want:

```bash
container exec -it jet-dev bash
```

Inside, you have rust, R 4.5.0, git, neovim, tmux, ripgrep, fd. First
time you want to run tests, install the Jupyter kernels once (they land
in the mounted repo, so they persist across container rebuilds):

```bash
scripts/install-dev-kernels.sh
cargo test --workspace
```

When you're done for the day:

```bash
container stop jet-dev
```

To resume next session: `container start jet-dev` then `exec` back in.
To rebuild the image after Containerfile changes: `container stop jet-dev
&& container rm jet-dev`, rebuild, then `run` again — the named volumes
keep your cargo cache.

Notes:

- `-d` gives you a detached container; `-it` on `run` is not supported
  in that mode. Attach interactively via `exec -it`.
- `CARGO_TARGET_DIR=/target` keeps Linux artifacts off the host bind-
  mount so they don't collide with your Mac's `target/`.
- The image pins R 4.5.0 via `rig` to match
  `scripts/install-dev-kernels.sh`'s snapshot pin — CRAN's apt repo
  only ships the current point release, so we can't get 4.5.0 from
  there directly.
- One-shot runs (e.g. CI-style) work too:
  `container run --rm -v "$PWD:/jet" ... jet-test bash -lc 'scripts/install-dev-kernels.sh && cargo test --workspace'`.

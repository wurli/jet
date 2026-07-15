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

An Ubuntu dev container for running `cargo build` / `cargo test` against
a Linux userland — useful for reproducing CI-only failures.

Requirements:

- macOS 15+ with Apple's `container` installed (`brew install --cask container`).

Build the image (once):

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

Open a shell into it:

```bash
container exec -it jet-dev bash
```

First time you want to run tests, install the Jupyter kernels once:

```bash
scripts/install-dev-kernels.sh
cargo test --workspace
```

To resume next session: `container start jet-dev` then `exec` back in.
To rebuild the image after Containerfile changes: `container stop jet-dev
&& container rm jet-dev`, rebuild, then `run` again — the named volumes
keep your cargo cache.

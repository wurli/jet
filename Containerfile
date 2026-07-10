FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Base tooling + interactive dev conveniences (nvim, git, less, procps for
# ps/top, etc.). This image is a dev container — meant to be started once
# and `exec`ed into, not a one-shot test runner.
#
# tini is the PID-1 init. Apple's `container` doesn't ship a reaping init
# in its default runtime, and jet's session-lifecycle tests assert that
# an orphaned kernel is fully dead (not a zombie) after its parent exits.
# Without a reaper, `libc::kill(pid, 0)` keeps returning 0 for dead-but-
# unreaped kernels and those tests fail.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl unzip build-essential pkg-config \
        libssl-dev tini \
        git less procps neovim tmux ripgrep fd-find \
    && rm -rf /var/lib/apt/lists/*

# R via rig. CRAN's noble-cran40 apt repo only ships the current point
# release; install-dev-kernels.sh pins R 4.5.0 for snapshot stability, so
# we use rig to install that specific version instead.
RUN curl -fsSL https://rig.r-pkg.org/deb/rig.gpg -o /etc/apt/trusted.gpg.d/rig.gpg \
    && echo "deb http://rig.r-pkg.org/deb rig main" > /etc/apt/sources.list.d/rig.list \
    && apt-get update && apt-get install -y --no-install-recommends r-rig \
    && rig add 4.5.0 \
    && rm -rf /var/lib/apt/lists/*

# Ark dlopens libR.so and R's bundled packages (utils.so, etc.), which are
# themselves linked against libR.so. The `R` shell wrapper sources
# `$R_HOME/etc/ldpaths` to set LD_LIBRARY_PATH, but jet launches ark
# directly and bypasses that — the loader can't find libR.so and ark
# panics on startup. Bake it into the image so every shell inherits it.
ENV LD_LIBRARY_PATH=/opt/R/4.5.0/lib/R/lib

# Rust (stable). Cargo/rustc land under /root/.cargo.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

# Repo is bind-mounted at run time (see BUILDING.md). JUPYTER_PATH points
# at the mount so kernel_spec::discover_specs picks up `<repo>/test-kernels/`.
WORKDIR /jet
ENV JUPYTER_PATH=/jet

# tini reaps zombies (see above); sleep-infinity keeps the container
# alive for `container exec`.
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["sleep", "infinity"]

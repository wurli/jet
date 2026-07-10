#!/usr/bin/env bash
# Bootstrap the Jupyter kernels the test suite expects to find under `kernels/`.
#
# Idempotent and works both locally and in CI:
#   1. Installs `uv` if it's not already on PATH.
#   2. Uses `uv` to install `ipykernel` into an isolated tool venv.
#   3. Writes kernelspecs for `python3` (signal mode) and `python3-message`
#      (message mode) into `kernels/` — both point at the same ipykernel
#      launcher via `uv tool run`, so a single ipykernel install serves
#      both interrupt-mode branches.
#   4. Downloads a pinned Ark release into `kernels/ark/` and writes its
#      kernelspec. R itself is NOT installed here — set it up out-of-band
#      (e.g. `r-lib/actions/setup-r` in CI, or `brew install r` locally).
#      The ark tests skip cleanly when R is missing.
#
# The repo root is passed to jet via `JUPYTER_PATH=<repo>` when running
# tests; `kernel_spec::discover_specs` picks up `$JUPYTER_PATH/kernels`.

set -euo pipefail

# ─── config ────────────────────────────────────────────────────────────
ARK_VERSION="${ARK_VERSION:-0.1.252}"
# Pin ipykernel so contributors see the same banner shape. The "Tip: ..."
# line ipython prints on startup is still randomised per-run — we mask
# it in tests/snapshots.rs — but everything else is now deterministic.
IPYKERNEL_VERSION="${IPYKERNEL_VERSION:-7.3.0}"
UV_VERSION="${UV_VERSION:-0.9.7}"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KERNELS_DIR="$REPO_ROOT/test-kernels"

# ─── platform detection ────────────────────────────────────────────────
uname_s=$(uname -s)
uname_m=$(uname -m)
case "$uname_s-$uname_m" in
  Linux-x86_64)   ark_asset="ark-${ARK_VERSION}-linux-x64.zip" ;;
  Linux-aarch64)  ark_asset="ark-${ARK_VERSION}-linux-arm64.zip" ;;
  Darwin-arm64)   ark_asset="ark-${ARK_VERSION}-darwin-arm64.zip" ;;
  Darwin-x86_64)  ark_asset="ark-${ARK_VERSION}-darwin-x64.zip" ;;
  *) echo "unsupported platform: $uname_s-$uname_m" >&2; exit 1 ;;
esac

# ─── uv ────────────────────────────────────────────────────────────────
if ! command -v uv >/dev/null 2>&1; then
  echo "==> installing uv ${UV_VERSION}"
  # Official installer. Pins the version; writes to $HOME/.local/bin.
  curl -LsSf "https://astral.sh/uv/${UV_VERSION}/install.sh" | sh
  export PATH="$HOME/.local/bin:$PATH"
fi
uv --version

# ─── ipykernel ─────────────────────────────────────────────────────────
# ipykernel is a library, not a CLI tool, so `uv tool install` won't work
# (no console entry points). Instead build a private venv under
# `.jet-dev/venv` and reference its interpreter directly from each
# kernelspec's `argv[0]`. Fully isolated from the user's environment.
VENV_DIR="$REPO_ROOT/.jet-dev/venv"
VENV_PY="$VENV_DIR/bin/python"
# Skip rebuild if the venv already has the pinned ipykernel installed —
# lets CI restore the venv from cache without re-downloading wheels.
if [[ -x "$VENV_PY" ]] \
  && [[ -n "$IPYKERNEL_VERSION" ]] \
  && "$VENV_PY" -c "import ipykernel; import sys; sys.exit(0 if ipykernel.__version__ == '$IPYKERNEL_VERSION' else 1)" 2>/dev/null; then
  echo "==> ipykernel ${IPYKERNEL_VERSION} already installed at $VENV_DIR"
else
  echo "==> creating ipykernel venv at $VENV_DIR"
  uv venv --clear "$VENV_DIR"
  if [[ -n "$IPYKERNEL_VERSION" ]]; then
    uv pip install --python "$VENV_PY" "ipykernel==$IPYKERNEL_VERSION"
  else
    uv pip install --python "$VENV_PY" ipykernel
  fi
fi

mkdir -p "$KERNELS_DIR"

write_python_kernelspec() {
  local name=$1
  local mode=$2
  local dir="$KERNELS_DIR/$name"
  mkdir -p "$dir"
  cat >"$dir/kernel.json" <<JSON
{
  "argv": [
    "$VENV_PY",
    "-m",
    "ipykernel_launcher",
    "-f",
    "{connection_file}"
  ],
  "display_name": "Python 3 ($name)",
  "language": "python",
  "interrupt_mode": "$mode"
}
JSON
  echo "==> wrote $dir/kernel.json (interrupt_mode=$mode)"
}

write_python_kernelspec "python3" "signal"
write_python_kernelspec "python3-message" "message"

# ─── ark ───────────────────────────────────────────────────────────────
ark_dir="$KERNELS_DIR/ark"
ark_bin="$ark_dir/ark"
if [[ -x "$ark_bin" ]] && [[ "$("$ark_bin" --version 2>/dev/null || true)" == *"$ARK_VERSION"* ]]; then
  echo "==> ark ${ARK_VERSION} already installed at $ark_bin"
else
  echo "==> downloading ark ${ARK_VERSION} ($ark_asset)"
  mkdir -p "$ark_dir"
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL -o "$tmp/ark.zip" \
    "https://github.com/posit-dev/ark/releases/download/${ARK_VERSION}/${ark_asset}"
  unzip -q -o "$tmp/ark.zip" -d "$ark_dir"
  chmod +x "$ark_bin"
  rm -rf "$tmp"
  trap - EXIT
fi

cat >"$ark_dir/kernel.json" <<JSON
{
  "argv": [
    "$ark_bin",
    "--connection_file",
    "{connection_file}",
    "--session-mode",
    "notebook",
    "--log",
    "$ark_dir/ark.log"
  ],
  "display_name": "Ark R Kernel",
  "language": "R",
  "env": {
    "RUST_LOG": "error"
  }
}
JSON
echo "==> wrote $ark_dir/kernel.json"

# ─── summary ───────────────────────────────────────────────────────────
echo
echo "Kernels ready under $KERNELS_DIR:"
ls "$KERNELS_DIR"
echo
echo "Run tests with:"
echo "  JUPYTER_PATH=$REPO_ROOT cargo test --workspace"

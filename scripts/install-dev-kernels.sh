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
# Pin ipykernel AND ipython so contributors see the same banner shape.
# ipykernel doesn't pin its ipython dep tightly, so leaving it floating
# means CI can end up on a newer ipython than local. The "Tip: ..." line
# ipython prints on startup is still randomised per-run — we mask it in
# tests/snapshots.rs — but everything else is now deterministic.
IPYKERNEL_VERSION="${IPYKERNEL_VERSION:-7.3.0}"
IPYTHON_VERSION="${IPYTHON_VERSION:-9.15.0}"
# The R version the ark kernel drives. Pinned so snapshots that capture
# the R startup banner survive local-vs-CI mismatches. CI's
# r-lib/actions/setup-r should use the same version; locally, install
# this version out-of-band (e.g. via rig or a matching brew formula).
R_VERSION="${R_VERSION:-4.5.0}"
UV_VERSION="${UV_VERSION:-0.9.7}"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KERNELS_DIR="$REPO_ROOT/test-kernels"

# ─── R version check ───────────────────────────────────────────────────
# Tests capture ark's R startup banner in snapshots, so the R version
# has to match across local and CI. Fail fast with a clear message if
# it doesn't. We only warn if R is missing entirely — ark tests skip
# cleanly in that case.
if command -v R >/dev/null 2>&1; then
  actual_r=$(R --version 2>&1 | head -n1 | awk '{print $3}')
  if [[ "$actual_r" != "$R_VERSION" ]]; then
    echo "ERROR: R version mismatch — expected $R_VERSION, found $actual_r" >&2
    echo "  Install R $R_VERSION (e.g. via rig: 'rig add $R_VERSION')," >&2
    echo "  or override the pin with R_VERSION=$actual_r if you know what you're doing." >&2
    exit 1
  fi
  echo "==> R $actual_r matches pin"
else
  echo "==> R not on PATH; ark tests will skip"
fi

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
# Skip rebuild if the venv already has both pinned versions installed —
# lets CI restore the venv from cache without re-downloading wheels.
if [[ -x "$VENV_PY" ]] \
  && "$VENV_PY" -c "
import sys, ipykernel, IPython
sys.exit(0 if ipykernel.__version__ == '$IPYKERNEL_VERSION'
             and IPython.__version__ == '$IPYTHON_VERSION' else 1)
" 2>/dev/null; then
  echo "==> ipykernel ${IPYKERNEL_VERSION} + ipython ${IPYTHON_VERSION} already installed at $VENV_DIR"
else
  echo "==> creating ipykernel venv at $VENV_DIR"
  uv venv --clear "$VENV_DIR"
  uv pip install --python "$VENV_PY" \
    "ipykernel==$IPYKERNEL_VERSION" \
    "ipython==$IPYTHON_VERSION"
fi

mkdir -p "$KERNELS_DIR"

write_python_kernelspec() {
  local name=$1
  local mode=$2
  local dir="$KERNELS_DIR/$name"
  mkdir -p "$dir"
  # JET_TEST_SPEC_VAR is baked into the spec so
  # `connect_inherits_parent_env_with_spec_winning_on_conflict` can
  # verify (a) spec env reaches the kernel, and (b) spec wins on conflict
  # with the parent env — without having to build a bespoke kernelspec at
  # test time (which pulls in whatever python3 happens to be on PATH).
  # --InteractiveShell.enable_tip=False suppresses IPython's random
  # "Tip: …" banner line, which would otherwise churn snapshots between
  # test runs. SOURCE_DATE_EPOCH also disables it, but only when the
  # banner is the *default* IPython banner — ipykernel customises the
  # banner so that path doesn't apply.
  cat >"$dir/kernel.json" <<JSON
{
  "argv": [
    "$VENV_PY",
    "-m",
    "ipykernel_launcher",
    "-f",
    "{connection_file}",
    "--InteractiveShell.enable_tip=False",
    "--InteractiveShell.banner1=Python test banner",
    "--InteractiveShell.banner2="
  ],
  "display_name": "Python 3 ($name)",
  "language": "python",
  "interrupt_mode": "$mode",
  "env": {
    "JET_TEST_SPEC_VAR": "from-spec"
  }
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
    "$ark_dir/ark.log",
    "--",
    "--quiet"
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

#!/usr/bin/env bash
# Build release on Serafin LOGIN (internet OK). Run before sbatch.
# Usage: bash scripts/ccad/prepare_release.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$ROOT/target/release/opendefalgsplitting"
export PATH="${HOME}/.cargo/bin:${PATH}"
export CARGO_HOME="${HOME}/.cargo"
export RUSTUP_HOME="${HOME}/.rustup"

cd "$ROOT"
echo "Building release on $(hostname)..."
cargo build --release
test -x "$BIN"
"$BIN" --help >/dev/null 2>&1 || true
echo "OK: $BIN"
ls -lh "$BIN"

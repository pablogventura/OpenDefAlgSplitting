#!/usr/bin/env bash
# Compile test binaries on Serafin LOGIN. Run before job_cargo_test.sbatch.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
export PATH="${HOME}/.cargo/bin:${PATH}"
export CARGO_HOME="${HOME}/.cargo"
export RUSTUP_HOME="${HOME}/.rustup"

cd "$ROOT"
bash scripts/ccad/prepare_release.sh
echo "Compiling tests (no-run) on $(hostname)..."
cargo test --release --no-run
echo "OK: test artifacts in target/release/deps/"

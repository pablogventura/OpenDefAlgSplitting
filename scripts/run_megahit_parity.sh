#!/usr/bin/env bash
# Run OpenDefAlgMerging ↔ Rust megahit_merge L1 parity.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${OPENDEFALGSPLITTING_BIN:-$ROOT/target/release/opendefalgsplitting}"
if [[ ! -x "$BIN" ]]; then
  echo "building release binary..."
  (cd "$ROOT" && cargo build --release -q)
fi
if [[ ! -d "$ROOT/oracles/OpenDefAlgMerging" ]]; then
  echo "error: missing oracles/OpenDefAlgMerging" >&2
  exit 2
fi
exec python3 "$ROOT/scripts/parity_megahit_oracle.py" \
  --bin "$BIN" \
  --csv "$ROOT/benches/parity/megahit_oracle_l1.csv" \
  "$@"

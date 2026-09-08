#!/usr/bin/env bash
# Submit the standard Serafin suite: cargo test, smoke ablation, safe ablation.
# Run on the cluster login node from the repo root, or via:
#   ssh pventura@serafin.ccad.unc.edu.ar 'bash ~/src/OpenDefAlgSplitting/scripts/ccad/submit_all.sh'
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CCAD="$ROOT/scripts/ccad"
STAMP_DIR="${HOME}/jobs/submit_logs"
mkdir -p \
  "${HOME}/jobs/cargo_test" \
  "${HOME}/jobs/ablation_smoke/out" \
  "${HOME}/jobs/ablation_safe/out" \
  "${HOME}/jobs/ablation_extended" \
  "$STAMP_DIR"

STAMP="${STAMP_DIR}/submit_$(date +%Y%m%d_%H%M%S).txt"
{
  echo "ROOT=$ROOT"
  echo "HOST=$(hostname)"
  echo "WHEN=$(date -Is)"
} > "$STAMP"

if [[ ! -x "${HOME}/.cargo/bin/cargo" ]]; then
  echo "error: install rustup in \$HOME first (see docs/CCAD_SERAFIN.md)" >&2
  exit 1
fi

echo "Preparing release binary on login (compute nodes have no crates.io)..."
bash "$CCAD/prepare_release.sh" | tee -a "$STAMP"

if [[ ! -x "$ROOT/target/release/opendefalgsplitting" ]]; then
  echo "error: missing $ROOT/target/release/opendefalgsplitting after prepare" >&2
  exit 1
fi

if [[ ! -d "$ROOT/model_examples" ]]; then
  echo "error: expected repo at $ROOT" >&2
  exit 1
fi

submit_one() {
  local script="$1"
  local jobdir="$2"
  local jid
  jid=$(sbatch --parsable "$script")
  echo "$jid" > "${jobdir}/jobid.txt"
  echo "SUBMITTED $(basename "$script") -> $jid" | tee -a "$STAMP"
}

submit_one "$CCAD/job_cargo_test.sbatch" "${HOME}/jobs/cargo_test"
submit_one "$CCAD/job_ablation_smoke.sbatch" "${HOME}/jobs/ablation_smoke"
submit_one "$CCAD/job_ablation_safe.sbatch" "${HOME}/jobs/ablation_safe"

echo "IDs also in $STAMP"
echo "Optional extended (multi, 2h): sbatch $CCAD/job_ablation_extended.sbatch"
squeue -u "${USER}"

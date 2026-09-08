#!/usr/bin/env bash
# Campaña para el paper: build en login + smoke + safe (sin extended).
# Uso en Serafin:
#   bash ~/src/OpenDefAlgSplitting/scripts/ccad/submit_paper_experiment.sh
# Desde la laptop (sync + submit remoto):
#   bash OpenDefAlgSplitting/scripts/ccad/sync_and_submit_paper.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CCAD="$ROOT/scripts/ccad"
STAMP_DIR="${HOME}/jobs/submit_logs"
mkdir -p \
  "${HOME}/jobs/ablation_smoke/out" \
  "${HOME}/jobs/ablation_safe/out" \
  "$STAMP_DIR"

STAMP="${STAMP_DIR}/paper_experiment_$(date +%Y%m%d_%H%M%S).txt"
{
  echo "SUITE=paper_experiment (smoke + safe)"
  echo "ROOT=$ROOT"
  echo "HOST=$(hostname)"
  echo "WHEN=$(date -Is)"
  git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo "GIT=unknown"
} | tee "$STAMP"

if [[ ! -x "${HOME}/.cargo/bin/cargo" ]]; then
  echo "error: install rustup in \$HOME first" >&2
  exit 1
fi

echo "=== prepare_release (login) ===" | tee -a "$STAMP"
bash "$CCAD/prepare_release.sh" | tee -a "$STAMP"

if [[ ! -x "$ROOT/target/release/opendefalgsplitting" ]]; then
  echo "error: missing release binary" >&2
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

submit_one "$CCAD/job_ablation_smoke.sbatch" "${HOME}/jobs/ablation_smoke"
submit_one "$CCAD/job_ablation_safe.sbatch" "${HOME}/jobs/ablation_safe"

echo "" | tee -a "$STAMP"
echo "Cuando terminen (mail + ~/jobs/*/DONE):" | tee -a "$STAMP"
echo "  bash $CCAD/postprocess_on_login.sh" | tee -a "$STAMP"
echo "  scp pventura@serafin.ccad.unc.edu.ar:~/jobs/ablation_safe/out/results_safe.csv docs/ablation/" | tee -a "$STAMP"
echo "" | tee -a "$STAMP"
squeue -u "${USER}" | tee -a "$STAMP"
echo "Log: $STAMP"

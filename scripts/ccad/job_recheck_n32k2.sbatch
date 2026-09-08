#!/bin/bash
#SBATCH --job-name=hit-recheck-n32k2
#SBATCH --partition=multi
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=64
#SBATCH --mem=0
#SBATCH --time=06:00:00
#SBATCH --output=/home/pventura/jobs/recheck_n32k2/slurm-%j.out
#SBATCH --error=/home/pventura/jobs/recheck_n32k2/slurm-%j.err
#SBATCH --mail-type=END,FAIL
#SBATCH --mail-user=pablogventura@gmail.com

# Local laptop aborted these n32_k2 HIT runs (RSS ~22GiB / timeout).
# Serafin nodes have ~126-250 GiB; run HIT then merge one model at a time.

set -euo pipefail
JOBDIR=/home/pventura/jobs/recheck_n32k2
mkdir -p "$JOBDIR/out"
# shellcheck source=/dev/null
source /home/pventura/src/OpenDefAlgSplitting/scripts/ccad/common.env.sh
ROOT=/home/pventura/src/OpenDefAlgSplitting
CORPUS="$ROOT/benches/scale/corpus/pilot"
BIN="$ROOT/target/release/opendefalgsplitting"
# Fewer threads: these instances are memory-heavy, not throughput-bound.
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-8}"

ccad_header | tee "$JOBDIR/header.txt"
cd "$ROOT"
rm -f "$JOBDIR/DONE"
ccad_require_binary "$JOBDIR"

MODELS=(
  random_magma_n32_k2_nondef_mix_i2.model
  random_magma_n32_k2_nondef_mix_i3.model
  random_magma_n32_k2_random_half_i0.model
  random_magma_n32_k2_random_half_i1.model
  random_magma_n32_k2_random_half_i4.model
)

CSV="$JOBDIR/out/verdict_recheck_n32k2.csv"
echo "model,hit,hit_ms,merge,merge_ms,agree" > "$CSV"

run_bench() {
  local engine="$1"
  local model="$2"
  # Single repeat; print last TSV data line only.
  "$BIN" --bench --repeat 1 --fragment qf --engine "$engine" "$model" \
    | awk -F'\t' 'NF>=3 && $1 !~ /^model$/ {line=$0} END{print line}'
}

for name in "${MODELS[@]}"; do
  m="$CORPUS/$name"
  if [[ ! -f "$m" ]]; then
    echo "MISSING $m" | tee -a "$JOBDIR/out/missing.txt"
    echo "${name},MISSING,,MISSING,,false" >> "$CSV"
    continue
  fi
  echo "=== $(date -Is) HIT $name avail=$(awk '/MemAvailable:/{printf "%.0fGi",$2/1024/1024}' /proc/meminfo) ==="
  hline=$(run_bench hit "$m" || true)
  h_ms=$(printf '%s\n' "$hline" | awk -F'\t' '{print $2}')
  h_res=$(printf '%s\n' "$hline" | awk -F'\t' '{print $3}')
  [[ -n "${h_res:-}" ]] || h_res=FAIL
  echo "HIT -> ${h_res} ms=${h_ms}"

  echo "=== $(date -Is) MERGE $name ==="
  mline=$(run_bench merge "$m" || true)
  m_ms=$(printf '%s\n' "$mline" | awk -F'\t' '{print $2}')
  m_res=$(printf '%s\n' "$mline" | awk -F'\t' '{print $3}')
  [[ -n "${m_res:-}" ]] || m_res=FAIL
  echo "MERGE -> ${m_res} ms=${m_ms}"

  agree=false
  if [[ "$h_res" == "$m_res" && "$h_res" != FAIL ]]; then
    agree=true
  fi
  echo "${name},${h_res},${h_ms},${m_res},${m_ms},${agree}" >> "$CSV"
done

mkdir -p "$ROOT/benches/scale/results"
cp -a "$CSV" "$ROOT/benches/scale/results/"
ccad_write_done OK "$JOBDIR" "RESULTS=$CSV" "$(wc -l < "$CSV") lines" "RAYON_NUM_THREADS=$RAYON_NUM_THREADS"

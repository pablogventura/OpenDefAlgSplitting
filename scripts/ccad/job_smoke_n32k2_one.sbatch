#!/bin/bash
#SBATCH --job-name=hit-smoke-n32k2-1
#SBATCH --partition=multi
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=16
#SBATCH --mem=0
#SBATCH --time=02:00:00
#SBATCH --output=/home/pventura/jobs/smoke_n32k2_one/slurm-%j.out
#SBATCH --error=/home/pventura/jobs/smoke_n32k2_one/slurm-%j.err
#SBATCH --mail-type=END,FAIL
#SBATCH --mail-user=pablogventura@gmail.com

# Smoke: one n32_k2 HIT model (decision-only) after TMH cache / RSS cap.
# Then submit job_recheck_n32k2.sbatch if this succeeds.

set -euo pipefail
JOBDIR=/home/pventura/jobs/smoke_n32k2_one
mkdir -p "$JOBDIR/out"
# shellcheck source=/dev/null
source /home/pventura/src/OpenDefAlgSplitting/scripts/ccad/common.env.sh
ROOT=/home/pventura/src/OpenDefAlgSplitting
CORPUS="$ROOT/benches/scale/corpus/pilot"
BIN="$ROOT/target/release/opendefalgsplitting"
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-4}"
# Fail before node OOM (~250 GiB nodes); override with HIT_MAX_RSS_MIB.
export HIT_MAX_RSS_MIB="${HIT_MAX_RSS_MIB:-180000}"

MODEL="${SMOKE_MODEL:-random_magma_n32_k2_nondef_mix_i2.model}"

ccad_header | tee "$JOBDIR/header.txt"
cd "$ROOT"
rm -f "$JOBDIR/DONE"
ccad_require_binary "$JOBDIR"

m="$CORPUS/$MODEL"
if [[ ! -f "$m" ]]; then
  ccad_write_done MISSING "$JOBDIR" "$m"
  exit 1
fi

CSV="$JOBDIR/out/smoke_one.csv"
echo "model,engine,ms,result" > "$CSV"
echo "=== $(date -Is) HIT $MODEL avail=$(awk '/MemAvailable:/{printf "%.0fGi",$2/1024/1024}' /proc/meminfo) HIT_MAX_RSS_MIB=$HIT_MAX_RSS_MIB ==="

set +e
out=$("$BIN" --bench --repeat 1 --fragment qf --engine hit "$m" 2>&1)
rc=$?
set -e
echo "$out" | tee "$JOBDIR/out/hit.log"
hline=$(printf '%s\n' "$out" | awk -F'\t' 'NF>=3 && $1 !~ /^model$/ {line=$0} END{print line}')
h_ms=$(printf '%s\n' "$hline" | awk -F'\t' '{print $2}')
h_res=$(printf '%s\n' "$hline" | awk -F'\t' '{print $3}')
[[ -n "${h_res:-}" ]] || h_res=FAIL
echo "${MODEL},hit,${h_ms},${h_res}" >> "$CSV"

if [[ "$rc" -ne 0 || "$h_res" == "FAIL" || "$h_res" == "TIMEOUT" ]]; then
  ccad_write_done FAIL "$JOBDIR" "HIT=$h_res rc=$rc"
  exit 1
fi

ccad_write_done OK "$JOBDIR" "HIT=$h_res ms=$h_ms" "MODEL=$MODEL"

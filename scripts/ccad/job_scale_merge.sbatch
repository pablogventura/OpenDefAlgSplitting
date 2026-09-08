#!/bin/bash
#SBATCH --job-name=hit-scale-merge
#SBATCH --partition=multi
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=64
#SBATCH --mem=0
#SBATCH --time=48:00:00
#SBATCH --output=/home/pventura/jobs/scale_merge/slurm-%j.out
#SBATCH --error=/home/pventura/jobs/scale_merge/slurm-%j.err
#SBATCH --mail-type=END,FAIL
#SBATCH --mail-user=pablogventura@gmail.com

set -euo pipefail
JOBDIR=/home/pventura/jobs/scale_merge
mkdir -p "$JOBDIR/out"
# shellcheck source=/dev/null
source /home/pventura/src/OpenDefAlgSplitting/scripts/ccad/common.env.sh
ROOT=/home/pventura/src/OpenDefAlgSplitting
MODE="${SCALE_MODE:-pilot}"
ENGINE=merge
MODEL_TIMEOUT_S="${SCALE_MERGE_TIMEOUT_S:-120}"

timeout_for() {
  local base="$1"
  if [[ "$base" == *_n128_* ]]; then echo "${SCALE_MERGE_TIMEOUT_S:-600}"
  elif [[ "$base" == *_n64_* ]]; then echo 300
  elif [[ "$base" == *_n32_* ]]; then echo 180
  else echo "${MODEL_TIMEOUT_S}"
  fi
}

ccad_header | tee "$JOBDIR/header.txt"
cd "$ROOT"
rm -f "$JOBDIR/DONE"
ccad_require_binary "$JOBDIR"

CORPUS="$ROOT/benches/scale/corpus/${MODE}"
if [[ ! -d "$CORPUS" ]]; then
  ccad_write_done CORPUS_MISSING "$JOBDIR" "Generate corpus on login first"
  exit 1
fi

CSV="$JOBDIR/out/results_${ENGINE}_${MODE}.csv"
echo "engine,model,ms,result" > "$CSV"
shopt -s nullglob
for m in "$CORPUS"/*.model; do
  to_s="$(timeout_for "$(basename "$m")")"
  set +e
  line=$(timeout --signal=KILL "${to_s}" \
    "$ROOT/target/release/opendefalgsplitting" \
    --bench --repeat 3 --fragment qf --engine "$ENGINE" "$m" 2>"$JOBDIR/out/last.err" | \
    awk -F'\t' 'NF>=3 && $1!="model" {print; exit}')
  ec=$?
  set -e
  if [[ $ec -eq 124 ]] || [[ $ec -eq 137 ]]; then
    echo "${ENGINE},${m},,TIMEOUT" >> "$CSV"
    continue
  fi
  if [[ -z "${line:-}" ]]; then
    echo "${ENGINE},${m},,ERROR" >> "$CSV"
    continue
  fi
  model=$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')
  ms=$(printf '%s\n' "$line" | awk -F'\t' '{print $2}' | sed 's/±.*//')
  res=$(printf '%s\n' "$line" | awk -F'\t' '{print $3}')
  echo "${ENGINE},${model},${ms},${res}" >> "$CSV"
done

mkdir -p "$ROOT/benches/scale/results"
cp -a "$CSV" "$ROOT/benches/scale/results/"
ccad_write_done OK "$JOBDIR" "RESULTS=$CSV" "$(wc -l < "$CSV") lines" "BASE_TIMEOUT_S=$MODEL_TIMEOUT_S"

#!/usr/bin/env bash
# Local full-scale megahit merge run. One model at a time; resumable.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BIN="$ROOT/target/release/opendefalgsplitting"
CORPUS="$ROOT/benches/scale/corpus/full"
OUT="$ROOT/benches/scale/results/results_merge_full.csv"
LOG="$ROOT/benches/scale/results/merge_full_run.log"

mkdir -p "$(dirname "$OUT")"
if [[ ! -f "$OUT" ]]; then
  echo 'engine,model,ms,result' > "$OUT"
fi

python3 - <<'PY' > /tmp/merge_full_done.txt
import csv
from pathlib import Path
p = Path("benches/scale/results/results_merge_full.csv")
if not p.is_file():
    raise SystemExit
with p.open(newline="", encoding="utf-8") as f:
    for row in csv.DictReader(f):
        name = Path(row.get("model", "")).name
        if name and name != "model":
            print(name)
PY

declare -A SEEN=()
while IFS= read -r n; do
  [[ -z "$n" ]] && continue
  SEEN["$n"]=1
done < /tmp/merge_full_done.txt

timeout_for() {
  local base
  base="$(basename "$1")"
  if [[ "$base" == *_n128_* ]]; then echo 600
  elif [[ "$base" == *_n64_* ]]; then echo 300
  elif [[ "$base" == *_n32_* ]]; then echo 180
  else echo 90
  fi
}

shopt -s nullglob
models=("$CORPUS"/*.model)
total=${#models[@]}
ok=0
to=0
err=0
skip=0
i=0
echo "START total=$total already=${#SEEN[@]}" | tee -a "$LOG"
for m in "${models[@]}"; do
  i=$((i + 1))
  name="$(basename "$m")"
  if [[ -n "${SEEN[$name]:-}" ]]; then
    skip=$((skip + 1))
    continue
  fi
  to_s="$(timeout_for "$m")"
  set +e
  line=$(timeout --signal=TERM "$to_s" "$BIN" \
    --bench --repeat 1 --fragment qf --engine merge "$m" 2>>"$LOG" | awk -F'\t' 'NF>=3 && $1!="model" {print; exit}')
  ec=$?
  set -e
  if [[ $ec -eq 124 ]] || [[ $ec -eq 137 ]]; then
    echo "merge,$m,,TIMEOUT" >> "$OUT"
    to=$((to + 1))
    echo "[$i/$total] TIMEOUT $name" | tee -a "$LOG"
    continue
  fi
  if [[ -z "${line:-}" ]]; then
    echo "merge,$m,,ERROR" >> "$OUT"
    err=$((err + 1))
    echo "[$i/$total] ERROR $name ec=$ec" | tee -a "$LOG"
    continue
  fi
  model=$(printf '%s\n' "$line" | awk -F'\t' '{print $1}')
  ms=$(printf '%s\n' "$line" | awk -F'\t' '{print $2}' | sed 's/±.*//')
  res=$(printf '%s\n' "$line" | awk -F'\t' '{print $3}')
  if [[ "$model" == "model" || -z "$res" ]]; then
    echo "merge,$m,,ERROR" >> "$OUT"
    err=$((err + 1))
    echo "[$i/$total] BADLINE $name" | tee -a "$LOG"
    continue
  fi
  echo "merge,${model},${ms},${res}" >> "$OUT"
  ok=$((ok + 1))
  if (( i % 25 == 0 || ok % 25 == 0 )); then
    echo "[$i/$total] ok=$ok to=$to err=$err skip=$skip last=$name ${ms}ms $res" | tee -a "$LOG"
    free -h | head -2 | tee -a "$LOG"
  fi
done
echo "DONE ok=$ok timeout=$to error=$err skip=$skip" | tee -a "$LOG"

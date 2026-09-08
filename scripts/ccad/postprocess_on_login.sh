#!/usr/bin/env bash
# Post-proceso en el LOGIN de Serafin (python3 no esta en nodos de computo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=/dev/null
source "$ROOT/scripts/ccad/common.env.sh"
OUT="$ROOT/docs/ablation"
mkdir -p "$OUT"

run_one() {
  local csv="$1"
  local txt="$2"
  if [[ ! -f "$csv" ]]; then
    echo "skip missing $csv"
    return 0
  fi
  echo "=== $csv ==="
  ccad_analyze_ablation_csv "$csv" "$txt"
}

run_one "$OUT/results_safe.csv" "$OUT/analysis_safe.txt"
run_one "$OUT/results.csv" "$OUT/analysis_smoke.txt"
run_one "$OUT/results_extended.csv" "$OUT/analysis_extended.txt"

echo "Done. Summaries in $OUT/analysis_*.txt"

#!/usr/bin/env bash
# Ablacion: baseline + cada mejora sola + combo (sin IG experimental en combo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/opendefalgsplitting"
OUT_DIR="$ROOT/docs/ablation"
mkdir -p "$OUT_DIR"
CSV="$OUT_DIR/results.csv"

cd "$ROOT"
if [[ ! -x "$BIN" ]]; then
  cargo build --release
fi

SMOKE=(
  model_examples/modeloqueanda.model
  model_examples/suma4.model
  model_examples/retrombo_nodef.model
  model_examples/msimple.model
)
MEDIUM=(
  model_examples/cadena5.model
  model_examples/gigante.model
  model_examples/16_T3_4_0.model
)

echo "variant,model,ms,steps,skipped,approx_reject,result,formula_size" > "$CSV"

run_one() {
  local variant="$1"
  local model="$2"
  shift 2
  local line
  if ! line=$("$BIN" --bench --repeat 3 --ablation-csv "$@" "$model" 2>/dev/null | tail -n 1); then
    echo "${variant},${model},ERR,0,0,0,ERROR,0" >> "$CSV"
    echo "  FAIL ${variant} ${model}"
    return 0
  fi
  if [[ -z "$line" || "$line" == model,* ]]; then
    echo "${variant},${model},ERR,0,0,0,ERROR,0" >> "$CSV"
    echo "  FAIL ${variant} ${model}"
    return 0
  fi
  echo "${variant},${line}" >> "$CSV"
  echo "  ok ${variant} ${model} => ${line}"
}

echo "=== SMOKE ==="
for m in "${SMOKE[@]}"; do
  [[ -f "$m" ]] || continue
  run_one baseline "$m"
  run_one skip_useless "$m" --skip-useless
  run_one ig_legacy "$m" -i --ig-sample 20
  run_one ig_experimental "$m" --ig-experimental --ig-sample 20
  run_one approx "$m" --approx-precheck
  run_one simplify "$m" --simplify
  run_one bfs "$m" --bfs
  run_one combo "$m" --skip-useless --approx-precheck --simplify
done

echo "=== MEDIUM ==="
for m in "${MEDIUM[@]}"; do
  [[ -f "$m" ]] || continue
  run_one baseline "$m"
  run_one skip_useless "$m" --skip-useless
  run_one ig_legacy "$m" -i --ig-sample 20
  run_one approx "$m" --approx-precheck
  run_one simplify "$m" --simplify
  run_one bfs "$m" --bfs
  run_one combo "$m" --skip-useless --approx-precheck --simplify
  run_one ig_experimental "$m" --ig-experimental --ig-sample 20
done

echo "=== CLASS ==="
"$BIN" --class \
  model_examples/modeloqueanda.model \
  model_examples/suma4.model \
  | tee "$OUT_DIR/class_smoke.txt"

echo "CSV -> $CSV"

#!/usr/bin/env bash
# Ablacion extendida: mas modelos, mas repeats, foco en baseline/skip/approx/combo (+ bfs/-i en subset).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/opendefalgsplitting"
OUT_DIR="$ROOT/docs/ablation"
mkdir -p "$OUT_DIR"
CSV="$OUT_DIR/results_extended.csv"
LOG="$OUT_DIR/extended_run.log"

cd "$ROOT"
cargo build --release

# Disponibilidad de RAM (evitar OOM en modelos grandes)
avail_kb=$(awk '/MemAvailable:/ {print $2}' /proc/meminfo)
avail_gb=$((avail_kb / 1024 / 1024))
echo "MemAvailable ~${avail_gb} GiB" | tee "$LOG"
if (( avail_gb < 4 )); then
  echo "RAM baja (<4 GiB); aborto." | tee -a "$LOG"
  exit 1
fi

SMALL=(
  model_examples/modeloqueanda.model
  model_examples/suma4.model
  model_examples/retrombo_nodef.model
  model_examples/msimple.model
  model_examples/retrombo.model
  model_examples/cadena4.model
  model_examples/malvada.model
  model_examples/algebra.model
  model_examples/miprueba.model
  model_examples/posetrombo.model
)

MEDIUM=(
  model_examples/cadena5.model
  model_examples/gigante.model
  model_examples/16_T3_4_0.model
  model_examples/p0_d0.0625_a2_u30_q5.model
  model_examples/modelosexperimento.model
  model_examples/modelosimplequefalla.model
  model_examples/modelosupertragico.model
  model_examples/retrombo_nodef_sinpura.model
  model_examples/retromboconstantes.model
)

# LARGE/HUGE deshabilitados: estres_paralelo / cadena20 llenan la RAM.
# Solo small+medium en esta suite.
LARGE=()
HUGE=()

REPEAT_SMALL=7
REPEAT_MEDIUM=5
REPEAT_LARGE=0

echo "variant,model,ms,steps,skipped,approx_reject,result,formula_size" > "$CSV"

run_one() {
  local variant="$1"
  local model="$2"
  local repeat="$3"
  shift 3
  if [[ ! -f "$model" ]]; then
    echo "  skip missing $model" | tee -a "$LOG"
    return 0
  fi
  local line
  if ! line=$("$BIN" --bench --repeat "$repeat" --ablation-csv "$@" "$model" 2>>"$LOG" | tail -n 1); then
    echo "${variant},${model},ERR,0,0,0,ERROR,0" >> "$CSV"
    echo "  FAIL ${variant} ${model}" | tee -a "$LOG"
    return 0
  fi
  if [[ -z "$line" || "$line" == model,* ]]; then
    echo "${variant},${model},ERR,0,0,0,ERROR,0" >> "$CSV"
    echo "  FAIL ${variant} ${model}" | tee -a "$LOG"
    return 0
  fi
  # line = model,ms,steps,...  -> prefix variant
  echo "${variant},${line}" >> "$CSV"
  echo "  ok ${variant} ${model} => ${line}" | tee -a "$LOG"
}

core_variants() {
  local m="$1"
  local r="$2"
  run_one baseline "$m" "$r"
  run_one skip_useless "$m" "$r" --skip-useless
  run_one approx "$m" "$r" --approx-precheck
  run_one combo "$m" "$r" --skip-useless --approx-precheck --simplify
}

extra_variants() {
  local m="$1"
  local r="$2"
  run_one simplify "$m" "$r" --simplify
  run_one bfs "$m" "$r" --bfs
  run_one ig_legacy "$m" "$r" -i --ig-sample 20
}

echo "=== SMALL (repeat=${REPEAT_SMALL}) ===" | tee -a "$LOG"
for m in "${SMALL[@]}"; do
  core_variants "$m" "$REPEAT_SMALL"
  extra_variants "$m" "$REPEAT_SMALL"
done

echo "=== MEDIUM (repeat=${REPEAT_MEDIUM}) ===" | tee -a "$LOG"
for m in "${MEDIUM[@]}"; do
  core_variants "$m" "$REPEAT_MEDIUM"
  extra_variants "$m" "$REPEAT_MEDIUM"
done

echo "=== LARGE core-only (repeat=${REPEAT_LARGE}) ===" | tee -a "$LOG"
for m in "${LARGE[@]}"; do
  core_variants "$m" "$REPEAT_LARGE"
done

if (( avail_gb >= 8 )); then
  echo "=== HUGE baseline+skip+combo (repeat=2, timeout 180s) ===" | tee -a "$LOG"
  for m in "${HUGE[@]}"; do
    for variant_cmd in \
      "baseline|" \
      "skip_useless|--skip-useless" \
      "combo|--skip-useless --approx-precheck --simplify"
    do
      variant="${variant_cmd%%|*}"
      flags="${variant_cmd#*|}"
      if [[ ! -f "$m" ]]; then continue; fi
      # shellcheck disable=SC2086
      if line=$(timeout 180 "$BIN" --bench --repeat 2 --ablation-csv $flags "$m" 2>>"$LOG" | tail -n 1); then
        if [[ -n "$line" && "$line" != model,* ]]; then
          echo "${variant},${line}" >> "$CSV"
          echo "  ok ${variant} ${m} => ${line}" | tee -a "$LOG"
        else
          echo "${variant},${m},TIMEOUT_OR_ERR,0,0,0,ERROR,0" >> "$CSV"
        fi
      else
        echo "${variant},${m},TIMEOUT,0,0,0,ERROR,0" >> "$CSV"
        echo "  TIMEOUT ${variant} ${m}" | tee -a "$LOG"
      fi
    done
  done
else
  echo "Skip HUGE (RAM < 8 GiB)" | tee -a "$LOG"
fi

echo "CSV -> $CSV" | tee -a "$LOG"
python3 "$ROOT/scripts/analyze_ablation.py" "$CSV" | tee "$OUT_DIR/analysis_extended.txt"
#!/usr/bin/env bash
# Ablacion segura: baseline + skip + approx + combo. Sin LARGE/HUGE.
# Chequea MemAvailable entre modelos y usa timeout por corrida.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/opendefalgsplitting"
OUT_DIR="$ROOT/docs/ablation"
mkdir -p "$OUT_DIR"
CSV="$OUT_DIR/results_safe.csv"
LOG="$OUT_DIR/safe_run.log"
MIN_AVAIL_GIB=8
TIMEOUT_SECS=90
REPEAT=5

cd "$ROOT"
if [[ ! -x "$BIN" ]]; then
  cargo build --release
fi

avail_gib() {
  awk '/MemAvailable:/ {printf "%d", $2/1024/1024}' /proc/meminfo
}

echo "MemAvailable start: $(avail_gib) GiB" | tee "$LOG"
if (( $(avail_gib) < MIN_AVAIL_GIB )); then
  echo "RAM disponible < ${MIN_AVAIL_GIB} GiB; aborto." | tee -a "$LOG"
  exit 1
fi

MODELS=(
  model_examples/modeloqueanda.model
  model_examples/suma4.model
  model_examples/retrombo_nodef.model
  model_examples/msimple.model
  model_examples/retrombo.model
  model_examples/cadena4.model
  model_examples/malvada.model
  model_examples/algebra.model
  model_examples/miprueba.model
  model_examples/cadena5.model
  model_examples/gigante.model
  model_examples/16_T3_4_0.model
  model_examples/p0_d0.0625_a2_u30_q5.model
  model_examples/modelosexperimento.model
  model_examples/modelosimplequefalla.model
  model_examples/retrombo_nodef_sinpura.model
  model_examples/retromboconstantes.model
  model_examples/cadena100.model
)

echo "variant,model,ms,steps,skipped,approx_reject,result,formula_size" > "$CSV"

run_one() {
  local variant="$1"
  local model="$2"
  shift 2
  if [[ ! -f "$model" ]]; then
    echo "  missing $model" | tee -a "$LOG"
    return 0
  fi
  local av
  av=$(avail_gib)
  if (( av < MIN_AVAIL_GIB )); then
    echo "  STOP RAM baja (${av} GiB) antes de ${variant} ${model}" | tee -a "$LOG"
    return 2
  fi
  local line rc=0
  set +e
  line=$(timeout --signal=KILL "${TIMEOUT_SECS}" "$BIN" --bench --repeat "$REPEAT" --ablation-csv "$@" "$model" 2>>"$LOG" | tail -n 1)
  rc=$?
  set -e
  if (( rc == 137 || rc == 124 )); then
    echo "${variant},${model},TIMEOUT,0,0,0,TIMEOUT,0" >> "$CSV"
    echo "  TIMEOUT ${variant} ${model} (rc=${rc})" | tee -a "$LOG"
    return 0
  fi
  if (( rc != 0 )) || [[ -z "$line" || "$line" == model,* ]]; then
    echo "${variant},${model},ERR,0,0,0,ERROR,0" >> "$CSV"
    echo "  FAIL ${variant} ${model} (rc=${rc})" | tee -a "$LOG"
    return 0
  fi
  echo "${variant},${line}" >> "$CSV"
  echo "  ok ${variant} ${model} => ${line} [avail=$(avail_gib)G]" | tee -a "$LOG"
}

echo "=== SAFE suite (repeat=${REPEAT}, timeout=${TIMEOUT_SECS}s) ===" | tee -a "$LOG"
for m in "${MODELS[@]}"; do
  run_one baseline "$m" --no-auto --no-skip-useless || break
  run_one skip_useless "$m" --no-auto --skip-useless || break
  run_one approx "$m" --no-auto --no-skip-useless --approx-precheck || break
  run_one combo "$m" --no-auto --skip-useless --approx-precheck --simplify || break
done

echo "CSV -> $CSV" | tee -a "$LOG"
# shellcheck source=/dev/null
if [[ -f "$ROOT/scripts/ccad/common.env.sh" ]]; then
  source "$ROOT/scripts/ccad/common.env.sh"
  ccad_analyze_ablation_csv "$CSV" "$OUT_DIR/analysis_safe.txt" | tee -a "$LOG"
else
  if command -v python3 >/dev/null 2>&1; then
    python3 "$ROOT/scripts/analyze_ablation.py" "$CSV" | tee "$OUT_DIR/analysis_safe.txt"
  else
    echo "WARN: python3 not found; skipping analysis (CSV is complete)." | tee -a "$LOG"
  fi
fi

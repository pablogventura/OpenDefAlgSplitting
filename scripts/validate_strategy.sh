#!/usr/bin/env bash
# Valida el selector auto vs oráculos (baseline sin skip, skip, approx, combo).
# Suite segura (sin estres/cadena20). Chequea MemAvailable y timeout por corrida.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/opendefalgsplitting"
OUT_DIR="$ROOT/docs/ablation"
mkdir -p "$OUT_DIR"
CSV="$OUT_DIR/results_strategy_validate.csv"
LOG="$OUT_DIR/strategy_validate.log"
SUMMARY="$OUT_DIR/strategy_validate_summary.txt"
MIN_AVAIL_GIB=8
TIMEOUT_SECS=90
REPEAT=3

cd "$ROOT"
cargo build --release

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

echo "=== STRATEGY validate (repeat=${REPEAT}, timeout=${TIMEOUT_SECS}s) ===" | tee -a "$LOG"
for m in "${MODELS[@]}"; do
  # baseline histórico: sin skip ni auto
  run_one baseline_noskip "$m" --no-auto --no-skip-useless || break
  run_one skip "$m" --no-auto --skip-useless || break
  run_one approx "$m" --no-auto --no-skip-useless --approx-precheck || break
  run_one combo "$m" --no-auto --skip-useless --approx-precheck --simplify || break
  # auto: sin flags de estrategia
  run_one auto "$m" || break
done

echo "CSV -> $CSV" | tee -a "$LOG"

python3 - "$CSV" "$SUMMARY" <<'PY'
import csv, sys
from collections import defaultdict

csv_path, summary_path = sys.argv[1], sys.argv[2]
rows = list(csv.DictReader(open(csv_path)))
by = defaultdict(dict)
for r in rows:
    by[r["model"]][r["variant"]] = r

oracle = "skip"  # mismo veredicto esperado; skip es el oráculo de velocidad correcta
mismatches = []
auto_ms = []
skip_ms = []
base_ms = []

for model, variants in sorted(by.items()):
    if "auto" not in variants or oracle not in variants:
        continue
    a, s = variants["auto"], variants[oracle]
    if a["result"] in ("TIMEOUT", "ERROR") or s["result"] in ("TIMEOUT", "ERROR"):
        mismatches.append((model, a["result"], s["result"], "timeout/error"))
        continue
    if a["result"] != s["result"]:
        mismatches.append((model, a["result"], s["result"], "verdict"))
    try:
        auto_ms.append(float(a["ms"]))
        skip_ms.append(float(s["ms"]))
        if "baseline_noskip" in variants and variants["baseline_noskip"]["ms"] not in ("TIMEOUT", "ERR"):
            base_ms.append(float(variants["baseline_noskip"]["ms"]))
    except ValueError:
        pass

def mean(xs):
    return sum(xs) / len(xs) if xs else float("nan")

lines = []
lines.append(f"models_compared={len(auto_ms)}")
lines.append(f"verdict_mismatches={len([m for m in mismatches if m[3]=='verdict'])}")
lines.append(f"timeout_or_error={len([m for m in mismatches if m[3]!='verdict'])}")
lines.append(f"mean_ms_auto={mean(auto_ms):.3f}")
lines.append(f"mean_ms_skip={mean(skip_ms):.3f}")
lines.append(f"mean_ms_baseline_noskip={mean(base_ms):.3f}")
if skip_ms and auto_ms:
    ratio = mean(auto_ms) / mean(skip_ms) if mean(skip_ms) else float("inf")
    lines.append(f"ratio_auto_over_skip={ratio:.4f}")
    lines.append(f"accept_auto_le_skip={mean(auto_ms) <= mean(skip_ms) * 1.05}")  # 5% tol ruido
for m in mismatches:
    lines.append(f"MISMATCH\t{m[0]}\tauto={m[1]}\toracle={m[2]}\tkind={m[3]}")

text = "\n".join(lines) + "\n"
print(text)
open(summary_path, "w").write(text)
PY

echo "SUMMARY -> $SUMMARY" | tee -a "$LOG"
cat "$SUMMARY" | tee -a "$LOG"

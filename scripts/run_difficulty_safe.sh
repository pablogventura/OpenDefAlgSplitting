#!/usr/bin/env bash
# Pipeline completo de predictores (Python only + analisis).
# Lean opcional al final con RUN_LEAN=1.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
QFDEF="$(cd "$ROOT/../qfdef" && pwd)"
# shellcheck source=/dev/null
source "$ROOT/../scripts/ram_guard.sh"

MIN_GIB="${MIN_AVAIL_GIB:-6}"
RUN_LEAN="${RUN_LEAN:-0}"
REAP_LEAN="${REAP_LEAN:-0}"

cd "$ROOT"
ram_require "$MIN_GIB"
(( REAP_LEAN )) && lean_reap_lsp_workers

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
)

OUT="$ROOT/docs/ablation/difficulty_metrics.csv"
TMP="$ROOT/docs/ablation/difficulty_metrics.partial.csv"
mkdir -p "$ROOT/docs/ablation"
echo "model,target,universe,k,inj_tuples,pat_classes,H_pat_univ,H_R,target_density,H_fp,fp_classes,pattern_mixes,fuel,skipped" > "$TMP"

for m in "${MODELS[@]}"; do
  ram_require "$MIN_GIB"
  line=$(python3 scripts/difficulty_metrics.py --models "$m" --emit-row 2>/dev/null)
  if [[ -n "$line" && "$line" != model,* ]]; then
    echo "$line" >> "$TMP"
    echo "  ok $m ($(ram_avail_gib) GiB libres)"
  else
    echo "  fail $m" >&2
  fi
done

mv "$TMP" "$OUT"
echo "wrote $OUT"

python3 scripts/analyze_difficulty.py
python3 "$QFDEF/scripts/random_graph_entropy.py" --grid --samples 30

if (( RUN_LEAN )); then
  ram_require 10
  bash "$QFDEF/scripts/run-entropy.sh" "$QFDEF/docs/entropy_crosscheck.csv"
fi

echo "done ($(ram_avail_gib) GiB libres)"

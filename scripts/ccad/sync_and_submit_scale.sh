#!/usr/bin/env bash
# Sync local OpenDefAlgSplitting to Serafin and submit scale jobs.
# Env: SCALE_MODE=pilot|full  SCALE_ENGINE=hit|merge|both
set -euo pipefail
LOCAL_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REMOTE="pventura@serafin.ccad.unc.edu.ar"
REMOTE_ROOT='~/src/OpenDefAlgSplitting'
export SCALE_MODE="${SCALE_MODE:-pilot}"
export SCALE_ENGINE="${SCALE_ENGINE:-both}"

if [[ ! -d "$LOCAL_ROOT/benches/scale/corpus/${SCALE_MODE}" ]] \
  || [[ -z "$(ls -A "$LOCAL_ROOT/benches/scale/corpus/${SCALE_MODE}" 2>/dev/null || true)" ]]; then
  echo "Generating ${SCALE_MODE} corpus locally..."
  if [[ "$SCALE_MODE" == "full" ]]; then
    python3 "$LOCAL_ROOT/scripts/generate_scale_corpus.py" \
      --manifest "$LOCAL_ROOT/benches/scale/manifest.json" \
      --out "$LOCAL_ROOT/benches/scale/corpus"
  else
    python3 "$LOCAL_ROOT/scripts/generate_scale_corpus.py" \
      --manifest "$LOCAL_ROOT/benches/scale/manifest.json" \
      --out "$LOCAL_ROOT/benches/scale/corpus" \
      --pilot-only --max-n 32
  fi
fi

rsync -az --delete \
  --exclude 'target/' --exclude '.git/' \
  --exclude 'oracles/OpenDefAlgMerging/venv/' \
  --exclude 'oracles/OpenDefAlgMerging/OpenDefAlgMerging_Running_Example.ipynb' \
  -e 'ssh -o BatchMode=yes' \
  "$LOCAL_ROOT/" \
  "${REMOTE}:${REMOTE_ROOT}/"

ssh -o BatchMode=yes "$REMOTE" \
  "export SCALE_MODE='$SCALE_MODE' SCALE_ENGINE='$SCALE_ENGINE' \
   SCALE_HIT_TIMEOUT_S='${SCALE_HIT_TIMEOUT_S:-}' \
   SCALE_MERGE_TIMEOUT_S='${SCALE_MERGE_TIMEOUT_S:-}'; \
   bash ${REMOTE_ROOT}/scripts/ccad/submit_scale.sh"

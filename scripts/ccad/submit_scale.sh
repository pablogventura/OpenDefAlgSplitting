#!/usr/bin/env bash
# Build release, generate corpus if missing, submit scale job(s).
# SCALE_MODE=pilot|full (default pilot)
# SCALE_ENGINE=hit|merge|both (default both)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CCAD="$ROOT/scripts/ccad"
export SCALE_MODE="${SCALE_MODE:-pilot}"
export SCALE_ENGINE="${SCALE_ENGINE:-both}"

mkdir -p \
  "${HOME}/jobs/scale_hit/out" \
  "${HOME}/jobs/scale_merge/out" \
  "$ROOT/benches/scale/results" \
  "$ROOT/benches/scale/corpus"

bash "$CCAD/prepare_release.sh"

if [[ ! -d "$ROOT/benches/scale/corpus/${SCALE_MODE}" ]] \
  || [[ -z "$(ls -A "$ROOT/benches/scale/corpus/${SCALE_MODE}" 2>/dev/null || true)" ]]; then
  echo "Generating ${SCALE_MODE} corpus..."
  if command -v python3 >/dev/null 2>&1; then
    py=python3
  elif [[ -x /usr/bin/python3 ]]; then
    py=/usr/bin/python3
  else
    echo "error: need python3 to generate corpus" >&2
    exit 1
  fi
  if [[ "$SCALE_MODE" == "full" ]]; then
    "$py" "$ROOT/scripts/generate_scale_corpus.py" \
      --manifest "$ROOT/benches/scale/manifest.json" \
      --out "$ROOT/benches/scale/corpus"
  else
    "$py" "$ROOT/scripts/generate_scale_corpus.py" \
      --manifest "$ROOT/benches/scale/manifest.json" \
      --out "$ROOT/benches/scale/corpus" \
      --pilot-only --max-n 32
  fi
fi

# Full campaign: generous per-model caps (manifest n128=600s); pilot keeps 120s default.
if [[ "$SCALE_MODE" == "full" ]]; then
  export SCALE_HIT_TIMEOUT_S="${SCALE_HIT_TIMEOUT_S:-600}"
  export SCALE_MERGE_TIMEOUT_S="${SCALE_MERGE_TIMEOUT_S:-600}"
fi

submitted=()
if [[ "$SCALE_ENGINE" == "hit" || "$SCALE_ENGINE" == "both" ]]; then
  jid_hit=$(sbatch --parsable --export=ALL,SCALE_MODE="$SCALE_MODE",SCALE_HIT_TIMEOUT_S="${SCALE_HIT_TIMEOUT_S:-120}" \
    "$CCAD/job_scale_hit.sbatch")
  echo "$jid_hit" > "${HOME}/jobs/scale_hit/jobid.txt"
  submitted+=("HIT=$jid_hit")
fi
if [[ "$SCALE_ENGINE" == "merge" || "$SCALE_ENGINE" == "both" ]]; then
  jid_merge=$(sbatch --parsable --export=ALL,SCALE_MODE="$SCALE_MODE",SCALE_MERGE_TIMEOUT_S="${SCALE_MERGE_TIMEOUT_S:-120}" \
    "$CCAD/job_scale_merge.sbatch")
  echo "$jid_merge" > "${HOME}/jobs/scale_merge/jobid.txt"
  submitted+=("MERGE=$jid_merge")
fi
echo "SUBMITTED scale ${submitted[*]} MODE=$SCALE_MODE ENGINE=$SCALE_ENGINE"
squeue -u "${USER}"

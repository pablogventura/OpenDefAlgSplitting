#!/usr/bin/env bash
# Sync repo a Serafin (sin target/) y lanzar campaña paper_experiment.
set -euo pipefail
LOCAL_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
REMOTE="pventura@serafin.ccad.unc.edu.ar"
REMOTE_ROOT='~/src/OpenDefAlgSplitting'

echo "=== rsync (no target/, no .git/) ==="
rsync -az --delete \
  --exclude 'target/' --exclude '.git/' \
  -e 'ssh -o BatchMode=yes' \
  "$LOCAL_ROOT/" \
  "${REMOTE}:${REMOTE_ROOT}/"

echo "=== submit on login ==="
ssh -o BatchMode=yes "$REMOTE" "bash ${REMOTE_ROOT}/scripts/ccad/submit_paper_experiment.sh"

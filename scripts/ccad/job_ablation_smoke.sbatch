#!/bin/bash
#SBATCH --job-name=hit-ablation-smoke
#SBATCH --partition=short
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=64
#SBATCH --mem=0
#SBATCH --time=01:00:00
#SBATCH --output=/home/pventura/jobs/ablation_smoke/slurm-%j.out
#SBATCH --error=/home/pventura/jobs/ablation_smoke/slurm-%j.err
#SBATCH --mail-type=END,FAIL
#SBATCH --mail-user=pablogventura@gmail.com

set -euo pipefail
JOBDIR=/home/pventura/jobs/ablation_smoke
mkdir -p "$JOBDIR/out"
# shellcheck source=/dev/null
source /home/pventura/src/OpenDefAlgSplitting/scripts/ccad/common.env.sh
ROOT=/home/pventura/src/OpenDefAlgSplitting

ccad_header | tee "$JOBDIR/header.txt"
cd "$ROOT"
rm -f "$JOBDIR/DONE"
ccad_require_binary "$JOBDIR"

if ! bash scripts/run_ablation.sh 2>&1 | tee "$JOBDIR/ablation.log"; then
  ccad_write_done ABLATION_FAIL "$JOBDIR" "See $JOBDIR/ablation.log"
  exit 1
fi

cp -a docs/ablation/results.csv "$JOBDIR/out/" 2>/dev/null || true
cp -a docs/ablation/class_smoke.txt "$JOBDIR/out/" 2>/dev/null || true
ccad_write_done OK "$JOBDIR" \
  "RESULTS=$JOBDIR/out/results.csv" \
  "$(wc -l < "$JOBDIR/out/results.csv" 2>/dev/null || echo 0) lines in results.csv"

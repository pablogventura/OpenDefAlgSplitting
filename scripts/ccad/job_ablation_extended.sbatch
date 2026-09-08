#!/bin/bash
#SBATCH --job-name=hit-ablation-ext
#SBATCH --partition=multi
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=64
#SBATCH --mem=0
#SBATCH --time=02:00:00
#SBATCH --output=/home/pventura/jobs/ablation_extended/slurm-%j.out
#SBATCH --error=/home/pventura/jobs/ablation_extended/slurm-%j.err
#SBATCH --mail-type=END,FAIL
#SBATCH --mail-user=pablogventura@gmail.com

set -euo pipefail
JOBDIR=/home/pventura/jobs/ablation_extended
mkdir -p "$JOBDIR/out"
# shellcheck source=/dev/null
source /home/pventura/src/OpenDefAlgSplitting/scripts/ccad/common.env.sh
ROOT=/home/pventura/src/OpenDefAlgSplitting

ccad_header | tee "$JOBDIR/header.txt"
cd "$ROOT"
rm -f "$JOBDIR/DONE"
ccad_require_binary "$JOBDIR"

if ! bash scripts/run_ablation_extended.sh 2>&1 | tee "$JOBDIR/ablation.log"; then
  ccad_write_done ABLATION_FAIL "$JOBDIR" "See $JOBDIR/ablation.log"
  exit 1
fi

cp -a docs/ablation/results_extended.csv "$JOBDIR/out/" 2>/dev/null || true
cp -a docs/ablation/extended_run.log "$JOBDIR/out/" 2>/dev/null || true
ccad_write_done OK "$JOBDIR" \
  "RESULTS=$JOBDIR/out/results_extended.csv" \
  "$(wc -l < "$JOBDIR/out/results_extended.csv" 2>/dev/null || echo 0) lines"

#!/bin/bash
#SBATCH --job-name=py-hit-n32k2
#SBATCH --partition=multi
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=8
#SBATCH --mem=0
#SBATCH --time=02:00:00
#SBATCH --output=/home/pventura/jobs/python_hit_n32k2/slurm-%j.out
#SBATCH --error=/home/pventura/jobs/python_hit_n32k2/slurm-%j.err
#SBATCH --mail-type=END,FAIL
#SBATCH --mail-user=pablogventura@gmail.com

# Pure-Python fopy HIT on hard n32_k2 pilot models (compare vs Rust TIMEOUT).

set -euo pipefail
JOBDIR=/home/pventura/jobs/python_hit_n32k2
mkdir -p "$JOBDIR/out"
ROOT=/home/pventura/src/OpenDefAlgSplitting
FOPY_SRC="${FOPY_SRC:-/home/pventura/src/fopy/src}"
export FOPY_HIT_BACKEND=python
export FOPY_SRC
export PYTHON_HIT_CORPUS="$ROOT/benches/scale/corpus/pilot"
export PYTHON_HIT_OUT="$JOBDIR/out/python_hit_n32k2.csv"
export PYTHON_HIT_MODELS="${PYTHON_HIT_MODELS:-random_magma_n32_k2_definable_eq_i0.model,random_magma_n32_k2_nondef_mix_i2.model,random_magma_n32_k2_random_half_i0.model}"
export PYTHONUNBUFFERED=1

{
  echo "host=$(hostname)"
  date -Is
  free -h
  echo "FOPY_SRC=$FOPY_SRC"
  python3 --version
  test -d "$FOPY_SRC"
  test -f "$ROOT/scripts/ccad/bench_python_hit_n32k2.py"
} | tee "$JOBDIR/header.txt"

cd "$ROOT"
rm -f "$JOBDIR/DONE"
python3 scripts/ccad/bench_python_hit_n32k2.py
status=$?
cp -a "$PYTHON_HIT_OUT" "$ROOT/benches/scale/results/" 2>/dev/null || true
if [[ "$status" -eq 0 ]]; then
  echo "OK $(date -Is)" > "$JOBDIR/DONE"
  cat "$PYTHON_HIT_OUT" >> "$JOBDIR/DONE"
else
  echo "FAIL rc=$status $(date -Is)" > "$JOBDIR/DONE"
fi
exit "$status"

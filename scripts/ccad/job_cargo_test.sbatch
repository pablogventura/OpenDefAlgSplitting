#!/bin/bash
#SBATCH --job-name=hit-cargo-test
#SBATCH --partition=short
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=64
#SBATCH --mem=0
#SBATCH --time=01:00:00
#SBATCH --output=/home/pventura/jobs/cargo_test/slurm-%j.out
#SBATCH --error=/home/pventura/jobs/cargo_test/slurm-%j.err
#SBATCH --mail-type=END,FAIL
#SBATCH --mail-user=pablogventura@gmail.com

set -euo pipefail
JOBDIR=/home/pventura/jobs/cargo_test
mkdir -p "$JOBDIR"
# shellcheck source=/dev/null
source /home/pventura/src/OpenDefAlgSplitting/scripts/ccad/common.env.sh
ROOT=/home/pventura/src/OpenDefAlgSplitting

ccad_header | tee "$JOBDIR/header.txt"
cd "$ROOT"
rm -f "$JOBDIR/DONE"

# Tests must be compiled on login (prepare_tests.sh or cargo test --release --no-run).
if ! cargo test --release 2>&1 | tee "$JOBDIR/test.log"; then
  ccad_write_done TEST_FAIL "$JOBDIR" \
    "Run on LOGIN first: bash ${ROOT}/scripts/ccad/prepare_tests.sh" \
    "See $JOBDIR/test.log"
  ccad_write_done TEST_FAIL "$JOBDIR" "See $JOBDIR/test.log"
  exit 1
fi

ccad_write_done OK "$JOBDIR" "cargo test --release passed" "LOG=$JOBDIR/test.log"

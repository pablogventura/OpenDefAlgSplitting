# Sourced by ccad sbatch bodies. Paths on Serafin.
export PATH="${HOME}/.cargo/bin:${PATH}"
export CARGO_HOME="${HOME}/.cargo"
export RUSTUP_HOME="${HOME}/.rustup"
export RUST_BACKTRACE=1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-16}"
# HIT caps Rayon via HitConfig.max_rayon_children (default 1). Keep a modest
# pool for other engines; do not rely on this alone for HIT memory safety.
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-8}"

ROOT="${ROOT:-${HOME}/src/OpenDefAlgSplitting}"
MAIL_TO="${MAIL_TO:-pablogventura@gmail.com}"

ccad_write_done() {
  local status="$1"
  local jobdir="$2"
  {
    echo "STATUS=${status}"
    echo "HOST=$(hostname)"
    echo "ENDED=$(date -Is)"
    echo "SLURM_JOB_ID=${SLURM_JOB_ID:-}"
    echo "ROOT=${ROOT}"
    shift 2
    printf '%s\n' "$@"
  } > "${jobdir}/DONE"
  if command -v mail >/dev/null 2>&1; then
    mail -s "[serafin] ${SLURM_JOB_NAME:-job} ${status} ${SLURM_JOB_ID:-?}" "${MAIL_TO}" < "${jobdir}/DONE" || true
  fi
}

ccad_header() {
  hostname
  date -Is
  free -h | head -2
  nproc
  if [[ -x "${ROOT}/target/release/opendefalgsplitting" ]]; then
    ls -lh "${ROOT}/target/release/opendefalgsplitting"
  else
    echo "BINARY=missing"
  fi
}

# Compute nodes have no crates.io; binary must exist before sbatch.
ccad_require_binary() {
  local jobdir="$1"
  local bin="${ROOT}/target/release/opendefalgsplitting"
  if [[ -x "$bin" ]]; then
    return 0
  fi
  ccad_write_done BINARY_MISSING "$jobdir" \
    "Run on LOGIN: bash ${ROOT}/scripts/ccad/prepare_release.sh" \
    "Do not rsync target/release from laptop (GLIBC mismatch on compute nodes)."
  exit 1
}

# Compute nodes often lack python3 on PATH; login has /usr/bin/python3.
ccad_python3() {
  if command -v python3 >/dev/null 2>&1; then
    command -v python3
    return 0
  fi
  if [[ -x /usr/bin/python3 ]]; then
    echo /usr/bin/python3
    return 0
  fi
  return 1
}

ccad_analyze_ablation_csv() {
  local csv="$1"
  local out_txt="$2"
  local py
  if ! py=$(ccad_python3); then
    echo "WARN: python3 not found; CSV complete at ${csv}; run analyze on login." >&2
    return 0
  fi
  "$py" "${ROOT}/scripts/analyze_ablation.py" "$csv" | tee "$out_txt"
}

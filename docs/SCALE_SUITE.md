# Scale suite (Rust HIT vs megahit merge)

Public, regenerable benchmark for QF definability scaling. Not the historical
Python thesis corpus.

## Design principles

- **Reproducible:** master seed + [`manifest.json`](../benches/scale/manifest.json).
- **Stratified families:** random magma, cyclic groups, chains (unary-ish), boolean lattices when size fits.
- **Paired engines:** same `(algebra, R)` for HIT and megahit merge.
- **Controlled targets:** definable (extension of a small QF formula) and non-definable (breaks IsoType purity).
- **Primary metrics:** verdict, wall-clock ms, timeout/OOM as explicit rows.
- **Size ladder:** `|U| in {4,8,16,32,64,128}`, `k in {1,2,3}` with skip when `|U|^k` exceeds megahit cap (200000).

## Density

- Pilot: `n_per_cell = 5` (subset of the same seeds).
- Full: `n_per_cell = 30`, report median.

## Regenerate corpus

```bash
python3 scripts/generate_scale_corpus.py --manifest benches/scale/manifest.json --out benches/scale/corpus
```

## Run locally (small)

```bash
cargo build --release
./target/release/opendefalgsplitting --bench --repeat 3 --fragment qf --engine hit \
  benches/scale/corpus/pilot/*.model
./target/release/opendefalgsplitting --bench --repeat 3 --fragment qf --engine merge \
  benches/scale/corpus/pilot/*.model
```

## Run locally (full)

```bash
cargo build --release
bash scripts/run_hit_full_local.sh      # -> results_hit_full.csv
bash scripts/run_merge_full_local.sh    # -> results_merge_full.csv
```

## Serafin

```bash
# Full campaign (HIT + merge), 48h wall, timeouts up to 600s:
SCALE_MODE=full SCALE_ENGINE=both bash scripts/ccad/sync_and_submit_scale.sh
# Pilot (default):
bash scripts/ccad/sync_and_submit_scale.sh
```

Results: `benches/scale/results/` and `~/jobs/scale_*/`.
Full Serafin jobs: HIT **410027**, MERGE **410028** (COMPLETED 2026-09-07/08;
`SERAFIN_FULL_JOBS.md`). Agree: `verdict_agree_full.csv` (2040/2040).

## Range suite (full target, info-gap)

Worst-case positive family $R=A^2$ on random magmas (matches Theorem info-gap):

```bash
python3 scripts/generate_scale_corpus.py \
  --manifest benches/scale/manifest_range_full.json \
  --out benches/scale/corpus_range
# then --bench HIT/merge -> results_*_range_full.csv
python3 scripts/plot_scale_hit_vs_merge.py \
  --hit benches/scale/results/results_hit_range_full.csv \
  --merge benches/scale/results/results_merge_range_full.csv \
  --out-dir benches/scale/results/range_full_plot \
  --classic --out-name hit_vs_merge_range_full.pdf
```

## Plots

```bash
python3 scripts/plot_scale_hit_vs_merge.py \
  --hit benches/scale/results/results_hit_pilot.csv \
  --merge benches/scale/results/results_merge_pilot.csv \
  --out-dir benches/scale/results/pilot_plot \
  --title-suffix 'Rust pilot, eager IsoType'
python3 scripts/plot_scale_hit_vs_merge.py \
  --hit benches/scale/results/results_hit_full.csv \
  --merge benches/scale/results/results_merge_full.csv \
  --out-dir benches/scale/results/full_plot \
  --title-suffix 'Rust full, eager IsoType'
```

## Engines

| Flag | Implementation |
|------|----------------|
| `--engine hit` (default auto) | HIT splitting |
| `--engine merge` | Megahit IsoType merge (`megahit_merge`), no `|U|<=6` |

Aut-orbit `iso_merge` (`|U|<=6`) remains in `morph.rs` for fragment parity tests; QF `--engine merge` uses megahit.

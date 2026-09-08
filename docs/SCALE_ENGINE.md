# Scale engine notes (HIT vs megahit merge)

## HIT (`--engine hit` / default QF path)

Partition refinement (`hit.rs`). May accept before completing IsoTypes (info gap).
Finished-impure / polarity-impure blocks split by compact `TupleModelHash`
(**eager IsoType by default**; `HIT_EAGER_ISOTYPE=0` restores wait-for-generator).
`HitConfig.max_rayon_children` default 1 (serial). `--bench` sets
`synthesize_formula=false` (verdict only; no huge QF witness DAG).

After equality-pattern preprocessing, the initial block enumerates only
tuples matching that pattern (not all of `A^k`). Otherwise foreign-pattern
rows keep full targets from early-accepting even when `R=A^k`.
## Megahit merge (`--fragment qf --engine merge`)

Implementation: `engines/megahit.rs` + `engines/tuple_model_hash.rs`.

Faithful port of historical [OpenDefAlgMerging](https://github.com/pablogventura/OpenDefAlgMerging)
`isOpenDef`:

- IsoType via `TupleModelHash` (layered HIT + relational fingerprint)
- Injective tuples `A^{(k)}` (star-order permutations)
- Orbit fusion via `propagar` / polarity clash => not definable
- **All `T*` jointly** (CLI and `--bench`), matching the oracle `main.py`
- Cap: `MEGAHIT_MAX_INJECTIVE = 200_000`

After HIT fixes (serial DFS, Arc ops, TMH finished-impure, bench without
formula, compact IsoType / lazy in-R-then-out-R, `HIT_MAX_RSS_MIB`, eager
IsoType on impure): hard pilot `n32_k2` cells finish in ~270 ms locally.
Empty targets (no `pattern`) no longer panic. Unary finished-impure regression
(`random_half` i0 `|a1`) stays DEFINABLE in ms.
fopy routes HIT via `fopy.finite.hit_rust` (`FOPY_HIT_BACKEND=rust`).

TMH speed knobs (`HIT_TMH_*`, `HIT_PRE_FINISHED=approx`): see
[`TMH_FACTORS.md`](TMH_FACTORS.md). Leave off by default; hard fix is eager IsoType.

Vendored oracle: `oracles/OpenDefAlgMerging/` (see `ORACLE.md`).

Parity harness:

```bash
bash scripts/run_megahit_parity.sh
```

Aut-orbit merge (`morph.rs` `check_qf_merge` / fopy `iso_merge`, `|U|<=6`) is a
**different** engine. It is not the megahit / IsoType oracle.

## Bench

`--bench` respects `--fragment` / `--engine` and prints an `engine` column.
For QF+merge it uses the joint multi-target path above.
For QF+hit, `--bench` disables formula synthesis (decision-only).

## Scale results (pilot + full)

- HIT pilot archive Serafin **409904**: `results_hit_pilot_serafin409904.csv`
  (10 `n32_k2` TIMEOUTs before eager IsoType).
- **Local HIT pilot (eager):** `results_hit_pilot.csv` - 240/240, 0 TIMEOUT,
  240/240 agree with `results_merge_pilot.csv` (`verdict_agree.csv`).
  Plot: `hit_vs_merge_pilot.pdf`.
- Merge pilot Serafin **409905**: `results_merge_pilot.csv`.
- Ablation safe **409903**.
- Full corpus: `benches/scale/corpus/full` (2040 models).
  - Authoritative Serafin: HIT **410027**, MERGE **410028** (COMPLETED;
    see `SERAFIN_FULL_JOBS.md`). CSVs: `results_hit_full.csv`,
    `results_merge_full.csv` (archives `*_serafin410027/410028.csv`).
  - 2040/2040, 0 TIMEOUT; **2040/2040** agree (`verdict_agree_full.csv`).
  - Local backup matched all verdicts (`archive_local_before_serafin_pull/`).
  - Plot: `hit_vs_merge_full.pdf` (stratified medians; parity-oriented).
  - Full-target range ($R=A^2$, random magmas): `manifest_range_full.json`,
    `results_*_range_full.csv`, plot `hit_vs_merge_range_full.pdf`
    (info-gap speed comparison; ~100x at `|U|=128`).
- L1 oracle parity: `scripts/run_megahit_parity.sh`.

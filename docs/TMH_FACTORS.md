# TMH factor matrix (HIT finished-impure)

Experimental knobs behind env flags (all **off** by default). Goal: cheaper
compact IsoType / finished-impure without changing verdicts vs merge.

| Flag | Factor | Role |
|------|--------|------|
| `HIT_TMH_CLOSURE_CACHE=1` | A | Reuse compact IsoType by generator tuple |
| `HIT_TMH_STREAM_PRODUCT=1` | E | Product enumeration without materializing Vecs |
| `HIT_TMH_PROXY=1` | B | Cheap inequality filter before full compact TMH |
| `HIT_TMH_RAYON=2` (or 4) | D | Rayon on in-R compact compute |
| `HIT_PRE_FINISHED=approx` | C | Approx early-reject before HIT |
| `HIT_TMH_STATS=1` | - | Dump counters; also `HIT_PROGRESS` while stepping |

Harness: `scripts/bench_tmh_factors.py` (CSV under `benches/scale/results/`).

## Results (local, 2026-09-05/06)

### Small (`n16_k2`, `n32_k1`) - `tmh_factors_round1_small.csv`

- All cells finish; **agree with merge = 1** where measured.
- Most cells never enter TMH (`tmh_computes=0`): HIT resolves by polarity / early splits.
- Factor deltas are noise (same order of ms).

### Micro (unary lazy fixture + `n4`) - `tmh_factors_round3_micro.csv`

- Unary half i0 stays **DEFINABLE** under A/E/B/C/A+E (~18-20 ms).
- No regression vs baseline.

### Hard (`random_magma_n32_k2_random_half_i0`) - rounds 2-3

- **TIMEOUT** under 45-120 s for baseline, C, A+E, A+E+B, D2.
- Peak RSS ~10-15 GiB in 45 s (proc sample); longer runs climb higher.
- `finished_impure=0`, `tmh_computes=0` until kill: time is spent in **HIT stepping**, not compact TMH.
- Progress (`HIT_TMH_STATS=1`): model has unary then binary targets. Unary finishes quickly; binary keeps stepping on small impure blocks (`block_tuples` down to 2) for a long time **without** finishing the generator, so finished-impure/TMH never runs.

## Hard timeout fix (eager IsoType)

Root cause of `n32_k2` TIMEOUT was **not** compact TMH cost: HIT spent wall-clock
in `skip_useless` generator stepping on small impure blocks and never set
`finished`, so finished-impure never ran (`finished_impure=0`).

Fix: `HitConfig.eager_isotype_on_impure` (default **on**). On polarity-impure
blocks, run compact IsoType immediately. Sound because TMH hashes ambient
IsoType of the original tuple `t`, independent of generator state. Disable with
`HIT_EAGER_ISOTYPE=0`.

Local recheck (`hit_eager_hard_recheck.csv`): **10/10** former TIMEOUTs finish
DEFINABLE in ~270 ms, agree with merge.

Full local HIT after eager: **2040/2040**, 0 TIMEOUT (`results_hit_full.csv`).
Serafin full jobs **410027** (HIT) / **410028** (merge) submitted.

## Conclusion (factors vs eager)

1. A/E/B/D/C stay **off by default**; they do not fix hard TIMEOUTs.
2. Hard fix is **eager IsoType** (default on), not TMH micro-opts.
3. Factor matrix above remains historical evidence of the pre-eager bottleneck.

## Reproduce

```bash
cargo build --release
python3 scripts/bench_tmh_factors.py --tier small \
  --factors baseline,A,E,B,D2,C,A+E \
  --out benches/scale/results/tmh_factors_round1_small.csv
HIT_TMH_STATS=1 ./target/release/opendefalgsplitting \
  --bench --repeat 1 --fragment qf --engine hit \
  benches/scale/corpus/pilot/random_magma_n32_k2_random_half_i0.model
```

# Serafin full scale jobs

- HIT: **410027** (`SCALE_MODE=full`, wall 48h, per-model timeout up to 600s)
- MERGE: **410028** (same)

Submitted 2026-09-06 via `SCALE_MODE=full SCALE_ENGINE=both bash scripts/ccad/sync_and_submit_scale.sh`.
Started 2026-09-07T14:39 (after `ReqNodeNotAvail` on rome). Both **COMPLETED** exit 0.

| Job | Host | Ended | Wall | CSV |
|-----|------|-------|------|-----|
| 410027 HIT | rome13 | 2026-09-07T20:36 | ~5h56m | `results_hit_full.csv` / `results_hit_full_serafin410027.csv` |
| 410028 MERGE | rome05 | 2026-09-08T00:08 | ~9h29m | `results_merge_full.csv` / `results_merge_full_serafin410028.csv` |

Verdicts (both engines): 1924 DEFINABLE, 116 NOT_DEFINABLE; **0 TIMEOUT**.
HIT vs merge: **2040/2040** agree (`verdict_agree_full.csv`).
Local backup under `archive_local_before_serafin_pull/` matches all verdicts.
Plot: `hit_vs_merge_full.pdf`.

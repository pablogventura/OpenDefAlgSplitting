# Selector de estrategias HIT

Criterio: **velocidad** con el mismo veredicto DEFINABLE / NOT DEFINABLE.

## Política (`select_strategy`)

| Regla | Efecto |
|-------|--------|
| Sin flags de estrategia | Modo **auto** |
| Backend unario (aridad 1) | Reject sound si mezcla huellas (Lema 4); Accept con φ trivial si ∅/A sin ops; si definible con ops, HIT sintetiza |
| `skip_useless` | Siempre ON en auto (Lema 1) |
| Mezcla de patrón sin ops de aridad > 0 | Rechazo inmediato NOT DEFINABLE (Lema 3a) |
| Chequeo de patrón con ops | No se ejecuta en auto (no decide flags; solo costo) |
| `approx_precheck` | OFF en auto (sin predictor fiable de early-exit; usar `--approx-precheck`) |
| `simplify` | OFF en auto (costo en fórmulas grandes; usar `--simplify`) |

Escape: `--no-auto`, flags manuales (`--skip-useless`, `--approx-precheck`, `-i`, `--bfs`, etc.), `--no-skip-useless`, `--explain-strategy`.

Umbrales documentados (referencia para approx manual): `APPROX_MAX_DOMAIN_TUPLES = 50000`, `APPROX_MAX_OPS = 8`.

Lemas: [STRATEGY_LEMMAS.md](STRATEGY_LEMMAS.md).

## Defaults

- `HitConfig::default().skip_useless_candidates = true`
- CLI sin flags -> auto (en la suite actual equivale a skip)

## Validación

Script: [`scripts/validate_strategy.sh`](../scripts/validate_strategy.sh) (suite segura, sin estres/cadena20).

Resumen ([ablation/strategy_validate_summary.txt](ablation/strategy_validate_summary.txt)):

```
models_compared=18
verdict_mismatches=0
timeout_or_error=0
mean_ms_auto=40.944
mean_ms_skip=41.073
mean_ms_baseline_noskip=47.325
ratio_auto_over_skip=0.9969
accept_auto_le_skip=True
```

Aceptación: mismatch = 0; media auto <= media skip; auto mejor que baseline sin skip.

### Tiempos por modelo (ms)

| Modelo | baseline_noskip | skip | auto | veredicto |
|--------|----------------:|-----:|-----:|-----------|
| 16_T3_4_0.model | 1.81 | 1.29 | 1.31 | NOT_DEFINABLE |
| algebra.model | 0.50 | 0.51 | 0.45 | NOT_DEFINABLE |
| cadena100.model | 46.79 | 33.12 | 33.54 | DEFINABLE |
| cadena4.model | 1.66 | 1.20 | 1.29 | DEFINABLE |
| cadena5.model | 12.78 | 6.74 | 7.03 | DEFINABLE |
| gigante.model | 736.94 | 653.19 | 649.53 | DEFINABLE |
| malvada.model | 0.79 | 0.69 | 0.62 | DEFINABLE |
| miprueba.model | 0.45 | 0.45 | 0.37 | NOT_DEFINABLE |
| modeloqueanda.model | 0.81 | 0.79 | 0.77 | DEFINABLE |
| modelosexperimento.model | 5.41 | 4.97 | 5.10 | DEFINABLE |
| modelosimplequefalla.model | 39.39 | 31.98 | 32.35 | DEFINABLE |
| msimple.model | 0.73 | 0.71 | 0.74 | DEFINABLE |
| p0_d0.0625_a2_u30_q5.model | 0.85 | 0.97 | 1.32 | NOT_DEFINABLE |
| retrombo.model | 0.68 | 0.63 | 0.58 | DEFINABLE |
| retrombo_nodef.model | 0.72 | 0.62 | 0.52 | NOT_DEFINABLE |
| retrombo_nodef_sinpura.model | 0.42 | 0.45 | 0.42 | NOT_DEFINABLE |
| retromboconstantes.model | 0.45 | 0.48 | 0.49 | DEFINABLE |
| suma4.model | 0.66 | 0.52 | 0.58 | NOT_DEFINABLE |

CSV: [ablation/results_strategy_validate.csv](ablation/results_strategy_validate.csv).

## Calibración

1. Encender approx ante `pattern_mix` empeoró el promedio cuando approx no corta (`modelosexperimento`, `modelosimplequefalla`, `gigante`).
2. `simplify` en auto sumaba ~15% en media (dominado por `gigante`).
3. Política final: auto ≈ skip puro + rechazo por patrón solo sin ops.

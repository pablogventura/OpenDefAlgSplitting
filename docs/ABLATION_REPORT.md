# Informe de ablacion: mejoras al algoritmo HIT

Ultima corrida reproducible: **2026-09-04**, CCAD Serafin, Slurm job **409903** (nodo `rome64`).

Binario: `cargo build --release` en login Serafin (`prepare_release.sh`).

Datos: `docs/ablation/results_safe.csv` (18 modelos x 4 variantes, **5** repeticiones, timeout 90 s).

Analisis: `docs/ablation/analysis_safe.txt` (`analyze_ablation.py`).

## Resumen ejecutivo

| Mejora | Resultado (suite safe) |
|--------|-------------------------|
| `--skip-useless` | **17/18** modelos mas rapido en ms (>5%); steps -58% a -89% en casos destacados; **0** mismatches de veredicto |
| `--approx-precheck` | Early-exit en 3 negativos (`approx_reject=1`); 5/18 mas rapido en ms |
| combo (skip+approx+simplify) | 10/18 mas rapido; veredictos iguales al baseline |
| `--ig-experimental` | **No usar** (cambia veredictos; ver smoke `gigante`) |

## Numeros citados en el paper

| Modelo | baseline steps | skip_useless steps |
|--------|----------------|-------------------|
| cadena5 | 287 | 32 |
| gigante | 476 | 200 |
| modeloqueanda | 12 | 4 |
| retrombo | 6 | 2 |
| 16_T3_4_0 | 19 | 2 |

Veredictos: identicos al baseline en las 72 filas del CSV safe.

## Reproducir

En Serafin (login):

```bash
bash scripts/ccad/submit_paper_experiment.sh
bash scripts/ccad/postprocess_on_login.sh   # opcional, requiere python3 en login
```

Desde laptop:

```bash
bash scripts/ccad/sync_and_submit_paper.sh
```

## Nota historica

Corridas locales de agosto 2026 (3 repeticiones, rama `feat/alg-improvements-ablation`) motivaron el diseno; el manuscrito cita la corrida Serafin de septiembre 2026.

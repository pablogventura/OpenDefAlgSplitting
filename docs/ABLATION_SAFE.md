# Ablación segura (completada)

Fecha: 2026-08-23  
Suite: 18 modelos, 4 variantes (`baseline`, `skip_useless`, `approx`, `combo`), 5 repeats, timeout 90s.  
Sin `estres_paralelo` / `cadena20`. RAM estable ~50 GiB disponibles todo el run.

CSV: `docs/ablation/results_safe.csv`  
Análisis: `docs/ablation/analysis_safe.txt`

## Tiempos (hallazgos)

### `--skip-useless`
- Más rápido en **13/18** modelos (>5%).
- Ahorros absolutos relevantes:
  - gigante: 690 → 654 ms (−36 ms)
  - cadena100: 47 → 31 ms (−35%)
  - modelosimplequefalla: 37 → 29 ms (−22%)
  - cadena5: 12.5 → 7.0 ms (−44%)
- Veredictos iguales al baseline.

### `--approx-precheck`
- Solo gana cuando hace early-exit (3/18): `16_T3_4_0` (1.72 → 0.44 ms), `miprueba`, `retrombo_nodef_sinpura`.
- Si no corta, puede ser muy caro: `modelosexperimento` 5.6 → 67 ms (~+1100%), `modelosimplequefalla` 37 → 96 ms.

### `combo` (skip + approx + simplify)
- Gana cuando approx corta o cuando skip domina (6/18).
- En definibles grandes suele ser **peor que solo skip** (gigante 690 → 809 ms) por el costo de approx.

## Recomendación

1. Default útil: **`--skip-useless`**.
2. **`--approx-precheck`** solo como opción agresiva para no-definibles / sospecha de early reject; no en el default.
3. No usar combo a ciegas en instancias grandes definibles.

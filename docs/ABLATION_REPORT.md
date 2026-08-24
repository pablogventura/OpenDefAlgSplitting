# Informe de ablación: mejoras al algoritmo HIT

Fecha: 2026-08-23  
Repo: `OpenDefAlgSplitting` (rama `feat/alg-improvements-ablation`)  
Binario: `cargo build --release`  
Datos: `docs/ablation/results.csv` (media de 3 repeticiones)

## Resumen ejecutivo

| Mejora | ¿Aporta? | Cómo aporta | Riesgo |
|--------|----------|-------------|--------|
| `--skip-useless` | **Sí (fuerte)** | Menos pasos y a menudo menos ms | Bajo: mismos veredictos |
| `--approx-precheck` | **Sí (selectivo)** | Early-exit en no definibles con mezcla de clases ≈ | Bajo si la huella es correcta; overhead si no rechaza |
| `--simplify` | Marginal | No baja ms ni steps en el bench | Bajo |
| `--bfs` | **No / negativo** | Mismos steps; a veces mucho más lento | Bajo (veredicto igual) |
| `-i` (legacy) | **Negativo** | Calcula IG y no lo usa: solo overhead | Bajo |
| `--ig-experimental` | **No usar** | Cambia veredictos (falsos NOT_DEFINABLE) | Alto: falla gate de corrección |
| Combo skip+approx+simplify | **Sí** | Hereda skip; approx acelera algunos NOT_DEFINABLE | Bajo |
| `--class` | Utilidad | Reporta veredictos multi-modelo | No es speedup |
| Constantes 0-arias | Ya estaba | El generador ya emite todas en fase Init/Iter | N/A |
| `max_steps` / stats | Instrumentación | Métricas `steps`/`skipped`/`approx_reject` | N/A |

**Recomendación práctica:** por defecto `baseline` o `--skip-useless`. En instancias sospechosas de no definibles, sumar `--approx-precheck`. No activar `--ig-experimental`. Evitar `-i` legacy (solo costo). BFS no conviene frente al DFS+Rayon actual.

## Metodología

- Variantes: `baseline`, cada flag sola, `combo` = `--skip-useless --approx-precheck --simplify`.
- Smoke: `modeloqueanda`, `suma4`, `retrombo_nodef`, `msimple`.
- Medio: `cadena5`, `gigante`, `16_T3_4_0`.
- Gate IG: si `--ig-experimental` cambia un DEFINABLE de smoke a NOT_DEFINABLE, queda fuera del combo (plan acordado).

## Hallazgos por mejora

### 1. Dedup / skip de candidatos inútiles (`--skip-useless`)

Aporta de verdad. Baja steps de forma consistente y a menudo el tiempo:

| Modelo | baseline ms / steps | skip ms / steps | Delta steps |
|--------|---------------------|-----------------|-------------|
| modeloqueanda | 1.32 / 12 | 0.74 / 4 | -67% |
| retrombo_nodef | 0.69 / 21 | 0.53 / 3 | -86% |
| cadena5 | 12.2 / 287 | 6.9 / 32 | -89% |
| gigante | 720 / 476 | 646 / 200 | -58% |
| 16_T3_4_0 | 2.06 / 11 | 1.24 / 2 | -82% |

Veredictos iguales al baseline. Es la mejora más clara de esta pasada.

### 2. Information gain

- **Legacy `-i`:** calcula IG y sigue el orden `step()`. Solo agrega costo (p.ej. cadena5 12 ms → 51 ms; gigante 720 ms → 1958 ms).
- **`--ig-experimental`:** aplica el mejor candidato + `advance_until`. En smoke/medio **rompe corrección**:
  - `modeloqueanda`, `msimple`, `cadena5`, `gigante`: baseline DEFINABLE → experimental NOT_DEFINABLE.
- Conclusión: el comentario histórico del código era correcto; el orden de splits no es libre si el generador/fork no se sincroniza de forma total. Queda flag experimental documentado, **fuera del combo**.

### 3. Precheck ≈ (`--approx-precheck`)

- En la mayoría de modelos no rechaza (`approx_reject=0`) y agrega un poco de overhead.
- En `16_T3_4_0`: `approx_reject=1`, `steps=0`, ~2.0 ms → ~0.6 ms, mismo NOT_DEFINABLE.
- Aporta cuando la huella detecta mezcla de clase; si no, es costo chico. Tiene sentido en el combo.

### 4. Simplificar fórmula (`--simplify`)

- Sin cambio de steps; ms dentro del ruido.
- `formula_size` en CSV quedó en 0 (el path `--bench` no retiene la fórmula). La simplificación AST está implementada (`Formula::simplify_ast`) pero no se midió tamaño en este harness.
- Aporte de performance: nulo en estos datos. Útil solo si se imprime/guarda la fórmula.

### 5. BFS (`--bfs`)

- Mismos steps que DFS.
- Peor o igual en tiempo (gigante 720 → 1530 ms; cadena5 12 → 18 ms).
- El DFS+Rayon actual gana; BFS no aporta en esta suite.

### 6. Terminación / `max_steps` / stats

- Rust ya no usa fuel; se instrumentaron `steps`, `skipped`, `approx_reject`.
- No hay speedup propio: es observabilidad (y tope de seguridad opcional).

### 7. Modo clase (`--class`)

```
modeloqueanda  DEFINABLE
suma4          NOT_DEFINABLE
CLASS_VERDICT  MIXED
```

Aporta para experimentos multi-modelo / open JALS de clase; no acelera una instancia.

### 8. Constantes 0-arias

El generador ya emite la primera constante en `Init` y el resto en `Iter` con aridad 0. Flag `emit_all_constants` documenta el comportamiento; no hay delta medible vs baseline.

## Combo recomendado

`combo` = skip + approx + simplify:

- Conserva veredictos del baseline en smoke.
- En `cadena5`: 12.2 ms / 287 steps → 10.1 ms / 32 steps.
- En `16_T3_4_0`: early approx (0 steps).
- En `gigante`: steps 476 → 200; ms similar o un poco peor (overhead approx sobre instancia grande definible).

Para instancias grandes definibles, a veces conviene **solo** `--skip-useless` sin approx.

## Limitaciones

1. Suite chica; no incluye `cadena20` / stress por tamaño.
2. `formula_size` no instrumentado en `--bench`.
3. Huella ≈ es subálgebra etiquetada finita: correcta para rechazo cuando mezcla, no es una formalización Lean de ≈.
4. IG experimental sigue abierto: haría falta una sincronización más profunda del generador (o búsqueda completa) antes de confiar en el orden IG.

## Cómo reproducir

```bash
cd OpenDefAlgSplitting
cargo build --release
cargo test --lib
bash scripts/run_ablation.sh
# CSV: docs/ablation/results.csv
```

Flags útiles:

```bash
./target/release/opendefalgsplitting --skip-useless model_examples/cadena5.model
./target/release/opendefalgsplitting --skip-useless --approx-precheck model_examples/16_T3_4_0.model
./target/release/opendefalgsplitting --bench --repeat 3 --ablation-csv --skip-useless model_examples/gigante.model
```

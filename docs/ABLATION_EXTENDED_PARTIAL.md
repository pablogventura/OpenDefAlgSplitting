# Ablación extendida (parcial) + incidente de RAM

## Qué pasó con la RAM

La suite extendida llegó a la sección **LARGE** e incluyó `estres_paralelo.model` (~250 KiB en disco, pero el algoritmo materializa el producto del universo / historias de tuplas). Eso llenó la memoria.

Estado actual: **~50 GiB disponibles**; no queda el proceso de ablación corriendo.

Cambios de seguridad ya aplicados en `scripts/run_ablation_extended.sh`:
- `LARGE=()` y `HUGE=()` (sin `estres_paralelo`, sin `cadena20`)
- No volver a lanzar esos modelos en ablación automática

`cadena100` sí terminó bien (~52 ms baseline) y está en el CSV.

## Datos que sí se completaron

CSV: `docs/ablation/results_extended.csv` (~18 modelos útiles; small+medium + cadena100).

### Tiempos: skip vs baseline

| Modelo | baseline ms | skip ms | Δ |
|--------|-------------|---------|---|
| gigante | 718 | 653 | **−65 ms (−9%)** |
| cadena100 | 52.5 | 31.7 | **−21 ms (−40%)** |
| modelosimplequefalla | 37.6 | 30.5 | **−7 ms (−19%)** |
| cadena5 | 11.7 | 7.0 | **−4.7 ms (−40%)** |
| 16_T3_4_0 | 1.8 | 1.1 | −0.7 ms (−41%) |

**skip_useless** fue >5% más rápido en **14/18** modelos. Sigue siendo el único win estable de wall-clock.

### approx

- Early-exit real en: `16_T3_4_0`, `miprueba`, `retrombo_nodef_sinpura`.
- En el resto suele **sumar** costo.
- Caso malo: `modelosexperimento` ~**+1222%** con approx (overhead enorme sin rechazo). Ahí approx no conviene.

### Otros

- **bfs / -i legacy**: siguen empeorando en varios medios.
- **combo**: a veces gana por skip; en gigante/definibles grandes a menudo pierde vs solo skip.
- Sin mismatches de veredicto en las variantes “seguras”.

## Conclusión (más segura con más datos)

1. **`--skip-useless`**: sí aporta en tiempos, también en cadena100.
2. **`--approx-precheck`**: solo aporta cuando corta; si no corta puede ser caro (experimento).
3. No meter `estres_paralelo` / `cadena20` en benches sin límite de memoria / timeout agresivo.

## Próximo paso sugerido

Repetir solo **baseline + skip + approx + combo** sobre small+medium (sin LARGE), con `timeout` por corrida y chequeo de `MemAvailable` entre modelos. Sin `estres_paralelo`.

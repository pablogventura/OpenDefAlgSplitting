# RAM y pipeline de predictores de dificultad

## Resumen

| Accion | Segura? | Notas |
|--------|---------|-------|
| `bash scripts/run_difficulty_safe.sh` | Si | Python, un modelo a la vez |
| `qfdef/scripts/run-entropy.sh` | Si | Solo binario, sin `lake exe` |
| `qfdef/scripts/build-safe.sh` | Si con RAM | `LEAN_NUM_THREADS=2` |
| `lake exe qfdef-entropy` | **No** | Puede recompilar 2000+ modulos en paralelo |
| `lake build` a secas | Cuidado | Igual; usar `build-safe.sh` |

## Comandos recomendados

```bash
# 1) Metricas + reporte (siempre primero)
cd OpenDefAlgSplitting
bash scripts/run_difficulty_safe.sh

# 2) Cross-check Lean (opcional; RUN_LEAN=1 en el mismo script)
RUN_LEAN=1 bash scripts/run_difficulty_safe.sh

# 3) Solo Lean, manual
cd qfdef
bash scripts/build-safe.sh qfdef-entropy    # una vez
bash scripts/run-entropy.sh
```

## Variables de entorno

```bash
# Menos procesos lean en paralelo (default en build-safe: 2)
export LEAN_NUM_THREADS=2

# Mas estricto si la maquina esta justa
export LEAN_NUM_THREADS=1
export MIN_AVAIL_GIB=10

# Liberar workers del LSP antes de un build grande (cierra un poco el IDE)
export REAP_LEAN=1
```

`LEAN_NUM_THREADS` limita cuantos modulos compila Lake a la vez. Es la palanca
principal hasta que Lake tenga `-j` estable.

## Que llenaba la RAM (causas)

1. **`lake exe` / `lake build` sin tope**: miles de procesos `lean` + mathlib.
2. **`difficulty_metrics.py` viejo**: explosion combinatoria en terminos (corregido).
3. **Workers LSP** (`lean --worker`): cada query MCP puede usar 1-3 GiB; se acumulan.
4. **`qfdef-entropy` con splitting**: ya usa `entropyRowLight` (sin `decideSliceCounted`).

## Limites Python (scripts actuales)

| Parametro | Valor |
|-----------|-------|
| MAX_INJ | 6000 |
| MAX_FP_UNIVERSE | 20 |
| MAX_FP_INJ | 800 |
| MAX_TERMS | 64 |
| cadena100 | excluido del default |

## Checklist antes de un job pesado

```bash
free -h                    # mirar MemAvailable, no solo "libre"
pgrep -c lean              # si hay muchos, esperar o REAP_LEAN=1
# No en paralelo: lake build + cargo build --release + Docker
```

## Si sigue OOM

1. Cerrar pestanas Firefox / otros agentes Cursor.
2. `export LEAN_NUM_THREADS=1` y `MIN_AVAIL_GIB=12`.
3. Compilar solo el exe: `lake build qfdef-entropy` (no `lake build` sin target).
4. No correr validacion Lean en la misma sesion que el IDE esta indexando mathlib.

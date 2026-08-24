# Lemas de estrategia HIT

Enunciados usados por el selector automático. Pruebas informales (no Lean).

## Lema 1 - Skip-useless preserva el veredicto

**Enunciado.** Sea B un bloque del HIT y c = (f, i) un candidato. Si al simular c sobre todas las tuplas de B:

1. todas caen en el mismo índice de resultado, y
2. ninguna agrega un valor nuevo a su historia,

entonces aplicar c no cambia la partición de B ni el conjunto de historias. Por inducción sobre el árbol de búsqueda, el veredicto DEFINABLE / NOT DEFINABLE del HIT con `skip_useless` coincide con el del HIT sin skip.

**Corolario.** Activar `skip_useless_candidates` es seguro para corrección y solo elimina trabajo redundante.

## Lema 2 - Mezcla de huella approx implica no QF-definible

**Enunciado.** Sea ≈_fp la relación "misma huella de subálgebra etiquetada generada por las coordenadas" (implementada en `approx::target_breaks_approx_classes`). Si existen a ∈ T y b ∉ T con a ≈_fp b, entonces T no es unión de clases de ≈_fp. En particular T no es definible por una fórmula cuantificador-libre que solo distingue clases de ≈_fp; el early-exit a NOT DEFINABLE es *sound* respecto de esa relación.

**Nota.** ≈_fp puede ser más fina o más gruesa que la ≈ del paper. El rechazo sigue siendo válido: si T mezcla una clase de ≈_fp, no puede ser QF-definible en el sentido del algoritmo (que no separa pares ≈_fp).

## Lema 3 - Mezcla de patrón de igualdad

Sea Pat(x) el patrón de igualdad de coordenadas de la tupla x (partición de {0..k-1} por x_i = x_j), como en `preprocessing::Pattern`.

### 3a - Sound sin operaciones de aridad > 0

Si el lenguaje no tiene símbolos de función de aridad ≥ 1, las fórmulas QF solo usan igualdades entre variables. Las clases de Pat son exactamente las órbitas relevantes: si un patrón P tiene alguna realización en T y otra fuera de T (sobre el mismo universo), T no es QF-definible. El rechazo inmediato es sound.

### 3b - Heurística con operaciones

Con operaciones, Pat ya no clasifica todas las clases ≈. `pattern_mix` solo se usa como señal de sospecha para decidir si conviene correr `approx_precheck`, no como rechazo sound por sí solo.

## Lema 4 - Backend unario (Castellano / PolyTime)

**Enunciado.** Sea T ⊆ A una relación unaria (aridad 1).

### 4a - Sin operaciones de aridad > 0

T es QF-definible (y positivamente definible) sii T = ∅ o T = A
(`qfDefinable_unary_noFunctions`, `positiveDefinable_unary_noFunctions`).
El selector emite `false` / `true` sin correr HIT.

### 4b - Con operaciones unarias (conteo de huellas)

Si existen a ∈ T, b ∉ T con la misma huella de cierre unario (sobre-aproximación
de ≈), entonces T no es QF-definible (`not_qfDefinable_of_approx_mix`).
Early-exit a NOT DEFINABLE es sound. Si el conteo afirma definible, HIT
sintetiza la fórmula (el backend no inventa φ).

## Lema 5 - Límites del selector

El selector (`select_strategy`) no garantiza tiempo óptimo. Solo:

1. preserva el veredicto respecto del HIT baseline (vía Lemas 1-4),
2. elige flags con umbrales calibrados en la suite segura de ablación.

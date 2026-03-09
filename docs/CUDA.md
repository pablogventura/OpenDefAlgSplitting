# Uso de CUDA en OpenDefAlgSplitting

## ¿Dónde podría ayudar CUDA?

El algoritmo tiene varios bloques de trabajo:

| Parte | Tipo de trabajo | ¿CUDA? | Motivo |
|-------|-----------------|--------|--------|
| **Cálculo de information gain** | Por cada paso: para muchos candidatos `(op, ti)` y muchas tuplas, calcular partición y IG. Muy paralelizable (cada par candidato×tupla es independiente). | **Sí, candidato principal** | Muchos candidatos × muchas tuplas; reducción (conteos por grupo, luego argmax IG) encaja en kernels + reducciones en GPU. |
| Exploración del árbol (ramas) | Ir explorando bloques; ramificación irregular, estructuras complejas (Block, Formula). | No recomendado | Mejor en CPU con rayon; portar a GPU sería muy costoso y el paralelismo ya está en CPU. |
| Varios targets | Un `is_open_def` por target; ya paralelizado con rayon. | Opcional | Si se añade CUDA al IG, cada target seguiría siendo un proceso CPU que podría llamar a la GPU para el IG. |
| Producto cartesiano / lista inicial de tuplas | Una vez al inicio. | No | Coste bajo frente al resto. |

Conclusión: **el único bloque donde CUDA puede compensar de forma clara es el cálculo de information gain** dentro de `Block::step()` (candidatos × tuplas, con reducción por candidato y elección del mejor).

## ¿Cuándo vale la pena?

- **Sí** si tienes modelos “grandes”: `|universe|^arity` (número de tuplas) en el rango de **decenas o cientos de miles** y **muchos candidatos** por paso (p. ej. `--ig-sample 0`). Ahí el coste del kernel y las transferencias se amortiza.
- **Probablemente no** en modelos pequeños (miles de tuplas, pocos candidatos): el coste de copiar datos a la GPU y lanzar kernels puede ser mayor que hacerlo en CPU con rayon.

Para decidir en tu máquina: usa el **modo `--bench`** (y opcionalmente `cargo bench`) para medir tiempos antes y después de una implementación CUDA del IG.

## Cómo sería una implementación CUDA del IG

1. **Datos a la GPU**: lista de tuplas (representación de `(t, in_target)` por tupla), lista de candidatos `(op, ti)` (con tablas de las operaciones para evaluar en GPU).
2. **Kernel**: cada hilo (o bloque) calcula para un `(candidato, tupla)` el grupo resultante (índice tras “simular” el paso) y si la tupla está en target; salida: `(candidato_id, grupo, in_target)`.
3. **Reducción**: por candidato, contar por `(grupo, in_target)` (en GPU con atomicAdd o reducciones por bloques), luego en CPU o en un segundo kernel calcular entropía/IG y argmax.
4. **Interfaz Rust**: usar `cudarc` o `cust` (Rust CUDA) para lanzar kernels; mantener la API actual y, si hay GPU disponible, delegar el bucle de candidatos×tuplas en la GPU; si no, caer al código actual (rayon).

La parte más delicada es **representar las operaciones en la GPU** (cada `op` es una tabla aridad → resultado); hay que aplanar esas tablas y que el kernel pueda evaluar `op(tuple[ti[0]], ..., tuple[ti[k]])` sin traer demasiada lógica a la GPU.

## Cómo medir antes y después

Usa el **modo benchmark** del binario para obtener tiempos reproducibles y comparar antes/después de cambios (p. ej. CUDA):

```bash
# Un modelo, una corrida
./target/release/opendefalgsplitting --bench model_examples/gigante.model

# Varios modelos
./target/release/opendefalgsplitting --bench model_examples/modeloqueanda.model model_examples/suma4.model

# Con information gain (donde tendría sentido CUDA) y varias repeticiones (media ± desv.)
./target/release/opendefalgsplitting --bench --repeat 5 -i --ig-sample 0 model_examples/gigante.model
```

La salida es en formato tabla: `model`, `ms` (o `media ± desv.`), `result` (DEFINABLE / NOT_DEFINABLE). Puedes guardar la salida antes y después de una mejora y comparar los tiempos.

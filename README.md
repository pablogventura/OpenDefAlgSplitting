# OpenDefAlgSplitting

Open Definability checker for finite algebras. Decides whether relations are definable in first-order logic from the operations of the algebra.

## Implementaciones

El proyecto está disponible en **Python** y **Rust**.

### Rust

**Compilar (o usar Make):**
```bash
cargo build --release
# o
make          # mismo que make release
./target/release/opendefalgsplitting your_model.model
```

**Make:** `make` (release), `make test`, `make clean`, `make cuda`, `make windows`, `make linux-static`. Ver `make help`.

**Binarios por CI (GitHub Actions):** al hacer push de un tag (p. ej. `git tag v1.0.0 && git push origin v1.0.0`) se compila el proyecto y se publica un **Release** en la pestaña **Releases** del repo con los binarios adjuntos: Linux (glibc), Linux estático (musl) y Windows (.exe). También siguen disponibles como Artifacts en el run de Actions.

Ejecutar tests:
```bash
cargo test --release --lib
```

**Information gain:** el algoritmo puede elegir el paso (op, ti) maximizando information gain en lugar del orden fijo del generador. Por defecto está desactivado. Opciones del binario:

- `--information-gain` o `-i` — activa information gain.
- `--ig-sample N` — número de candidatos a muestrear (por defecto 20 cuando IG está activo).

Ejemplo:
```bash
./target/release/opendefalgsplitting modelo.model --information-gain --ig-sample 30
# o forma corta:
./target/release/opendefalgsplitting -i modelo.model --ig-sample 30
```

**Paralelización (rayon):** se aprovechan varios núcleos en (1) comprobación de cada relación objetivo en paralelo, (2) cálculo de information gain por candidatos y por tuplas, y (3) exploración en paralelo de las ramas del árbol cuando hay varios hijos.

**CUDA (opcional):** con `cargo build --release --features cuda` el cálculo de information gain (con `-i`) puede ejecutarse en GPU cuando todos los candidatos son operaciones binarias. Si no hay GPU o falla, se usa la ruta CPU. Ver [docs/CUDA.md](docs/CUDA.md).

**Benchmark (medir tiempos):** para comparar antes/después de mejoras (p. ej. ver [docs/CUDA.md](docs/CUDA.md)):
```bash
./target/release/opendefalgsplitting --bench model_examples/modeloqueanda.model model_examples/suma4.model
./target/release/opendefalgsplitting --bench --repeat 5 -i model_examples/gigante.model
```
Salida: tabla `model`, `ms` (o media ± desv. si `--repeat > 1`), `result`.

**Compilar para Windows (desde Linux):** instala el target y mingw, luego:
```bash
rustup target add x86_64-pc-windows-gnu
# Debian/Ubuntu: sudo apt install mingw-w64
make windows
# → target/x86_64-pc-windows-gnu/release/opendefalgsplitting.exe
```
El `.exe` generado con el target **gnu** suele ser autocontenido (no requiere instalar nada en Windows).

**Compilar en Windows (en una PC con Windows):** instala Rust con [rustup](https://rustup.rs/) (rustup-init.exe), abre una terminal (PowerShell o CMD) en la raíz del proyecto y ejecuta:
```cmd
cargo build --release
```
O bien: `build.bat` (hace lo mismo). El ejecutable queda en `target\release\opendefalgsplitting.exe`. Para ejecutarlo:
```cmd
target\release\opendefalgsplitting.exe tu_modelo.model
```
Tests: `cargo test`. Con CUDA: `cargo build --release --features cuda` (necesitas drivers y toolkit CUDA en Windows).

**Compartir el binario:** el binario de `cargo build --release` en Linux enlaza con la glibc del sistema; en otra máquina Linux moderna suele funcionar sin instalar nada. Para **máxima portabilidad** (p. ej. distribuir un solo ejecutable sin dependencias de glibc):
```bash
make linux-static   # requiere: rustup target add x86_64-unknown-linux-musl, musl-tools
# → target/x86_64-unknown-linux-musl/release/opendefalgsplitting
```
Ese binario es estático y se puede copiar a cualquier x86_64 Linux.

### Python

## Requirements

- Python 3.7+
- Dependencies: `termcolor`, `pytest` (install with `pip install -r requirements.txt`)

## Static analysis and formatting

Development dependencies include Ruff (linter + formatter) and Pyright (type checker):

```bash
pip install -r requirements-dev.txt
```

Run the analysis:

```bash
ruff check .          # Lint
ruff format .         # Format code (or --check to only verify)
pyright               # Type checking
pytest testing/ --cov=. --cov-report=term-missing --durations=15   # Tests with coverage and timing
```

Or in one go:

```bash
ruff check . && ruff format --check . && pyright
```

Or run all checks (ruff, format, radon, pyright, tests with coverage):

```bash
./check_all.sh
```

## Code quality analysis

Additional dev dependencies: `radon` (complexity, maintainability) and `pylint` (duplication).

| Tool   | Métrica                | Descripción                                      |
|--------|------------------------|--------------------------------------------------|
| Radon  | Cyclomatic Complexity  | Complejidad ciclomática (McCabe) por función     |
| Radon  | Maintainability Index  | Índice de mantenibilidad (0–100)                 |
| Radon  | Raw metrics            | SLOC, comentarios, LLOC                          |
| Ruff   | C901                   | Funciones con complejidad > 40                   |
| Pylint | R0801                  | Duplicación de código (>3 líneas similares)      |

Run the full quality report:

```bash
./quality_report.sh
```

This reports complexity grades (A=1–5 best, F=51+ worst), maintainability (A=20+, B=10–19, C=0–9), and code duplication.

To generate an HTML coverage report:

```bash
pytest testing/ --cov=. --cov-report=html
# Open htmlcov/index.html in a browser
```

## Usage

To run a definability check:

```bash
python3 main.py your_model.model
```

Where `your_model.model` is a file containing your algebra. All relations whose names start with "T" are checked by Open Definability.

The output will be the answer to definability and, if not definable, the counterexample.

## Format of model files

Line comments must start with "#". Empty lines are ignored. Example:

```# This file contains the evil model```

The first line should contain each element of universe separated by a space. Example:

```0 1 2```

A relation should start with a declaration line with the name, number of tuples and arity separated by one space. Next lines should be one for each tuple in the relation containing the relation tuple separated by a space. Example:

```
E 2 3
0 1 2
2 1 0
```

An operation should start with a declaration line with the name and arity separated by one space. Next lines should be one for each tuple in the relation containing a tuple for the graph relation of the operation separated by a space. Example:

```
+ 2
0 0 0
0 1 1
0 2 2
1 0 1
1 1 2
1 2 0
2 0 2
2 1 0
2 2 1
```

A constant should start with a declaration line with the name and 0 (will be a 0-arity operation) separated by one space. Next line should be the element. Example:

```
Zero 0
0
```

## Generating model examples

Use the unified interface to generate example models:

```bash
cd testing/generadores
python3 genera_modelos.py --help
```

Models must include a target relation (name starting with "T") for definability checking.

**Important:** The `--target` option adds a *random* target. Random targets are typically **not definable** (the algorithm will find a counterexample). To test **definable** cases, see [Generating definable targets](#generating-definable-targets) below.

### Global options

- **`-o ARCHIVO`**: save output to file (default: stdout)
- **`--target ARIDAD DENSIDAD`**: add random target relation T0 (arity and density between 0 and 1)

### Model types (detailed)

#### `boole` — Boolean algebra

Complete Boolean algebra with 2^n elements. The universe is {0, 1, …, 2^n − 1} with meet (∧) and join (∨) as binary operations. Every finite Boolean algebra is isomorphic to a power set algebra; this generates the algebra on 2^n elements.

**Parameters:**
- `n` (int): exponent such that the algebra has 2^n elements

**Operations:** `m` (meet, binary), `j` (join, binary)

**Example:** `genera_modelos.py boole 3 -o boole8.model` → Boolean algebra with 8 elements

---

#### `aleatorio` — Random algebra

Algebra with arbitrary operations whose outputs are chosen uniformly at random. Useful for stress testing or exploring “generic” structures without algebraic laws.

**Parameters:**
- `cardinalidad` (int): size of the universe
- `--subs` (int, default 0): number of subuniverses (for advanced use)
- `--tam-subs` (int, default 0): size of subuniverses
- `--aridades` (list, default [2]): arities of operations, e.g. `2 2` for two binary operations (f0, f1)

**Operations:** f0, f1, … for each arity in the list

**Example:** `genera_modelos.py aleatorio 7 --aridades 2 2 --target 2 0.1 -o random.model`

---

#### `grupo-abeliano` — Abelian group (product of cyclic groups)

Finite abelian group as a direct product of cyclic groups Z_{n₁} × Z_{n₂} × ⋯. Every finite abelian group is isomorphic to such a product.

**Parameters:**
- `ordenes` (list of ints): orders of the cyclic factors, e.g. `2 2 2` for Z₂×Z₂×Z₂

**Operations:** `Sum` (binary), `Neg` (unary), `Zero` (constant)

**Example:** `genera_modelos.py grupo-abeliano 2 2 2 --target 2 0.5 -o z2z2z2.model` → Z₂×Z₂×Z₂ (8 elements)

---

#### `grupo-abeliano-diverso` — Diverse abelian group

Abelian group of size 2^k, built by randomly decomposing k as a sum of positive integers and taking the product of cyclic groups of those sizes. Produces more varied structures than a fixed decomposition.

**Parameters:**
- `k` (int): size of the group is 2^k

**Operations:** `Sum` (binary), `Neg` (unary), `Zero` (constant)

**Example:** `genera_modelos.py grupo-abeliano-diverso 4 --target 2 0.3 -o grupo16.model`

---

#### `grupo-no-abeliano` — Non-abelian group (permutation group)

Subgroup of the symmetric group S_k on k symbols. The universe is a subset of all k-permutations, closed under composition and inverse. Includes non-abelian groups and symmetric groups.

**Parameters:**
- `k` (int): works in S_k (k-permutations)
- `generadores` (int): number of random generators to start with
- `--cardinalidad-exacta` (int, optional): desired subgroup size; the generator retries until this size is reached

**Operations:** `Id` (identity, constant), `O` (composition, binary), `I` (inverse, unary)

**Example:** `genera_modelos.py grupo-no-abeliano 3 2 --target 2 0.2 -o s3.model`

---

#### `reticulado` — Distributive lattice

Distributive lattice as a sublattice of a Boolean algebra. Takes a random subset of a Boolean algebra and closes it under meet and join. Every finite distributive lattice embeds into a Boolean algebra.

**Parameters:**
- `ancho` (int): ambient Boolean algebra has 2^ancho elements
- `muestra` (int): number of random elements used to generate the sublattice

**Operations:** `m` (meet, binary), `j` (join, binary)

**Example:** `genera_modelos.py reticulado 3 5 --target 2 0.2 -o ret.model`

---

#### `target` — Target relation only (no algebra)

Generates only a random target relation T0 over a universe of given size. No operations. Useful for testing or combining with hand-written algebras.

**Parameters:**
- `cardinalidad` (int): universe size
- `aridad` (int): arity of the target relation
- `densidad` (float, 0–1): fraction of possible tuples that belong to T0

**Example:** `genera_modelos.py target 10 2 0.3 -o solo_target.model`

---

## Generating definable targets

Targets added with `--target` are random and usually **not definable**. To generate targets that *are* definable (for testing definable cases or difficult definable examples), use **`formulaaleatoria.py`**.

This script takes a model on stdin, appends a target T0 defined by a random first-order formula built from the algebra's operations, and prints the complete model. The formula is guaranteed to define T0, so the algorithm must "rediscover" it—deeper formulas yield harder instances.

**Usage:**
```bash
cd testing/generadores
python3 genera_alg_random.py 8 0 0 "[2]" | python3 formulaaleatoria.py 2 '{"f0":2}' > definible.model
```

**Parameters:**
- `formulaaleatoria.py ARIDAD SIM` — arity of T0 and symbol dictionary (operation names → arity; must match the model)
- Symbol dicts by algebra type (the value is the **arity** of each operation; if wrong, the parser raises "Arity not correct"):
  - **aleatorio** (one binary f0): `'{"f0":2}'`
  - **aleatorio** (f0, f1 both binary): `'{"f0":2,"f1":2}'`
  - **boole**: `'{"m":2,"j":2}'`
  - **grupo-abeliano-diverso**: `'{"Sum":2,"Neg":1,"Zero":0}'`

**Examples:**
```bash
# Definable target on random algebra (f0 binary)
python3 genera_alg_random.py 10 0 0 "[2]" | python3 formulaaleatoria.py 2 '{"f0":2}' > def_aleatorio.model

# Definable target on Boolean algebra
python3 genera_boole.py 3 | python3 formulaaleatoria.py 2 '{"m":2,"j":2}' > def_boole.model

# Definable target on abelian group
python3 genera_grupo_abeliano_diverso.py 4 | python3 formulaaleatoria.py 2 '{"Sum":2,"Neg":1,"Zero":0}' > def_grupo.model
```

The formula depth and number of subformulas are fixed in `formulaaleatoria.py` (default: depth 3, 2 subformulas). Editing these values yields harder definable instances.

**Difficult definable example** (larger universe, two operations, arity 3):
```bash
python3 genera_alg_random.py 12 0 0 "[2,2]" | python3 formulaaleatoria.py 3 '{"f0":2,"f1":2}' > modelo_dificil_definible.model
```

---

## Defining relations by formula in model files

The parser supports declaring a relation T by a formula instead of listing tuples. Format:

```
T0(x,y,z) eq(term1,term2) | -eq(term3,term4) & ...
```

- Variables: `x`, `y`, `z`, etc.
- Use `eq(a,b)` for equality (not `==`).
- Use operations from the algebra as function symbols (e.g. `m`, `j` for meet/join; `f0`, `f1` for aleatorio).
- Connect with `&`, `|`, `-` for conjunction, disjunction, negation.

Example (see `model_examples/modeloqueanda.model`):
```
T0(x,y,z) -eq(z,y) & -eq(z,x) & -eq(y,x) & eq(y,f0(z, f0(x, y)))
```

---

## Other generators and batch processing

- **`agregaformula.py`** — batch: adds formula-defined targets to gzipped `.modelwt` models (alg_random_wt, boole_wt, grupo_abeliano_diverso_wt). Run from `testing/generadores`.
- **`formulaaleatoria.py`** — core script for definable targets; used by agregaformula and can be piped directly.

---

## Running tests

To run the definability test suite (parser, definable models, non-definable models, edge cases):

```bash
pip install -r requirements.txt
pytest testing/tests_definibilidad/ -v
```

Or with a virtual environment:

```bash
python -m venv .venv
.venv/bin/pip install -r requirements.txt
.venv/bin/pytest testing/tests_definibilidad/ -v
```

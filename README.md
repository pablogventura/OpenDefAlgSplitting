# OpenDefAlgSplitting

Open Definability checker for finite algebras. Decides whether relations are definable in first-order logic from the operations of the algebra.

## Requirements

- Python 3.7+
- Dependencies: `termcolor` (install with `pip install termcolor`)

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

Models must include a target relation (name starting with "T") for definability checking. Use `--target ARIDAD DENSIDAD` when generating models.

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

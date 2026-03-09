# OpenDefAlgSplitting

Open definability checker for finite algebras. It decides whether relations are definable in first-order logic from the operations of the algebra.

**This repository (default branch) contains the Rust implementation.** The original **Python** implementation is on the **[python](https://github.com/pablogventura/OpenDefAlgSplitting/tree/python)** branch.

---

## Table of contents

- [Quick start](#quick-start)
- [Installation](#installation)
- [Usage](#usage)
- [Command-line options](#command-line-options)
- [Model file format](#model-file-format)
- [Building from source](#building-from-source)
- [Releases and pre-built binaries](#releases-and-pre-built-binaries)
- [Tests](#tests)
- [Optional features](#optional-features)
- [Python version](#python-version)

---

## Quick start

1. Get a pre-built binary from [Releases](https://github.com/pablogventura/OpenDefAlgSplitting/releases), or build from source (see [Installation](#installation)).
2. Run the checker on a model file:
   ```bash
   ./opendefalgsplitting your_model.model
   ```
   On Windows: `opendefalgsplitting.exe your_model.model`
3. The program prints **DEFINABLE** or **NOT DEFINABLE** (and a counterexample when not definable). All relations whose names start with `T` in the model are checked.

---

## Installation

### Option A: Download a release

Go to [Releases](https://github.com/pablogventura/OpenDefAlgSplitting/releases), pick a version, and download the asset for your platform:

- **Linux (glibc):** `opendefalgsplitting-<version>-linux-x86_64`
- **Linux (static, portable):** `opendefalgsplitting-<version>-linux-x86_64-musl`
- **Windows:** `opendefalgsplitting-<version>-windows-x86_64.exe`

Make the Linux binary executable: `chmod +x opendefalgsplitting-*-linux-x86_64`.

### Option B: Build from source

You need [Rust](https://rustup.rs/) (install via `rustup`).

```bash
git clone https://github.com/pablogventura/OpenDefAlgSplitting.git
cd OpenDefAlgSplitting
cargo build --release
```

The executable is `target/release/opendefalgsplitting` (Linux/macOS) or `target\release\opendefalgsplitting.exe` (Windows). See [Building from source](#building-from-source) for cross-compilation and optional features.

---

## Usage

```bash
opendefalgsplitting [OPTIONS] [MODEL_FILE]
```

- **MODEL_FILE:** path to a model file (see [Model file format](#model-file-format)). If omitted, the program reads the model from stdin.
- **OPTIONS:** see [Command-line options](#command-line-options).

**Examples:**

```bash
./opendefalgsplitting model_examples/modeloqueanda.model
./opendefalgsplitting -i --ig-sample 20 model_examples/gigante.model
./opendefalgsplitting --bench --repeat 5 model_examples/modeloqueanda.model
```

Output: for each target relation (names starting with `T`), the program prints whether it is definable and, if not, a counterexample.

---

## Command-line options

| Option | Short | Description |
|--------|--------|-------------|
| `--help` | `-h` | Print help and exit. |
| `--information-gain` | `-i` | **Experimental, not recommended.** Enable information-gain-based candidate sampling (see [Optional features](#optional-features)). |
| `--ig-sample N` | | When `-i` is set, sample up to N candidates per step (default: 20). Use a large value or omit for no limit. |
| `--bench` | | Benchmark mode: run the checker on the given model(s) and print timing (and result). |
| `--repeat N` | | In `--bench` mode, run N times and print mean ± std deviation. |

**Examples:**

```bash
./opendefalgsplitting -h
./opendefalgsplitting -i modelo.model --ig-sample 30
./opendefalgsplitting --bench model_examples/modeloqueanda.model model_examples/suma4.model
./opendefalgsplitting --bench --repeat 5 -i model_examples/gigante.model
```

---

## Model file format

- **Comments:** lines starting with `#` are ignored. Empty lines are ignored.
- **Universe:** first non-comment line is a space-separated list of elements (e.g. `0 1 2`).
- **Relations:** declaration line `NAME N_TUPLES ARITY`, then one line per tuple (space-separated). Relations whose name starts with `T` are treated as **targets** and checked for definability.
- **Operations:** declaration line `NAME ARITY`, then one line per entry in the graph of the operation (inputs and output, space-separated). For a binary op, each line is `a b result`.
- **Constants:** declaration line `NAME 0`, then one line with the element.

**Example (relation):**
```
E 2 3
0 1 2
2 1 0
```

**Example (binary operation):**
```
+ 2
0 0 0
0 1 1
1 0 1
1 1 0
```

**Defining a target by formula:** you can declare a target relation by a first-order formula instead of listing tuples. Syntax:

```
T0(x,y,z) eq(term1,term2) | -eq(term3,term4) & ...
```

Use `eq(a,b)` for equality; use algebra operations as function symbols; connect with `&`, `|`, `-` (and, or, not). Example: `T0(x,y) eq(x,y)` defines the diagonal.

See `model_examples/` in the repository for full examples.

---

## Building from source

### Linux / macOS

```bash
cargo build --release
# Binary: target/release/opendefalgsplitting
```

**Using Make (if available):**

```bash
make          # same as make release
make test     # run tests
make clean    # remove target/
make help     # list all targets
```

**Static Linux binary (max portability, no glibc dependency):**

```bash
rustup target add x86_64-unknown-linux-musl
# On Debian/Ubuntu: sudo apt install musl-tools
cargo build --release --target x86_64-unknown-linux-musl
# Binary: target/x86_64-unknown-linux-musl/release/opendefalgsplitting
```

Or: `make linux-static` (after installing the musl target and musl-tools).

### Windows (native)

1. Install [Rust](https://rustup.rs/) (run `rustup-init.exe`).
2. In a terminal (PowerShell or CMD) at the project root:
   ```cmd
   cargo build --release
   ```
   Or run `build.bat`. The executable is `target\release\opendefalgsplitting.exe`.

### Cross-compile to Windows from Linux

```bash
rustup target add x86_64-pc-windows-gnu
# Debian/Ubuntu: sudo apt install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
# Binary: target/x86_64-pc-windows-gnu/release/opendefalgsplitting.exe
```

Or: `make windows`.

---

## Releases and pre-built binaries

When a [tag](https://github.com/pablogventura/OpenDefAlgSplitting/tags) is pushed (e.g. `v1.0.0`), GitHub Actions build the project and attach the binaries to a **Release**. You can download them from the [Releases](https://github.com/pablogventura/OpenDefAlgSplitting/releases) page. The same run also publishes the build outputs as **Artifacts** in the Actions tab.

---

## Tests

```bash
cargo test
```

Runs unit and integration tests (parser, algorithm, definable/non-definable model checks). For integration tests that run the built binary: `cargo test` from the repo root (they use the debug binary by default).

---

## Optional features

### Information gain (`-i`, `--ig-sample`) — *experimental, not recommended*

This feature is **experimental** and **not recommended** for normal use. The algorithm can use information gain to rank candidates; by default it uses a fixed generator order so that results are deterministic and correct. The options `--information-gain` / `-i` and `--ig-sample N` enable and tune sampling (e.g. for benchmarking or future heuristics). The current implementation keeps the same exploration order as without `-i`, so the result does not change. Prefer running without `-i` unless you have a specific reason (e.g. benchmarking).

### CUDA (GPU)

If you build with the `cuda` feature, the information-gain computation (when using `-i`) can run on the GPU when all candidates are binary operations. If no GPU is available or the data does not qualify, the CPU path is used.

```bash
cargo build --release --features cuda
```

Requires NVIDIA drivers and CUDA toolkit. See [docs/CUDA.md](docs/CUDA.md) for details.

### Benchmarking (`--bench`, `--repeat`)

To measure runtimes:

```bash
./opendefalgsplitting --bench model_examples/modeloqueanda.model
./opendefalgsplitting --bench --repeat 5 -i model_examples/gigante.model
```

Output: table with `model`, `ms` (or mean ± std if `--repeat > 1`), and `result` (DEFINABLE / NOT_DEFINABLE).

### Parallelism

The Rust implementation uses [rayon](https://github.com/rayon-rs/rayon) to parallelize: (1) checking multiple target relations, (2) information-gain computation over candidates and tuples, and (3) exploring multiple branches of the algorithm tree when there are several children.

---

## Python version

The original implementation and the full Python documentation (model generators, quality tools, test suite) live on the **[python](https://github.com/pablogventura/OpenDefAlgSplitting/tree/python)** branch. That branch includes:

- Python 3.7+ and dependencies (`termcolor`, `pytest`)
- Model generators (Boolean, random, abelian groups, lattices, etc.) and definable-target generation
- Static analysis (Ruff, Pyright), formatting, and code quality scripts
- Detailed usage and model file format for the Python checker

Switch to that branch or open it on GitHub to work with the Python version.

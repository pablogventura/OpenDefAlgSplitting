# Experimentos en CCAD Serafin (Slurm)

Cluster recomendado para **todas** las pruebas y ablaciones de este repo:
[serafin.ccad.unc.edu.ar](https://wiki.ccad.unc.edu.ar/).

No correr benches pesados en el nodo de login ni en una laptop compartida.
Serafin exige **nodo completo** (pedir ≥60 CPUs; los `rome*` tienen 64).

| Recurso | Valor típico |
|---------|----------------|
| Login | `serafin.ccad.unc.edu.ar`, user `pventura` |
| Compute | partición `short` (wall 1 h) o `multi` (wall 2 d) |
| Nodo | 64 CPUs, ~126–250 GiB RAM |
| Código | `~/src/OpenDefAlgSplitting` |
| Jobs / logs | `~/jobs/<nombre>/` |
| Rust | `~/.cargo` (rustup en `$HOME`) |

Documentación CCAD (wiki local + guía de experimentos):
[docs/ccad/README.md](../../docs/ccad/README.md)

Wiki en línea: https://wiki.ccad.unc.edu.ar/tutoriales/slurm-script.html

## Setup una sola vez

Desde tu máquina (clave SSH ya autorizada):

```bash
# 1) Sync del árbol de trabajo (sin target/)
rsync -az --delete \
  --exclude 'target/' --exclude '.git/' \
  -e 'ssh -o BatchMode=yes' \
  ./OpenDefAlgSplitting/ \
  pventura@serafin.ccad.unc.edu.ar:~/src/OpenDefAlgSplitting/

# 2) Rust en el home del cluster
ssh pventura@serafin.ccad.unc.edu.ar
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustc --version
```

Re-sync antes de cada campaña si cambió el código local.

## Build en login (obligatorio)

Los **nodos de cómputo no tienen internet** (`crates.io` falla con "transfer too slow").
Nunca `cargo build` / `cargo fetch` dentro del `sbatch`.

En el **login**, antes de `sbatch`:

```bash
cd ~/src/OpenDefAlgSplitting
bash scripts/ccad/prepare_release.sh   # binario release
# opcional, solo para job_cargo_test:
bash scripts/ccad/prepare_tests.sh     # compila tests (--no-run)
```

Alternativa desde la laptop (solo código; **no** el binario release):

```bash
rsync -az --delete --exclude 'target/' --exclude '.git/' \
  ./OpenDefAlgSplitting/ pventura@serafin.ccad.unc.edu.ar:~/src/OpenDefAlgSplitting/
# Luego en login: bash scripts/ccad/prepare_release.sh
```

No rsync de `target/release/opendefalgsplitting` desde la laptop: los nodos `rome*`
tienen glibc más vieja y el binario falla con `GLIBC_2.29 not found`.

`submit_all.sh` llama a `prepare_release.sh` automáticamente.

## Qué correr siempre vía Slurm

Scripts listos en `scripts/ccad/`:

| Job | Script | Partición | Qué ejecuta | Salida |
|-----|--------|-----------|-------------|--------|
| Tests | `job_cargo_test.sbatch` | `short` | `cargo test --release` | `~/jobs/cargo_test/` |
| Smoke+medium ablation | `job_ablation_smoke.sbatch` | `short` | `scripts/run_ablation.sh` | `~/jobs/ablation_smoke/` |
| Safe ablation (18 modelos) | `job_ablation_safe.sbatch` | `short` o `multi` | `scripts/run_ablation_safe.sh` | `~/jobs/ablation_safe/` |
| Extended ablation | `job_ablation_extended.sbatch` | `multi` | `scripts/run_ablation_extended.sh` | `~/jobs/ablation_extended/` |

Enviar:

```bash
ssh pventura@serafin.ccad.unc.edu.ar
cd ~/src/OpenDefAlgSplitting
# copiar plantillas si hace falta
cp -a scripts/ccad/*.sbatch ~/jobs/ 2>/dev/null || true

sbatch scripts/ccad/job_cargo_test.sbatch
sbatch scripts/ccad/job_ablation_smoke.sbatch
sbatch scripts/ccad/job_ablation_safe.sbatch
# si wall > 1 h:
sbatch scripts/ccad/job_ablation_extended.sbatch
```

Helper que envía la suite completa en orden (test → smoke → safe) y deja IDs:

```bash
bash scripts/ccad/submit_all.sh
```

**Campaña paper** (solo smoke + safe, la que cita el manuscrito):

```bash
# en Serafin (login):
bash scripts/ccad/submit_paper_experiment.sh

# desde la laptop (rsync + submit):
bash scripts/ccad/sync_and_submit_paper.sh
```

Los nodos de cómputo **no tienen** `python3` en el PATH. La ablación escribe el CSV igual;
el análisis (`analyze_ablation.py`) es opcional en el nodo y se corre en el **login** después:

```bash
bash scripts/ccad/postprocess_on_login.sh
```

## Reglas de los `#SBATCH` (no olvidar)

- `--partition=short` o `multi`
- `--nodes=1 --ntasks=1 --cpus-per-task=64` (nodo exclusivo)
- `--mem=0` (toda la RAM del nodo)
- `--time=01:00:00` en `short`; hasta `2-00:00:00` en `multi`
- `--mail-type=END,FAIL` y `--mail-user=pablogventura@gmail.com`
- Logs en `~/jobs/<job>/slurm-%j.{out,err}`

Al terminar, cada job escribe `~/jobs/<job>/DONE` con `STATUS=...` y manda mail.
Si falla el build o el script, `DONE` queda con `STATUS=*_FAIL` y exit ≠ 0.

## Aviso local (opcional)

Desde la laptop, con el repo local:

```bash
bash "paper splitting/ccad/watch_job.sh" ablation_smoke
# o
bash "paper splitting/ccad/watch_job.sh" ablation_safe
```

El watcher hace poll por SSH cada 60 s, copia `DONE` y el CSV a
`paper splitting/ccad/`, e intenta `notify-send`.

## Metodología a citar en el paper

Al publicar resultados de esta máquina, anotar:

- Cluster: CCAD UNC, Serafin (`rome*`)
- Job id Slurm y commit / fecha del sync
- `cargo build --release` en **login** (ver arriba), Rust estable (versión en el log)
- Suite: smoke (`run_ablation.sh`), safe (18 modelos, 5 repeats, timeout 90 s), etc.
- Wall-clock del binario (`--bench`); no tiempos de cola Slurm

## Qué no hacer

- No `cargo build` / ablación larga en el **login** salvo `prepare_release.sh` (build puntual).
- No `cargo build` en **nodos de cómputo** (sin red a crates.io).
- No pedir menos de ~60 CPUs (Slurm rechaza el job).
- No activar `--ig-experimental` en combos “oficiales” (cambia veredictos).
- No afirmar “seeds del experimento 32 vs 128” desde este checkout: eso es histórico Python (tesis); aquí solo tests + ablación Rust.

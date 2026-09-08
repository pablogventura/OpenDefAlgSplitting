#!/usr/bin/env python3
"""Run pure-Python fopy HIT on selected scale models (Serafin / local).

Forces FOPY_HIT_BACKEND=python so Rust is never used.
"""

from __future__ import annotations

import csv
import os
import resource
import sys
import time
from pathlib import Path

os.environ["FOPY_HIT_BACKEND"] = "python"

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CORPUS = ROOT / "benches" / "scale" / "corpus" / "pilot"
DEFAULT_MODELS = [
    "random_magma_n32_k2_definable_eq_i0.model",
    "random_magma_n32_k2_nondef_mix_i2.model",
    "random_magma_n32_k2_random_half_i0.model",
]


def rss_mib() -> float:
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024.0


def main() -> int:
    corpus = Path(os.environ.get("PYTHON_HIT_CORPUS", DEFAULT_CORPUS))
    out = Path(os.environ.get("PYTHON_HIT_OUT", "python_hit_n32k2.csv"))
    names = os.environ.get("PYTHON_HIT_MODELS", "").strip()
    models = [n.strip() for n in names.split(",") if n.strip()] if names else list(DEFAULT_MODELS)

    fopy_src = Path(os.environ.get("FOPY_SRC", Path.home() / "src" / "fopy" / "src"))
    if not fopy_src.is_dir():
        fopy_src = ROOT.parent / "fopy" / "src"
    if not fopy_src.is_dir():
        print(f"ERROR: fopy src not found at {fopy_src}", flush=True)
        return 2
    sys.path.insert(0, str(fopy_src))

    from fopy.finite.hit import Counterexample, HitConfig, is_open_def
    from fopy.finite.hit_rust import resolve_hit_backend
    from fopy.parse import parse_model

    backend = resolve_hit_backend()
    print(f"FOPY_SRC={fopy_src} backend={backend} corpus={corpus}", flush=True)
    if backend != "python":
        print("ERROR: backend is not python; aborting", flush=True)
        return 2

    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["model", "ms", "result", "max_rss_mib"])
        for name in models:
            path = Path(name) if name.startswith("/") else corpus / name
            print(
                f"=== {time.strftime('%Y-%m-%dT%H:%M:%S')} START {path.name} rss={rss_mib():.1f}MiB ===",
                flush=True,
            )
            if not path.is_file():
                w.writerow([path.name, "", "MISSING", f"{rss_mib():.1f}"])
                f.flush()
                print("MISSING", path, flush=True)
                continue
            t0 = time.perf_counter()
            try:
                model = parse_model(path, preprocess=True)
                targets = [r for s, r in sorted(model.relations.items()) if s.startswith("T")]
                if not targets:
                    raise RuntimeError("no T* target relation")
                result = is_open_def(model, targets, HitConfig())
                ms = (time.perf_counter() - t0) * 1000.0
                label = "NOT_DEFINABLE" if isinstance(result, Counterexample) else "DEFINABLE"
            except Exception as exc:  # noqa: BLE001
                ms = (time.perf_counter() - t0) * 1000.0
                label = f"ERROR:{type(exc).__name__}:{exc}"
            row_rss = rss_mib()
            w.writerow([path.name, f"{ms:.3f}", label, f"{row_rss:.1f}"])
            f.flush()
            print(
                f"=== DONE {path.name} result={label} ms={ms:.1f} max_rss={row_rss:.1f}MiB ===",
                flush=True,
            )
    print(f"wrote {out}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

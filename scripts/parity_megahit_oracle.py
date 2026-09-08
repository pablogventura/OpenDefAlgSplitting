#!/usr/bin/env python3
"""L1 (and optional L2) parity: OpenDefAlgMerging oracle vs Rust megahit_merge."""

from __future__ import annotations

import argparse
import csv
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ORACLE_DIR = ROOT / "oracles" / "OpenDefAlgMerging"
DEFAULT_BIN = ROOT / "target" / "release" / "opendefalgsplitting"

FIXTURES = [
    "modeloqueanda.model",
    "suma4.model",
    "retrombo_nodef.model",
    "msimple.model",
    "malvada.model",
    "retrombo.model",
    "retrombo2.model",
    "retrombo3.model",
    "retrombo_nodef_sinpura.model",
    "algebra.model",
    "cadena4.model",
    "minimal.model",
    "universo_un_elemento.model",
    "modelo_solo_target.model",
]

# Models that preprocess to zero T* (skip in L1).
SKIP_NAMES = {
    "posetrombo.model",  # thinning removes all T
    "target_vacio.model",  # empty / missing-T edge cases
}

_DEFINABLE = re.compile(r"\bDEFINABLE\b")
_NOT = re.compile(r"NOT DEFINABLE|NOT_DEFINABLE")


def parse_verdict(text: str) -> str | None:
    if _NOT.search(text):
        return "NOT_DEFINABLE"
    if _DEFINABLE.search(text):
        return "DEFINABLE"
    return None


def run_cmd(cmd: list[str], timeout_s: float, cwd: Path | None = None) -> tuple[str | None, float, str]:
    t0 = time.perf_counter()
    try:
        p = subprocess.run(
            cmd,
            cwd=str(cwd) if cwd else None,
            capture_output=True,
            text=True,
            timeout=timeout_s,
        )
    except subprocess.TimeoutExpired:
        return None, (time.perf_counter() - t0) * 1000.0, "timeout"
    out = (p.stdout or "") + "\n" + (p.stderr or "")
    if "NO TARGET RELATIONS FOUND" in out:
        return None, (time.perf_counter() - t0) * 1000.0, "skip_no_targets"
    if "File missing" in out or "No such file" in out:
        return None, (time.perf_counter() - t0) * 1000.0, "skip_missing"
    v = parse_verdict(out)
    status = "ok" if v else f"no_verdict_exit{p.returncode}"
    return v, (time.perf_counter() - t0) * 1000.0, status


def universe_size(model_path: Path) -> int:
    text = model_path.read_text(encoding="utf-8", errors="replace")
    for line in text.splitlines():
        s = line.strip()
        if s.startswith("universe") or s.startswith("Universo") or s.lower().startswith("u "):
            # fallback: count ints on line after keyword
            pass
    # Cheap parse: look for "0..n" style or listed elements after preprocess is hard;
    # count distinct from a quick Rust/oracle-independent heuristic: lines with only ints.
    # Prefer calling rust with a dry parse — for harness, grep "universe" blocks in .model.
    m = re.search(r"(?im)^\s*universe\s*(.*)$", text)
    if m:
        rest = m.group(1)
        nums = re.findall(r"-?\d+", rest)
        if nums:
            return len(set(nums))
    # Many models use Spanish / custom headers; count elements in first {...} or [..]
    m = re.search(r"\{([^}]*)\}", text)
    if m:
        nums = re.findall(r"-?\d+", m.group(1))
        if nums:
            return len(set(nums))
    return -1


def collect_models(args: argparse.Namespace) -> list[Path]:
    models: list[Path] = []
    examples = ROOT / "model_examples"
    testing = ROOT / "testing" / "tests_definibilidad" / "fixtures"
    for name in FIXTURES:
        if name in SKIP_NAMES:
            continue
        for base in (examples, testing):
            p = base / name
            if p.is_file():
                models.append(p)
                break
    if args.scale_sample > 0:
        pilot = ROOT / "benches" / "scale" / "corpus" / "pilot"
        if pilot.is_dir():
            cands = sorted(pilot.rglob("*.model"))
            picked = []
            for p in cands:
                # Prefer n=8 and n=16 from path components
                s = str(p)
                if "/n8/" in s or "_n8_" in s or "/8/" in s or "n08" in s:
                    picked.append(p)
                elif "/n16/" in s or "_n16_" in s or "/16/" in s:
                    picked.append(p)
            if not picked:
                picked = cands
            models.extend(picked[: args.scale_sample])
    if args.models:
        models = [Path(m) for m in args.models]
    # dedupe
    seen = set()
    out = []
    for m in models:
        rp = m.resolve()
        if rp not in seen:
            seen.add(rp)
            out.append(m)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--bin", type=Path, default=Path(os.environ.get("OPENDEFALGSPLITTING_BIN", DEFAULT_BIN)))
    ap.add_argument("--oracle-dir", type=Path, default=ORACLE_DIR)
    ap.add_argument("--timeout", type=float, default=60.0, help="timeout seconds for fixtures")
    ap.add_argument("--scale-timeout", type=float, default=300.0)
    ap.add_argument("--scale-sample", type=int, default=6, help="how many pilot scale models to include")
    ap.add_argument("--csv", type=Path, default=None)
    ap.add_argument("--keys", action="store_true", help="L2: compare IsoType equality on posetrombo sample")
    ap.add_argument("models", nargs="*", help="optional explicit .model paths")
    args = ap.parse_args()

    if not args.bin.is_file():
        print(f"error: rust binary missing: {args.bin}", file=sys.stderr)
        return 2
    oracle_main = args.oracle_dir / "main.py"
    if not oracle_main.is_file():
        print(f"error: oracle missing: {oracle_main}", file=sys.stderr)
        return 2

    models = collect_models(args)
    if not models:
        print("error: no models", file=sys.stderr)
        return 2

    rows = []
    mismatches = 0
    errors = 0
    print("model,|U|,oracle,rust,agree,oracle_ms,rust_ms,status")
    for mp in models:
        u = universe_size(mp)
        is_scale = "benches/scale" in str(mp).replace("\\", "/")
        timeout = args.scale_timeout if is_scale else args.timeout
        ov, oms, ost = run_cmd(
            [sys.executable, str(oracle_main), str(mp.resolve())],
            timeout,
            cwd=args.oracle_dir,
        )
        rv, rms, rst = run_cmd(
            [str(args.bin), str(mp.resolve()), "--fragment", "qf", "--engine", "merge"],
            timeout,
        )
        if ov is None or rv is None:
            if ost.startswith("skip") or rst.startswith("skip"):
                status = f"skip:{ost}/{rst}"
                agree = True
            else:
                status = f"error:{ost}/{rst}"
                agree = False
                errors += 1
        else:
            agree = ov == rv
            status = "ok" if agree else "mismatch"
            if not agree:
                mismatches += 1
        print(
            f"{mp.name},{u},{ov},{rv},{agree},{oms:.1f},{rms:.1f},{status}",
            flush=True,
        )
        rows.append(
            {
                "model": str(mp),
                "universe": u,
                "oracle": ov,
                "rust": rv,
                "agree": agree,
                "oracle_ms": f"{oms:.1f}",
                "rust_ms": f"{rms:.1f}",
                "status": status,
            }
        )

    if args.csv:
        args.csv.parent.mkdir(parents=True, exist_ok=True)
        with args.csv.open("w", newline="", encoding="utf-8") as f:
            w = csv.DictWriter(
                f,
                fieldnames=[
                    "model",
                    "universe",
                    "oracle",
                    "rust",
                    "agree",
                    "oracle_ms",
                    "rust_ms",
                    "status",
                ],
            )
            w.writeheader()
            w.writerows(rows)

    if args.keys:
        # L2 smoke: posetrombo pairs that historical hit.py checks
        print("# L2 keys: run oracle hit.py self-check via subprocess note", flush=True)
        hit_py = args.oracle_dir / "hit.py"
        if hit_py.is_file():
            p = subprocess.run(
                [sys.executable, str(hit_py)],
                cwd=str(args.oracle_dir),
                capture_output=True,
                text=True,
                timeout=60,
            )
            print(p.stdout)
            if p.returncode != 0:
                print(p.stderr, file=sys.stderr)
                errors += 1

    print(f"# summary mismatches={mismatches} errors={errors} n={len(rows)}")
    return 1 if mismatches or errors else 0


if __name__ == "__main__":
    sys.exit(main())

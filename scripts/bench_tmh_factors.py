#!/usr/bin/env python3
"""Matrix bench for HIT TMH speed knobs vs merge pilot verdicts.

Env factor flags (orthogonal, default off in the binary):
  HIT_TMH_CLOSURE_CACHE, HIT_TMH_STREAM_PRODUCT, HIT_TMH_PROXY,
  HIT_TMH_RAYON, HIT_PRE_FINISHED=approx, HIT_TMH_STATS=1
"""

from __future__ import annotations

import argparse
import csv
import os
import re
import resource
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "release" / "opendefalgsplitting"
MERGE_CSV = ROOT / "benches" / "scale" / "results" / "results_merge_pilot.csv"
CORPUS = ROOT / "benches" / "scale" / "corpus" / "pilot"

STATS_RE = re.compile(
    r"HIT_TMH_STATS computes=(\d+) cache_hits=(\d+) proxy_rejects=(\d+) "
    r"proxy_full=(\d+)(?: finished_impure=(\d+))?"
)
PROGRESS_RE = re.compile(
    r"HIT_PROGRESS steps=(\d+) computes=(\d+) cache_hits=(\d+) "
    r"proxy_rejects=(\d+) proxy_full=(\d+) finished_impure=(\d+) block_tuples=(\d+)"
)


def max_rss_mib_children() -> float:
    return resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss / 1024.0


def load_merge() -> dict[str, str]:
    out: dict[str, str] = {}
    if not MERGE_CSV.is_file():
        return out
    with MERGE_CSV.open(newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            name = Path(row["model"]).name
            out[name] = row["result"].strip()
    return out


def parse_stats(stderr: str) -> dict[str, str]:
    out = {
        "tmh_computes": "",
        "tmh_cache_hits": "",
        "proxy_rejects": "",
        "proxy_full": "",
        "finished_impure": "",
        "hit_steps": "",
        "last_block_tuples": "",
    }
    for m in PROGRESS_RE.finditer(stderr):
        out["hit_steps"] = m.group(1)
        out["tmh_computes"] = m.group(2)
        out["tmh_cache_hits"] = m.group(3)
        out["proxy_rejects"] = m.group(4)
        out["proxy_full"] = m.group(5)
        out["finished_impure"] = m.group(6)
        out["last_block_tuples"] = m.group(7)
    m = None
    for m in STATS_RE.finditer(stderr):
        pass
    if m:
        out["tmh_computes"] = m.group(1)
        out["tmh_cache_hits"] = m.group(2)
        out["proxy_rejects"] = m.group(3)
        out["proxy_full"] = m.group(4)
        if m.group(5) is not None:
            out["finished_impure"] = m.group(5)
    return out


def child_peak_rss_mib(pid: int) -> float:
    try:
        with open(f"/proc/{pid}/status", encoding="utf-8") as f:
            for line in f:
                if line.startswith("VmHWM:"):
                    kib = int(line.split()[1])
                    return kib / 1024.0
    except OSError:
        pass
    return 0.0


def run_one(
    model: Path,
    env_extra: dict[str, str],
    timeout_s: int,
) -> dict[str, str]:
    env = os.environ.copy()
    for k in (
        "HIT_TMH_CLOSURE_CACHE",
        "HIT_TMH_STREAM_PRODUCT",
        "HIT_TMH_PROXY",
        "HIT_TMH_RAYON",
        "HIT_PRE_FINISHED",
        "HIT_TMH_STATS",
        "HIT_MAX_RSS_MIB",
        "RAYON_NUM_THREADS",
    ):
        env.pop(k, None)
    env.update(env_extra)
    env.setdefault("HIT_TMH_STATS", "1")
    env.setdefault("RAYON_NUM_THREADS", env_extra.get("HIT_TMH_RAYON", "1"))

    t0 = time.perf_counter()
    proc = subprocess.Popen(
        [
            str(BIN),
            "--bench",
            "--repeat",
            "1",
            "--fragment",
            "qf",
            "--engine",
            "hit",
            str(model),
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        cwd=str(ROOT),
    )
    timed_out = False
    peak_rss = 0.0
    try:
        while True:
            try:
                stdout, stderr = proc.communicate(timeout=1.0)
                break
            except subprocess.TimeoutExpired:
                peak_rss = max(peak_rss, child_peak_rss_mib(proc.pid))
                if time.perf_counter() - t0 >= timeout_s:
                    timed_out = True
                    proc.send_signal(signal.SIGTERM)
                    try:
                        stdout, stderr = proc.communicate(timeout=3.0)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        stdout, stderr = proc.communicate()
                    break
        rc = proc.returncode if proc.returncode is not None else 124
    except Exception as e:
        proc.kill()
        stdout, stderr = "", f"harness_error: {e}"
        rc = 1
        timed_out = False

    peak_rss = max(peak_rss, child_peak_rss_mib(proc.pid) if proc.pid else 0.0)

    ms = ""
    result = "TIMEOUT" if timed_out else "FAIL"
    for line in (stdout or "").splitlines():
        if "\t" not in line or line.startswith("model"):
            continue
        parts = line.split("\t")
        if len(parts) >= 3:
            ms, result = parts[1], parts[2]

    stats = parse_stats(stderr or "")
    return {
        "ms": ms,
        "result": result,
        "timeout": "1" if timed_out else "0",
        "rc": str(rc if not timed_out else 124),
        "wall_s": f"{time.perf_counter() - t0:.3f}",
        "max_rss_mib_children": f"{max_rss_mib_children():.1f}",
        "max_rss_mib_proc": f"{peak_rss:.1f}",
        **stats,
    }


FACTORS: dict[str, dict[str, str]] = {
    "baseline": {},
    "A": {"HIT_TMH_CLOSURE_CACHE": "1"},
    "E": {"HIT_TMH_STREAM_PRODUCT": "1"},
    "B": {"HIT_TMH_PROXY": "1"},
    "D2": {"HIT_TMH_RAYON": "2"},
    "C": {"HIT_PRE_FINISHED": "approx"},
    "A+E": {"HIT_TMH_CLOSURE_CACHE": "1", "HIT_TMH_STREAM_PRODUCT": "1"},
    "A+B": {"HIT_TMH_CLOSURE_CACHE": "1", "HIT_TMH_PROXY": "1"},
    "E+B": {"HIT_TMH_STREAM_PRODUCT": "1", "HIT_TMH_PROXY": "1"},
    "A+E+B": {
        "HIT_TMH_CLOSURE_CACHE": "1",
        "HIT_TMH_STREAM_PRODUCT": "1",
        "HIT_TMH_PROXY": "1",
    },
}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--tier",
        choices=("micro", "small", "hard", "all"),
        default="small",
    )
    ap.add_argument("--timeout-micro", type=int, default=30)
    ap.add_argument("--timeout-small", type=int, default=90)
    ap.add_argument("--timeout-hard", type=int, default=240)
    ap.add_argument(
        "--factors",
        default="baseline,A,E,B,D2,C,A+E,A+B,E+B,A+E+B",
        help="Comma-separated factor set names",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=ROOT / "benches" / "scale" / "results" / "tmh_factors_matrix.csv",
    )
    args = ap.parse_args()

    if not BIN.is_file():
        print(f"missing release binary: {BIN} (cargo build --release)", file=sys.stderr)
        return 2

    merge = load_merge()
    models: list[tuple[str, Path, int]] = []

    if args.tier in ("micro", "all"):
        u = Path("/tmp/hit_lazy_unary_half_i0.model")
        if u.is_file():
            models.append(("micro", u, args.timeout_micro))
        for name in sorted(CORPUS.glob("random_magma_n4_k1_definable_eq_i0.model")):
            models.append(("micro", name, args.timeout_micro))

    if args.tier in ("small", "all"):
        for pat in (
            "random_magma_n16_k2_definable_eq_i0.model",
            "random_magma_n16_k2_nondef_mix_i0.model",
            "random_magma_n16_k2_random_half_i0.model",
            "random_magma_n32_k1_definable_eq_i0.model",
        ):
            p = CORPUS / pat
            if p.is_file():
                models.append(("small", p, args.timeout_small))

    if args.tier in ("hard", "all"):
        p = CORPUS / "random_magma_n32_k2_random_half_i0.model"
        if p.is_file():
            models.append(("hard", p, args.timeout_hard))

    factor_names = [x.strip() for x in args.factors.split(",") if x.strip()]
    args.out.parent.mkdir(parents=True, exist_ok=True)

    fieldnames = [
        "tier",
        "factor_set",
        "model",
        "ms",
        "result",
        "merge",
        "agree_merge",
        "timeout",
        "wall_s",
        "max_rss_mib_children",
        "max_rss_mib_proc",
        "tmh_computes",
        "tmh_cache_hits",
        "proxy_rejects",
        "proxy_full",
        "finished_impure",
        "hit_steps",
        "last_block_tuples",
        "rc",
    ]
    with args.out.open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=fieldnames)
        w.writeheader()
        for tier, model, to in models:
            for fname in factor_names:
                flags = FACTORS.get(fname)
                if flags is None:
                    print(f"unknown factor {fname}", file=sys.stderr)
                    continue
                print(f"=== {tier} {fname} {model.name} to={to}s ===", flush=True)
                row = run_one(model, flags, to)
                mname = model.name
                mv = merge.get(mname, "")
                agree = ""
                if (
                    mv
                    and row["result"] in ("DEFINABLE", "NOT_DEFINABLE")
                    and row["timeout"] != "1"
                ):
                    agree = "1" if row["result"] == mv else "0"
                out_row = {
                    "tier": tier,
                    "factor_set": fname,
                    "model": mname,
                    "merge": mv,
                    "agree_merge": agree,
                    **row,
                }
                w.writerow(out_row)
                f.flush()
                print(
                    f"  -> {row['result']} ms={row['ms']} wall={row['wall_s']}s "
                    f"rss={row['max_rss_mib_proc']} computes={row['tmh_computes']} "
                    f"fi={row['finished_impure']} steps={row['hit_steps']} agree={agree}",
                    flush=True,
                )
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

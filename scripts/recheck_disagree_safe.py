#!/usr/bin/env python3
"""One-by-one HIT vs merge recheck with RAM/timeout guards. Compact --bench output."""
from __future__ import annotations

import csv
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "release" / "opendefalgsplitting"
CORPUS = ROOT / "benches" / "scale" / "corpus" / "pilot"
DIS = ROOT / "benches" / "scale" / "results" / "verdict_disagree.csv"
OUT = ROOT / "benches" / "scale" / "results" / "verdict_recheck_after_hit_fix.csv"
LOG = ROOT / "benches" / "scale" / "results" / "verdict_recheck_live.log"
MIN_AVAIL_KIB = 10 * 1024 * 1024  # abort below 10 GiB available


def mem_avail_kib() -> int:
    with open("/proc/meminfo") as f:
        for line in f:
            if line.startswith("MemAvailable:"):
                return int(line.split()[1])
    return 0


def timeout_for(name: str) -> int:
    if "_n4_" in name:
        return 20
    if "_n8_" in name:
        return 30
    if "_n16_" in name:
        return 45
    if "_n32_k1_" in name:
        return 60
    if "_n32_k2_" in name:
        return 75
    return 45


def kill_tree(pid: int) -> None:
    try:
        os.killpg(pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    time.sleep(0.4)
    try:
        os.killpg(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def parse_bench(text: str) -> str | None:
    # last data line: model\tms\tresult\tengine  OR similar
    # also tolerate DEFINABLE / NOT DEFINABLE
    for line in reversed(text.splitlines()):
        line = line.strip()
        if not line or line.startswith("model"):
            continue
        parts = line.split("\t")
        if len(parts) >= 3:
            res = parts[2].strip().upper().replace(" ", "_")
            if res in ("DEFINABLE", "NOT_DEFINABLE"):
                return res
            if "NOT" in res and "DEFINABLE" in res:
                return "NOT_DEFINABLE"
            if res == "DEFINABLE":
                return "DEFINABLE"
        m = re.search(r"\b(NOT DEFINABLE|DEFINABLE)\b", line)
        if m:
            return "NOT_DEFINABLE" if m.group(1).startswith("NOT") else "DEFINABLE"
    return None


def run_engine(model: Path, engine: str, to: int) -> str:
    cmd = [
        str(BIN),
        "--bench",
        "--repeat",
        "1",
        "--fragment",
        "qf",
        "--engine",
        engine,
        str(model),
    ]
    p = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=True,
    )
    try:
        out, err = p.communicate(timeout=to)
    except subprocess.TimeoutExpired:
        kill_tree(p.pid)
        try:
            p.communicate(timeout=5)
        except Exception:
            pass
        return "TIMEOUT"
    text = (out or "") + "\n" + (err or "")
    v = parse_bench(text)
    if v is None:
        # keep a short hint in log only
        hint = text[-300:].replace("\n", " ")
        log(f"    parse_fail hint: {hint!r}")
        return "PARSE_FAIL"
    return v


def log(msg: str) -> None:
    print(msg, flush=True)
    with LOG.open("a") as lf:
        lf.write(msg + "\n")


def model_key(name: str):
    n = int(re.search(r"_n(\d+)_", name).group(1))
    k = int(re.search(r"_k(\d+)_", name).group(1))
    return (n, k, name)


def main() -> int:
    if not BIN.is_file():
        print(f"missing binary: {BIN}", file=sys.stderr)
        return 2
    rows = list(csv.DictReader(DIS.open()))
    models = [r["model"] for r in sorted(rows, key=lambda r: model_key(r["model"]))]
    LOG.write_text("")
    log(f"START n={len(models)} avail={mem_avail_kib()/1024/1024:.1f}Gi")

    with OUT.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["model", "hit", "merge", "agree", "note"])
        ok = bad = to_n = abort = 0
        for i, name in enumerate(models, 1):
            av = mem_avail_kib()
            if av < MIN_AVAIL_KIB:
                log(f"RAM low {av/1024/1024:.1f}Gi abort at {name}")
                w.writerow([name, "ABORT", "ABORT", "false", "ram_low"])
                abort += 1
                break
            to = timeout_for(name)
            log(f"[{i}/{len(models)} {av/1024/1024:.0f}Gi] {name} to={to}s")
            h = run_engine(CORPUS / name, "hit", to)
            log(f"  HIT={h}")
            time.sleep(0.15)
            av = mem_avail_kib()
            if av < MIN_AVAIL_KIB:
                log(f"RAM low after HIT {av/1024/1024:.1f}Gi")
                w.writerow([name, h, "ABORT", "false", "ram_low_after_hit"])
                abort += 1
                break
            m = run_engine(CORPUS / name, "merge", to)
            log(f"  MERGE={m}")
            if "TIMEOUT" in (h, m):
                agree, note = False, "timeout"
                to_n += 1
            elif "PARSE_FAIL" in (h, m):
                agree, note = False, "parse_fail"
                bad += 1
            elif h == m:
                agree, note = True, "ok"
                ok += 1
            else:
                agree, note = False, "disagree"
                bad += 1
            w.writerow([name, h, m, "true" if agree else "false", note])
            f.flush()
            log(f"  agree={agree} note={note}")

    log(f"FINAL ok={ok} bad={bad} timeout={to_n} abort={abort} avail={mem_avail_kib()/1024/1024:.1f}Gi")
    print("--- non-agree ---", flush=True)
    with OUT.open() as f:
        for row in csv.DictReader(f):
            if row["agree"] != "true":
                print(
                    f"{row['model']}: hit={row['hit']} merge={row['merge']} note={row['note']}",
                    flush=True,
                )
    return 0 if abort == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Analiza CSV de ablacion: deltas de tiempo vs baseline por variante y modelo."""
import csv
import sys
from collections import defaultdict
from pathlib import Path


def main() -> None:
    path = Path(sys.argv[1] if len(sys.argv) > 1 else "docs/ablation/results_extended.csv")
    rows = []
    with path.open() as f:
        reader = csv.DictReader(f)
        for r in reader:
            if r.get("ms") in (None, "ERR", ""):
                continue
            try:
                r["ms_f"] = float(r["ms"])
                r["steps_i"] = int(r["steps"])
                r["skipped_i"] = int(r["skipped"])
                r["approx_i"] = int(r["approx_reject"])
            except ValueError:
                continue
            rows.append(r)

    by_model = defaultdict(dict)  # type: ignore[var-annotated]
    for r in rows:
        by_model[r["model"]][r["variant"]] = r

    print("=== Deltas de tiempo vs baseline (ms y %) ===\n")
    print(
        f"{'model':<42} {'base_ms':>8} {'skip%':>8} {'approx%':>8} {'combo%':>8} "
        f"{'bfs%':>8} {'ig%':>8} {'base_steps':>10} {'skip_steps':>10}"
    )

    skip_wins = 0
    skip_n = 0
    approx_wins = 0
    approx_n = 0
    combo_wins = 0
    combo_n = 0
    verdict_mismatch = []

    for model, vars_ in sorted(by_model.items()):
        base = vars_.get("baseline")
        if not base:
            continue
        bms = base["ms_f"]
        bst = base["steps_i"]
        bres = base["result"]

        def pct(name: str) -> str:
            v = vars_.get(name)
            if not v:
                return "—"
            if v["result"] != bres and name == "ig_experimental":
                return "BAD"
            if bms <= 0:
                return "—"
            d = (v["ms_f"] - bms) / bms * 100.0
            return f"{d:+.1f}"

        def steps(name: str) -> str:
            v = vars_.get(name)
            return str(v["steps_i"]) if v else "—"

        for name in ("skip_useless", "approx", "combo", "bfs", "ig_legacy", "simplify"):
            v = vars_.get(name)
            if not v:
                continue
            if v["result"] != bres:
                verdict_mismatch.append((model, name, bres, v["result"]))

        if "skip_useless" in vars_:
            skip_n += 1
            if vars_["skip_useless"]["ms_f"] < bms * 0.95:
                skip_wins += 1
        if "approx" in vars_:
            approx_n += 1
            if vars_["approx"]["ms_f"] < bms * 0.95:
                approx_wins += 1
        if "combo" in vars_:
            combo_n += 1
            if vars_["combo"]["ms_f"] < bms * 0.95:
                combo_wins += 1

        short = model.replace("model_examples/", "")
        print(
            f"{short:<42} {bms:8.2f} {pct('skip_useless'):>8} {pct('approx'):>8} "
            f"{pct('combo'):>8} {pct('bfs'):>8} {pct('ig_legacy'):>8} "
            f"{bst:10d} {steps('skip_useless'):>10}"
        )

    print("\n=== Resumen (>5% mas rapido que baseline) ===")
    print(f"skip_useless: {skip_wins}/{skip_n} modelos")
    print(f"approx:       {approx_wins}/{approx_n} modelos")
    print(f"combo:        {combo_wins}/{combo_n} modelos")

    print("\n=== Early-exit approx (approx_reject > 0) ===")
    for model, vars_ in sorted(by_model.items()):
        v = vars_.get("approx")
        if v and v["approx_i"] > 0:
            base = vars_.get("baseline")
            bms = base["ms_f"] if base else float("nan")
            print(
                f"  {model}: reject={v['approx_i']} ms={v['ms_f']:.2f} "
                f"(baseline {bms:.2f}) result={v['result']}"
            )

    if verdict_mismatch:
        print("\n=== WARNING: veredicto distinto al baseline ===")
        for model, name, a, b in verdict_mismatch:
            print(f"  {model} [{name}]: {a} -> {b}")
    else:
        print("\nSin mismatches de veredicto (salvo flags no incluidos / ausentes).")

    # Ranking por ahorro absoluto de skip
    print("\n=== Top ahorros absolutos con skip_useless (ms) ===")
    savings = []
    for model, vars_ in by_model.items():
        if "baseline" in vars_ and "skip_useless" in vars_:
            d = vars_["baseline"]["ms_f"] - vars_["skip_useless"]["ms_f"]
            savings.append((d, model, vars_["baseline"]["ms_f"], vars_["skip_useless"]["ms_f"]))
    savings.sort(reverse=True)
    for d, model, b, s in savings[:12]:
        print(f"  {d:+8.1f} ms  {model}  ({b:.1f} -> {s:.1f})")


if __name__ == "__main__":
    main()

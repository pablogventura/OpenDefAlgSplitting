#!/usr/bin/env python3
"""Join difficulty metrics with ablation CSV; Spearman + figures + report."""

from __future__ import annotations

import argparse
import csv
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
METRICS = ROOT / "docs" / "ablation" / "difficulty_metrics.csv"
RESULTS = ROOT / "docs" / "ablation" / "results_safe.csv"
REPORT = ROOT / "docs" / "ablation" / "DIFFICULTY_REPORT.md"
FIG_STEPS = ROOT / "docs" / "ablation" / "fig_difficulty_steps.pdf"

PAPER_MODELS = [
    "model_examples/modeloqueanda.model",
    "model_examples/cadena5.model",
    "model_examples/gigante.model",
    "model_examples/16_T3_4_0.model",
    "model_examples/retrombo_nodef.model",
    "model_examples/suma4.model",
    "model_examples/msimple.model",
]


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def spearman(xs: list[float], ys: list[float]) -> float | None:
    n = len(xs)
    if n < 3 or len(ys) != n:
        return None

    def ranks(vals: list[float]) -> list[float]:
        order = sorted(range(n), key=lambda i: vals[i])
        r = [0.0] * n
        i = 0
        while i < n:
            j = i
            while j + 1 < n and vals[order[j + 1]] == vals[order[i]]:
                j += 1
            avg = (i + j + 2) / 2.0
            for t in range(i, j + 1):
                r[order[t]] = avg
            i = j + 1
        return r

    rx, ry = ranks(xs), ranks(ys)
    mx = sum(rx) / n
    my = sum(ry) / n
    num = sum((rx[i] - mx) * (ry[i] - my) for i in range(n))
    den_x = math.sqrt(sum((rx[i] - mx) ** 2 for i in range(n)))
    den_y = math.sqrt(sum((ry[i] - my) ** 2 for i in range(n)))
    if den_x == 0 or den_y == 0:
        return None
    return num / (den_x * den_y)


def fval(row: dict[str, str], key: str) -> float | None:
    raw = row.get(key, "")
    if raw == "" or raw.upper() in ("TIMEOUT", "ERR", "ERROR"):
        return None
    try:
        return float(raw)
    except ValueError:
        return None


def join_rows(metrics: list[dict], results: list[dict], variant: str = "baseline") -> list[dict]:
    by_model: dict[str, dict] = {}
    for r in results:
        if r.get("variant") != variant:
            continue
        by_model[r["model"]] = r

    joined: list[dict] = []
    for m in metrics:
        model = m["model"]
        bench = by_model.get(model)
        if not bench:
            continue
        row = {**m}
        row["ms"] = bench.get("ms", "")
        row["steps"] = bench.get("steps", "")
        row["result"] = bench.get("result", "")
        joined.append(row)
    return joined


def latex_table(rows: list[dict], models: list[str]) -> str:
    subset = [r for r in rows if r["model"] in models]
    lines = [
        r"\begin{tabular}{lrrrrr}",
        r"\hline",
        r"Model & $|A|$ & $k$ & steps & $H_{\mathrm{pat}}$ & $H_{\mathrm{fp}}$ \\",
        r"\hline",
    ]
    for r in subset:
        name = Path(r["model"]).stem.replace("_", r"\_")
        hfp = r.get("H_fp", "") or "-"
        lines.append(
            f"{name} & {r['universe']} & {r['k']} & {r['steps']} & "
            f"{r['H_pat_univ']} & {hfp} \\\\"
        )
    lines.extend([r"\hline", r"\end{tabular}"])
    return "\n".join(lines)


def try_scatter(joined: list[dict], out: Path) -> str | None:
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        return None

    xs: list[float] = []
    ys: list[float] = []
    labels: list[str] = []
    for r in joined:
        x = fval(r, "H_fp")
        y = fval(r, "steps")
        if x is None or y is None:
            continue
        xs.append(x)
        ys.append(y)
        labels.append(Path(r["model"]).stem)

    if len(xs) < 2:
        return None

    fig, ax = plt.subplots(figsize=(5, 4))
    ax.scatter(xs, ys, alpha=0.75)
    for x, y, lab in zip(xs, ys, labels, strict=True):
        ax.annotate(lab, (x, y), fontsize=6, alpha=0.8)
    ax.set_xlabel(r"$H_{\mathrm{fp}}$ (bits, fuel fixed)")
    ax.set_ylabel("splitting steps (baseline)")
    ax.set_title("Exploratory: steps vs term fingerprint entropy")
    fig.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out)
    plt.close(fig)
    return str(out)


def build_report(joined: list[dict], fig_path: str | None) -> str:
    steps = [fval(r, "steps") for r in joined]
    ms = [fval(r, "ms") for r in joined]
    valid_idx = [i for i in range(len(joined)) if steps[i] is not None and ms[i] is not None]

    metrics_keys = [
        ("H_pat_univ", r"$H_{\mathrm{pat}}$"),
        ("H_fp", r"$H_{\mathrm{fp}}$"),
        ("H_R", r"$H(R)$"),
        ("universe", r"$|A|$"),
        ("inj_tuples", r"$|A^{(k)}|$"),
        ("pat_classes", "pat classes"),
    ]

    lines = [
        "# Difficulty predictors (exploratory)",
        "",
        "Join of `difficulty_metrics.csv` and `results_safe.csv` (baseline variant).",
        f"Sample size: n={len(valid_idx)} models with valid steps.",
        "",
        "## Spearman rho (steps and ms vs static metrics)",
        "",
        "| metric | rho(steps) | rho(ms) |",
        "|--------|------------|---------|",
    ]

    for key, label in metrics_keys:
        xs = [fval(joined[i], key) for i in valid_idx]
        ys = [steps[i] for i in valid_idx]
        zs = [ms[i] for i in valid_idx]
        pairs_s = [(x, y) for x, y in zip(xs, ys, strict=True) if x is not None and y is not None]
        pairs_m = [(x, z) for x, z in zip(xs, zs, strict=True) if x is not None and z is not None]
        rho_s = spearman([p[0] for p in pairs_s], [p[1] for p in pairs_s]) if len(pairs_s) >= 3 else None
        rho_m = spearman([p[0] for p in pairs_m], [p[1] for p in pairs_m]) if len(pairs_m) >= 3 else None
        rs = f"{rho_s:.3f}" if rho_s is not None else "n/a"
        rm = f"{rho_m:.3f}" if rho_m is not None else "n/a"
        lines.append(f"| {label} | {rs} | {rm} |")

    lines.extend(
        [
            "",
            "**Disclaimer:** n is small; correlations are exploratory, not universal predictors.",
            "Information-gain split ordering (experimental) is not pattern entropy.",
            "",
            "## Paper table (7 representative models)",
            "",
            "```latex",
            latex_table(joined, PAPER_MODELS),
            "```",
            "",
            "## Full joined table",
            "",
            "| model | k | steps | ms | H_pat | H_fp | H_R |",
            "|-------|---|-------|----|-------|------|-----|",
        ]
    )

    for r in sorted(joined, key=lambda x: x["model"]):
        lines.append(
            f"| `{r['model']}` | {r['k']} | {r.get('steps','')} | {r.get('ms','')} | "
            f"{r['H_pat_univ']} | {r.get('H_fp','')} | {r['H_R']} |"
        )

    if fig_path:
        lines.extend(["", f"Scatter figure: `{fig_path}`"])

    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--metrics", type=Path, default=METRICS)
    parser.add_argument("--results", type=Path, default=RESULTS)
    parser.add_argument("--report", type=Path, default=REPORT)
    parser.add_argument("--fig", type=Path, default=FIG_STEPS)
    parser.add_argument("--variant", default="baseline")
    args = parser.parse_args()

    if not args.metrics.is_file():
        print(f"missing {args.metrics}; run difficulty_metrics.py first", file=sys.stderr)
        sys.exit(1)
    if not args.results.is_file():
        print(f"missing {args.results}", file=sys.stderr)
        sys.exit(1)

    metrics = read_csv(args.metrics)
    results = read_csv(args.results)
    joined = join_rows(metrics, results, variant=args.variant)
    fig = try_scatter(joined, args.fig)
    report = build_report(joined, fig)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(report, encoding="utf-8")
    print(f"wrote {args.report} ({len(joined)} rows joined)")
    if fig:
        print(f"wrote {fig}")


if __name__ == "__main__":
    main()

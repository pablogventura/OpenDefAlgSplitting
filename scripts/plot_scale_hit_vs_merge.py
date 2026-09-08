#!/usr/bin/env python3
"""Plot HIT vs megahit merge scale CSVs (Rust pilot/full). Prefers gnuplot."""

from __future__ import annotations

import argparse
import csv
import math
import re
import subprocess
from collections import defaultdict
from pathlib import Path


def parse_n(model: str) -> int | None:
    m = re.search(r"_n(\d+)", Path(model).stem)
    return int(m.group(1)) if m else None


def load_csv(path: Path) -> list[tuple[int, float]]:
    rows: list[tuple[int, float]] = []
    with path.open(newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if row.get("ms") == "ms":
                continue
            try:
                ms = float(row["ms"])
            except (KeyError, ValueError):
                continue
            n = parse_n(row.get("model", ""))
            if n is None:
                continue
            rows.append((n, ms))
    return rows


def median(xs: list[float]) -> float:
    ys = sorted(xs)
    if not ys:
        return float("nan")
    m = len(ys) // 2
    if len(ys) % 2:
        return ys[m]
    return 0.5 * (ys[m - 1] + ys[m])


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--hit", type=Path, required=True)
    ap.add_argument("--merge", type=Path, required=True)
    ap.add_argument("--out-dir", type=Path, required=True)
    ap.add_argument("--title-suffix", default="Rust pilot")
    ap.add_argument(
        "--classic",
        action="store_true",
        help="Thesis-style B/W plot (Cardinality / Time (s), circles vs triangles)",
    )
    ap.add_argument(
        "--merge-timeout-s",
        type=float,
        default=0.0,
        help="If >0, drop merge medians at/above this wall (s) so the curve leaves the window",
    )
    ap.add_argument(
        "--out-name",
        default="hit_vs_merge_scale.pdf",
        help="Output PDF basename inside --out-dir",
    )
    args = ap.parse_args()

    hit = load_csv(args.hit)
    merge = load_csv(args.merge)
    args.out_dir.mkdir(parents=True, exist_ok=True)

    by_hit: dict[int, list[float]] = defaultdict(list)
    by_merge: dict[int, list[float]] = defaultdict(list)
    for n, ms in hit:
        by_hit[n].append(ms)
    for n, ms in merge:
        by_merge[n].append(ms)

    ns = sorted(set(by_hit) | set(by_merge))
    dat = args.out_dir / "hit_vs_merge.dat"
    summary = args.out_dir / "hit_vs_merge_summary.csv"
    with dat.open("w", encoding="utf-8") as f:
        f.write("# n hit_s merge_s\n")
        for n in ns:
            hs = by_hit.get(n, [])
            ms = by_merge.get(n, [])
            hy = median(hs) / 1000.0 if hs else float("nan")
            my = median(ms) / 1000.0 if ms else float("nan")
            if args.merge_timeout_s > 0 and not math.isnan(my) and my >= args.merge_timeout_s:
                my = float("nan")
            f.write(f"{n} {hy} {my}\n")
    with summary.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["n", "hit_median_s", "merge_median_s", "hit_n", "merge_n"])
        for n in ns:
            hs = by_hit.get(n, [])
            ms = by_merge.get(n, [])
            my = median(ms) / 1000.0 if ms else float("nan")
            if args.merge_timeout_s > 0 and not math.isnan(my) and my >= args.merge_timeout_s:
                my_s = ""
            else:
                my_s = f"{my:.6g}" if ms else ""
            w.writerow(
                [
                    n,
                    f"{median(hs)/1000:.6g}" if hs else "",
                    my_s,
                    len(hs),
                    len(ms),
                ]
            )

    out_pdf = args.out_dir / args.out_name
    if args.classic:
        xtics = ", ".join(f'"{n}" {n}' for n in ns)
        gp = f"""
set terminal pdfcairo size 5.5,3.6 font 'Helvetica,11'
set output '{out_pdf}'
set xlabel 'Cardinality'
set ylabel 'Time (s)'
set logscale y
set grid
set key top left
set xtics ({xtics})
plot '{dat}' using 1:2 with linespoints lw 2 pt 7 lc rgb '#000000' title 'Splitting algorithm', \\
     '' using 1:3 with linespoints lw 2 pt 9 lc rgb '#000000' title 'Merging algorithm'
"""
    else:
        gp = f"""
set terminal pdfcairo size 5.5,3.6 font 'Helvetica,10'
set output '{out_pdf}'
set xlabel '|U|'
set ylabel 'median wall-clock (s)'
set title 'HIT vs IsoType merge ({args.title_suffix})'
set logscale y
set grid
set key top left
plot '{dat}' using 1:2 with linespoints lw 2 pt 7 title 'HIT', \\
     '' using 1:3 with linespoints lw 2 pt 5 title 'IsoType merge'
"""
    subprocess.run(["gnuplot"], input=gp, text=True, check=True)
    print(f"wrote {out_pdf}")
    print(f"wrote {summary}")
    print(f"hit_rows={len(hit)} merge_rows={len(merge)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

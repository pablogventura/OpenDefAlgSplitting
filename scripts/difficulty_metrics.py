#!/usr/bin/env python3
"""Static difficulty metrics for OpenDefAlgSplitting .model files.

Memory-safe: streams injective tuples, caps term generation, skips heavy
instances when RAM or combinatorics exceed thresholds.
"""

from __future__ import annotations

import argparse
import csv
import gc
import itertools
import math
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Iterator, Sequence

ROOT = Path(__file__).resolve().parents[1]
FOPY_SRC = ROOT.parent / "fopy" / "src"
if FOPY_SRC.is_dir() and str(FOPY_SRC) not in sys.path:
    sys.path.insert(0, str(FOPY_SRC))

from fopy.finite.models import Model  # noqa: E402
from fopy.finite.relops import Operation  # noqa: E402
from fopy.parse.model import parse_model  # noqa: E402

DEFAULT_MODELS = [
    "model_examples/modeloqueanda.model",
    "model_examples/suma4.model",
    "model_examples/retrombo_nodef.model",
    "model_examples/msimple.model",
    "model_examples/retrombo.model",
    "model_examples/cadena4.model",
    "model_examples/malvada.model",
    "model_examples/algebra.model",
    "model_examples/miprueba.model",
    "model_examples/cadena5.model",
    "model_examples/gigante.model",
    "model_examples/16_T3_4_0.model",
    "model_examples/p0_d0.0625_a2_u30_q5.model",
    "model_examples/modelosexperimento.model",
    "model_examples/modelosimplequefalla.model",
    "model_examples/retrombo_nodef_sinpura.model",
    "model_examples/retromboconstantes.model",
    # cadena100 omitted by default (|A|=100); add explicitly if needed
]

DEFAULT_FUEL = 4
MIN_AVAIL_KIB = 4 * 1024 * 1024  # 4 GiB MemAvailable
MAX_INJ = 6000
MAX_FP_UNIVERSE = 20
MAX_FP_INJ = 800
MAX_TERMS = 64
MAX_POOL_PI = 12
MAX_PATTERN_DOMAIN = 12_000
MAX_FINGERPRINT_TERMS = 48

Term = tuple


def mem_available_kib() -> int:
    try:
        with open("/proc/meminfo", encoding="utf-8") as f:
            for line in f:
                if line.startswith("MemAvailable:"):
                    return int(line.split()[1])
    except OSError:
        return 10**9
    return 10**9


def inj_count(n: int, k: int) -> int:
    if k > n or k < 0:
        return 0
    c = 1
    for i in range(k):
        c *= n - i
    return c


def shannon(weights: Sequence[int]) -> float:
    total = sum(weights)
    if total == 0:
        return 0.0
    h = 0.0
    for w in weights:
        if w == 0:
            continue
        p = w / total
        h -= p * math.log(p, 2)
    return h


def binary_entropy(num: int, den: int) -> float:
    if den == 0:
        return 0.0
    p = num / den
    if p in (0.0, 1.0):
        return 0.0
    return -p * math.log(p, 2) - (1 - p) * math.log(1 - p, 2)


def pattern_key(tup: tuple[int, ...]) -> tuple[tuple[int, ...], ...]:
    blocks: dict[int, list[int]] = {}
    for i, v in enumerate(tup):
        blocks.setdefault(v, []).append(i)
    classes = [tuple(sorted(idxs)) for idxs in blocks.values()]
    classes.sort(key=lambda c: c[0])
    return tuple(classes)


def iter_injective(universe: Sequence[int], k: int) -> Iterator[tuple[int, ...]]:
    n = len(universe)
    if inj_count(n, k) > MAX_INJ:
        raise ValueError(f"injective P({n},{k}) exceeds MAX_INJ={MAX_INJ}")
    yield from itertools.permutations(universe, k)


def count_target_on_injective(
    target: set[tuple[int, ...]], universe: Sequence[int], k: int
) -> int:
    u = set(universe)
    n = 0
    for t in target:
        if len(t) != k:
            continue
        if len(set(t)) != k:
            continue
        if all(x in u for x in t):
            n += 1
    return n


def pattern_stats(universe: Sequence[int], k: int) -> tuple[float, int, int]:
    counts: Counter = Counter()
    total = 0
    for t in iter_injective(universe, k):
        counts[pattern_key(t)] += 1
        total += 1
    return shannon(counts.values()), len(counts), total


def list_pi_bounded(pool: list[Term], arity: int, cap: int) -> list[tuple[Term, ...]]:
    if arity == 0:
        return [()]
    pool = pool[:MAX_POOL_PI]
    out: list[tuple[Term, ...]] = []

    def rec(depth: int, pref: tuple[Term, ...]) -> bool:
        if len(out) >= cap:
            return False
        if depth == arity:
            out.append(pref)
            return True
        for t in pool:
            if not rec(depth + 1, pref + (t,)):
                return False
        return True

    rec(0, ())
    return out


def generate_terms(
    witnesses: list[Term],
    new_witnesses: list[Term],
    operations: dict[str, Operation],
    cap: int,
) -> list[Term]:
    pool = (witnesses + new_witnesses)[:MAX_POOL_PI]
    news: list[Term] = []
    new_set = set(new_witnesses)
    for op in operations.values():
        if op.arity == 0:
            continue
        for args in list_pi_bounded(pool, op.arity, cap):
            if any(a in new_set for a in args):
                news.append(("app", op.sym, args))
                if len(news) >= cap:
                    return news
    return news


def accumulate_terms(k: int, operations: dict[str, Operation], fuel: int) -> list[Term]:
    vars_: list[Term] = [("var", i) for i in range(k)]
    acc: list[Term] = list(vars_)
    acc_set = set(acc)
    news = list(vars_)
    for _ in range(fuel):
        if len(acc) >= MAX_TERMS:
            break
        gen: list[Term] = []
        for t in generate_terms(acc, news, operations, MAX_TERMS - len(acc)):
            if t not in acc_set:
                gen.append(t)
        if not gen:
            break
        for t in gen:
            acc.append(t)
            acc_set.add(t)
        news = gen
    return acc[:MAX_FINGERPRINT_TERMS]


def eval_term(
    term: Term, assignment: tuple[int, ...], operations: dict[str, Operation]
) -> int | None:
    if term[0] == "var":
        return assignment[term[1]]
    _tag, sym, args = term
    op = operations.get(sym)
    if op is None:
        return None
    vals: list[int] = []
    for sub in args:
        v = eval_term(sub, assignment, operations)
        if v is None:
            return None
        vals.append(v)
    return op.call(vals)


def term_fingerprint(
    terms: list[Term], assignment: tuple[int, ...], operations: dict[str, Operation]
) -> tuple[bool, ...]:
    vals = [eval_term(t, assignment, operations) for t in terms]
    bits: list[bool] = []
    for i, vi in enumerate(vals):
        for j, vj in enumerate(vals):
            bits.append(vi is not None and vj is not None and vi == vj)
    return tuple(bits)


def fingerprint_entropy_stream(
    universe: Sequence[int],
    k: int,
    operations: dict[str, Operation],
    fuel: int,
) -> tuple[float, int]:
    terms = accumulate_terms(k, operations, fuel)
    counts: Counter = Counter()
    for t in iter_injective(universe, k):
        counts[term_fingerprint(terms, t, operations)] += 1
    return shannon(counts.values()), len(counts)


def pattern_mixes_target(universe: Sequence[int], k: int, target: set[tuple[int, ...]]) -> bool:
    n = len(universe)
    if n**k > MAX_PATTERN_DOMAIN:
        return False
    by_pat: dict[tuple[tuple[int, ...], ...], tuple[bool, bool]] = {}
    for tup in itertools.product(universe, repeat=k):
        key = pattern_key(tup)
        in_t = tup in target
        inside, outside = by_pat.get(key, (False, False))
        if in_t:
            inside = True
        else:
            outside = True
        by_pat[key] = (inside, outside)
        if inside and outside:
            return True
    return False


def pick_primary_target(model: Model):
    targets = sorted(model.targets.values(), key=lambda r: (-r.arity, r.sym))
    if not targets:
        rels = [r for r in model.relations.values() if r.sym.startswith("T")]
        targets = sorted(rels, key=lambda r: (-r.arity, r.sym))
    if not targets:
        raise ValueError("no target relation")
    return targets[0]


@dataclass
class MetricRow:
    model: str
    target: str
    universe: int
    k: int
    inj_tuples: int
    pat_classes: int
    h_pat_univ: float
    h_r: float
    target_density: float
    h_fp: float | None
    fp_classes: int | None
    pattern_mixes: bool
    fuel: int
    skipped: str

    def as_csv(self) -> dict[str, str | int | float]:
        return {
            "model": self.model,
            "target": self.target,
            "universe": self.universe,
            "k": self.k,
            "inj_tuples": self.inj_tuples,
            "pat_classes": self.pat_classes,
            "H_pat_univ": f"{self.h_pat_univ:.6f}",
            "H_R": f"{self.h_r:.6f}",
            "target_density": f"{self.target_density:.6f}",
            "H_fp": "" if self.h_fp is None else f"{self.h_fp:.6f}",
            "fp_classes": "" if self.fp_classes is None else self.fp_classes,
            "pattern_mixes": int(self.pattern_mixes),
            "fuel": self.fuel,
            "skipped": self.skipped,
        }


def metrics_for_model(path: Path, fuel: int = DEFAULT_FUEL) -> MetricRow:
    if mem_available_kib() < MIN_AVAIL_KIB:
        raise RuntimeError("MemAvailable below 4 GiB; aborting")

    model = parse_model(path)
    target = pick_primary_target(model)
    universe = model.universe
    k = target.arity
    n = len(universe)
    skipped: list[str] = []

    ic = inj_count(n, k)
    h_pat, pat_classes, inj_total = pattern_stats(universe, k)
    target_inj = count_target_on_injective(target.r, universe, k)
    h_r = binary_entropy(target_inj, inj_total)
    density = target_inj / inj_total if inj_total else 0.0

    mixes = False
    if n**k <= MAX_PATTERN_DOMAIN:
        mixes = pattern_mixes_target(universe, k, target.r)
    else:
        skipped.append("pattern_mixes")

    h_fp: float | None = None
    fp_classes: int | None = None
    if (
        n <= MAX_FP_UNIVERSE
        and ic <= MAX_FP_INJ
        and k > 0
        and k <= 3
        and mem_available_kib() >= MIN_AVAIL_KIB
    ):
        h_fp, fp_classes = fingerprint_entropy_stream(universe, k, model.operations, fuel)
    else:
        skipped.append("H_fp")

    rel_model = str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else path.as_posix()

    del model
    gc.collect()

    return MetricRow(
        model=rel_model,
        target=target.sym,
        universe=n,
        k=k,
        inj_tuples=inj_total,
        pat_classes=pat_classes,
        h_pat_univ=h_pat,
        h_r=h_r,
        target_density=density,
        h_fp=h_fp,
        fp_classes=fp_classes,
        pattern_mixes=mixes,
        fuel=fuel,
        skipped=";".join(skipped),
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models", nargs="*", default=DEFAULT_MODELS)
    parser.add_argument("--fuel", type=int, default=DEFAULT_FUEL)
    parser.add_argument(
        "--out",
        type=Path,
        default=ROOT / "docs" / "ablation" / "difficulty_metrics.csv",
    )
    parser.add_argument(
        "--emit-row",
        action="store_true",
        help="print one CSV data row to stdout (for batch wrappers)",
    )
    args = parser.parse_args()

    if not args.emit_row:
        print(f"MemAvailable: {mem_available_kib() // 1024 // 1024} GiB", flush=True)
    rows: list[MetricRow] = []
    for rel in args.models:
        path = ROOT / rel
        if not path.is_file():
            print(f"skip missing {path}", file=sys.stderr, flush=True)
            continue
        try:
            if not args.emit_row:
                print(f"metrics {rel} ...", flush=True)
            row = metrics_for_model(path, fuel=args.fuel)
            rows.append(row)
            if args.emit_row:
                fields = row.as_csv()
                print(",".join(str(fields[k]) for k in fields))
                return
            print(
                f"  ok inj={row.inj_tuples} skipped={row.skipped or '-'}",
                flush=True,
            )
        except Exception as exc:  # noqa: BLE001
            print(f"error {path}: {exc}", file=sys.stderr, flush=True)
        gc.collect()

    if not rows:
        print("no rows", file=sys.stderr)
        sys.exit(1)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = list(rows[0].as_csv().keys())
    with args.out.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow(row.as_csv())
    print(f"wrote {len(rows)} rows to {args.out}", flush=True)


if __name__ == "__main__":
    main()

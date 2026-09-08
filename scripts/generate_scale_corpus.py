#!/usr/bin/env python3
"""Generate regenerable scale corpus (.model) from benches/scale/manifest.json."""
from __future__ import annotations

import argparse
import hashlib
import json
import random
from pathlib import Path
from typing import List, Optional


def write_universe(n: int) -> str:
    return " ".join(str(i) for i in range(n))


def write_op_table(name: str, arity: int, table, n: int) -> str:
    lines = ["%s %d" % (name, arity)]
    if arity == 0:
        lines.append(str(table))
        return "\n".join(lines) + "\n"
    if arity == 1:
        for a in range(n):
            lines.append("%d %d" % (a, table[a]))
    elif arity == 2:
        for a in range(n):
            for b in range(n):
                lines.append("%d %d %d" % (a, b, table[a][b]))
    else:
        raise ValueError("unsupported arity %s" % arity)
    return "\n".join(lines) + "\n"


def random_magma(n: int, rng: random.Random):
    return [[rng.randrange(n) for _ in range(n)] for _ in range(n)]


def cyclic_add(n: int):
    return [[(a + b) % n for b in range(n)] for a in range(n)]


def chain_succ(n: int):
    return [min(a + 1, n - 1) for a in range(n)]


def all_k_tuples(n: int, k: int):
    if k == 0:
        yield []
        return
    cur = [0] * k
    while True:
        yield list(cur)
        i = k - 1
        while i >= 0:
            cur[i] += 1
            if cur[i] < n:
                break
            cur[i] = 0
            i -= 1
        if i < 0:
            return


def target_definable_eq(n: int, k: int) -> List[List[int]]:
    rows = []
    for t in all_k_tuples(n, k):
        if k == 1 and t[0] == 0:
            rows.append(t)
        elif k >= 2 and t[0] == t[1]:
            rows.append(t)
    return rows


def target_full(n: int, k: int) -> List[List[int]]:
    """R = A^k (definable; HIT should accept pattern pieces immediately)."""
    return [list(t) for t in all_k_tuples(n, k)]


def target_definable_fixed(n: int, k: int) -> List[List[int]]:
    if k != 1:
        return target_definable_eq(n, k)
    return [[0]]


def target_random_half(n: int, k: int, rng: random.Random) -> List[List[int]]:
    rows = []
    for t in all_k_tuples(n, k):
        if rng.random() < 0.5:
            rows.append(t)
    return rows


def target_nondef_mix(n: int, k: int) -> List[List[int]]:
    rows = []
    for t in all_k_tuples(n, k):
        if t[0] % 2 == 0:
            rows.append(t)
    if k >= 1 and n >= 2:
        odd = [1] + [0] * (k - 1)
        even = [0] * k
        rows = [r for r in rows if r != odd]
        if even not in rows:
            rows.append(even)
    return rows


TARGET_BUILDERS = {
    "definable_eq": lambda n, k, rng: target_definable_eq(n, k),
    "definable_fixed": lambda n, k, rng: target_definable_fixed(n, k),
    "random_half": target_random_half,
    "nondef_mix": lambda n, k, rng: target_nondef_mix(n, k),
    "full": lambda n, k, rng: target_full(n, k),
}


def write_target(name: str, rows: list, k: int) -> str:
    lines = ["%s %d %d" % (name, len(rows), k)]
    for r in rows:
        lines.append(" ".join(str(x) for x in r))
    return "\n".join(lines) + "\n"


def build_model(
    family: str, n: int, k: int, target_id: str, seed: int, max_tuples: int
) -> Optional[str]:
    if n**k > max_tuples:
        return None
    rng = random.Random(seed)
    parts = [
        "# scale corpus family=%s n=%d k=%d target=%s seed=%d"
        % (family, n, k, target_id, seed),
        write_universe(n),
    ]
    if family == "random_magma":
        parts.append(write_op_table("f", 2, random_magma(n, rng), n))
    elif family == "cyclic_group":
        parts.append(write_op_table("add", 2, cyclic_add(n), n))
    elif family == "chain_unary":
        parts.append(write_op_table("succ", 1, chain_succ(n), n))
    else:
        raise ValueError(family)
    builder = TARGET_BUILDERS[target_id]
    rows = builder(n, k, rng)
    parts.append(write_target("T0", rows, k))
    return "\n".join(parts)


def cell_seeds(master: int, family: str, n: int, k: int, target: str, count: int) -> List[int]:
    digest = hashlib.md5(
        ("%d:%s:%d:%d:%s" % (master, family, n, k, target)).encode()
    ).hexdigest()
    base = int(digest[:8], 16)
    return [base + i * 9973 for i in range(count)]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--pilot-only", action="store_true")
    ap.add_argument("--max-n", type=int, default=0, help="if >0, skip sizes above this")
    args = ap.parse_args()
    man = json.loads(args.manifest.read_text())
    master = int(man["master_seed"])
    max_tuples = int(man.get("max_tuples", 200000))
    n_cells = int(man["n_per_cell_pilot"] if args.pilot_only else man["n_per_cell_full"])
    mode = "pilot" if args.pilot_only else "full"
    out_root = args.out / mode
    out_root.mkdir(parents=True, exist_ok=True)
    written = 0
    skipped = 0
    index = []
    for fam in man["families"]:
        fid = fam["id"]
        for n in fam["sizes"]:
            if args.max_n > 0 and n > args.max_n:
                continue
            for k in fam["k"]:
                for tid in fam["targets"]:
                    for i, seed in enumerate(
                        cell_seeds(master, fid, n, k, tid, n_cells)
                    ):
                        text = build_model(fid, n, k, tid, seed, max_tuples)
                        if text is None:
                            skipped += 1
                            continue
                        name = "%s_n%d_k%d_%s_i%d.model" % (fid, n, k, tid, i)
                        path = out_root / name
                        path.write_text(text)
                        index.append(
                            {
                                "file": str(path.relative_to(args.out)),
                                "family": fid,
                                "n": n,
                                "k": k,
                                "target": tid,
                                "seed": seed,
                            }
                        )
                        written += 1
    (args.out / ("index_%s.json" % mode)).write_text(json.dumps(index, indent=2))
    print("wrote=%d skipped_cap=%d -> %s" % (written, skipped, out_root))


if __name__ == "__main__":
    main()

# !/usr/bin/env python

from __future__ import annotations

from functools import total_ordering
from typing import Any

from first_order import formulas


@total_ordering
class Relation:
    """
    Relation
    """

    def __init__(
        self,
        sym: str,
        arity: int,
        rel: set | None = None,
        pattern: Any = None,
        superrel: Relation | None = None,
    ) -> None:
        self.syntax_sym = formulas.RelSym(sym, arity)
        self.sym = sym
        self.arity = arity
        self.r = rel if rel is not None else set()
        self.pattern = pattern
        if superrel is None:
            self.superrel = self
        else:
            self.superrel = superrel

    def add(self, t: tuple) -> None:
        if len(t) != self.arity:
            raise ValueError(f"{t} is not of arity {self.arity}")
        self.r.add(t)

    def __repr__(self):
        return f"{self.sym} : {self.r}"

    def __call__(self, *args: Any) -> bool:
        return args in self.r

    def __len__(self):
        return len(self.r)

    def __iter__(self):
        return iter(self.r)

    def spectrum(self) -> set:
        result = set()
        for t in self:
            result.add(len(set(t)))
        return result

    def restrict(self, subuniverse: list | set) -> Relation:
        result = Relation(self.sym, self.arity)
        subuniverse = set(subuniverse)
        for t in self.r:
            if set(t) <= subuniverse:
                result.add(t)
        return result

    def __hash__(self):
        return hash(frozenset(self.r))

    def __eq__(self, other):
        return (self.sym, self.r) == (other.sym, other.r)

    def __ne__(self, other):
        return not (self == other)

    def __lt__(self, other: Relation) -> bool:
        return self.arity > other.arity or self.sym < self.sym  # TODO no ordena bien los symbolos


class Operation:
    """
    Operation
    """

    def __init__(self, sym: str, arity: int) -> None:
        self.syntax_sym = formulas.OpSym(sym, arity)
        self.sym = sym
        self.arity = arity
        self.op = {}

    def add(self, t: tuple) -> None:
        if len(t) - 1 != self.arity:
            raise ValueError(f"{t[:-1]} is not of arity {self.arity}")
        self.op[t[:-1]] = t[-1]

    def __repr__(self):
        return f"{self.sym} : {self.op}"

    def __call__(self, *args: Any) -> Any:
        return self.op[args]

    def __len__(self):
        return len(self.op)

    # def __iter__(self):
    #    return iter(self.r)

    # def spectrum(self):
    #    result = set()
    #    for t in self:
    #        result.add(len(set(t)))
    #    return result

    def restrict(self, subuniverse: list | set) -> Operation:
        result = Operation(self.sym, self.arity)
        subuniverse = set(subuniverse)
        for t in self.op:
            if set(t) <= subuniverse:
                result.add(t + (self.op[t],))
        return result

    def graph_rel(self) -> Relation:
        rel = {t + (self.op[t],) for t in self.op}
        return Relation("g" + self.sym, self.arity + 1, rel)

#!/usr/bin/env python

from __future__ import annotations

from typing import Any

from misc import indent


class Homomorphism:
    values: dict

    def __init__(self, d: dict, source: Any, target: Any, subtype: set | list) -> None:
        self.values = d
        self.source = source
        self.target = target
        self.subtype = subtype

    def __call__(self, x: Any) -> Any:
        try:
            return self.values[x]
        except KeyError:
            return None

    def vcall(self, xvector: tuple) -> tuple:
        return tuple(self(x) for x in xvector)

    def __repr__(self) -> str:
        result = "Homomorphism(\n"
        for a, b in self.values.items():
            result += f"  {a}->{b}\n"
        result += "from:\n"
        result += indent(repr(self.source))
        result += "to:\n"
        result += indent(repr(self.target))
        result += ")"
        return result

    def homo_wrt(self, subtype: set | list) -> bool:
        if self.source.rels_sizes(subtype) > self.target.rels_sizes(subtype):
            return False
        for r in subtype:
            for t in self.source.relations[r]:
                if not self.target.relations[r](*tuple(self(x) for x in t)):
                    return False
        return True


class Isomorphism:
    values: dict

    def __init__(self, d: dict, source: Any, target: Any, subtype: set | list) -> None:
        self.values = d
        self.source = source
        self.target = target
        self.subtype = subtype

    def __call__(self, x: Any) -> Any:
        try:
            return self.values[x]
        except KeyError:
            return None

    def inverse(self) -> Isomorphism:
        return Isomorphism(
            {v: k for k, v in self.values.items()}, self.target, self.source, self.subtype
        )

    def vcall(self, xvector: tuple) -> tuple:
        return tuple(self(x) for x in xvector)

    def __repr__(self) -> str:
        result = "Isomorphism(\n"
        for a, b in self.values.items():
            result += f"  {a}->{b}\n"
        result += "from:\n"
        result += indent(repr(self.source))
        result += "to:\n"
        result += indent(repr(self.target))
        result += ")"
        return result

    def iso_wrt(self, subtype: set | list) -> bool:
        if self.source.rels_sizes(subtype) != self.target.rels_sizes(subtype):
            print("tamaos distintos")
            return False
        for r in subtype:
            for t in self.source.relations[r]:
                if not self.target.relations[r](*tuple(self(x) for x in t)):
                    print(t, tuple(self(x) for x in t))
                    return False
        return True


class Automorphism:
    values: dict

    def __init__(self, d: dict, model: Any, subtype: set | list) -> None:
        self.values = d
        self.model = model
        self.subtype = subtype

    def __call__(self, x: Any) -> Any:
        try:
            return self.values[x]
        except KeyError:
            return None

    def vcall(self, xvector: tuple) -> tuple:
        return tuple(self(x) for x in xvector)

    def __repr__(self) -> str:
        result = "Automorphism(\n"
        for a, b in self.values.items():
            result += f"  {a}->{b}\n"
        result += "from:\n"
        result += indent(repr(self.model))
        result += ")"
        return result

    def aut_wrt(self, subtype: set | list) -> bool:
        for r in subtype:
            for t in self.model.relations[r]:
                if not self.model.relations[r](*tuple(self(x) for x in t)):
                    return False
        return True

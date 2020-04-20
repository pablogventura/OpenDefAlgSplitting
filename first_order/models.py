# -*- coding: utf-8 -*-
#!/usr/bin/env python

from itertools import product
from misc import indent


class PartialOrderedDict(dict):
    def __lt__(self, other):  # <
        return self <= other and self != other

    def __gt__(self, other):  # >
        return self >= other and self != other

    def __le__(self, other):  # <=
        for k in self:
            if not self[k] <= other[k]:
                return False
        return True

    def __ge__(self, other):  # >=
        for k in self:
            if not self[k] >= other[k]:
                return False
        return True


class Model(object):
    def __init__(self, universe, relations, operations):
        """
        Model
        Input: a universe list, relations dict, operations dict
        """
        self.universe = sorted(universe)
        self.relations = relations
        self.operations = operations

    def restrict(self, subuniverse):
        """
        restricion de un subuniverso a ciertas relaciones
        """
        relations = {}
        for r in self.relations:
            relations[r] = self.relations[r].restrict(subuniverse)
        operations = {}
        for o in self.operations:
            operations[o] = self.operations[o].restrict(subuniverse)
        return Model(subuniverse, relations, operations)

    def substructure(self, generators):
        news = set(generators)
        universe = set()

        while news:
            local_news = set()
            for operation in self.operations:
                arity = self.operations[operation].arity
                for t in product(universe | news, repeat=arity):
                    if any(e in news for e in t):  # tiene alguno nuevo
                        local_news.add(self.operations[operation](*t))

            local_news -= universe
            local_news -= news
            universe |= news
            news = local_news
        return self.restrict(universe)

    def rels_sizes(self, subtype):

        return PartialOrderedDict({r: len(self.relations[r]) for r in subtype})

    def __repr__(self):
        result = "Model(universe=%s,\nrelations=\n" % self.universe
        for sym in sorted(self.relations):
            result += indent(self.relations[sym]) + "\n"
        result += "operations=\n"
        for sym in sorted(self.operations):
            result += indent(self.operations[sym]) + "\n"
        result += ")"
        return result

    def __len__(self):
        return len(self.universe)

    def spectrum(self, subtype):
        result = set()
        return result.union(*[self.relations[r].spectrum() for r in subtype])

    def to_relational_model(self):
        relations = dict(self.relations)
        for op in self.operations:
            rel = self.operations[op].graph_rel()
            relations[rel.sym] = rel
        return Model(self.universe, relations, dict())

    def rel_minion_name(self, r):
        return r.replace("|", "b").replace("-", "e")

    def minion_tables(self, subtype):
        result = ""
        for r in subtype:
            result += "%s %s %s\n" % (self.rel_minion_name(r),
                                      len(self.relations[r]), self.relations[r].arity)
            for t in self.relations[r]:
                result += " ".join(str(self.universe.index(x)) for x in t) + "\n"
            result += "\n"
        return result[:-1]

    def minion_constraints(self, subtype):
        result = ""
        # table([f[0],f[0],f[0]],bv)
        result = ""
        for r in subtype:
            for t in self.relations[r]:
                result += "table([f["
                result += "],f[".join(str(self.universe.index(x)) for x in t)
                result += "]],%s)\n" % self.rel_minion_name(r)
        return result

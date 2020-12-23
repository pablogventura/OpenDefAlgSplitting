# -*- coding: utf-8 -*-
# !/usr/bin/env python

from functools import total_ordering
from first_order import formulas

@total_ordering
class Relation(object):
    """
    Relation
    """
    
    def __init__(self, sym, arity, rel=set(), pattern=None, superrel=None):
        self.syntax_sym = formulas.RelSym(sym,arity)
        self.sym = sym
        self.arity = arity
        self.r = rel
        self.pattern = pattern
        if superrel is None:
            self.superrel = self
        else:
            self.superrel = superrel
    
    def add(self, t):
        if len(t) != self.arity:
            raise ValueError('%s is not of arity %s' % (t, self.arity))
        self.r.add(t)
    
    def __repr__(self):
        return "%s : %s" % (self.sym, self.r)
    
    def __call__(self, *args):
        return args in self.r
    
    def __len__(self):
        return len(self.r)
    
    def __iter__(self):
        return iter(self.r)
    
    def spectrum(self):
        result = set()
        for t in self:
            result.add(len(set(t)))
        return result
    
    def restrict(self, subuniverse):
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
    
    def __lt__(self, other):
        return self.arity > other.arity or self.sym < self.sym  # TODO no ordena bien los symbolos


class Operation(object):
    """
    Operation
    """
    
    def __init__(self, sym, arity):
        self.syntax_sym = formulas.OpSym(sym, arity)
        self.sym = sym
        self.arity = arity
        self.op = dict()
    
    def add(self, t):
        if len(t) - 1 != self.arity:
            raise ValueError('%s is not of arity %s' % (t[:-1], self.arity))
        self.op[t[:-1]] = t[-1]
    
    def __repr__(self):
        return "%s : %s" % (self.sym, self.op)
    
    def __call__(self, *args):
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
    
    def restrict(self, subuniverse):
        result = Operation(self.sym, self.arity)
        subuniverse = set(subuniverse)
        for t in self.op:
            if set(t) <= subuniverse:
                result.add(t + (self.op[t],))
        return result
    
    def graph_rel(self):
        rel = {t + (self.op[t],) for t in self.op}
        return Relation("g" + self.sym, self.arity + 1, rel)

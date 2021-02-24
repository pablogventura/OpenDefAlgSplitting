from collections import defaultdict, OrderedDict
from first_order import formulas
from first_order.relops import Relation, Operation
from misc import indent


class Pattern(object):
    def __init__(self, t):
        self.tuple = t
        self.pruned_tuple = []
        pattern = defaultdict(set)
        for i, a in enumerate(t):
            pattern[a].add(i)
            if len(pattern[a]) == 1:
                self.pruned_tuple.append(a)
        self.pruned_tuple = tuple(self.pruned_tuple)
        self.pattern = frozenset(frozenset(s) for s in pattern.values())
    
    def name(self):
        result = "|"
        for cls in self.pattern:
            result += ",".join(str(i) for i in cls)
            result += "|"
        result += "a%s" % len(self.pruned_tuple)
        return result
    
    def __hash__(self):
        return hash(self.pattern)
    
    def __eq__(self,other):
        return hash(self.pattern) == hash(other.pattern)
    
    def preprocessed_formula(self):
        f = formulas.true()
        vs = formulas.variables(*list(range(len(self.tuple))))
        representantes = [sorted(cls)[0] for cls in self.pattern]
        vs = [vs[i] for i in representantes]
        for v in vs:
            for w in vs:
                if v is not w:
                    f = f & -formulas.eq(v, w)
        return f
    
    def postprocessed_formula(self):
        f = formulas.true()
        vs = formulas.variables(*list(range(len(self.tuple))))
        differents = []
        for cls in self.pattern:
            # aca decimos las partes que son iguales
            cls = list(cls)
            for i,j in zip(cls,cls[1:]):
                f = f & formulas.eq(vs[i], vs[j])
            # aca viene el momento en que la formula dice que son distintos
            representante = vs[min(cls)]
            for otro in differents:
                f = f & -formulas.eq(representante, otro)
            differents.append(representante)
        return f
    
    def __repr__(self):
        result = "Pattern(\n"
        result += indent("tuple = %s\n" % (self.tuple,))
        result += indent("pattern = %s\n" % self.pattern)
        result += indent("formula = %s\n" % self.formula())
        result += ")"
        return result

def quotient(s, f):
    # cociente del conjunto s, por la funcion f
    result = {e: [e] for e in s}
    for a in s:
        if a not in result:
            continue
        for b in s:
            if b not in result or a == b:
                continue
            if f(b) == f(a):
                result[a] += result[b]
                del result[b]
    return result

def limpia(t):
    result = set()
    for e in t:
        result.add(t.index(e))
    return sorted(result)


def preprocesamiento(T):
    result = []
    q = quotient(T, patron)
    for p in q:
        indices = limpia(p)
        result.append(set())
        for t in q[p]:
            result[-1].add(tuple(t[i] for i in indices))
    return set(frozenset(e) for e in result)


def formula_patron(t):
    f = formulas.true()
    vs = formulas.variables(*list(range(len(t))))
    tn = tuple(OrderedDict.fromkeys(t))
    free_vars = tuple(vs[i] for i in (t.index(e) for e in tn))
    for i, a in enumerate(t):
        for j, b in enumerate(t):
            if j <= i:
                continue
            if a == b:
                f = f & formulas.eq(vs[i], vs[j])
            else:
                f = f & -formulas.eq(vs[i], vs[j])
    return f, tn, free_vars


def preprocesamiento2(target):

    pruned_relations = defaultdict(list)
    for t in target.r:
        pattern = Pattern(t)
        pruned_relations[pattern].append(pattern.pruned_tuple)
    result = []
    for pattern in pruned_relations:
        first_tuple = pruned_relations[pattern][0]
        arity = len(first_tuple)
        patron_name = "_".join(str(i) for i in first_tuple)
        result.append(Relation(target.sym + pattern.name(),arity,pruned_relations[pattern],pattern,target))
    return result

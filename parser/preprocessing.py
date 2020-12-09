from collections import defaultdict, OrderedDict
from first_order import formulas
from first_order.relops import Relation, Operation


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


def patron(t):
    # cociente del conjunto s, por la funcion f
    result = defaultdict(set)
    for i, a in enumerate(t):
        result[a].add(i)
    return set(frozenset(s) for s in result.values())


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
    for i, a in enumerate(t):
        for j, b in enumerate(t):
            if j <= i:
                continue
            if a == b:
                f = f & formulas.eq(vs[i], vs[j])
            else:
                f = f & -formulas.eq(vs[i], vs[j])
    return f, tn


def preprocesamiento2(target):
    #target_formula = formulas.RelSym(target.sym,target.arity)(*formulas.variables(*list(range(target.arity))))
    pruned_relations = defaultdict(list)
    for t in target.r:
        formula, pruned_tuple = formula_patron(t)
        pruned_relations[formula].append(pruned_tuple)
    result = []
    for formula in pruned_relations:
        first_tuple = pruned_relations[formula][0]
        arity = len(first_tuple)
        patron_name = "_".join(str(i) for i in first_tuple)
        result.append(Relation(target.sym + "a%sp%s" % (arity,patron_name),arity,pruned_relations[formula],formula,target))
    print(len(result))
    # return list(fs.values()),list(ts.values())
    return result

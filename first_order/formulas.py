#!/usr/bin/env python
# -*- coding: utf8 -*-

# TERMS

from __future__ import annotations

from myunicode import subscript
from itertools import product, combinations
from collections import defaultdict
from typing import Any, Iterable, Iterator, Set, Tuple


class Term(object):
    """
    Clase general de los terminos de primer orden
    """
    def __init__(self):
        pass

    def free_vars(self) -> Set[Variable]:
        raise NotImplementedError

    def evaluate(self, model: Any, vector: dict) -> Any:
        """
        Evalua el termino en el modelo para el vector de valores
        """
        raise NotImplementedError

    def __hash__(self) -> int:
        raise NotImplemented

    def __lt__(self, other: Term) -> bool:
        if self.grade() == other.grade():
            return repr(self)<repr(other)
        else:
            return self.grade() < other.grade()
    
    def grade(self) -> int:
        raise NotImplementedError

    def __eq__(self, other: object) -> bool:
        return hash(self) == hash(other)

class Variable(Term):
    """
    Variable de primer orden
    """
    sym: str

    def __init__(self, sym: int | str) -> None:
        if isinstance(sym,int):
            self.sym = "x" + subscript(sym)
        else:
            self.sym = sym

    def __repr__(self):
        return self.sym

    def __hash__(self) -> int:
        return hash(self.sym)

    def free_vars(self) -> Set[Variable]:
        return {self}

    def grade(self) -> int:
        return 0

    def evaluate(self, model: Any, vector: dict) -> Any:
        try:
            return vector[self]
        except KeyError:
            raise ValueError("Free variable %s is not defined" % (self))

class OpSym(object):
    """
    Simbolo de operacion de primer orden
    """
    op: str
    arity: int

    def __init__(self, op: str, arity: int) -> None:
        self.op = op
        self.arity = arity

    def __call__(self, *args: Term) -> OpTerm:
        if len(args) != self.arity:
            raise ValueError("Arity not correct")
        for a in args:
            if not isinstance(a, Term):
                raise ValueError("%s isn't a term" % a)
        return OpTerm(self,args)
    
    def __hash__(self) -> int:
        return hash((self.op,self.arity))
    
    def __repr__(self):
        return self.op

class OpTerm(Term):
    """
    Termino de primer orden de la aplicacion de una funcion
    """
    sym: OpSym
    args: tuple

    def __init__(self, sym: OpSym, args: tuple) -> None:
        self.sym = sym
        self.args = args

    def __repr__(self):
        result = repr(self.sym)
        result += "("
        result += ", ".join(map(repr,self.args))
        result += ")"
        return result
    
    def __hash__(self) -> int:
        return hash((self.sym,self.args))
    
    def grade(self):
        if self.args:
            return 1 + max(t.grade() for t in self.args)
        else:
            return 0

    def free_vars(self) -> Set[Variable]:
        if self.args:
            return set.union(*[f.free_vars() for f in self.args])
        else:
            return set()

    def evaluate(self, model: Any, vector: dict) -> Any:
        if self.args:
            args = [t.evaluate(model,vector) for t in self.args]
            return model.operations[self.sym.op](*args)
        else:
            return model.operations[self.sym.op]()

# FORMULAS
# Memoización de fórmulas para evitar crear duplicados en & y |
_formula_and_cache = {}
_formula_or_cache = {}


class Formula(object):
    """
    Clase general de las formulas de primer orden

    >>> x,y,z = variables("x","y","z") # declaracion de variables de primer orden

    >>> R = RelSym("R",2) # declaro una relacion R de aridad 2

    >>> f = OpSym("f",3) # declaro una operacion f de aridad 3

    >>> R(x,y) | R(y,x) & R(y,z)
    (R(x, y) ∨ (R(y, x) ∧ R(y, z)))

    >>> -R(f(x,y,z),y) | R(y,x) & R(y,z)
    (¬ R(f(x, y, z), y) ∨ (R(y, x) ∧ R(y, z)))

    >>> a = forall(x, -R(f(x,y,z),y))
    >>> a
    ∀ x ¬ R(f(x, y, z), y)
    >>> a.free_vars() == {y,z}
    True

    >>> a = R(x,x) & a
    >>> a
    (R(x, x) ∧ ∀ x ¬ R(f(x, y, z), y))
    >>> a.free_vars() == {x, y, z}
    True

    >>> exists(x, R(f(x,y,z),y))
    ∃ x R(f(x, y, z), y)

    >>> (-(true() & true() & false())) | false()
    ⊤

    """
    orphan_vars: Set[Variable]

    def __init__(self, orphan_vars: Set[Variable] | None = None) -> None:
        self.orphan_vars = orphan_vars if orphan_vars is not None else set()

    def __and__(self, other: Formula) -> Formula:
        if isinstance(other, AndFormula):
            return other & self
        elif isinstance(self,TrueFormula):
            other.orphan_vars |= self.free_vars()
            return other
        elif isinstance(other,TrueFormula):
            self.orphan_vars |= other.free_vars()
            return self
        elif isinstance(self,FalseFormula) or isinstance(other,FalseFormula):
            return false(self.free_vars() | other.free_vars())
        elif self == -other:
            return false(self.free_vars() | other.free_vars())

        key = (id(self), id(other)) if id(self) < id(other) else (id(other), id(self))
        if key not in _formula_and_cache:
            _formula_and_cache[key] = AndFormula([self, other])
        return _formula_and_cache[key]

    def __or__(self, other: Formula) -> Formula:
        if isinstance(other, OrFormula):
            return other | self
        elif isinstance(self,FalseFormula):
            other.orphan_vars |= self.free_vars()
            return other
        elif isinstance(other,FalseFormula):
            self.orphan_vars |= other.free_vars()
            return self
        elif isinstance(self,TrueFormula) or isinstance(other,TrueFormula):
            return true(self.free_vars() | other.free_vars())
        elif self == -other:
            return true(self.free_vars() | other.free_vars())

        key = (id(self), id(other)) if id(self) < id(other) else (id(other), id(self))
        if key not in _formula_or_cache:
            _formula_or_cache[key] = OrFormula([self, other])
        return _formula_or_cache[key]

    def __neg__(self):
        if isinstance(self,TrueFormula):
            return false(self.free_vars())
        elif isinstance(self,FalseFormula):
            return true(self.free_vars())

        return NegFormula(self)

    def free_vars(self) -> Set[Variable]:
        return self.orphan_vars

    def fresh_variable(self) -> Variable:
        # devuelve una variable fresca para usar en la formula sin conflictos
        raise NotImplementedError

    def variables_in(self) -> Set[Variable]:
        # devuelve las variables que se usan en la formula, libres o no.
        return self.free_vars()

    def __bool__(self) -> bool:
        return isinstance(self, TrueFormula)

    def satisfy(self, model: Any, vector: dict) -> bool:
        raise NotImplementedError

    def __eq__(self, other: object) -> bool:
        return hash(self) == hash(other)

    def __hash__(self) -> int:
        raise NotImplementedError

    def implied_declaration(self) -> list[Variable]:
        return sorted(self.free_vars())

    def extension(self, model: Any, arity: int | None = None) -> set:
        result = set()
        vs = self.implied_declaration()
        for t in product(model.universe,repeat=len(vs)):
            if self.satisfy(model,{vs[i]:t[i] for i in range(len(t))}):
                result.add(t)
        return result

class NegFormula(Formula):
    """
    Negacion de una formula
    """
    f: Formula

    def __init__(self, f: Formula, orphan_vars: Set[Variable] | None = None) -> None:
        super().__init__(orphan_vars if orphan_vars is not None else set())
        self.f = f

    def __repr__(self) -> str:
        return "¬ %s (%s)" % (self.f, ",".join(str(x) for x in self.free_vars()))

    def __neg__(self) -> Formula:
        return self.f

    def __hash__(self) -> int:
        return hash(("-", self.f))

    def free_vars(self) -> Set[Variable]:
        return self.f.free_vars()

    def variables_in(self) -> Set[Variable]:
        return self.f.variables_in()

    def satisfy(self, model: Any, vector: dict) -> bool:
        return not self.f.satisfy(model,vector)


class BinaryOpFormula(Formula):
    """
    Clase general de las formulas tipo f1 η ... η fn
    """
    subformulas: frozenset

    def __init__(self, subformulas: Iterable[Formula], orphan_vars: Set[Variable] | None = None) -> None:
        super().__init__(orphan_vars if orphan_vars is not None else set())
        self.subformulas = frozenset(subformulas)

    def free_vars(self) -> Set[Variable]:
        result = set()
        for f in self.subformulas:
            result = result.union(f.free_vars())
        return result
        
    def variables_in(self) -> Set[Variable]:
        result: Set[Variable] = set()
        for f in self.subformulas:
            result = result.union(f.variables_in())
        return result


class OrFormula(BinaryOpFormula):
    """
    Disjuncion entre formulas
    """
    def __hash__(self) -> int:
        return hash(("or",self.subformulas))
            
    def __repr__(self):
        result = " ∨ ".join(sorted(str(f) for f in self.subformulas))
        result = "(" + result + ")"
        result += "(" + ",".join(str(x) for x in self.free_vars()) + ")"
        return result

    def __or__(self, other: Formula) -> Formula:
        if isinstance(self, FalseFormula):
            other.orphan_vars |= self.free_vars()
            return other
        elif isinstance(other,FalseFormula):
            self.orphan_vars |= other.free_vars()
            return self
        elif isinstance(other,OrFormula):
            for a in self.subformulas:
                if -a in other.subformulas:
                    return true(self.free_vars()|other.free_vars())
            return OrFormula(self.subformulas | other.subformulas)
        elif -other in self.subformulas:
            return true(self.free_vars()|other.free_vars())
        return OrFormula(self.subformulas | {other})

    def satisfy(self,model,vector):
        # el or y el and de python son lazy
        return any(f.satisfy(model,vector) for f in self.subformulas)

class AndFormula(BinaryOpFormula):
    """
    Conjuncion entre formulas
    """
    def __hash__(self) -> int:
        return hash(("and",self.subformulas))
                
    def __repr__(self):
        result = " ∧ ".join(sorted(str(f) for f in self.subformulas))
        result = "(" + result + ")"
        result += "(" + ",".join(str(x) for x in self.free_vars()) + ")"
        return result

    def __and__(self, other: Formula) -> Formula:
        if isinstance(self, TrueFormula):
            other.orphan_vars |= self.free_vars()
            return other
        elif isinstance(other,TrueFormula):
            self.orphan_vars |= other.free_vars()
            return self
        elif isinstance(other,AndFormula):
            for a in self.subformulas:
                if -a in other.subformulas:
                    return false(self.free_vars()|other.free_vars())
            return AndFormula(self.subformulas | other.subformulas)
        elif -other in self.subformulas:
            return false(self.free_vars()|other.free_vars())
        return AndFormula(self.subformulas | {other})

    def satisfy(self, model: Any, vector: dict) -> bool:
        # el or y el and de python son lazy
        return all(f.satisfy(model, vector) for f in self.subformulas)


class RelSym(object):
    """
    Simbolo de relacion de primer orden
    """
    rel: str
    arity: int

    def __init__(self, rel: str, arity: int) -> None:
        self.rel = rel
        self.arity = arity

    def __call__(self, *args: Term) -> RelFormula:
        if len(args) != self.arity:
            raise ValueError("Arity not correct")
        for a in args:
            if not isinstance(a, Term):
                raise ValueError("%s isn't a term" % a)

        return RelFormula(self,args)

    def __repr__(self):
        return self.rel
    def __hash__(self) -> int:
        return hash((self.rel,self.arity))

class RelFormula(Formula):
    """
    Formula de primer orden de la aplicacion de una relacion
    """
    sym: RelSym
    args: tuple

    def __init__(self, sym: RelSym, args: tuple, orphan_vars: Set[Variable] | None = None) -> None:
        super().__init__(orphan_vars if orphan_vars is not None else set())
        self.sym = sym
        self.args = args

    def __repr__(self) -> str:
        result = repr(self.sym)
        result += "("
        result += ", ".join(map(repr,self.args))
        result += ")"
        result += "(" + ",".join(str(x) for x in self.free_vars()) + ")"
        return result

    def free_vars(self) -> Set[Variable]:
        return set.union(*[f.free_vars() for f in self.args])

    def variables_in(self) -> Set[Variable]:
        return self.free_vars()

    def satisfy(self, model: Any, vector: dict) -> bool:
        args = [t.evaluate(model,vector) for t in self.args]
        return model.relations[self.sym.rel](*args)
    def __hash__(self) -> int:
        return hash((self.sym,self.args))

class EqFormula(Formula):
    """
    Formula de primer orden que es una igualdad entre terminos
    """
    t1: Term
    t2: Term

    def __init__(self, t1: Term, t2: Term, orphan_vars: Set[Variable] | None = None) -> None:
        super().__init__(orphan_vars)
        if not (isinstance(t1, Term) and isinstance(t2, Term)):
            raise ValueError("Must be terms:%s %s" % (t1,t2))
        if t2 < t1:
            t1, t2 = t2, t1
        self.t1 = t1
        self.t2 = t2

    def __repr__(self) -> str:
        return "%s == %s (%s)" % (self.t1,self.t2, ",".join(str(x) for x in self.free_vars()))

    def free_vars(self) -> Set[Variable]:
        return set.union(self.t1.free_vars(), self.t2.free_vars())

    def variables_in(self) -> Set[Variable]:
        return self.free_vars()

    def satisfy(self, model: Any, vector: dict) -> bool:
        return self.t1.evaluate(model,vector) == self.t2.evaluate(model,vector)
    def __hash__(self) -> int:
        return hash((self.t1,self.t2))

class QuantifierFormula(Formula):
    """
    Clase general de una formula con cuantificador
    """
    var: Variable
    f: Formula

    def __init__(self, var: Variable, f: Formula, orphan_vars: Set[Variable] | None = None) -> None:
        super().__init__(orphan_vars if orphan_vars is not None else set())
        self.var = var
        self.f = f

    def free_vars(self) -> Set[Variable]:
        return self.f.free_vars() - {self.var}

    def variables_in(self) -> Set[Variable]:
        return set.union(self.f.free_vars(), {self.var})


class ForAllFormula(QuantifierFormula):
    """
    Formula Universal
    """
    def __repr__(self):
        return "∀ %s %s (%s)" % (self.var, self.f, ",".join(str(x) for x in self.free_vars()))

    def satisfy(self, model: Any, vector: dict) -> bool:
        for i in model.universe:
            vector[self.var] = i
            if not self.f.satisfy(model,vector):
                return False
        return True
    def __hash__(self) -> int:
        return hash(("forall",self.var,self.f))

class ExistsFormula(QuantifierFormula):
    """
    Formula Existencial
    """
    def __repr__(self):
        return "∃ %s %s (%s)" % (self.var, self.f, ",".join(str(x) for x in self.free_vars()))

    def satisfy(self, model: Any, vector: dict) -> bool:
        vector = vector.copy()
        for i in model.universe:
            vector[self.var] = i
            if self.f.satisfy(model,vector):
                return True
        return False
    def __hash__(self) -> int:
        return hash(("exists",self.var,self.f))

class TrueFormula(Formula):
    """
    Formula de primer orden constantemente verdadera
    """

    def __repr__(self):
        return "⊤(%s)" % ",".join(str(x) for x in self.free_vars())

    def satisfy(self, model: Any, vector: dict) -> bool:
        return True
    def __bool__(self):
        return True

    def extension(self, model: Any, arity: int | None = None) -> set:
        if arity is None:
            raise ValueError("Extension of a non declared formula")
        return set(product(model.universe, repeat=arity))
    def __hash__(self) -> int:
        return hash(repr(self))

class FalseFormula(Formula):
    """
    Formula de primer orden constantemente falsa
    """

    def __repr__(self):
        return "⊥(%s)" % ",".join(str(x) for x in self.free_vars())

    def satisfy(self, model: Any, vector: dict) -> bool:
        return False
    def __bool__(self):
        return False
    def extension(self, model: Any, arity: int | None = None) -> set:
        if arity is None:
            raise ValueError("Extension of a non declared formula")
        return set()
    def __hash__(self) -> int:
        return hash(repr(self))
# Shortcuts

def variables(*lvars: str | int) -> list[Variable]:
    """
    Declara variables de primer orden
    """
    return [Variable(x) for x in lvars]

def forall(var: Variable, formula: Formula) -> ForAllFormula:
    """
    Devuelve la formula universal
    """
    return ForAllFormula(var, formula)

def eq(t1: Term, t2: Term) -> Formula | EqFormula:
    if hash(t1)==hash(t2):
        return true(t1.free_vars()|t2.free_vars())
    return EqFormula(t1,t2)

def exists(var: Variable, formula: Formula) -> ExistsFormula:
    """
    Devuelve la formula existencial
    """
    return ExistsFormula(var, formula)

def true(orphan_vars: Set[Variable] | None = None) -> TrueFormula:
    """
    Devuelve la formula True
    """
    return TrueFormula(orphan_vars if orphan_vars is not None else set())


def false(orphan_vars: Set[Variable] | None = None) -> FalseFormula:
    """
    Devuelve la formula False
    """
    return FalseFormula(orphan_vars if orphan_vars is not None else set())


# Formulas generators

def grafico(term: Term, vs: list[Variable], model: Any) -> tuple[tuple[tuple[Any, ...], Any], ...]:
    result = {}
    for tupla in product(model.universe, repeat=len(vs)):
        result[tupla] = term.evaluate(model,{v:a for v,a in zip(vs,tupla)})
    return tuple(sorted(result.items()))

def generate_terms(funtions: Iterable[OpSym], vs: list[Variable], model: Any) -> list[Term]:
    """
    Devuelve todos los terminos (en realidad solo para infimo y supremo)
    usando las funciones y las variables con un anidaminento de rec
    """
    result = []
    graficos = set()

    for v in vs:
        g = grafico(v,vs,model)
        if not g in graficos:
            result.append(v)
            graficos.add(g)
    nuevos=[1]
    while nuevos:
        nuevos =[]
        for f in funtions:
            for ts in product(result,repeat=f.arity):
                g = grafico(f(*ts),vs,model)
                if not g in graficos:
                    nuevos.append(f(*ts))
                    graficos.add(g)
            result += nuevos
    return result

def atomics(relations: Iterable[RelSym], terms: list[Term], equality: bool = True) -> Iterator[Formula]:
    """
    Genera todas las formulas atomicas con relations
    de arity variables libres

    >>> R = RelSym("R",2)
    >>> vs = variables(*range(2))
    >>> list(atomics([R],vs))
    [R(x₀, x₀), R(x₀, x₁), R(x₁, x₀), R(x₁, x₁), x₀ == x₁]
    >>> list(atomics([R],vs,equality=False))
    [R(x₀, x₀), R(x₀, x₁), R(x₁, x₀), R(x₁, x₁)]
    """
    terms
    for r in relations:
        for t in product(terms,repeat=r.arity):
            yield r(*t)

    if equality:
        for t in combinations(terms,2):
            yield eq(*t)

def fo_type_to_relsym(fo_type: Any) -> list[RelSym]:
    """
    Devuelve una lista de RelSym para un tipo
    """
    result = []
    for r in fo_type.relations:
        result.append(RelSym(r,fo_type.relations[r]))

    return result

def fo_type_to_opsym(fo_type: Any) -> list[OpSym]:
    """
    Devuelve una lista de OpSym para un tipo
    """
    result = []
    for f in fo_type.operations:
        result.append(OpSym(f,fo_type.operations[f]))

    return result

def bolsas(model: Any, arity: int) -> dict[Formula, list]:
    """
    Algoritmo estilo Carlos para generar el algebra de lindenbaum
    de abiertas definibles en el modelo con la aridad dada

    >>> from . import fotheories
    >>> j=fotheories.SetsED.find_models(4)[2]
    >>> r = RelSym("r",1)
    >>> x0, = variables(0)
    >>> bolsas(j,1) == {- r(x0): [(0,)], r(x0): [(1,), (2,), (3,)]}
    True
    """
    result = {true(): list(product(model.universe,repeat=arity))}
    vs = variables(*range(arity))
    # lo comentado es para usar terminos con funciones y no solo variables
    terms = generate_terms(fo_type_to_opsym(model.fo_type),vs,model)
    formulas = atomics(fo_type_to_relsym(model.fo_type),terms)
    for formula in formulas:
        nuevas = defaultdict(list)
        for foriginal,bolsa in result.items():
            for tupla in bolsa:
                # TODO CUANDO UNA FORMULA NO TIENE NADIE QUE LA SATISFACE
                # O TODOS LA SATISFACEN, NO VALE LA PENA AGREGARLA
                if formula.satisfy(model,{v:i for v,i in zip(vs, tupla)}):
                    nuevas[foriginal & formula].append(tupla)
                else:
                    nuevas[foriginal & (-formula)].append(tupla)
        result = nuevas

    return dict(result)













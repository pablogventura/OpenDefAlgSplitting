# -*- coding: utf-8 -*-
# !/usr/bin/env python

"""
Modulo para calcular HIT de una tupla en un modelo
"""
import sys
from first_order import formulas
from itertools import product, tee, permutations, chain
from collections import defaultdict
from parser.parser import parser
from time import time
from misc import indent
import sys
import datetime

from termcolor import colored
from math import log2

global model
# Si True, en cada paso se elige (op, ti) que maximiza information gain.
# Si False, se usa el orden fijo del generador (comportamiento original).
USE_INFORMATION_GAIN = False


def _entropy(in_count, out_count):
    """Entropía de una partición binaria (in target / out of target). 0*log2(0) := 0."""
    n = in_count + out_count
    if n == 0:
        return 0.0
    p = in_count / n
    q = out_count / n
    return -(p * log2(p) if p > 0 else 0) - (q * log2(q) if q > 0 else 0)


def _information_gain(tuples, partition):
    """
    Information gain de una partición respecto a la polaridad (in/out of target).
    partition: dict (index, polarity) -> list of TupleHistory
    """
    n = len(tuples)
    if n == 0:
        return 0.0
    in_total = sum(1 for th in tuples if all(tg or tg is None for tg in th.polarity))
    out_total = n - in_total
    h_before = _entropy(in_total, out_total)
    h_after = 0.0
    for group in partition.values():
        g_in = sum(1 for th in group if all(tg or tg is None for tg in th.polarity))
        g_out = len(group) - g_in
        h_after += (len(group) / n) * _entropy(g_in, g_out)
    return h_before - h_after

def check_formula(formula, target):
    extension = formula.extension(model, target.arity)
    target = set(target.r)
    if target == extension:
        print(colored("Formula successfully checked","green"))
        #print("Extension:")
        #for t in (extension):
        #    print(" ".join(str(e) for e in t))
        #print("Target:")
        #for t in (target):
        #    print(" ".join(str(e) for e in t))
    else:
        print("Extension len: %s" % len(extension))
        print("Target len:    %s" % len(target))
        print("Intersection len:    %s" % len(target.intersection(extension)))
        print("Faltan:")
        for t in (target - extension):
            print(" ".join(str(e) for e in t))
        print("Sobran:")
        for t in (extension - target):
            print(" ".join(str(e) for e in t))
        print(colored("Formula failed!","red"))
    print("#"*80)


class Counterexample(Exception):
    def __init__(self, a):
        super(Counterexample, self).__init__(repr(a))


def permutations_forced(not_forced_elems, forced_elems, repeat):
    for t in product(not_forced_elems + forced_elems, repeat=repeat):
        if any(e in forced_elems for e in t):
            yield t


class TupleHistory:
    """
    Clase de la tupla con su historia
    Recibe una tupla de indices desde el generador y hace crecer la historia
    """
    
    def __init__(self, t, targets):
        self.t = t
        self.history = list(t)
        self.polarity = tuple(tg(*t) for tg in targets)
        self.has_generated = False  # TODO capaz si genero al arrancar
    
    def __eq__(self, other):
        return self.t == other.t and self.history == other.history
    
    def step(self, op, ti):
        """
        Toma la operacion y la tupla de indices
        Devuelve el indice devuelto
        """
        x = op(*[self.history[i] for i in ti])  # resultado de la operacion
        try:
            xi = self.history.index(x)  # indice en la historia
            self.has_generated = False
            return xi
        except ValueError:
            self.history.append(x)
            self.has_generated = True
            return len(self.history) - 1

    def simulate_step(self, op, ti):
        """
        Como step() pero sin mutar: devuelve (indice, has_generated).
        Sirve para predecir la partición y calcular information gain.
        """
        x = op(*[self.history[i] for i in ti])
        try:
            xi = self.history.index(x)
            return (xi, False)
        except ValueError:
            return (len(self.history), True)
    
    def __hash__(self):
        return hash((self.t, tuple(self.history)))
    
    def __repr__(self):
        return "TupleHistory(t=%s,h=%s,p=%s)" % (self.t, self.history, self.polarity)


class IndicesTupleGenerator:
    """
    Clase de HIT pero de indices, toma un modelo ambiente y la tupla generadora
    """
    
    def __init__(self, operations, arity, generator, viejos, nuevos, sintactico=[], last_term=None):
        """
        Devuelve tuplas para hacer HIT parcial de indices
        En operaciones estan las operaciones del modelo de la aridad arity
        generator es el generador de tuplas heredado, si se tiene que volver a calcular viene None
        viejos son los elementos viejos
        nuevos son los elementos que se estan generando ahora
        """
        self.sintactico = sintactico
        self.viejos = viejos
        self.nuevos = nuevos
        self.arity = arity
        self.ops = operations
        self.forked = False
        self.finished = False
        self.last_term = last_term  # ultima term
        
        assert type(self.ops) == dict
        
        if generator is None:
            if 0 in self.ops:
                #hay constantes entre las operaciones
                self.generator = ((constant,tuple()) for constant in self.ops[0])
            else:
                self.generator = iter([])
            #chain(*[product(self.ops[arity], permutations_forced(self.viejos, self.nuevos, arity)) for arity in sorted(self.ops.keys())])
        else:
            self.generator = generator

    
    def step(self):
        if self.forked:
            raise ValueError("This generator was forked!")
        while not self.finished:
            try:
                f, ti = next(self.generator)
                fsym = formulas.OpSym(f.sym, f.arity)
                self.last_term = fsym(*[self.sintactico[i] for i in ti])
                return (f, ti)  # devuelve la operacion y la tupla de indices
            
            except StopIteration:
                if self.nuevos:
                    self.generator = chain(*[product(self.ops[arity], permutations_forced(self.viejos, self.nuevos, arity)) for arity in self.ops])
                    self.viejos += self.nuevos
                    self.nuevos = []  # todos se gastaron para hacer el nuevo generador
                    self.finished = False
                else:
                    self.finished = True
    
    def enumerate_candidates(self):
        """
        Lista de (op, ti) posibles para el estado actual (viejos, nuevos).
        Usado para elegir el paso por information gain en lugar del orden fijo.
        """
        return list(chain(*[
            product(self.ops[arity], permutations_forced(self.viejos, self.nuevos, arity))
            for arity in self.ops
        ]))

    def set_last_term(self, op, ti):
        """Fija last_term para la (op, ti) elegida (p. ej. por IG) antes de aplicar el paso."""
        fsym = formulas.OpSym(op.sym, op.arity)
        self.last_term = fsym(*[self.sintactico[i] for i in ti])

    def formula_diferenciadora(self, index):
        """Asumo que acaban de diferenciarse"""
        return formulas.eq(self.last_term, self.sintactico[index])
    
    def hubo_nuevo(self):
        if self.forked:
            raise ValueError("This generator was forked!")
        self.nuevos.append(len(self.viejos) + len(self.nuevos))
        self.sintactico.append(self.last_term)
    
    def fork(self, quantity):
        if self.forked:
            raise ValueError("This generator was forked!")
        self.forked = True
        result = []
        generators = tee(self.generator, quantity)
        for i in range(quantity):
            result.append(
                IndicesTupleGenerator(self.ops, self.arity, generators[i], list(self.viejos), list(self.nuevos),
                                      list(self.sintactico), self.last_term)) # TODO FALTA USAR LAS VARIABLES ORIGINALES?
        return result


class Block():
    """
    Clase del bloque que va llevando el mismo hit
    """
    
    def __init__(self, operations, tuples, targets, generator=None, formula=None, fs=None):
        """
        :param tuples_in_targets: tuplas en el target
        :param tuples_out_targets: tuplas fuera del target
        :param targets: relacion target
        """
        self.targets = targets
        self.operations = operations
        self.tuples = tuples
        self.arity = targets[0].arity
        if formula is None:
            raise NotImplemented("Bloque sin formula original proveniente del preprocesamiento")
            self.formula = formulas.true()
            self.fs = [formulas.true()] * len(self.targets)
        else:
            self.formula = formula
            self.fs = fs
        if generator is None:
            assert len(self.formula.free_vars()) > 0
            self.generator = IndicesTupleGenerator(self.operations, self.arity, None, [], list(range(self.arity)),
                                                   sorted(self.formula.free_vars()))
        else:
            self.generator = generator
    
    def finished(self):
        return self.generator.finished
        
    
    def is_all_in_targets(self):
        return all(tg or tg is None for th in self.tuples for tg in th.polarity)
    
    def is_disjunt_to_targets(self):
        return all((not tg) or tg is None for th in self.tuples for tg in th.polarity)
    
    def step(self):
        """
        Hace un paso en hit a todas las tuplas.
        Si USE_INFORMATION_GAIN: elige (op, ti) que maximiza information gain.
        Si no: usa el orden fijo del generador (comportamiento original).
        Devuelve una lista de nuevos bloques.
        """
        if USE_INFORMATION_GAIN:
            candidates = self.generator.enumerate_candidates()
            if not candidates:
                self.generator.finished = True
                return [self]
            best_ig = -1.0
            best_op, best_ti = None, None
            for op, ti in candidates:
                part = defaultdict(list)
                for th in self.tuples:
                    idx, _ = th.simulate_step(op, ti)
                    part[(idx, th.polarity)].append(th)
                ig = _information_gain(self.tuples, part)
                if ig > best_ig:
                    best_ig = ig
                    best_op, best_ti = op, ti
            op, ti = best_op, best_ti
            self.generator.set_last_term(op, ti)
        else:
            result = defaultdict(lambda: defaultdict(list))
            try:
                op, ti = self.generator.step()
            except TypeError:
                assert self.generator.finished
                return [self]
            for th in self.tuples:
                result[th.step(op, ti)][th.polarity].append(th)
            if len(result.keys()) == 1:
                for i, index in enumerate(result.keys()):
                    tuples_new_block = result[index]
                    if any(th[0].has_generated for th in tuples_new_block.values()):
                        assert all(th[0].has_generated for th in tuples_new_block.values())
                        self.generator.hubo_nuevo()
                return [self]
            generators = self.generator.fork(len(result.keys()))
            results = []
            fneg = formulas.true()
            negados = []
            for i, index in enumerate(result.keys()):
                tuples_new_block = result[index]
                if any(th[0].has_generated for th in tuples_new_block.values()):
                    generators[i].hubo_nuevo()
                    negados.append((i, index))
                else:
                    f = self.formula & generators[i].formula_diferenciadora(index)
                    fneg = fneg & -generators[i].formula_diferenciadora(index)
                    tuples_new_block = [th for l in tuples_new_block.values() for th in l]
                    results.append(Block(self.operations, tuples_new_block, self.targets, generators[i], f, self.fs))
            for i, index in negados:
                tuples_new_block = result[index]
                tuples_new_block = [th for l in tuples_new_block.values() for th in l]
                f = self.formula & fneg
                results.append(Block(self.operations, tuples_new_block, self.targets, generators[i], f, self.fs))
            return results

        result = defaultdict(lambda: defaultdict(list))
        for th in self.tuples:
            result[th.step(op, ti)][th.polarity].append(th)

        if len(result.keys()) == 1:
            # todas las tuplas avanzaron en el bloque pero no se dividio
            # no hay necesidad de tocar formulas, porque todo se sigue cumpliendo
            # TODO ACA ANOTO SI HUBO NUEVO
            for i, index in enumerate(result.keys()):
                tuples_new_block = result[index]
                if any(th[0].has_generated for th in tuples_new_block.values()):
                    assert all(th[0].has_generated for th in tuples_new_block.values())
                    self.generator.hubo_nuevo()
            return [self]
        else:
            # el bloque se ha dividido
            generators = self.generator.fork(len(result.keys()))
            results = []
            fneg = formulas.true()
            negados = []
            for i, index in enumerate(result.keys()):
                tuples_new_block = result[index]
                if any(th[0].has_generated for th in tuples_new_block.values()):
                    # alguien genero dentro del bloque (todos generan)
                    generators[i].hubo_nuevo()
                    negados.append((i,index)) # se genero alguien distinto a todos
                else:
                    # el bloque no ha generado nada nuevo
                    f = self.formula & generators[i].formula_diferenciadora(index)  # formula valida
                    
                    fneg = fneg & -generators[i].formula_diferenciadora(index)
                    # estoy diciendo que es distinto solamente los que se acaban de dividir
                    # es suficiente porque la formula del bloque que se esta dividiendo
                    # implica que es distinto a los demas
                    tuples_new_block = [th for l in tuples_new_block.values() for th in l]
                    results.append(Block(self.operations, tuples_new_block, self.targets, generators[i], f, self.fs))
            for i, index in negados:
                tuples_new_block = result[index]
                tuples_new_block = [th for l in tuples_new_block.values() for th in l]
                f = self.formula & fneg
                results.append(Block(self.operations, tuples_new_block, self.targets, generators[i], f, self.fs))
                
            return results
    
    def __repr__(self):
        result = "Block(\n"
        for tuple in self.tuples:
            result += indent(tuple) + "\n"
        result += indent(self.formula) + "\n"
        result += ")\n"
        return result


def is_open_def_iterative(block):
    """
    Versión iterativa del algoritmo: misma lógica que is_open_def_recursive
    pero con pila explícita para evitar límite de recursión de Python.
    """
    stack = [("block", block)]
    accumulate_stack = []  # list of (children, results_list) waiting for more results
    while stack:
        tag, top = stack.pop()
        if tag == "block":
            b = top
            if b.is_all_in_targets():
                stack.append(("result", b.formula))
            elif b.is_disjunt_to_targets():
                stack.append(("result", formulas.false()))
            elif b.finished():
                raise Counterexample(b.tuples)
            else:
                children = b.step()
                if len(children) == 1 and children[0] is b:
                    stack.append(("block", b))
                else:
                    accumulate_stack.append((children, []))
                    for child in reversed(children):
                        stack.append(("block", child))
        elif tag == "result":
            formula = top
            if not accumulate_stack:
                return formula
            children, results = accumulate_stack.pop()
            results.append(formula)
            if len(results) == len(children):
                combined = formulas.false()
                for r in results:
                    combined = combined | r
                stack.append(("result", combined))
            else:
                accumulate_stack.append((children, results))
                stack.append(("block", children[len(results)]))
    return formulas.false()


def is_open_def(model, targets):
    targets = sorted(targets, key=lambda tg: tg.sym)
    #assert len(targets) == 1
    assert not model.relations
    
    tuples = set(TupleHistory(t, targets) for t in permutations(model.universe, r=targets[0].arity))
    operations = defaultdict(list)
    for op in model.operations.values():
        operations[op.arity].append(op)
    for arity in operations:
        operations[arity].sort(key=lambda o: o.sym)
    operations = dict(operations)
    start_block = Block(operations, tuples, targets, formula=targets[0].pattern.preprocessed_formula())
    # el posproceso debe reemplazar nombres no solo agregar una formula
    return is_open_def_iterative(start_block)


def main():
    assert sys.version_info >= (3, 7), "Need Python 3.7+"
    global model
    print_formulas = True
    check_solution = True
    check_partial_solutions = True
    today = datetime.datetime.today()
    print(today.strftime('%Y-%m-%d %H:%M:%S.%f'))
    try:
        model = parser(sys.argv[1], preprocess=True)
    except IndexError:
        model = parser()
    print("*" * 20)
    targets_rels = tuple(model.relations[sym] for sym in model.relations.keys() if sym[0] == "T")
    if not targets_rels:
        print("ERROR: NO TARGET RELATIONS FOUND")
        return
    targets = defaultdict(list)
    for t in targets_rels:
        del model.relations[t.sym]
        targets[t.arity].append(t)
    formula = formulas.false()
    start_hit = time()
    print("Deciding definability for subrelations")
    for arity in sorted(targets.keys()):
        targets_rels = targets[arity]
        if not targets_rels:
            print("ERROR: NO TARGET RELATIONS FOUND")
            return
        for target_rel in targets_rels:
            try:
                f = is_open_def(model, [target_rel])
                print("\t%s is definable" % targets[arity][0].sym)
                if print_formulas:
                    print("by %s" % f)
                if check_partial_solutions:
                    check_formula(f,target_rel)

                
                formula = formula | (f  & target_rel.pattern.postprocessed_formula()) # aca se posprocesa

            except Counterexample as e:
                print("NOT DEFINABLE")
                print("\tCounterexample: %s" % e)
                time_hit = time() - start_hit
                print("Elapsed time: %s" % time_hit)
                return
    print("DEFINABLE")
    if print_formulas:
        print("\t%s := %s" % (targets_rels[0].sym[:-2], formula))
    time_hit = time() - start_hit
    print("Elapsed time: %s" % time_hit)
    if check_solution:
        check_formula(formula,targets_rels[0].superrel)


if __name__ == "__main__":
    main()

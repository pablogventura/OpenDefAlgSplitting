# -*- coding: utf-8 -*-
# !/usr/bin/env python

"""
Modulo para calcular HIT de una tupla en un modelo
"""
from first_order import formulas
from itertools import product, tee, permutations, chain
from collections import defaultdict
from parser.parser import parser
from time import time
from misc import indent
import sys
import datetime

from termcolor import colored

global model

def check_formula(formula, target):
    print("#"*80)
    print("Probando formula:")
    print(formula)
    extension = formula.extension(model, target.arity)
    target = set(target.r)
    if target == extension:
        print(colored("Formula successfully checked","green"))
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
        print(colored("Formula failed!",red))
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
        if generator is None:
            self.generator = iter([])
        else:
            self.generator = generator
        self.nuevos = nuevos
        self.arity = arity
        self.ops = operations
        self.forked = False
        self.finished = False
        self.last_term = last_term  # ultima term
        
        assert type(self.ops) == dict
    
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
            self.generator = IndicesTupleGenerator(self.operations, self.arity, None, [], list(range(self.arity)),
                                                   sorted(self.formula.variables_in()))
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
        Hace un paso en hit a todas las tuplas
        Devuelve una lista de nuevos bloques
        """
        result = defaultdict(lambda: defaultdict(list))
        try:
            op, ti = self.generator.step()
        except TypeError:
            # step devolvio none, asi que ya termino
            assert self.generator.finished
            return [self]
        
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


def is_open_def_recursive(block):
    """
    Algoritmo "posta", es recursivo
        un bloque tiene tuplas acompañadas por su historia parcial y un hit parcial que etiqueta al bloque
    input: un bloque mixto
    output:
    """
    if block.is_all_in_targets():
        return block.formula
    elif block.is_disjunt_to_targets():
        return formulas.false()
    elif block.finished():
        raise Counterexample(block.tuples)
        # como es un bloque mixto, no es defel hit parcial esta terminado, no definible y termino
    blocks = block.step()
    formula = formulas.false()
    for b in blocks:
        recursive_call = is_open_def_recursive(b)
        formula = formula | recursive_call
    
    return formula


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
    return is_open_def_recursive(start_block)


def main():
    global model
    today = datetime.datetime.today()
    print(today.strftime('%Y-%m-%d %H:%M:%S.%f'))
    check_solution = True
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
                print("\t%s is definable by %s" % (targets[arity][0].sym,f))
                if True:
                    check_formula(f,target_rel)

                
                formula = formula | (f  & target_rel.pattern.postprocessed_formula()) # aca se posprocesa

            except Counterexample as e:
                print("NOT DEFINABLE")
                print("\tCounterexample: %s" % e)
                time_hit = time() - start_hit
                print("Elapsed time: %s" % time_hit)
                return
    print("DEFINABLE")
    print("\t%s := %s" % (targets_rels[0].sym[:-2], formula))
    time_hit = time() - start_hit
    print("Elapsed time: %s" % time_hit)
    if check_solution:
        check_formula(formula,targets_rels[0].superrel)


if __name__ == "__main__":
    main()

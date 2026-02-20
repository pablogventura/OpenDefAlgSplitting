# !/usr/bin/env python

"""
Modulo para calcular HIT de una tupla en un modelo
"""

from __future__ import annotations

import datetime
import sys
from collections import defaultdict
from itertools import chain, permutations, product, tee
from math import log2
from parser.parser import parser
from time import time
from typing import Any, Iterator

from termcolor import colored

from first_order import formulas
from misc import indent

global model
# Si True, en cada paso se elige (op, ti) que maximiza information gain.
# Si False, se usa el orden fijo del generador (comportamiento original).
USE_INFORMATION_GAIN = False
# Si USE_INFORMATION_GAIN y este valor es int, se muestrean solo K candidatos (None = todos).
IG_SAMPLE = 20


def _entropy(in_count: int, out_count: int) -> float:
    """Entropía de una partición binaria (in target / out of target). 0*log2(0) := 0."""
    n = in_count + out_count
    if n == 0:
        return 0.0
    p = in_count / n
    q = out_count / n
    return -(p * log2(p) if p > 0 else 0) - (q * log2(q) if q > 0 else 0)


def _information_gain_from_counts(
    n: int, in_total: int, partition_counts: dict[tuple, tuple[int, int]]
) -> float:
    """
    Information gain a partir de conteos.
    partition_counts: dict clave -> (in_count, out_count); la clave puede ser (index, in_target).
    """
    if n == 0:
        return 0.0
    out_total = n - in_total
    h_before = _entropy(in_total, out_total)
    h_after = 0.0
    for g_in, g_out in partition_counts.values():
        group_size = g_in + g_out
        h_after += (group_size / n) * _entropy(g_in, g_out)
    return h_before - h_after


def check_formula(formula: formulas.Formula, target: Any) -> None:
    extension = formula.extension(model, target.arity)
    target = set(target.r)
    if target == extension:
        print(colored("Formula successfully checked", "green"))
        # print("Extension:")
        # for t in (extension):
        #    print(" ".join(str(e) for e in t))
        # print("Target:")
        # for t in (target):
        #    print(" ".join(str(e) for e in t))
    else:
        print(f"Extension len: {len(extension)}")
        print(f"Target len:    {len(target)}")
        print(f"Intersection len:    {len(target.intersection(extension))}")
        print("Faltan:")
        for t in target - extension:
            print(" ".join(str(e) for e in t))
        print("Sobran:")
        for t in extension - target:
            print(" ".join(str(e) for e in t))
        print(colored("Formula failed!", "red"))
    print("#" * 80)


class Counterexample(Exception):
    def __init__(self, a: Any) -> None:
        super().__init__(repr(a))


def permutations_forced(not_forced_elems: list, forced_elems: list, repeat: int) -> Iterator[tuple]:
    for t in product(not_forced_elems + forced_elems, repeat=repeat):
        if any(e in forced_elems for e in t):
            yield t


class TupleHistory:
    """
    Clase de la tupla con su historia
    Recibe una tupla de indices desde el generador y hace crecer la historia.
    _index_map mantiene valor -> índice para búsqueda O(1).
    in_target cachea la polaridad (in/out of target) para evitar recalcular.
    """

    def __init__(self, t: tuple, targets: list) -> None:
        self.t = t
        self.history = list(t)
        self._index_map = {x: i for i, x in enumerate(self.history)}
        self.polarity = tuple(tg(*t) for tg in targets)
        self.in_target = all(tg or tg is None for tg in self.polarity)
        self.has_generated = False

    def __eq__(self, other: object) -> bool:
        return self.t == other.t and self.history == other.history

    def step(self, op: Any, ti: tuple[int, ...]) -> int:
        """
        Toma la operacion y la tupla de indices. Devuelve el indice devuelto.
        Búsqueda O(1) vía _index_map.
        """
        x = op(*[self.history[i] for i in ti])
        xi = self._index_map.get(x)
        if xi is not None:
            self.has_generated = False
            return xi
        self.history.append(x)
        self._index_map[x] = len(self.history) - 1
        self.has_generated = True
        return len(self.history) - 1

    def simulate_step(self, op: Any, ti: tuple[int, ...]) -> tuple[int, bool]:
        """
        Como step() pero sin mutar: devuelve (indice, has_generated).
        Usa _index_map para O(1).
        """
        x = op(*[self.history[i] for i in ti])
        xi = self._index_map.get(x)
        if xi is not None:
            return (xi, False)
        return (len(self.history), True)

    def __hash__(self) -> int:
        return hash((self.t, tuple(self.history)))

    def __repr__(self) -> str:
        return f"TupleHistory(t={self.t},h={self.history},p={self.polarity})"


class IndicesTupleGenerator:
    """
    Clase de HIT pero de indices, toma un modelo ambiente y la tupla generadora
    """

    def __init__(
        self,
        operations: dict,
        arity: int,
        generator: Iterator | None,
        viejos: list,
        nuevos: list,
        sintactico: list | None = None,
        last_term: Any = None,
    ) -> None:
        """
        Devuelve tuplas para hacer HIT parcial de indices
        En operaciones estan las operaciones del modelo de la aridad arity
        generator es el generador de tuplas heredado, si se tiene que volver a calcular viene None
        viejos son los elementos viejos
        nuevos son los elementos que se estan generando ahora
        """
        self.sintactico = sintactico if sintactico is not None else []
        self.viejos = viejos
        self.nuevos = nuevos
        self.arity = arity
        self.ops = operations
        self.forked = False
        self.finished = False
        self.last_term = last_term  # ultima term
        self._formula_diff_cache = {}  # (last_term, index) -> formula, para memoización

        assert isinstance(self.ops, dict)

        if generator is None:
            if 0 in self.ops:
                # hay constantes entre las operaciones
                self.generator = ((constant, ()) for constant in self.ops[0])
            else:
                self.generator = iter([])
            # chain(*[product(self.ops[arity], permutations_forced(self.viejos, self.nuevos, arity)) for arity in sorted(self.ops.keys())])
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
                    self.generator = chain(
                        *[
                            product(
                                self.ops[arity],
                                permutations_forced(self.viejos, self.nuevos, arity),
                            )
                            for arity in self.ops
                        ]
                    )
                    self.viejos += self.nuevos
                    self.nuevos = []  # todos se gastaron para hacer el nuevo generador
                    self.finished = False
                else:
                    self.finished = True

    def enumerate_candidates(self) -> Iterator[tuple[Any, tuple[int, ...]]]:
        """
        Generador de (op, ti) posibles para el estado actual (viejos, nuevos).
        Usado para elegir el paso por information gain; permite corte temprano sin materializar todos.
        """
        for arity in self.ops:
            yield from product(
                self.ops[arity],
                permutations_forced(self.viejos, self.nuevos, arity),
            )

    def set_last_term(self, op: Any, ti: tuple[int, ...]) -> None:
        """Fija last_term para la (op, ti) elegida (p. ej. por IG) antes de aplicar el paso."""
        fsym = formulas.OpSym(op.sym, op.arity)
        self.last_term = fsym(*[self.sintactico[i] for i in ti])

    def formula_diferenciadora(self, index: int) -> formulas.Formula:
        """Asumo que acaban de diferenciarse. Resultado memoizado."""
        key = (self.last_term, index)
        if key not in self._formula_diff_cache:
            self._formula_diff_cache[key] = formulas.eq(self.last_term, self.sintactico[index])
        return self._formula_diff_cache[key]

    def hubo_nuevo(self) -> None:
        if self.forked:
            raise ValueError("This generator was forked!")
        self.nuevos.append(len(self.viejos) + len(self.nuevos))
        self.sintactico.append(self.last_term)

    def fork(self, quantity: int) -> list[IndicesTupleGenerator]:
        if self.forked:
            raise ValueError("This generator was forked!")
        self.forked = True
        # tee() solo cuando hay split (quantity >= 2); cada hijo necesita su propia copia del iterador.
        result = []
        generators = tee(self.generator, quantity)
        for i in range(quantity):
            g = IndicesTupleGenerator(
                self.ops,
                self.arity,
                generators[i],
                list(self.viejos),
                list(self.nuevos),
                list(self.sintactico),
                self.last_term,
            )
            g._formula_diff_cache = dict(self._formula_diff_cache)
            result.append(g)
        return result


class Block:
    """
    Clase del bloque que va llevando el mismo hit
    """

    def __init__(
        self,
        operations: dict,
        tuples: list[TupleHistory],
        targets: list,
        generator: IndicesTupleGenerator | None = None,
        formula: formulas.Formula | None = None,
        fs: list | None = None,
    ) -> None:
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
            raise NotImplementedError(
                "Bloque sin formula original proveniente del preprocesamiento"
            )
            self.formula = formulas.true()
            self.fs = [formulas.true()] * len(self.targets)
        else:
            self.formula = formula
            self.fs = fs
        if generator is None:
            assert len(self.formula.free_vars()) > 0
            self.generator = IndicesTupleGenerator(
                self.operations,
                self.arity,
                None,
                [],
                list(range(self.arity)),
                sorted(self.formula.free_vars()),
            )
        else:
            self.generator = generator

    def finished(self) -> bool:
        return self.generator.finished

    def is_all_in_targets(self) -> bool:
        return all(th.in_target for th in self.tuples)

    def is_disjunt_to_targets(self) -> bool:
        return all(not th.in_target for th in self.tuples)

    def step(self) -> list[Block]:
        """
        Hace un paso en hit a todas las tuplas.
        Si USE_INFORMATION_GAIN: elige (op, ti) que maximiza information gain (conteos, corte temprano, muestreo).
        Si no: usa el orden fijo del generador.
        Un solo dict result[(index, polarity)] para la partición.
        """
        if USE_INFORMATION_GAIN:
            cand_gen = self.generator.enumerate_candidates()
            if IG_SAMPLE is not None:
                cand_list = list(cand_gen)
                if not cand_list:
                    self.generator.finished = True
                    return [self]
                cand_gen = iter(cand_list[:IG_SAMPLE])
            else:
                first = next(cand_gen, None)
                if first is None:
                    self.generator.finished = True
                    return [self]
                cand_gen = chain([first], cand_gen)
            n = len(self.tuples)
            in_total = sum(1 for th in self.tuples if th.in_target)
            h_before = _entropy(in_total, n - in_total) if n else 0.0
            best_ig = -1.0
            best_op, best_ti = None, None
            for op, ti in cand_gen:
                part = defaultdict(lambda: [0, 0])  # (index, in_target) -> [in_c, out_c]
                for th in self.tuples:
                    idx, _ = th.simulate_step(op, ti)
                    key = (idx, th.in_target)
                    if th.in_target:
                        part[key][0] += 1
                    else:
                        part[key][1] += 1
                ig = _information_gain_from_counts(
                    n, in_total, {k: tuple(v) for k, v in part.items()}
                )
                if ig > best_ig:
                    best_ig = ig
                    best_op, best_ti = op, ti
                    if h_before > 0 and ig >= h_before - 1e-12:
                        break
            if best_op is None:
                self.generator.finished = True
                return [self]
            op, ti = best_op, best_ti
            self.generator.set_last_term(op, ti)
        else:
            try:
                op, ti = self.generator.step()
            except TypeError:
                assert self.generator.finished
                return [self]
            any_has_gen = False
            result = defaultdict(list)
            for th in self.tuples:
                idx = th.step(op, ti)
                any_has_gen = any_has_gen or th.has_generated
                result[idx].append(th)
            if len(result) == 1:
                if any_has_gen:
                    self.generator.hubo_nuevo()
                return [self]
            generators = self.generator.fork(len(result))
            results = []
            fneg = formulas.true()
            negados = []
            for i, (index, tuples_new_block) in enumerate(result.items()):
                if any(th.has_generated for th in tuples_new_block):
                    generators[i].hubo_nuevo()
                    negados.append((i, index))
                else:
                    f = self.formula & generators[i].formula_diferenciadora(index)
                    fneg = fneg & -generators[i].formula_diferenciadora(index)
                    results.append(
                        Block(
                            self.operations,
                            tuples_new_block,
                            self.targets,
                            generators[i],
                            f,
                            self.fs,
                        )
                    )
            for i, index in negados:
                tuples_new_block = result[index]
                f = self.formula & fneg
                results.append(
                    Block(
                        self.operations, tuples_new_block, self.targets, generators[i], f, self.fs
                    )
                )
            return results

        result = defaultdict(list)
        for th in self.tuples:
            idx = th.step(op, ti)
            result[idx].append(th)
        if len(result) == 1:
            only_index = next(iter(result))
            if any(th.has_generated for th in result[only_index]):
                self.generator.hubo_nuevo()
            return [self]
        num_groups = len(result)
        generators = self.generator.fork(num_groups)
        results = []
        fneg = formulas.true()
        negados = []
        for i, (index, tuples_new_block) in enumerate(result.items()):
            if any(th.has_generated for th in tuples_new_block):
                generators[i].hubo_nuevo()
                negados.append((i, index))
            else:
                f = self.formula & generators[i].formula_diferenciadora(index)
                fneg = fneg & -generators[i].formula_diferenciadora(index)
                results.append(
                    Block(
                        self.operations, tuples_new_block, self.targets, generators[i], f, self.fs
                    )
                )
        for i, index in negados:
            tuples_new_block = result[index]
            f = self.formula & fneg
            results.append(
                Block(self.operations, tuples_new_block, self.targets, generators[i], f, self.fs)
            )
        return results

    def __repr__(self) -> str:
        result = "Block(\n"
        for tuple in self.tuples:
            result += indent(tuple) + "\n"
        result += indent(self.formula) + "\n"
        result += ")\n"
        return result


def is_open_def_iterative(block: Block) -> formulas.Formula:
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


def is_open_def(model: Any, targets: list) -> formulas.Formula:
    targets = sorted(targets, key=lambda tg: tg.sym)
    # assert len(targets) == 1
    assert not model.relations

    # Lista ordenada para iteración determinista (set daría orden arbitrario entre ejecuciones).
    tuples = sorted(
        [TupleHistory(t, targets) for t in permutations(model.universe, r=targets[0].arity)],
        key=lambda th: th.t,
    )
    operations = defaultdict(list)
    for op in model.operations.values():
        operations[op.arity].append(op)
    for arity in operations:
        operations[arity].sort(key=lambda o: o.sym)
    operations = dict(operations)
    start_block = Block(
        operations, tuples, targets, formula=targets[0].pattern.preprocessed_formula()
    )
    # el posproceso debe reemplazar nombres no solo agregar una formula
    return is_open_def_iterative(start_block)


def main() -> None:
    assert sys.version_info >= (3, 7), "Need Python 3.7+"
    global model
    print_formulas = True
    check_solution = True
    check_partial_solutions = False
    today = datetime.datetime.today()
    print(today.strftime("%Y-%m-%d %H:%M:%S.%f"))
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
                print(f"\t{targets[arity][0].sym} is definable")
                if print_formulas:
                    print(f"by {f}")
                if check_partial_solutions:
                    check_formula(f, target_rel)

                formula = formula | (
                    f & target_rel.pattern.postprocessed_formula()
                )  # aca se posprocesa

            except Counterexample as e:
                print("NOT DEFINABLE")
                print(f"\tCounterexample: {e}")
                time_hit = time() - start_hit
                print(f"Elapsed time: {time_hit}")
                return
    print("DEFINABLE")
    if print_formulas:
        print(f"\t{targets_rels[0].sym[:-2]} := {formula}")
    time_hit = time() - start_hit
    print(f"Elapsed time: {time_hit}")
    if check_solution:
        check_formula(formula, targets_rels[0].superrel)


if __name__ == "__main__":
    main()

# !/usr/bin/env python

from __future__ import annotations

import gzip
import sys
from parser import preprocessing  # type: ignore[reportAttributeAccessIssue]
from typing import Any, Callable

from first_order import formulas
from first_order.models import Model
from first_order.relops import Operation, Relation


class ParserError(Exception):
    """
    Sintax error while parsing
    """

    def __init__(self, line: int, path: str | None, message: str) -> None:
        super().__init__((f"Line {line} of {path}: ") + message)


def c_input(line: str | bytes) -> str:
    """
    Clean input
    """
    if isinstance(line, bytes):
        line = line.decode("utf-8")
    if "#" in line:
        idx = line.find("#")
        line = line[:idx] if idx >= 0 else line
    return line.strip()


def parse_universe(line: str) -> list:
    # el universo puede estar hecho de strings, de tuplas,etc
    return [eval(i) for i in line.split()]


def barajador(t: tuple, d1: tuple, d2: tuple) -> tuple:
    result = list(d1)
    for i, k in enumerate(d2):
        for j in (j for j, x in enumerate(d1) if x == k):
            result[j] = t[i]
    return tuple(result)


def parse_defformula(line: str, universe: list, relations: dict, operations: dict) -> Any:
    # R(x,y) m(x,y) == j(x,y)
    # print("%s interpreted as:" % line)
    if "==" in line:
        raise ValueError("Must use 'eq(x,y)' to represent 'x==y'")
    entorno = {}
    entorno["model"] = Model(universe, relations, operations)
    entorno.update({r: relations[r].syntax_sym for r in relations})
    entorno.update({f: operations[f].syntax_sym for f in operations})
    entorno["eq"] = formulas.eq
    sym, line = line.split("(", 1)
    declaracion, formula = line.split(")", 1)
    declaracion = declaracion.split(",")
    if len(declaracion) > len(set(declaracion)):
        raise ValueError("por ahora no se pueden declarar repitiendo variables")
    declaracion = tuple(formulas.Variable(v) for v in declaracion)

    vars = {v.sym: v for v in declaracion}
    entorno.update(vars)
    try:
        formula = eval(formula, globals(), entorno)
    except NameError as err:
        raise ValueError(f"Missing variables in the declaration of {sym}") from err
    if set(formula.free_vars()) > set(declaracion):
        raise ValueError(f"Missing variables in the declaration of {sym}")
    if isinstance(formula, formulas.Formula):
        entorno["formula"] = formula
        entorno["arity"] = len(vars)

        valores = eval("formula.extension(model,arity)", globals(), entorno)
        decl_list = formula.implied_declaration()
        valores = {barajador(t, declaracion, tuple(decl_list)) for t in valores}
        if len(valores) == 0:
            print(f"WARNING: Formula {formula} has empty extension")
        result = Relation(sym, len(declaracion))
        result.r = valores

        return result
    elif isinstance(formula, formulas.Term):
        raise NotImplementedError("Functions declared by formula not implemented")


def parse_defrel(line: str) -> tuple[Any, int]:
    sym, ntuples, arity = line.split()
    ntuples, arity = int(ntuples), int(arity)
    if arity == 0:
        raise ValueError(
            f"{sym} is 0-arity relation, to declare a constant declare 0-arity operation"
        )
    return Relation(sym, arity), ntuples


def parse_defop(line: str) -> Any:
    sym, arity = line.split()
    arity = int(arity)
    return Operation(sym, arity)


def parse_tuple(line: str, universe: list) -> tuple:
    t = tuple(map(eval, line.split()))
    assert all(i in universe for i in t), f"Tuple {t} is not in the universe {universe}"
    return t


def random_delete_tuples(number: int) -> Callable[[Any], Any]:
    def f(rel):
        for _i in range(number):
            rel.r.pop()
        return rel

    return f


def parser(path: str | None = None, preprocess: bool = True, verbose: bool = True) -> Model:
    """
    New parser
    """
    if path:
        try:
            f = gzip.open(path, "rb")
            f.readline()
            f.seek(0)
        except FileNotFoundError:
            raise ParserError(-1, path, "File missing") from None
        except OSError:
            f = open(path, "rb")
    else:
        f = sys.stdin
    relations = {}
    operations = {}
    current_rel = None
    current_op = None
    rel_missing_tuples = 0
    op_missing_tuples = 0
    universe = None
    decorators = []
    linenumber = -1
    for linenumber, line in enumerate(f):
        assert current_op is None or current_rel is None
        try:
            line = c_input(line)
            if line:
                if universe is None:
                    # tiene que ser el universo!
                    universe = parse_universe(line)
                elif current_rel is None and current_op is None:
                    if "@" in line:
                        decorators.append(eval(line[1:]))
                    elif "(" in line:
                        # empieza una relacion u operacion definida por formula
                        relop = parse_defformula(line, universe, relations, operations)
                        if isinstance(relop, Relation):
                            for d in reversed(decorators):
                                relop = d(relop)
                            decorators = []
                            relations[relop.sym] = relop

                            # TODO APLICACION DE DECORADORES
                        else:
                            operations[relop.sym] = relop
                            # TODO APLICACION DE DECORADORES
                    elif line.count(" ") == 1:
                        # empieza una operacion
                        current_op = parse_defop(line)
                        op_missing_tuples = len(universe) ** current_op.arity
                        if verbose:
                            print(f"universe {universe}")
                            print(f"{current_op.sym} tuples: {op_missing_tuples}")
                    elif line.count(" ") == 2:
                        # empieza una relacion
                        current_rel, rel_missing_tuples = parse_defrel(line)
                        if verbose:
                            try:
                                print(
                                    "{} density: {:f}".format(
                                        current_rel.sym,
                                        float(rel_missing_tuples)
                                        / (len(universe) ** current_rel.arity),
                                    )
                                )
                            except (ValueError, ZeroDivisionError):
                                print("WARNING: no pudo calcular la densidad")
                else:
                    if current_rel is not None:
                        # continua una relacion
                        try:
                            if rel_missing_tuples:
                                current_rel.add(parse_tuple(line, universe))
                                rel_missing_tuples -= 1
                            if not rel_missing_tuples:
                                relations[current_rel.sym] = current_rel
                                current_rel = None
                                # TODO APLICACION DE DECORADORES
                        except ValueError:
                            print(line)
                            raise
                    elif current_op is not None:
                        # continua una operacion
                        if op_missing_tuples:
                            current_op.add(parse_tuple(line, universe))
                            op_missing_tuples -= 1
                        if not op_missing_tuples:
                            operations[current_op.sym] = current_op
                            current_op = None
                            # TODO APLICACION DE DECORADORES
        except Exception as e:
            raise ParserError(linenumber, path, str(e.args[0]) if e.args else str(e)) from e
            # TODO el manejo de errores no deberia imprimir excepciones por pantalla
    if universe is None:
        raise ParserError(linenumber, path, "Universe not defined")

    if current_rel is not None and rel_missing_tuples > 0:
        raise ParserError(linenumber, path, f"Missing tuples for relation {current_rel.sym}")
    if current_op is not None:
        raise ParserError(linenumber, path, f"Missing tuples for operation {current_op.sym}")

    if preprocess:
        prep_relations = set()
        for sym in relations:
            if sym.startswith("T"):
                rel = relations[sym]
                prep_relations = prep_relations.union(preprocessing.preprocesamiento2(rel))
        relations = {sym: relations[sym] for sym in relations if not sym.startswith("T")}
        if verbose:
            print(f"Target thinning turned T into {len(prep_relations)} Ts")
        for r in prep_relations:
            relations[r.sym] = r

    return Model(universe, relations, operations)


if __name__ == "__main__":
    MODEL = parser()
    # print(MODEL)

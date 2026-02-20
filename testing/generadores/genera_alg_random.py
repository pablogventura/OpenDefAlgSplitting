from __future__ import annotations

import datetime
import os
import sys
from itertools import product
from random import randint

if "OPENDEFALG_RANDOM_SEED" in os.environ:
    import random
    random.seed(int(os.environ["OPENDEFALG_RANDOM_SEED"]))


def generador(tA: int, t: int, c: int, fs: list[int]) -> None:
    # tA es el tamaño de la estructura ambiente
    # t es la cantidad de subconjuntos
    # c es el tamaño de esos subconjuntos
    # fs es una lista de aridades de funciones
    # fc es una lista de booleanos para hacer a la funcion hiperconmutativa
    print(f"# Generated {datetime.datetime.now():%Y-%m-%d %H:%M:%S}")
    print(
        f"# Parameters: |A| = {tA}, |MaxSubs| = {t}, |ms| = {c} with ms in MaxSubs, Arities = {fs}"
    )

    # result += "# Random Seed: %s\n" % seed
    cardinality = tA
    print(" ".join(str(e) for e in range(cardinality)))
    for i, arity in enumerate(fs):
        print(f"f{i} {arity}")
        for values in product(range(cardinality), repeat=arity):
            fvalues = randint(0, cardinality - 1)
            print(" ".join(str(e) for e in values) + f" {fvalues}")


def main() -> None:
    try:
        tA, t, c, fs = sys.argv[1:5]
        try:
            float(sys.argv[7])
        except (ValueError, IndexError):
            pass
        tA = int(tA)
        t = int(t)
        c = int(c)
        fs = [int(i) for i in eval(fs)]
        assert all(i >= 0 for i in fs)

    except (ValueError, IndexError):
        print(""" Toma en este orden:
            Cardinalidad de la estructura ambiente
            Cantidad de subuniversos
            Cardinalidad de esos subuniversos
            Lista de aridades de funciones base
            """)
        sys.exit(1)
    generador(tA, t, c, fs)


if __name__ == "__main__":
    main()

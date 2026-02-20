import os
import sys
from itertools import product
from random import sample

if "OPENDEFALG_RANDOM_SEED" in os.environ:
    import random

    random.seed(int(os.environ["OPENDEFALG_RANDOM_SEED"]))


def c_input(line):
    """
    Clean input
    """
    if "#" in line:
        line = line[: line.find("#")]
    return line.strip()


def parse_universe(line):
    # el universo puede estar hecho de strings, de tuplas,etc
    return [eval(i) for i in line.split()]


def random_target(universe, tarity, density):
    print(f"T0 {int((len(universe) ** tarity) * density)} {tarity}\n")
    for i in sample(
        list(product(universe, repeat=tarity)), int((len(universe) ** tarity) * density)
    ):
        print(" ".join(map(str, i)))


def main():
    try:
        universe_arg, arity, density = sys.argv[1:4]
        arity = int(arity)
        density = float(density)
    except (ValueError, IndexError):
        print(
            "Toma el universo (elementos separados por espacio) o cardinalidad, la aridad y la densidad del target aleatorio"
        )
        return
    # Si el primer arg tiene espacio, es universo explícito (p. ej. "0 2 4 5 6 7"); si no, es cardinalidad
    if " " in universe_arg:
        universe = parse_universe(universe_arg)
    else:
        cardinality = int(universe_arg)
        universe = list(range(cardinality))
    print(f"# Target random arity={arity}, density={density}")
    random_target(universe, arity, density)


if __name__ == "__main__":
    main()

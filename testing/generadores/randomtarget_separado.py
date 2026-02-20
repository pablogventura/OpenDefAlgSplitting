from random import sample
from itertools import product
import sys


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
    print("T0 %s %s\n" % (int((len(universe) ** tarity) * density), tarity))
    for i in sample(
        list(product(universe, repeat=tarity)), int((len(universe) ** tarity) * density)
    ):
        print(" ".join(map(str, i)))


def main():
    try:
        universe_arg, arity, density = sys.argv[1:4]
        arity = int(arity)
        density = float(density)
    except:
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
    print("# Target random arity=%s, density=%s" % (arity, density))
    random_target(universe, arity, density)


if __name__ == "__main__":
    main()

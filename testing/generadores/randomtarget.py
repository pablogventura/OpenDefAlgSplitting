import sys
from itertools import product
from random import sample


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
        arity, density = sys.argv[1:3]
        arity = int(arity)
        density = float(density)
    except (ValueError, IndexError):
        print(
            "Toma la aridad y la densidad del target aleatorio y lo agrega a un archivo model que este entrando por la stdin"
        )
        return
    print(f"# Target random arity={arity}, density={density}")
    universe = None
    try:
        while not universe:
            line = input()
            print(line)
            line = c_input(line)
            if line:
                universe = parse_universe(line)
        while True:
            line = input()
            print(line)
    except EOFError:
        random_target(universe, arity, density)


if __name__ == "__main__":
    main()

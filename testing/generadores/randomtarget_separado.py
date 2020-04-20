from random import sample
from itertools import product
import sys

def c_input(line):
    """
    Clean input
    """
    if "#" in line:
        line = line[:line.find("#")]
    return line.strip()

def parse_universe(line):
    # el universo puede estar hecho de strings, de tuplas,etc
    return [eval(i) for i in line.split()]

def random_target(universe,tarity,density):
    print("T0 %s %s\n" % (int((len(universe) ** tarity) * density), tarity))
    for i in sample(list(product(universe, repeat=tarity)), int((len(universe) ** tarity) * density)):
        print(" ".join(map(str, i)))

def main():
    try:
        cardinality,arity, density = sys.argv[1:4]
        arity = int(arity)
        density = float(density)
    except:
        print("Toma el tamaño del universo, la aridad y la densidad del target aleatorio y lo agrega a un archivo model que este entrando por la stdin")
        return
    print("# Target random arity=%s, density=%s" % (arity,density))
    universe = list(range(cardinality))
    random_target(universe,arity,density)


if __name__ == "__main__":
    main()

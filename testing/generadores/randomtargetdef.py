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

def random_term(ops):
    pass

def main():
    try:
        arity, density, syms, arities = sys.argv[1:5]
        arity = int(arity)
        density = float(density)
        print(syms)
        syms = eval(syms)
        arities = eval(arities)
        ops = []
        for s,a in zip(syms,arities):
            ops.append(OpSym(s,a))
        print(ops)
    except:
        raise
        print("Toma la aridad y la profundidad del target aleatorio, una lista de simbolos, y una lista de aridades y lo agrega a un archivo model que este entrando por la stdin")
        return
    # print("# Target random arity=%s, density=%s" % (arity,density))
    # universe = None
    # try:
    #     while not universe:
    #         line = input()
    #         print(line)
    #         line = c_input(line)
    #         if line:
    #             universe = parse_universe(line)
    #     while True:
    #         line = input()
    #         print(line)
    # except EOFError:
    #     random_target(universe,arity,density)


if __name__ == "__main__":
    main()

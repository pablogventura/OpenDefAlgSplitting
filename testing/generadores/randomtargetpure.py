import random
from itertools import permutations
from math import factorial
import sys

def iter_sample_fast(iterable, samplesize):
    results = []
    iterator = iter(iterable)
    # Fill in the first samplesize elements:
    try:
        for _ in range(samplesize):
            results.append(next(iterator))
    except StopIteration:
        raise ValueError("Sample larger than population.")
    random.shuffle(results)  # Randomize their positions
    for i, v in enumerate(iterator, samplesize):
        r = random.randint(0, i)
        if r < samplesize:
            results[r] = v  # at a decreasing rate, replace random items
    return results

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
    tuplas = permutations(universe, r=tarity)
    card = len(universe)
    cantidad = factorial(card) // factorial(card-tarity)
    target = []
    for i in iter_sample_fast(tuplas, cantidad):
        if random.random() < density:
            target.append(" ".join(map(str, i)))
    print("T0 %s %s\n" % (len(target), tarity))
    for t in target:
        print(t)

def main():
    try:
        arity, density = sys.argv[1:3]
        arity = int(arity)
        density = float(density)
    except:
        print("Toma la aridad y la densidad del target aleatorio y lo agrega a un archivo model que este entrando por la stdin")
        return
    print("# Target random arity pure=%s, density=%s" % (arity,density))
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
        random_target(universe,arity,density)


if __name__ == "__main__":
    main()

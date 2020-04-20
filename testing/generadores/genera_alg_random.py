import datetime
import sys
from random import randint
from itertools import product


def generador(tA, t, c, fs):
    # tA es el tamaño de la estructura ambiente
    # t es la cantidad de subconjuntos
    # c es el tamaño de esos subconjuntos
    # fs es una lista de aridades de funciones
    # fc es una lista de booleanos para hacer a la funcion hiperconmutativa
    print('# Generated {0:%Y-%m-%d %H:%M:%S}'.format(datetime.datetime.now()))
    print('# Parameters: |A| = %s, |MaxSubs| = %s, |ms| = %s with ms in MaxSubs, Arities = %s' % (
        tA, t, c, fs))

    # result += "# Random Seed: %s\n" % seed
    cardinality = tA
    print(" ".join(str(e) for e in range(cardinality)))
    for i, arity in enumerate(fs):
        print("f%s %s" % (i, arity))
        for values in product(range(cardinality), repeat=arity):
            fvalues = randint(1,cardinality)
            print(" ".join(str(e) for e in values) + " %s" % fvalues)


def main():
    try:
        tA, t, c, fs = sys.argv[1:5]
        try:
            density = float(sys.argv[7])
        except:
            density = 1
        tA = int(tA)
        t = int(t)
        c = int(c)
        fs = [int(i) for i in eval(fs)]
        assert all(i >= 0 for i in fs)

    except:
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

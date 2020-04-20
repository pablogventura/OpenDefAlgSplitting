from itertools import product,permutations
from random import sample
import sys

# para todo grupo hay un k tal que es un subgrupo del grupo de k-permutaciones
# genera grupos en general ( no solo no abelianos)
def clean_print(value,universe):
    print(" ".join(str(universe.index(v)) for v in value))

def inverse(a):
    
    r = []
    for i in range(len(a)):
        r.append(a.index(i))
    return tuple(r)

def compose(a,b):
    assert len(a)==len(b)
    r = []
    for i in range(len(a)):
        r.append(b[a[i]])
    return tuple(r)

def generador(k, cant_generadores, cardinalidad_exacta=None):
    """
    :param k: toma el k del del que van a ser las k-permutaciones
    :param cant_generadores: cantidad de generadores aleatorios
    q para cuando uno quiere que sea clavado un universo y vaya reintentando
    """
    permutaciones = set(permutations(range(k)))
    print("# Cantidad de permutaciones posibles: %s" % len(permutaciones))
    
    sigo=True
    universe = set(sample(permutaciones, cant_generadores))
    universe.add(tuple(range(k)))
    while sigo:
        sigo = False
        for a,b in product(universe,universe):
            
            r = compose(a,b)
            if r not in universe:
                universe.add(r)
                sigo = True
        if (cardinalidad_exacta and (len(universe) > cardinalidad_exacta) or (cardinalidad_exacta and (not sigo and len(universe) < cardinalidad_exacta))):
            universe = set(sample(permutaciones, cant_generadores))
            universe.add(tuple(range(k)))
            sigo = True
    universe = sorted(universe)
    print("# Quedaron: %s" % len(universe))
    print(" ".join(str(i) for i in range(len(universe))))
    print("")
    print("Id 0")
    clean_print([tuple(range(k))], universe)
    print("")
    print("O 2")
    for a, b in product(universe, universe):
        r = compose(a, b)
        clean_print((a,b,r),universe)
    print("")
    print("I 1")
    for a in universe:
        r = inverse(a)
        clean_print((a,r),universe)
    print("")


def main():
    try:
        k, cant_generadores = [int(i) for i in sys.argv[1:3]]
    except:
        print("Toma un k tal que para el grupo de k permutaciones")
        print("Toma una cantidad de generadores original para empezar a generar el subgrupo")
        print("Toma opcionalmente el tamaño del subgrupo generado")
        sys.exit(1)
    try:
        cardinalidad_exacta = int(sys.argv[3])
    except:
        cardinalidad_exacta=None
    generador(k, cant_generadores, cardinalidad_exacta)


if __name__ == "__main__":
    main()

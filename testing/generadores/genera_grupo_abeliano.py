from itertools import product
import sys

#todo grupo abeliano finito es un producto directo de Z_k

def clean_print(value,universe):
    print(" ".join(str(universe.index(v)) for v in value))
        


def generador(numeros):
    universe = list(product(*list(range(i) for i in numeros)))
    print(" ".join(str(i) for i in range(len(universe))))
    print("")
    print("Sum 2")
    for a, b in product(universe, universe):
        r = []
        for i in range(len(a)):
            r.append((a[i]+b[i]) % numeros[i])
        clean_print((a, b, tuple(r)),universe)
    print("")
    print("Neg 1")
    for a in universe:
        r = []
        for i in range(len(a)):
            r.append((-a[i]) % numeros[i])
        clean_print((a, tuple(r)),universe)
    print("")
    print("Zero 0")
    clean_print([(0,)*len(numeros)],universe)
    print("")



def main():
    try:
        numeros = [int(i) for i in sys.argv[1:]]
    except:
        print("Toma los i de los Z_i que seran multiplicados")
        sys.exit(1)

    generador(numeros)


if __name__ == "__main__":
    main()

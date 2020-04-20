from itertools import product
import random
import sys

#todo grupo abeliano finito es un producto directo de Z_k

def clean_print(value,universe):
    print(" ".join(str(universe.index(v)) for v in value))
        


def generador(numeros):
    universe = list(product(*list(range(i) for i in numeros)))

    print("#Abelian Group: %s" % "x".join([("Z_%s" % i) for i in numeros]))
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

def multiconjunto_que_suma(k):
    result = []
    total = 0
    while total!=k:
        n=random.randint(1,k-total)
        result.append(n)
        total+=n
    #result = {i:result.count(i) for i in result}
    return result

def valores_para_generar(k):
    result = []
    m = multiconjunto_que_suma(k)
    for i in m:
        result.append(2**i)
    return result

def main():
    try:
        car = int(sys.argv[1])
    except:
        print("Toma k tal que 2**k es el tamaño del grupo abeliano")
        sys.exit(1)
    numeros = valores_para_generar(car)
    generador(numeros)


if __name__ == "__main__":
    main()

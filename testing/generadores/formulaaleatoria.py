from random import choice, randint
import sys



def variables_libres(cantidad):
    return {chr(122-i):0 for i in range(cantidad)}

def funciones(simbolos):
    return {k:simbolos[k] for k in simbolos if simbolos[k] > 0}

def variables(simbolos):
    return {k:simbolos[k] for k in simbolos if simbolos[k] == 0}

def funcion_aleatoria(simbolos):
    f = funciones(simbolos)
    sym = choice(list(f.keys()))
    return (sym,f[sym])

def variable_aleatoria(simbolos):
    v = variables(simbolos)
    sym = choice(list(v.keys()))
    return sym

def termino_aleatorio_exacta(p, simbolos):
    """
    Devuelve un termino aleatorio de profundidad p
    """
    if p == 0:
        return variable_aleatoria(simbolos)
    
    s, a = funcion_aleatoria(simbolos)
    subterminos = []
    i_exacto = randint(0,a-1)
    
    for i in range(a):
        if i == i_exacto:
            subterminos.append(termino_aleatorio_exacta(p-1,simbolos))
        else:
            subterminos.append(termino_aleatorio_no_exacta(p-1,simbolos))
    result=""
    if subterminos:
        result = ", ".join(subterminos)
        result = "(%s)" % result
    result = s + result
    return result


def termino_aleatorio_no_exacta(p, simbolos):
    """
    Devuelve un termino aleatorio de profundidad p
    """
    p = randint(0,p)
    return termino_aleatorio_exacta(p,simbolos)


def formula_aleatoria(p,f, simbolos, aridad):
    """
    Una formula aleatoria con al menos un termino de profundidad exacta p
    con f cantidad de subformulas
    """
    simbolos.update(variables_libres(aridad))
    result = "T0(%s) " % ",".join(reversed(list(variables_libres(aridad).keys())))

    i_exacto = randint(0,f-1)
    for i in range(f):
        subformula = ""
        if i == i_exacto:
            subformula += termino_aleatorio_exacta(p,simbolos)
        else:
            subformula += termino_aleatorio_no_exacta(p,simbolos)
        subformula += " == "
        subformula += termino_aleatorio_no_exacta(p,simbolos)
        subformula = "(%s)" % subformula
        if randint(0,1) == 1:
            subformula = "-" + subformula
        result += subformula
        if randint(0,1) == 1:
            result += " | "
        else:
            result += " & "
    result = result[:-3]
    return result
        
        
        
        
        
        
def main():
    try:
        arity = sys.argv[1]
        arity = int(arity)
    except e:
        print(e)
        print("Toma la aridad, y genera la formurmula que agrega a un archivo model que este entrando por la stdin")
        return
    try:
        while True:
            print(input())
            
    except EOFError:
        sim = {"f0":2,"f1":3}
        print(formula_aleatoria(3,2, sim, arity))


if __name__ == "__main__":
    main()
        
        
        
        
        
        
        
        
        
        

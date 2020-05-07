from random import choice, randint

simbolos = {"f0":2,"f1":3}

variables = ["x","y","z"]

def simbolo_aleatorio():
    sym = choice(list(simbolos.keys()))
    return (sym,simbolos[sym])



def termino_aleatorio_exacta(p):
    """
    Devuelve un termino aleatorio de profundidad p
    """
    if p == 0:
        return choice(variables)
    s, a = simbolo_aleatorio()
    subterminos = []
    i_exacto = randint(0,a-1)
    
    for i in range(a):
        if i == i_exacto:
            subterminos.append(termino_aleatorio_exacta(p-1))
        else:
            subterminos.append(termino_aleatorio_no_exacta(p-1))
    print(subterminos)
    result=""
    if subterminos:
        result = ", ".join(subterminos)
        result = "(%s)" % result
    result = s + result
    return result


def termino_aleatorio_no_exacta(p):
    """
    Devuelve un termino aleatorio de profundidad p
    """
    if p == 0:
        return choice(variables)
    s, a = simbolo_aleatorio()
    subterminos = []
    i_exacto = randint(0,a-1)
    
    for i in range(a):
        subterminos.append(termino_aleatorio_no_exacta(p-1))
    
    result=""
    if subterminos:
        result = ", ".join(subterminos)
        result = "(%s)" % result
    result = s + result
    return result


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
    p = randint(0,p)
    return termino_aleatorio_exacta(p)


def formula_aleatoria(p,f):
    """
    Una formula aleatoria con al menos un termino de profundidad exacta p
    con f cantidad de subformulas
    """
    result = ""
    i_exacto = randint(0,f-1)
    for i in range(f):
        subformula = ""
        if i == i_exacto:
            subformula += termino_aleatorio_exacta(p)
        else:
            subformula += termino_aleatorio_no_exacta(p)
        subformula += " == "
        subformula += termino_aleatorio_no_exacta(p)
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
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        

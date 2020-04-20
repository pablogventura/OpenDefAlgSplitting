
import os
from random import randint

def generar(*args, cuantity=100):
    # genera alg random
    args = [str(i) for i in args]

    for i in range(cuantity):
        filename = os.path.join(args[0]+"_wt", '_'.join(args[1:] + [str(i)]))
        filename += ".modelwt"
        if os.path.isfile(filename):
            continue
        filename = '"' + filename + '"'
        try:
            os.mkdir(args[0]+"_wt")
        except:
            pass
        script = "genera_" + args[0] + ".py"
        argumentos = '" "'.join(args[1:])
        if argumentos:
            argumentos = '"' + argumentos + '"'
        print("python3 " + script + " " + argumentos + " | gzip > " + filename)
        os.system("python3 " + script + " " + argumentos + " | gzip > " + filename)

for i in [3,4,5,6,7,8]: # con 2**9 ya se hacen 50 gigas los ejemplos de alg_random
    generar("grupo_abeliano_diverso",i)
    generar("boole", i)
    generar("alg_random",2**i,0,0,[2,3])

from time import sleep
import os
from random import randint
path_mega_hit = "../mega_hit_test"
def generar(*args, quantity=400):
    # genera alg random
    args = [str(i) for i in args]
    for i in range(quantity):

        filename = os.path.join(path_mega_hit, args[0], '_'.join(args[1:] + [str(i)]))
        filename += ".model"

        if os.path.isfile(filename):
            continue
        filename = '"' + filename + '"'
        try:
            os.mkdir(os.path.join(path_mega_hit, args[0]))
        except:
            pass
        script = "genera_" + args[0] + ".py"
        argumentos = '" "'.join(args[1:])
        if argumentos:
            argumentos = '"' + argumentos + '"'
        print("python3 " + script + " " + argumentos + " > " + filename)
        os.system("python3 " + script + " " + argumentos + " > " + filename)

# generar("boole",2)
# generar("boole",3)
# generar("boole",4)
# generar("boole",5)
generar("grupo_abeliano",4)
generar("grupo_abeliano",2,4)
generar("grupo_abeliano",2,2,4)
generar("grupo_abeliano",2,4,4)
# generar("alg_random",4,0,0,[2,3],[False,False],2)
# generar("alg_random",8,0,0,[2,3],[False,False],2)
# generar("alg_random",16,0,0,[2,3],[False,False],2)
# generar("alg_random",32,0,0,[2,3],[False,False],2)



import os
from random import randint

def generar(directory, cardinality, arity, densidad, cuantity=100):

    for i in range(cuantity):
        filename = os.path.join(directory, '_'.join([str(cardinality),str(arity),str(densidad)]))
        filename += "_NODEF.target.gz"
        if os.path.isfile(filename):
            continue
        filename = '"' + filename + '"'
        try:
            os.mkdir(directory)
        except:
            pass

        print("python3 randomtarget_separado.py %s %s %s" % (cardinality,arity, densidad) + " | gzip > " + filename)
        os.system("python3 randomtarget_separado.py %s %s %s" % (cardinality,arity, densidad) + " | gzip > " + filename)

for cardinality in [8]:
    for arity in [3]:
        for densidad in [0.1]:
            generar("targets",cardinality,arity,densidad)

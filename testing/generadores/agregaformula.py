import gzip
import os
from random import randint

def c_input(line):
    """
    Clean input
    """
    line = line.decode("ascii")
    if "#" in line:
        line = line[:line.find("#")]
    return line.strip()

def parse_universe(line):
    # el universo puede estar hecho de strings, de tuplas,etc
    return [eval(i) for i in line.split()]




def cardinalidad(archivo):
    f=gzip.open(archivo,"r")
    while True:
        line = c_input(f.readline())
        if line:
            return len(parse_universe(line))


def generar(aridad, archivo):
    # genera alg random
    c = cardinalidad(archivo)
    directorio = os.path.dirname(archivo)
    directorio = os.path.join(directorio, "formula")
    try:
        os.mkdir(directorio)
    except:
        pass
    filename = os.path.join(directorio,str(c)+"_T"+str(aridad)+"_"+os.path.basename(archivo)[:-2])
    
    if "alg_random_wt" in archivo:
        sim={'f0':2}
    elif "boole_wt" in archivo:
        sim={'m':2,'j':2}
    elif "grupo_abeliano_diverso_wt" in archivo:
        sim={'Sum':2,'Neg':1,'Zero':0}
    else:
        assert(False,"No es de ninguna signatura conocida")

    os.system('gunzip -c "%s" | python3 formulaaleatoria.py %s "%s" | gzip > "%s" ' % (archivo, aridad, sim, filename))


from glob import glob
for archivo in (y for x in os.walk(".") for y in glob(os.path.join(x[0], '*.modelwt'))):
    for aridad in [3]:
        generar(aridad,archivo)

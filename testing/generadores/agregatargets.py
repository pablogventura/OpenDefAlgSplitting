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


def generar(aridad, densidad, archivo):
    # genera alg random
    c = cardinalidad(archivo)

    directorio = os.path.dirname(archivo)
    directorio = os.path.join(directorio, "targets")
    try:
        os.mkdir(directorio)
    except:
        pass
    filename = os.path.join(directorio,str(c)+"_T"+str(aridad)+"_"+str(densidad)+"_"+os.path.basename(archivo)[:-2])

    os.system('gunzip -c "%s" | python3 randomtarget.py %s %s | gzip > "%s" ' % (archivo, aridad, densidad,filename))


from glob import glob
for archivo in (y for x in os.walk(".") for y in glob(os.path.join(x[0], '*.modelwt'))):
    for aridad in [3]:
        for densidad in [0.5]:
            generar(aridad,densidad,archivo)

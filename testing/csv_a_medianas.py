import csv
import os
import sys
import statistics
from pathlib import Path
from collections import defaultdict

csv_file = sys.argv[1]

csv_file = open(csv_file, 'r')
reader = csv.reader(csv_file, delimiter=',', quotechar='"')
datos = defaultdict(lambda:defaultdict(list))
#wr.writerow([filename, estructura, target, definable, cardinality, elapsed_time, error])
for fn, estructura, target, definable, cardinality, elapsed_time, error in reader:
    error = eval(error)
    if not error:
        if not definable:
            definable = "None"
        definable = eval(definable)
        cardinality = int(cardinality)
        elapsed_time = float(elapsed_time)
        if (target == "formula" and definable) and (elapsed_time != float("inf")):
            datos[(estructura,definable)][cardinality].append(elapsed_time)
        elif (target == "random" and not definable) and (elapsed_time != float("inf")):
            datos[(estructura,definable)][cardinality].append(elapsed_time)
        else:
            pass
            #print("El modelo %s pretendia ser %s pero dio %s" % (fn,target,definable))
            #print(error)
            #print(elapsed_time)

csv_file.close()


datos_procesados = defaultdict(lambda:defaultdict(list))
for estructura, definable in datos:
    for cardinality in datos[(estructura,definable)]:
        cantidad = len(datos[(estructura,definable)][cardinality])
        maximo = max(datos[(estructura,definable)][cardinality])
        minimo = min(datos[(estructura,definable)][cardinality])
        mediana = statistics.median(datos[(estructura,definable)][cardinality])
        try:
            stdev = statistics.stdev(datos[(estructura,definable)][cardinality])
        except statistics.StatisticsError:
            stdev = None
        datos_procesados[(estructura,definable)][cardinality] = (cantidad, maximo, minimo, mediana, stdev)

for estructura, definable in datos_procesados:
    print((estructura,definable))
    for cardinality in datos_procesados[(estructura,definable)]:
        print(cardinality)
        print(datos_procesados[(estructura,definable)][cardinality])
        

import csv
import os
import sys
from pathlib import Path

directory = sys.argv[1]

files = [os.path.join(dp, f) for dp, dn, fn in os.walk(directory) for f in fn]

results_file = open("results.csv", 'w')
wr = csv.writer(results_file, quoting=csv.QUOTE_ALL)
wr.writerow(["filename", "estructura", "target", "definable", "cardinality", "elapsed_time", "error"])

for i, f in enumerate(files):
    if f.endswith(".stderr"):
        filename = f
        cardinality = int(filename.split("/")[-1].split("_")[0])
        if "grupo_abeliano" in filename:
            estructura = "grupo_abeliano"
        elif "boole" in filename:
            estructura = "boole"
        elif "alg_random" in filename:
            estructura = "alg_random"
        else:
            raise ValueError("No es de ninguna estructura conocida?")
        if "formula" in filename:
            target = "formula"
        elif "target" in filename:
            target = "random"
        else:
            raise ValueError("No es de ningun tipo conocido?")
        definable = None
        not_definable = None
        timeout = None
        elapsed_time = None
        error = None
        if Path(f).stat().st_size > 0:
            datafile = open(f,"r")
            for line in datafile:
                if 'Traceback' in line or 'ERROR' in line or "failed" in line:
                    error = 28
                if 'NOT DEFINABLE' in line:
                    not_definable = True
                elif 'DEFINABLE' in line:
                    definable = True
                elif line.startswith("Elapsed time: "):
                    elapsed_time = float(line[len("Elapsed time: "):-1])
            datafile.close()
        else:
            timeout = True
            
        if timeout == True:
            elapsed_time = float('inf')
            definable = None
        else:
            if definable is None and not_definable is None and not error:
                error = 44
            elif definable == not_definable:
                ValueError("Inconsistencia")
            elif not_definable:
                definable = False
            if elapsed_time == None and not error:
                error = 50
        if error is None:
            error = False
        wr.writerow([filename, estructura, target, definable, cardinality, elapsed_time, error])
results_file.close()

import csv
import os
import sys
from pathlib import Path

directory = sys.argv[1]

files = [os.path.join(dp, f) for dp, dn, fn in os.walk(directory) for f in fn]

results_file = open("results.csv", 'w')
wr = csv.writer(results_file, quoting=csv.QUOTE_ALL)

for i, f in enumerate(files):
    if f.endswith(".stderr"):
        filename = f
        print(f)
        definable = None
        not_definable = None
        timeout = None
        elapsed_time = None
        error = None
        #print(Path(f).stat().st_size)
        if Path(f).stat().st_size > 0:
            
            datafile = open(f,"r")
            for line in datafile:
                if 'Traceback' in line or 'ERROR' in line or "failed" in line:
                    error = 28
                if 'NOT DEFINABLE' in line:
                    print("NODEF")
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
        else:
            if definable is None and not_definable is None:
                error = 44
            elif definable == not_definable:
                ValueError("Inconsistencia")
            elif not_definable:
                definable = False
            if elapsed_time == None:
                error = 50
        if error is None:
            error = False
        wr.writerow([filename,definable,elapsed_time,error])
results_file.close()

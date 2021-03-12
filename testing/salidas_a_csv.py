
import os
from pathlib import Path

#https://stackoverflow.com/questions/2084069/create-a-csv-file-with-values-from-a-python-list


files = [os.path.join(dp, f) for dp, dn, fn in os.walk(".") for f in fn]

row = []
for i, f in enumerate(files):
    if f.endswith(".stderr"):
        filename = f

        definable = None
        nondefinable = None
        timeout = None
        elapsed_time = None
        error = None
        print(Path(f).stat().st_size)
        if Path(f).stat().st_size > 0:
            
            datafile = open(f,"r")
            line = datafile.readline()
            while line:
                if 'Traceback' in line or 'ERROR' in line:
                    error = True
                if 'NOT DEFINABLE' in line:
                    not_definable = True
                elif 'DEFINABLE' in line:
                    definable = True
                elif line.startswith("Elapsed time: "):
                    elapsed_time = float(line[len("Elapsed time: "):-1])
                
                line = datafile.readline()
            datafile.close()
        else:
            timeout = True
            
        if timeout == True:
            elapsed_time = float('inf')
        else:
            if definable is None and nondefinable is None:
                error = True
            elif definable == nondefinable:
                ValueError("Inconsistencia")
            elif nondefinable:
                definable = False
            if elapsed_time == None:
                error = True
        if error is None:
            error = False
        
        print((filename,definable,elapsed_time,error))

"""
Toma como argumento el directorio donde estan los modelos
"""
from testing.shell_non_blocking import ShellProc
import os
cores = 13
procs = []
try:

    files = [os.path.join(dp, f) for dp, dn, fn in os.walk("testing/mega_hit_test") for f in fn]
    for i,f in enumerate(files):
        if f.endswith(".model") and not os.path.exists(f.replace(".model",".megahit")):
            print("%s%%" % (i / len(files)))
            while len(procs) >= cores:
                procs = [p for p in procs if p.is_running()]
            procs.append(ShellProc('timeout -s 9 30m python3 main.py "%s" > "%s"' % (f,f.replace(".model",".megahit"))))
            
except KeyboardInterrupt:
    pass
    

"""
Toma como argumento el directorio donde estan los modelos
"""
from testing.shell_non_blocking import ShellProc
from random import shuffle
import os
cores = 13
procs = []
try:

    files = [os.path.join(dp, f) for dp, dn, fn in os.walk("testing/") for f in fn]
    shuffle(files)
    for i, f in enumerate(files):
        if f.endswith(".model") and not os.path.exists(f.replace(".model", ".megahit")):
            print("%s%%" % (i / len(files)))
            while len(procs) >= cores:
                procs = [p for p in procs if p.is_running()]
            procs.append(ShellProc('timeout -s 9 120m python3 main.py "%s" > "%s" 2 > "%s"' % (f, f.replace(".model", ".megahit"), f.replace(".model", ".stderr"))))

except KeyboardInterrupt:
    pass

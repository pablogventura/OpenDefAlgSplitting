"""
Toma como argumento el directorio donde estan los modelos
"""

import os
from random import shuffle

from testing.shell_non_blocking import ShellProc

cores = 13
procs = []
try:
    files = [os.path.join(dp, f) for dp, _dn, fn in os.walk("testing/") for f in fn]
    shuffle(files)
    for i, f in enumerate(files):
        if f.endswith(".model") and not os.path.exists(f.replace(".model", ".megahitold")):
            print("%s%%" % (i / len(files)))
            while len(procs) >= cores:
                procs = [p for p in procs if p.is_running()]
            procs.append(
                ShellProc(
                    'timeout -s 9 250m python3 ../OpenDefAlg/main.py "{}" > "{}" 2 > "{}"'.format(
                        f, f.replace(".model", ".megahitold"), f.replace(".model", ".stderr")
                    )
                )
            )

except KeyboardInterrupt:
    pass

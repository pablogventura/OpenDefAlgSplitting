"""
Toma como argumento el directorio donde estan los modelos
"""

import os

from testing.shell_non_blocking import ShellProc

cores = 5
procs = []
try:
    from glob import glob

    for _i, f in enumerate(
        y for x in os.walk("testing") for y in glob(os.path.join(x[0], "*.model"))
    ):
        if not os.path.exists(f.replace(".model", ".mega")):
            print(".", end="")
            while len(procs) >= cores:
                procs = [p for p in procs if p.is_running()]
            procs.append(
                ShellProc(
                    'python3 ../OpenDefAlg/main.py "{}" > "{}"'.format(
                        f, f.replace(".model", ".mega")
                    )
                )
            )

except KeyboardInterrupt:
    pass

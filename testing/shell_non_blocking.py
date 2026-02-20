import subprocess
from typing import Union

import psutil


class ShellProc:
    def __init__(self, command: Union[str, list[str]]) -> None:
        self.sp = subprocess.Popen(
            command, shell=True, stdin=None, stdout=None, stderr=None, close_fds=True
        )
        self.pp = psutil.Process(self.sp.pid)

    def is_running(self) -> bool:
        return self.pp.is_running() and not self.pp.status() == psutil.STATUS_ZOMBIE

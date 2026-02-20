#!/usr/bin/env python

import os
import pickle
from typing import Any


def object_to_file(obj: Any, path: str) -> None:
    """
    Guarda un objeto en un archivo

    >>> object_to_file([1,3],"testfile")
    >>> l = file_to_object("testfile")
    >>> l
    [1, 3]
    >>> remove("testfile")
    """
    f = open(path, "wb")
    pickle.dump(obj, f)
    f.close()


def file_to_object(path: str) -> Any:
    """
    Lee un objeto en un archivo

    >>> object_to_file([1,3],"testfile")
    >>> l = file_to_object("testfile")
    >>> l
    [1, 3]
    >>> remove("testfile")
    """
    f = open(path, "rb")
    obj = pickle.load(f)
    f.close()
    return obj


def create_pipe(path: str) -> None:
    """
    Crea un named pipe

    """
    try:
        os.mkfifo(path)
    except OSError:
        # ya existe
        pass


def remove(path: str) -> None:
    """
    Elimina un archivo
    """
    try:
        os.remove(path)
    except OSError:
        # no existe
        pass


def write(path: str, data: str) -> None:
    """
    Escribe datos en un archivo
    """
    f = open(path, "w")
    f.write(data)
    f.close()


if __name__ == "__main__":
    import doctest

    doctest.testmod()

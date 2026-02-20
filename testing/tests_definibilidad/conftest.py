# -*- coding: utf-8 -*-
"""
Configuración y fixtures para tests de definibilidad.
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# Añadir raíz del proyecto al path
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))


def run_main(model_path, timeout=60):
    """Ejecuta main.py sobre un modelo y devuelve la salida completa."""
    main_script = PROJECT_ROOT / "main.py"
    result = subprocess.run(
        [sys.executable, str(main_script), str(model_path)],
        capture_output=True,
        text=True,
        timeout=timeout,
        cwd=str(PROJECT_ROOT),
    )
    return result.stdout + result.stderr


def run_generador(comando, args, target=None, cwd=None):
    """Ejecuta un generador y devuelve su salida.
    Si target=(aridad, densidad), añade target aleatorio (--target).
    """
    generadores_dir = PROJECT_ROOT / "testing" / "generadores"
    script = generadores_dir / "genera_modelos.py"
    cwd = cwd or str(generadores_dir)
    cmd = [sys.executable, str(script), comando] + [str(a) for a in args]
    if target:
        cmd.extend(["--target", str(target[0]), str(target[1])])
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=30,
        cwd=cwd,
    )
    if result.returncode != 0:
        raise RuntimeError(f"Generador falló: {result.stderr or result.stdout}")
    return result.stdout


def run_formulaaleatoria(model_stdin, arity, sim_dict):
    """Genera modelo con target definible por fórmula aleatoria."""
    generadores_dir = PROJECT_ROOT / "testing" / "generadores"
    formula_script = generadores_dir / "formulaaleatoria.py"
    result = subprocess.run(
        [sys.executable, str(formula_script), str(arity), repr(sim_dict)],
        input=model_stdin,
        capture_output=True,
        text=True,
        timeout=30,
        cwd=str(generadores_dir),
    )
    if result.returncode != 0:
        raise RuntimeError(f"formulaaleatoria falló: {result.stderr}")
    return result.stdout


@pytest.fixture
def tmp_model_file():
    """Fixture que proporciona un archivo temporal para guardar modelos."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
        yield f.name
    try:
        os.unlink(f.name)
    except OSError:
        pass


@pytest.fixture
def fixtures_dir():
    """Directorio de fixtures de modelos."""
    return Path(__file__).parent / "fixtures"

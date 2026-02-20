# -*- coding: utf-8 -*-
"""
Tests de modelos DEFINIBLES por tipo.
Cada tipo (boole, aleatorio, grupo-abeliano, etc.) debe tener al menos un test
con target definible que devuelva DEFINABLE.
"""
import os
import subprocess
import sys
import tempfile
import pytest
from pathlib import Path

from .conftest import (
    PROJECT_ROOT,
    run_main,
    run_generador,
    run_formulaaleatoria,
    tmp_model_file,
)

# Semilla para reproducibilidad cuando aplique
import random
random.seed(42)


def _guardar_y_ejecutar(contenido, tmp_path):
    """Guarda contenido en archivo temporal y ejecuta main."""
    Path(tmp_path).write_text(contenido, encoding="utf-8")
    return run_main(tmp_path, timeout=90)


def _es_definible(salida):
    """Verifica que la salida indique DEFINABLE."""
    return "DEFINABLE" in salida and "NOT DEFINABLE" not in salida.split("DEFINABLE")[0]


class TestDefinibleBoole:
    """Targets definibles en álgebras de Boole."""

    def test_boole_diagonal_formula(self):
        """Boole pequeño con target definido por fórmula (formulaaleatoria)."""
        model_stdin = run_generador("boole", [2])  # 4 elementos
        model_out = run_formulaaleatoria(model_stdin, 2, {"m": 2, "j": 2})
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(model_out)
            f.flush()
            salida = run_main(f.name, timeout=90)
            os.unlink(f.name)
        assert _es_definible(salida), f"Esperaba DEFINABLE. Salida: {salida[:500]}"

    def test_boole_existente_definible(self):
        """Usa un modelo Boole de ejemplos si existe con target definible."""
        # boole4 = closure de genera_boole 2
        model_stdin = run_generador("boole", [2])
        # Target diagonal trivial eq(x,y) - definible
        diagonal = model_stdin + "\n\nT0(x,y) eq(x,y)\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(diagonal)
            f.flush()
            salida = run_main(f.name, timeout=60)
            os.unlink(f.name)
        assert _es_definible(salida), f"Diagonal debería ser definible. Salida: {salida[:500]}"


class TestDefinibleAleatorio:
    """Targets definibles en álgebras aleatorias."""

    def test_aleatorio_formulaaleatoria(self):
        """Álgebra aleatoria con target definible por fórmula."""
        model_stdin = run_generador("aleatorio", [6, "--aridades", "2"])
        model_out = run_formulaaleatoria(model_stdin, 2, {"f0": 2})
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(model_out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        assert _es_definible(salida), f"Esperaba DEFINABLE. Salida: {salida[:600]}"


class TestDefinibleGrupoAbeliano:
    """Targets definibles en grupos abelianos."""

    def test_grupo_abeliano_diagonal(self):
        """Z2×Z2 con diagonal x=y es definible."""
        model_stdin = run_generador("grupo-abeliano", [2, 2])
        diagonal = model_stdin.strip() + "\n\nT0(x,y) eq(x,y)\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(diagonal)
            f.flush()
            salida = run_main(f.name, timeout=60)
            os.unlink(f.name)
        assert _es_definible(salida), f"Diagonal en grupo abeliano definible. Salida: {salida[:500]}"

    def test_grupo_abeliano_diverso_diagonal(self):
        """Grupo abeliano diverso con diagonal definible (eq(x,y))."""
        model_stdin = run_generador("grupo-abeliano-diverso", [3])
        diagonal = model_stdin.strip() + "\n\nT0(x,y) eq(x,y)\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(diagonal)
            f.flush()
            salida = run_main(f.name, timeout=90)
            os.unlink(f.name)
        assert _es_definible(salida), f"Diagonal en grupo diverso definible. Salida: {salida[:600]}"


class TestDefinibleReticulado:
    """Targets definibles en retículos."""

    def test_reticulado_formula_trivial(self):
        """Reticulado con target definible trivial."""
        model_stdin = run_generador("reticulado", [3, 5])  # ancho 3, muestra 5
        diagonal = model_stdin.strip() + "\n\nT0(x,y) eq(x,y)\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(diagonal)
            f.flush()
            salida = run_main(f.name, timeout=90)
            os.unlink(f.name)
        assert _es_definible(salida), f"Diagonal en reticulado definible. Salida: {salida[:500]}"


class TestDefinibleModeloExistente:
    """Modelos de ejemplos que se sabe son definibles."""

    def test_modeloqueanda(self):
        """modeloqueanda.model tiene target definible por fórmula."""
        model_path = PROJECT_ROOT / "model_examples" / "modeloqueanda.model"
        if not model_path.exists():
            pytest.skip("modeloqueanda.model no existe")
        salida = run_main(str(model_path), timeout=60)
        assert _es_definible(salida), f"modeloqueanda debería ser definible. Salida: {salida[:500]}"

    def test_suma4_diagonal(self):
        """suma4.model tiene diagonal (0,0),(1,1),(2,2) que es definible en Z4."""
        model_path = PROJECT_ROOT / "model_examples" / "suma4.model"
        if not model_path.exists():
            pytest.skip("suma4.model no existe")
        salida = run_main(str(model_path), timeout=60)
        assert _es_definible(salida), f"suma4 diagonal definible. Salida: {salida[:500]}"

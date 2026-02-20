# -*- coding: utf-8 -*-
"""
Tests de modelos NO DEFINIBLES por tipo.
Targets aleatorios sobre álgebras (típicamente no definibles).
"""
import os
import tempfile
import pytest
from pathlib import Path

from .conftest import (
    PROJECT_ROOT,
    run_main,
    run_generador,
)


def _es_no_definible(salida):
    """Verifica que la salida indique NOT DEFINABLE."""
    return "NOT DEFINABLE" in salida


def _es_definible(salida):
    """Verifica que la salida indique DEFINABLE."""
    return "DEFINABLE" in salida and "NOT DEFINABLE" not in salida.split("DEFINABLE")[0]


class TestNoDefinibleBoole:
    """Target aleatorio en Boole: típicamente no definible."""

    def test_boole_target_aleatorio(self):
        """Boole con target aleatorio (densidad media)."""
        out = run_generador("boole", [3], target=(2, 0.4))  # 8 elementos, aridad 2
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        assert _es_no_definible(salida), f"Target aleatorio en Boole debería ser NO DEFINIBLE. Salida: {salida[:500]}"


class TestNoDefinibleAleatorio:
    """Target aleatorio en álgebra aleatoria."""

    def test_aleatorio_target_aleatorio(self):
        """Álgebra aleatoria con target aleatorio: típicamente no definible.
        Nota: tras preprocesamiento a veces quedan pocas tuplas que pueden ser definibles por azar.
        """
        out = run_generador("aleatorio", [10, "--aridades", "2", "2"], target=(2, 0.4))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        # Debe terminar con resultado (DEFINABLE o NOT DEFINABLE)
        assert _es_definible(salida) or _es_no_definible(salida), f"Debe dar definible o no. Salida: {salida[:500]}"


class TestNoDefinibleGrupoAbeliano:
    """Target aleatorio en grupo abeliano."""

    def test_grupo_abeliano_target_aleatorio(self):
        """Z2×Z2×Z2 con target aleatorio."""
        out = run_generador("grupo-abeliano", [2, 2, 2], target=(2, 0.5))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        assert _es_no_definible(salida), f"Target aleatorio en grupo abeliano NO DEFINIBLE. Salida: {salida[:500]}"

    def test_grupo_abeliano_diverso_target_aleatorio(self):
        """Grupo abeliano diverso con target aleatorio."""
        out = run_generador("grupo-abeliano-diverso", [4], target=(2, 0.3))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        assert _es_no_definible(salida), f"Target aleatorio en grupo diverso NO DEFINIBLE. Salida: {salida[:500]}"


class TestNoDefinibleGrupoNoAbeliano:
    """Target aleatorio en grupo no abeliano."""

    def test_grupo_no_abeliano_target_aleatorio(self):
        """S3 (o subgrupo) con target aleatorio."""
        out = run_generador("grupo-no-abeliano", [3, 2], target=(2, 0.2))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        assert _es_no_definible(salida), f"Target aleatorio en grupo no abeliano NO DEFINIBLE. Salida: {salida[:500]}"


class TestNoDefinibleReticulado:
    """Target aleatorio en retículo."""

    def test_reticulado_target_aleatorio(self):
        """Reticulado con target aleatorio."""
        out = run_generador("reticulado", [3, 5], target=(2, 0.25))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        assert _es_no_definible(salida), f"Target aleatorio en reticulado NO DEFINIBLE. Salida: {salida[:500]}"


class TestNoDefinibleModeloExistente:
    """Modelos de ejemplos que se sabe son no definibles."""

    def test_retrombo_nodef(self):
        """retrombo_nodef.model es conocido como NO DEFINIBLE."""
        model_path = PROJECT_ROOT / "model_examples" / "retrombo_nodef.model"
        if not model_path.exists():
            pytest.skip("retrombo_nodef.model no existe")
        salida = run_main(str(model_path), timeout=60)
        assert _es_no_definible(salida), f"retrombo_nodef debe ser NO DEFINIBLE. Salida: {salida[:500]}"


class TestNoDefinibleTargetSolo:
    """Modelo solo con target (sin operaciones) -> no definible."""

    def test_target_solo_sin_operaciones(self):
        """Target aleatorio sin álgebra (usa fixture): no hay términos, no definible."""
        model_path = PROJECT_ROOT / "testing" / "tests_definibilidad" / "fixtures" / "modelo_solo_target.model"
        if not model_path.exists():
            pytest.skip("Fixture modelo_solo_target no existe")
        salida = run_main(str(model_path), timeout=60)
        assert _es_no_definible(salida), f"Solo target sin ops debe ser NO DEFINIBLE. Salida: {salida[:500]}"

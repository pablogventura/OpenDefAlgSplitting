"""
Tests de rendimiento: verifican que operaciones críticas completen en tiempo razonable.
También tests adicionales de fórmulas y parser.
"""

import os
import tempfile
import time
from pathlib import Path

import pytest

from .conftest import run_generador, run_main

FIXTURES = Path(__file__).parent / "fixtures"


# --- Tests de velocidad ---


class TestVelocidadMain:
    """main.py debe completar en tiempo razonable para modelos pequeños/medios."""

    def test_boole_pequeno_completa_rapido(self):
        """Boole 4 elementos con diagonal: < 5 s."""
        out = run_generador("boole", [2])
        diagonal = out.strip() + "\n\nT0(x,y) eq(x,y)\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(diagonal)
            f.flush()
            t0 = time.perf_counter()
            salida = run_main(f.name, timeout=30)
            elapsed = time.perf_counter() - t0
            os.unlink(f.name)
        assert "DEFINABLE" in salida or "NOT DEFINABLE" in salida
        assert elapsed < 5.0, f"Tardó {elapsed:.2f}s (límite 5s)"

    def test_modelo_minimal_completa_rapido(self):
        """Fixture minimal: < 3 s."""
        path = FIXTURES / "minimal.model"
        if not path.exists():
            pytest.skip("Fixture minimal.model no existe")
        t0 = time.perf_counter()
        salida = run_main(str(path), timeout=30)
        elapsed = time.perf_counter() - t0
        assert "DEFINABLE" in salida or "NOT DEFINABLE" in salida or "ERROR" in salida
        assert elapsed < 3.0, f"Tardó {elapsed:.2f}s (límite 3s)"


class TestVelocidadParser:
    """El parser debe ser rápido para modelos típicos."""

    def test_parse_modelo_pequeno_rapido(self):
        """10 parses de minimal en < 1 s."""
        from parser.parser import parser

        path = FIXTURES / "minimal.model"
        if not path.exists():
            pytest.skip("Fixture minimal.model no existe")
        t0 = time.perf_counter()
        for _ in range(10):
            parser(str(path), verbose=False)
        elapsed = time.perf_counter() - t0
        assert elapsed < 1.0, f"10 parses tardaron {elapsed:.2f}s"


# --- Tests adicionales de fórmulas ---


class TestFormulasBasicas:
    """Tests de fórmulas de primer orden."""

    def test_eq_symmetry(self):
        """eq(x,y) y eq(y,x) tienen la misma extensión para x,y."""
        from first_order import formulas
        from first_order.models import Model

        m = Model([0, 1], {}, {})
        x, y = formulas.Variable("x"), formulas.Variable("y")
        eq_xy = formulas.eq(x, y)
        eq_yx = formulas.eq(y, x)
        ext_xy = eq_xy.extension(m, 2)
        ext_yx = eq_yx.extension(m, 2)
        assert ext_xy == ext_yx

    def test_true_extension_total(self):
        """true() tiene extensión total (todas las tuplas)."""
        from itertools import product

        from first_order import formulas
        from first_order.models import Model

        m = Model([0, 1, 2], {}, {})
        f = formulas.true()
        ext = f.extension(m, 2)
        expected = set(product([0, 1, 2], repeat=2))
        assert ext == expected

    def test_false_extension_vacia(self):
        """false() tiene extensión vacía."""
        from first_order import formulas
        from first_order.models import Model

        m = Model([0, 1], {}, {})
        f = formulas.false()
        ext = f.extension(m, 2)
        assert ext == set()

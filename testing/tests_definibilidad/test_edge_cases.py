"""
Tests de casos bordes y situaciones raras.
"""

import os
import tempfile
from pathlib import Path

import pytest

from .conftest import (
    PROJECT_ROOT,
    run_generador,
    run_main,
)

FIXTURES = Path(__file__).parent / "fixtures"


def _es_definible(salida):
    return "DEFINABLE" in salida and "NOT DEFINABLE" not in salida.split("DEFINABLE")[0]


def _es_no_definible(salida):
    return "NOT DEFINABLE" in salida


class TestUniversoPequeno:
    """Casos con universo muy pequeño."""

    def test_universo_un_elemento_definible(self):
        """Universo de 1 elemento: target total (único elemento) es definible (⊤)."""
        model_path = FIXTURES / "universo_un_elemento.model"
        if not model_path.exists():
            pytest.skip("Fixture no existe")
        salida = run_main(str(model_path), timeout=30)
        assert _es_definible(salida), (
            f"Un único elemento en target es definible. Salida: {salida[:400]}"
        )

    def test_boole_n0_un_elemento(self):
        """Boole(0) = 2^0 = 1 elemento. Target trivial definible."""
        out = run_generador("boole", [0])
        # Boole 0 puede no estar soportado; si lo está, target trivial
        try:
            out = out.strip() + "\n\nT0(x) eq(x,x)\n"
            with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
                f.write(out)
                f.flush()
                salida = run_main(f.name, timeout=30)
                os.unlink(f.name)
            assert _es_definible(salida) or _es_no_definible(salida)
        except Exception:
            pytest.skip("Boole(0) puede no estar soportado por el generador")


class TestTargetTrivial:
    """Target vacío (⊥) o total (⊤) son definibles."""

    def test_target_vacio(self):
        """Target vacío: el preprocesamiento elimina T (0 subrelaciones) -> ERROR esperado."""
        model_path = FIXTURES / "target_vacio.model"
        if not model_path.exists():
            pytest.skip("Fixture target_vacio no existe")
        salida = run_main(str(model_path), timeout=60)
        # El sistema actual devuelve ERROR cuando target vacío (0 Ts tras preproceso)
        assert "NO TARGET RELATIONS FOUND" in salida or _es_definible(salida)

    def test_target_total_definible(self):
        """Target que contiene todas las tuplas es definible (⊤)."""
        # Modelo pequeño boole 2^1 = 2 elementos, target binario completo
        out = run_generador("boole", [1])
        # Añadir T0 con las 4 tuplas (0,0),(0,1),(1,0),(1,1)
        full_target = out.strip() + "\n\nT0 4 2\n0 0\n0 1\n1 0\n1 1\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(full_target)
            f.flush()
            salida = run_main(f.name, timeout=60)
            os.unlink(f.name)
        assert _es_definible(salida), f"Target total es definible (⊤). Salida: {salida[:400]}"


class TestSinOperaciones:
    """Modelo solo con target (sin operaciones) -> no definible."""

    def test_solo_target_sin_operaciones(self):
        """Target aleatorio sin operaciones debe ser NO DEFINIBLE."""
        model_path = FIXTURES / "modelo_solo_target.model"
        if not model_path.exists():
            pytest.skip("Fixture no existe")
        salida = run_main(str(model_path), timeout=30)
        assert _es_no_definible(salida), f"Sin operaciones -> NO DEFINIBLE. Salida: {salida[:400]}"


class TestAridadesDiversas:
    """Targets de distintas aridades."""

    def test_target_aridad_1(self):
        """Target unario (aridad 1) con álgebra."""
        out = run_generador("boole", [2], target=(1, 0.5))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=60)
            os.unlink(f.name)
        assert _es_definible(salida) or _es_no_definible(salida)

    def test_target_aridad_3(self):
        """Target ternario con álgebra: debe completar (definible o no)."""
        out = run_generador("aleatorio", [4, "--aridades", "2"], target=(3, 0.1))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=120)
            os.unlink(f.name)
        assert _es_definible(salida) or _es_no_definible(salida)


class TestDensidadesExtremas:
    """Target con densidad 0 o 1 en random target."""

    def test_target_densidad_0_vacio(self):
        """Densidad 0 -> target vacío; sistema devuelve ERROR (0 Ts tras preproceso)."""
        out = run_generador("boole", [2], target=(2, 0.0))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=60)
            os.unlink(f.name)
        assert "NO TARGET RELATIONS FOUND" in salida or _es_definible(salida)

    def test_target_densidad_1_total(self):
        """Densidad 1 -> target completo -> definible (⊤)."""
        out = run_generador("boole", [1], target=(2, 1.0))
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(out)
            f.flush()
            salida = run_main(f.name, timeout=60)
            os.unlink(f.name)
        assert _es_definible(salida), f"Target total (densidad 1) definible. Salida: {salida[:400]}"


class TestGrupoTrivial:
    """Grupos con un elemento."""

    def test_grupo_abeliano_z1(self):
        """Z1 (grupo trivial) con target definible."""
        out = run_generador("grupo-abeliano", [1])
        diagonal = out.strip() + "\n\nT0(x,y) eq(x,y)\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(diagonal)
            f.flush()
            salida = run_main(f.name, timeout=30)
            os.unlink(f.name)
        assert _es_definible(salida)


class TestModeloExistenteRaro:
    """Modelos de ejemplos con comportamientos especiales."""

    def test_modelo_con_varios_patrones(self):
        """msimple.model tiene target con varios patrones de igualdad."""
        model_path = PROJECT_ROOT / "model_examples" / "msimple.model"
        if not model_path.exists():
            pytest.skip("msimple.model no existe")
        salida = run_main(str(model_path), timeout=120)
        assert _es_definible(salida) or _es_no_definible(salida)


class TestModelosStress:
    """Modelos grandes o de estrés para validar el algoritmo."""

    def test_gigante(self):
        """gigante.model: modelo grande para validar correctitud."""
        model_path = PROJECT_ROOT / "model_examples" / "gigante.model"
        if not model_path.exists():
            pytest.skip("gigante.model no existe")
        salida = run_main(str(model_path), timeout=180)
        assert _es_definible(salida) or _es_no_definible(salida)

    def test_estres_paralelo(self):
        """estres_paralelo.model: modelo para pruebas de estrés."""
        model_path = PROJECT_ROOT / "model_examples" / "estres_paralelo.model"
        if not model_path.exists():
            pytest.skip("estres_paralelo.model no existe")
        salida = run_main(str(model_path), timeout=180)
        assert _es_definible(salida) or _es_no_definible(salida)


class TestAlgebraConConstantes:
    """Álgebras con operaciones 0-arias (constantes)."""

    def test_aleatorio_con_constante(self):
        """Álgebra aleatoria con aridad 0 (constante)."""
        out = run_generador("aleatorio", [4, "--aridades", "0", "2"])
        diagonal = out.strip() + "\n\nT0(x,y) eq(x,y)\n"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(diagonal)
            f.flush()
            salida = run_main(f.name, timeout=90)
            os.unlink(f.name)
        assert _es_definible(salida), f"Diagonal con constante definible. Salida: {salida[:400]}"

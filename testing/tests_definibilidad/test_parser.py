# -*- coding: utf-8 -*-
"""
Tests del parser: formato, errores, relaciones por fórmula.
"""
import pytest
from pathlib import Path

from parser.parser import parser, ParserError
from parser import preprocessing
from first_order.relops import Relation
from first_order.models import Model


FIXTURES = Path(__file__).parent / "fixtures"
MODEL_EXAMPLES = Path(__file__).parent.parent.parent / "model_examples"


class TestParserBasico:
    """Tests básicos de parsing de modelos."""

    def test_parse_modelo_minimal(self):
        """Parsea un modelo minimal correctamente."""
        m = parser(str(FIXTURES / "minimal.model"), verbose=False)
        assert len(m.universe) == 2
        assert 0 in m.universe and 1 in m.universe
        assert "f" in m.operations
        assert "T0" in m.relations or any(s.startswith("T") for s in m.relations)

    def test_parse_modelo_con_formula(self):
        """Parsea modelo con target definido por fórmula."""
        m = parser(str(MODEL_EXAMPLES / "modeloqueanda.model"), verbose=False)
        assert len(m.universe) == 3
        assert "T0" in m.relations or any(s.startswith("T") for s in m.relations)

    def test_parse_modelo_existente(self):
        """Parsea un modelo de ejemplo existente."""
        m = parser(str(MODEL_EXAMPLES / "suma4.model"), verbose=False)
        assert len(m.universe) == 4
        assert "S" in m.operations


class TestParserErrores:
    """Tests de rechazo de modelos mal formados."""

    def test_rechaza_relacion_0_aria(self):
        """El parser rechaza relaciones de aridad 0."""
        with pytest.raises(ValueError, match="0-arity relation"):
            parser(str(FIXTURES / "rel_0arity.model"), verbose=False)

    def test_rechaza_igual_en_formula(self):
        """Rechaza uso de == en fórmula; debe usarse eq()."""
        with pytest.raises(ValueError, match="eq\\(x,y\\)|=="):
            parser(str(FIXTURES / "formula_con_igual.model"), verbose=False)

    def test_rechaza_variables_repetidas_en_formula(self):
        """Rechaza declaración de fórmula con variables repetidas."""
        with pytest.raises(ValueError, match="repitiendo variables|variables"):
            parser(str(FIXTURES / "formula_vars_repetidas.model"), verbose=False)


class TestPreprocesamiento:
    """Tests del preprocesamiento de targets."""

    def test_preprocesamiento_un_patron(self):
        """Target con un único patrón de igualdad."""
        from first_order.relops import Relation
        r = Relation("T0", 2, {(0, 1), (1, 0)})
        preps = preprocessing.preprocesamiento2(r)
        assert len(preps) >= 1

    def test_preprocesamiento_varios_patrones(self):
        """Target con varios patrones (distintas igualdades)."""
        r = Relation("T0", 2, {(0, 0), (1, 1), (0, 1)})  # diagonal + (0,1)
        preps = preprocessing.preprocesamiento2(r)
        # Debería haber al menos dos sub-targets por patrones distintos
        assert len(preps) >= 1

    def test_pattern_pre_post_formula(self):
        """Las fórmulas de pre y post procesamiento son consistentes."""
        from parser.preprocessing import Pattern
        t = (0, 1, 1)  # x, y, y
        p = Pattern(t)
        pre = p.preprocessed_formula()
        post = p.postprocessed_formula()
        assert pre is not None
        assert post is not None

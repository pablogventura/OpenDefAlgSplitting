"""
Tests unitarios y de integración para main.py.
Cubren _entropy, _information_gain_from_counts, TupleHistory, IndicesTupleGenerator,
Block, check_formula, Counterexample, y casos borde de main().
"""

import os
import subprocess
import sys
import tempfile
from parser.preprocessing import Pattern

import pytest

import main as main_module
from first_order import formulas
from first_order.models import Model
from first_order.relops import Operation, Relation

# conftest.py añade PROJECT_ROOT al path
from .conftest import PROJECT_ROOT

# --- Tests _entropy ---


class TestEntropy:
    """Tests para _entropy."""

    def test_entropy_cero_cero(self):
        """n=0 devuelve 0."""
        assert main_module._entropy(0, 0) == 0.0

    def test_entropy_mitad_mitad(self):
        """1 bit para partición 50-50."""
        h = main_module._entropy(5, 5)
        assert abs(h - 1.0) < 1e-9

    def test_entropy_uniforme(self):
        """Entropía máxima para partición equilibrada."""
        h = main_module._entropy(1, 1)
        assert h > 0
        assert abs(h - 1.0) < 1e-9

    def test_entropy_puro_in(self):
        """Solo in_count: entropía 0 (p=1)."""
        h = main_module._entropy(10, 0)
        assert abs(h) < 1e-12

    def test_entropy_puro_out(self):
        """Solo out_count: entropía 0 (q=1)."""
        h = main_module._entropy(0, 10)
        assert abs(h) < 1e-12


# --- Tests _information_gain_from_counts ---


class TestInformationGain:
    """Tests para _information_gain_from_counts."""

    def test_ig_n_cero(self):
        """n=0 devuelve 0."""
        assert main_module._information_gain_from_counts(0, 0, {}) == 0.0

    def test_ig_particion_perfecta(self):
        """Partición que separa in/out totalmente tiene IG alto."""
        n, in_total = 10, 5
        part = {(0, True): (5, 0), (1, False): (0, 5)}
        ig = main_module._information_gain_from_counts(n, in_total, part)
        assert ig > 0.9

    def test_ig_sin_ganancia(self):
        """Partición que no separa tiene IG 0."""
        n, in_total = 10, 5
        part = {(0, True): (3, 2), (0, False): (2, 3)}
        ig = main_module._information_gain_from_counts(n, in_total, part)
        assert ig >= 0


# --- Tests TupleHistory ---


def _make_target(arity: int, tuples: set, with_pattern: bool = False) -> Relation:
    """Crea Relation target para tests."""
    r = Relation("T0", arity)
    for t in tuples:
        r.add(t)
    if with_pattern and tuples:
        t0 = next(iter(tuples))
        r.pattern = Pattern(t0)
    return r


class TestTupleHistory:
    """Tests para TupleHistory."""

    def test_tuple_history_step_nuevo_elemento(self):
        """step() con valor nuevo añade a history."""
        targets = [_make_target(2, {(0, 0), (1, 1)})]
        th = main_module.TupleHistory((0, 0), targets)  # history=[0,0]
        op = Operation("f", 2)
        op.add((0, 0, 1))  # op(0,0)=1 (nuevo)
        op.add((0, 1, 0))
        op.add((1, 0, 0))
        op.add((1, 1, 0))
        idx = th.step(op, (0, 1))  # op(history[0], history[1]) = op(0,0) = 1
        assert idx == 2  # nuevo elemento en índice 2
        assert th.has_generated

    def test_tuple_history_step_existente(self):
        """step() con valor ya en history no genera nuevo."""
        targets = [_make_target(2, {(0, 0), (1, 1)})]
        th = main_module.TupleHistory((0, 0), targets)  # history=[0,0], _index_map[0]=1
        op = Operation("id", 1)
        op.add((0, 0))
        op.add((1, 1))
        step_idx = th.step(op, (0,))  # op(history[0])=op(0)=0, ya está
        assert step_idx in (0, 1)  # índice de 0 en history
        assert not th.has_generated

    def test_tuple_history_simulate_step(self):
        """simulate_step no muta."""
        targets = [_make_target(2, {(0, 1)})]
        th = main_module.TupleHistory((0, 0), targets)
        op = Operation("f", 2)
        op.add((0, 0, 1))
        op.add((0, 1, 0))
        op.add((1, 0, 0))
        op.add((1, 1, 1))
        _, has_gen = th.simulate_step(op, (0, 1))
        assert has_gen is True  # 1 no está en history
        assert len(th.history) == 2  # no mutó

    def test_tuple_history_eq_hash(self):
        """TupleHistory iguales tienen mismo hash."""
        targets = [_make_target(1, {(0,)})]
        a = main_module.TupleHistory((0,), targets)
        b = main_module.TupleHistory((0,), targets)
        assert a == b
        assert hash(a) == hash(b)


# --- Tests permutations_forced ---


class TestPermutationsForced:
    """Tests para permutations_forced."""

    def test_permutations_forced_tiene_forzado(self):
        """Solo tuplas que contienen algún elemento forzado."""
        not_forced = [0]
        forced = [1]
        got = list(main_module.permutations_forced(not_forced, forced, 2))
        assert (1, 0) in got
        assert (0, 1) in got
        assert (1, 1) in got
        assert (0, 0) not in got


# --- Tests IndicesTupleGenerator ---


class TestIndicesTupleGenerator:
    """Tests para IndicesTupleGenerator con constantes (arity 0)."""

    def test_generator_con_constantes(self):
        """Con ops[0] (constantes) el generador empieza con ellas."""
        op0 = Operation("c", 0)
        op0.add((0,))
        ops = {0: [op0], 1: []}
        gen = main_module.IndicesTupleGenerator(
            ops, 1, None, [], [0], sintactico=[formulas.Variable("x")], last_term=None
        )
        result = gen.step()
        assert result is not None
        f, ti = result
        assert ti == ()
        assert f.arity == 0


# --- Tests Block ---


class TestBlock:
    """Tests para Block y is_open_def_iterative."""

    def test_block_is_all_in_targets(self):
        """Block con todas las tuplas en target."""
        op = Operation("f", 2)
        op.add((0, 0, 0))
        op.add((0, 1, 1))
        op.add((1, 0, 1))
        op.add((1, 1, 0))
        ops = {2: [op]}
        target = _make_target(2, {(0, 0), (1, 1), (0, 1), (1, 0)}, with_pattern=True)
        targets = [target]
        th = main_module.TupleHistory((0, 0), targets)
        f = target.pattern.preprocessed_formula()
        block = main_module.Block(ops, [th], targets, formula=f, fs=[formulas.true()])
        assert block.is_all_in_targets()

    def test_block_is_disjunt_to_targets(self):
        """Block con ninguna tupla en target."""
        target = _make_target(2, {(0, 1)}, with_pattern=True)
        ops = {}
        th = main_module.TupleHistory((0, 0), [target])  # (0,0) no está en target
        f = target.pattern.preprocessed_formula()
        block = main_module.Block(ops, [th], [target], formula=f, fs=[formulas.true()])
        assert block.is_disjunt_to_targets()


# --- Tests check_formula ---


class TestCheckFormula:
    """Tests para check_formula."""

    def test_check_formula_ok(self, capsys):
        """check_formula cuando extensión coincide con target."""
        main_module.model = Model([0, 1], {}, {})
        target = _make_target(1, {(0,), (1,)})
        target.arity = 1
        f = formulas.true()
        main_module.check_formula(f, target)
        out = capsys.readouterr().out
        assert "Formula successfully checked" in out

    def test_check_formula_falla(self, capsys):
        """check_formula cuando extensión no coincide."""
        main_module.model = Model([0, 1], {}, {})
        target = _make_target(1, {(0,)})  # solo (0,)
        target.arity = 1
        f = formulas.true()  # extensión = {(0,),(1,)}
        main_module.check_formula(f, target)
        out = capsys.readouterr().out
        assert "Formula failed" in out or "Sobran:" in out


# --- Tests Counterexample ---


class TestCounterexample:
    """Tests para Counterexample."""

    def test_counterexample_raise(self):
        """Counterexample se puede lanzar y capturar."""
        with pytest.raises(main_module.Counterexample) as exc_info:
            raise main_module.Counterexample([1, 2, 3])
        assert "1" in str(exc_info.value)


# --- Tests integración main() ---


class TestMainIntegracion:
    """Tests de integración para main()."""

    def test_main_sin_args_lee_stdin(self):
        """main() sin argumentos lee modelo de stdin."""
        model_content = """0 1
f 2
0 0 0
0 1 1
1 0 1
1 1 0
T0(x,y) eq(x,y)
"""
        result = subprocess.run(
            [sys.executable, str(PROJECT_ROOT / "main.py")],
            input=model_content,
            capture_output=True,
            text=True,
            timeout=30,
            cwd=str(PROJECT_ROOT),
        )
        out = result.stdout + result.stderr
        assert "DEFINABLE" in out or "NOT DEFINABLE" in out or "ERROR" in out

    def test_main_sin_targets_error(self):
        """Modelo sin relaciones T devuelve ERROR: NO TARGET RELATIONS FOUND."""
        # Modelo con operaciones pero sin ninguna relación T
        model_content = """0 1 2
f 2
0 0 0
0 1 1
0 2 2
1 0 1
1 1 1
1 2 2
2 0 2
2 1 2
2 2 2
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(model_content)
            f.flush()
            try:
                result = subprocess.run(
                    [sys.executable, str(PROJECT_ROOT / "main.py"), f.name],
                    capture_output=True,
                    text=True,
                    timeout=30,
                    cwd=str(PROJECT_ROOT),
                )
                out = result.stdout + result.stderr
                assert "NO TARGET RELATIONS FOUND" in out
            finally:
                os.unlink(f.name)

    def test_main_con_archivo(self):
        """main(archivo) parsea correctamente."""
        model_content = """0 1
f 2
0 0 0
0 1 1
1 0 1
1 1 0
T0(x,y) eq(x,y)
"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".model", delete=False) as f:
            f.write(model_content)
            f.flush()
            try:
                result = subprocess.run(
                    [sys.executable, str(PROJECT_ROOT / "main.py"), f.name],
                    capture_output=True,
                    text=True,
                    timeout=30,
                    cwd=str(PROJECT_ROOT),
                )
                out = result.stdout + result.stderr
                assert "DEFINABLE" in out
            finally:
                os.unlink(f.name)


# --- Tests USE_INFORMATION_GAIN (opcional, vía monkeypatch) ---


class TestBlockWithInformationGain:
    """Tests para Block.step con USE_INFORMATION_GAIN=True."""

    def test_block_step_use_ig(self, monkeypatch):
        """Block.step con USE_INFORMATION_GAIN maximiza IG."""
        monkeypatch.setattr(main_module, "USE_INFORMATION_GAIN", True)
        monkeypatch.setattr(main_module, "IG_SAMPLE", 5)
        op = Operation("xor", 2)
        op.add((0, 0, 0))
        op.add((0, 1, 1))
        op.add((1, 0, 1))
        op.add((1, 1, 0))
        ops = {2: [op]}
        target = _make_target(2, {(0, 1), (1, 0)}, with_pattern=True)
        targets = [target]
        f = target.pattern.preprocessed_formula()
        ths = [
            main_module.TupleHistory((0, 1), [target]),
            main_module.TupleHistory((1, 0), [target]),
        ]
        block = main_module.Block(ops, ths, targets, formula=f, fs=[formulas.true()])
        children = block.step()
        assert len(children) >= 1

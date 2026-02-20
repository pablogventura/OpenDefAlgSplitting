#!/bin/bash
# Análisis de calidad de código: complejidad, mantenibilidad, duplicación.
# Uso: ./quality_report.sh
set -e
cd "$(dirname "$0")"

PY="python3"
if [ -d .venv ]; then
    PY=".venv/bin/python"
elif [ -d venv ]; then
    PY="venv/bin/python"
fi

# Código principal (excluir testing y generadores)
SRC="main.py first_order parser misc myunicode.py files.py"

echo "=== Complejidad ciclomática (radon cc) ==="
echo "Grados: A=1-5, B=6-10, C=11-20, D=21-50, E=51+. Mostrando C y peores (-n C)"
$PY -m radon cc $SRC -n C -s -a 2>/dev/null || true

echo ""
echo "=== Índice de mantenibilidad (radon mi) ==="
echo "A=20+, B=10-19, C=0-9. Mostrando C y peores (-n C)"
$PY -m radon mi $SRC -n C 2>/dev/null || true

echo ""
echo "=== Métricas raw: SLOC, comentarios, etc. ==="
$PY -m radon raw $SRC -s 2>/dev/null | head -40 || true

echo ""
echo "=== Duplicación de código (pylint R0801) ==="
$PY -m pylint --disable=all --enable=duplicate-code \
    --max-line-length=100 $SRC 2>/dev/null || echo "(sin duplicados o pylint no instalado)"

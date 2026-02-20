#!/bin/bash
# Ejecuta todo el análisis estático y los tests.
# Uso: ./check_all.sh  (o bash check_all.sh)
set -e
cd "$(dirname "$0")"

PY="python3"
if [ -d .venv ]; then
    PY=".venv/bin/python"
elif [ -d venv ]; then
    PY="venv/bin/python"
fi

echo "=== ruff check ==="
$PY -m ruff check .
echo "OK"

echo ""
echo "=== ruff format --check ==="
$PY -m ruff format --check .
echo "OK"

echo ""
echo "=== radon (complejidad + mantenibilidad) ==="
$PY -m radon cc main.py first_order parser misc myunicode.py files.py -n D -a -q 2>/dev/null || true
$PY -m radon mi main.py first_order parser misc myunicode.py files.py -n C -q 2>/dev/null || true

echo ""
echo "=== pyright ==="
$PY -m pyright
echo "OK"

echo ""
echo "=== pytest con cobertura (incluye subprocesos como main.py) ==="
$PY -m coverage run -m pytest testing/ -q
$PY -m coverage combine
$PY -m coverage report --show-missing
echo "OK"

echo ""
echo "=== Todo pasado ==="

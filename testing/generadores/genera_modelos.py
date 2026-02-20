#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Interfaz unificada para generar ejemplos de modelos de álgebras finitas.

Uso típico:
    python genera_modelos.py boole 3 -o modelo_boole.model
    python genera_modelos.py aleatorio 7 --aridades 2 --target 2 0.1 -o modelo.model
    python genera_modelos.py grupo-abeliano 2 2 2 --target 2 0.5
"""
import argparse
import io
import subprocess
import sys
from pathlib import Path

# Directorio base para importar generadores
GENERADORES_DIR = Path(__file__).resolve().parent


def _run_generator(script_name, args, capture=True):
    """Ejecuta un generador y devuelve su salida."""
    script = GENERADORES_DIR / script_name
    if not script.exists():
        raise FileNotFoundError(f"No se encuentra el generador: {script_name}")
    cmd = [sys.executable, str(script)] + [str(a) for a in args]
    result = subprocess.run(cmd, capture_output=capture, text=True, cwd=str(GENERADORES_DIR))
    if result.returncode != 0:
        if not capture:
            sys.exit(result.returncode)
        raise RuntimeError(f"Generador falló: {result.stderr or result.stdout}")
    return result.stdout if capture else None


def _extract_cardinality(content):
    """Extrae la cardinalidad del universo de la primera línea del modelo."""
    lines = content.strip().split("\n")
    for line in lines:
        line_clean = line.split("#")[0].strip()
        if line_clean:
            return len(line_clean.split())
    return 8


def _add_target(content, arity, density):
    """Añade un target aleatorio al contenido del modelo."""
    cardinality = _extract_cardinality(content)
    result = subprocess.run(
        [sys.executable, str(GENERADORES_DIR / "randomtarget_separado.py"),
         str(cardinality), str(arity), str(density)],
        capture_output=True, text=True, cwd=str(GENERADORES_DIR)
    )
    if result.returncode != 0:
        raise RuntimeError(f"Error al generar target: {result.stderr}")
    return content.rstrip() + "\n\n" + result.stdout


def cmd_boole(args):
    """Álgebra de Boole con 2^n elementos."""
    out = _run_generator("genera_boole.py", [args.n])
    if args.target:
        out = _add_target(out, args.target[0], args.target[1])
    return out


def cmd_aleatorio(args):
    """Álgebra con operaciones aleatorias."""
    aridades = args.aridades or [2]
    aridades_str = str(aridades)
    out = _run_generator("genera_alg_random.py", [
        args.cardinalidad, args.subs or 0, args.tam_subs or 0, aridades_str
    ])
    if args.target:
        out = _add_target(out, args.target[0], args.target[1])
    return out


def cmd_grupo_abeliano(args):
    """Grupo abeliano como producto Z_n1 x Z_n2 x ..."""
    out = _run_generator("genera_grupo_abeliano.py", args.ordenes)
    if args.target:
        out = _add_target(out, args.target[0], args.target[1])
    return out


def cmd_grupo_abeliano_diverso(args):
    """Grupo abeliano diverso de tamaño 2^k."""
    out = _run_generator("genera_grupo_abeliano_diverso.py", [args.k])
    if args.target:
        out = _add_target(out, args.target[0], args.target[1])
    return out


def cmd_grupo_no_abeliano(args):
    """Grupo (de permutaciones) no abeliano."""
    a = [args.k, args.generadores]
    if args.cardinalidad_exacta is not None:
        a.append(args.cardinalidad_exacta)
    out = _run_generator("genera_grupo_no_abeliano.py", a)
    if args.target:
        out = _add_target(out, args.target[0], args.target[1])
    return out


def cmd_reticulado(args):
    """Reticulado distributivo (subálgebra de Boole)."""
    out = _run_generator("genera_ret.py", [args.ancho, args.muestra])
    if args.target:
        out = _add_target(out, args.target[0], args.target[1])
    return out


def cmd_target(args):
    """Solo target aleatorio (sin operaciones)."""
    return _run_generator("randomtarget_separado.py", [
        args.cardinalidad, args.aridad, args.densidad
    ])


def main():
    # Parser base con opciones globales (usar parents para que las hereden los subcomandos)
    global_parser = argparse.ArgumentParser(add_help=False)
    global_parser.add_argument(
        "-o", "--output",
        metavar="ARCHIVO",
        help="Archivo de salida (por defecto: stdout)",
    )
    global_parser.add_argument(
        "--target",
        nargs=2,
        metavar=("ARIDAD", "DENSIDAD"),
        type=lambda x: int(x) if "." not in x else float(x),
        help="Añadir relación target T0 aleatoria: aridad (int) y densidad (0-1)",
    )

    parser = argparse.ArgumentParser(
        description="Genera ejemplos de modelos de álgebras finitas para OpenDefAlgSplitting.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        parents=[global_parser],
        epilog="""
Ejemplos:
  genera_modelos.py boole 3 -o boole8.model
  genera_modelos.py aleatorio 8 --aridades 2 2 --target 2 0.1 -o random.model
  genera_modelos.py grupo-abeliano 2 2 2 --target 2 0.5 -o z2z2z2.model
  genera_modelos.py target 10 2 0.3 -o solo_target.model
        """,
    )

    subparsers = parser.add_subparsers(dest="comando", required=True, help="Tipo de modelo")

    # boole
    p_boole = subparsers.add_parser("boole", parents=[global_parser], help="Álgebra de Boole (2^n elementos)")
    p_boole.add_argument("n", type=int, help="Exponente: álgebra tendrá 2^n elementos")
    p_boole.set_defaults(func=cmd_boole)

    # aleatorio
    p_aleatorio = subparsers.add_parser("aleatorio", parents=[global_parser], help="Álgebra con operaciones aleatorias")
    p_aleatorio.add_argument("cardinalidad", type=int, help="Cardinalidad del universo")
    p_aleatorio.add_argument("--subs", type=int, default=0, help="Cantidad de subuniversos (0)")
    p_aleatorio.add_argument("--tam-subs", type=int, default=0, help="Tamaño de subuniversos (0)")
    p_aleatorio.add_argument("--aridades", nargs="+", type=int, default=None,
                             help="Aridades de operaciones, ej: 2 2 para dos binarias")
    p_aleatorio.set_defaults(func=cmd_aleatorio)

    # grupo-abeliano
    p_ga = subparsers.add_parser("grupo-abeliano", parents=[global_parser], help="Grupo abeliano Z_n1 x Z_n2 x ...")
    p_ga.add_argument("ordenes", nargs="+", type=int, help="Órdenes cíclicos, ej: 2 2 2 para Z2³")
    p_ga.set_defaults(func=cmd_grupo_abeliano)

    # grupo-abeliano-diverso
    p_gad = subparsers.add_parser("grupo-abeliano-diverso", parents=[global_parser], help="Grupo abeliano diverso de tamaño 2^k")
    p_gad.add_argument("k", type=int, help="Tamaño = 2^k")
    p_gad.set_defaults(func=cmd_grupo_abeliano_diverso)

    # grupo-no-abeliano
    p_gna = subparsers.add_parser("grupo-no-abeliano", parents=[global_parser], help="Grupo de permutaciones (no abeliano)")
    p_gna.add_argument("k", type=int, help="k para grupo de k-permutaciones")
    p_gna.add_argument("generadores", type=int, help="Cantidad de generadores iniciales")
    p_gna.add_argument("--cardinalidad-exacta", type=int, default=None,
                       help="Cardinalidad deseada del subgrupo (opcional)")
    p_gna.set_defaults(func=cmd_grupo_no_abeliano)

    # reticulado
    p_ret = subparsers.add_parser("reticulado", parents=[global_parser], help="Reticulado distributivo (subálgebra de Boole)")
    p_ret.add_argument("ancho", type=int, help="Álgebra de Boole ambiente: 2^ancho elementos")
    p_ret.add_argument("muestra", type=int, help="Número de elementos para generar subuniverso")
    p_ret.set_defaults(func=cmd_reticulado)

    # target solo
    p_target = subparsers.add_parser("target", parents=[global_parser], help="Solo target aleatorio (sin operaciones)")
    p_target.add_argument("cardinalidad", type=int, help="Cardinalidad del universo")
    p_target.add_argument("aridad", type=int, help="Aridad del target")
    p_target.add_argument("densidad", type=float, help="Densidad (0-1)")
    p_target.set_defaults(func=cmd_target)

    args = parser.parse_args()

    # Validar --target cuando aplica
    if hasattr(args, "func") and args.func != cmd_target and args.target:
        arity_val = args.target[0]
        density_val = args.target[1]
        if not isinstance(arity_val, int) or not (0 <= density_val <= 1):
            parser.error("--target: aridad debe ser entero, densidad entre 0 y 1")
        args.target = (int(arity_val), float(density_val))

    output = args.func(args)
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
        print(f"Modelo guardado en: {args.output}", file=sys.stderr)
    else:
        print(output, end="")


if __name__ == "__main__":
    main()

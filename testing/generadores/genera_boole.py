import sys

from genera_ret import closure


def main() -> None:
    try:
        ancho = int(sys.argv[1])
    except (ValueError, IndexError):
        print("Toma i tal que el algebra de boole tenga 2^i elementos")
        sys.exit(1)
    muestra = 2**ancho

    closure(ancho, muestra)


if __name__ == "__main__":
    main()

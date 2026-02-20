#!/usr/bin/env python

from typing import Union


def subscript(string: Union[int, str]) -> str:
    """
    Devuelve un texto en subindice usando unicode

    >>> subscript(234)
    '₂₃₄'
    >>> subscript("esto es una prueba")
    'ₑₛₜₒ ₑₛ ᵤₙₐ ₚᵣᵤₑbₐ'
    """
    string = str(string)
    table = str.maketrans("0123456789aehijklmnoprstuvxə+-=()", "₀₁₂₃₄₅₆₇₈₉ₐₑₕᵢⱼₖₗₘₙₒₚᵣₛₜᵤᵥₓₔ₊₋₌₍₎")
    return string.translate(table)

from parser.parser import parser
from random import sample
import sys
from hit import TupleModelHash
from time import time
from interfaces.minion import is_isomorphic
from colorama import Fore, Style


def main():
    model = parser(sys.argv[1], verbose=False)
    model.relations = {} # este test es sin relaciones
    
    for i in range(10):
        if len(model.universe) >= 3:
            ta = sample(model.universe, 3)
            tb = sample(model.universe, 3)
        else:
            ta = sample(model.universe, len(model.universe)-1)
            tb = sample(model.universe, len(model.universe)-1)
        
        start_hit = time()
        hit_return = TupleModelHash(model, ta) == TupleModelHash(model, tb)
        time_hit = time() - start_hit
        
        start_minion = time()
        a = model.substructure(ta)
        b = model.substructure(tb)
        subtype = sorted(model.relations.keys())
        ra = a.to_relational_model()
        rb = b.to_relational_model()
        minion_return = is_isomorphic(ra, rb, subtype) != False
        time_minion = time() - start_minion
        print("*" * 80)
        if time_hit <= time_minion:
            print(Fore.GREEN + "Hit(%s) == Hit(%s) = %s, hit/minion= %s" % (
            ta, tb, hit_return, time_hit / time_minion) + Style.RESET_ALL)
            print(Fore.RED + "Minion(%s) == Minion(%s) = %s, minion/hit= %s" % (
            ta, tb, minion_return, time_minion / time_hit) + Style.RESET_ALL)
        else:
            print(Fore.RED + "Hit(%s) == Hit(%s) = %s, hit/minion= %s" % (
            ta, tb, hit_return, time_hit / time_minion) + Style.RESET_ALL)
            print(Fore.GREEN + "Minion(%s) == Minion(%s) = %s, minion/hit= %s" % (
            ta, tb, minion_return, time_minion / time_hit) + Style.RESET_ALL)


if __name__ == "__main__":
    main()
from parser.parser import parser
from random import sample,seed
import sys
from hit import TupleModelHash
from time import time
from interfaces.minion import is_isomorphic
from itertools import product

def main():
    model = parser(sys.argv[1], verbose=False)
    model.relations = {} # este test es sin relaciones
    
    tuples=[]
    for i in range(50):
        try:
            tuples.append(tuple(sample(model.universe, 3)))
        except ValueError:
            tuples.append(tuple(sample(model.universe, 2)))
    # estan generadas las tuplas
        
    start_hit = time()
    hits = []
    for t in tuples:
        hits.append(TupleModelHash(model, t))
    return_hits=[]
    for h1,h2 in product(hits,hits):
        return_hits.append(h1==h2)
    time_hit = time() - start_hit
        
    start_minion = time()
    substructures = dict()
    for t in tuples:
        substructures[t] = model.substructure(t).to_relational_model()
    subtype = sorted(substructures[tuples[0]].relations.keys())
    # print("Subtipo %s" % subtype)
    return_minion=[]
    for t1, t2 in product(substructures.keys(), substructures.keys()):
        return_minion.append(bool(is_isomorphic(substructures[t1], substructures[t2], subtype, t1, t2)))
    time_minion = time() - start_minion
    return_hits= "".join(str(int(i)) for i in return_hits)
    return_minion = "".join(str(int(i)) for i in return_minion)

    print("Hit    = %s" % time_hit)
    print("Minion = %s" % time_minion)

if __name__ == "__main__":
    j=main()
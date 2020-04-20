from collections import defaultdict
import os
from functools import reduce # Valid in Python 2.6+, required in Python 3
import operator

results = defaultdict(list)
files = [os.path.join(dp, f) for dp, dn, fn in os.walk("testing/mega_hit_test") for f in fn]
for i, f in enumerate(files):
    if f.endswith(".megahit"):
        
        _,_,dir,filename = f.split("/")
        filename = filename[:-len(".megahit")]
        file = open(f,"r")
        try:
            counter=0
            while "*" not in file.readline():  # asteriscos basura
                counter+=1
                if counter>20:
                    raise ValueError
            file.readline() #definible
            if dir.endswith("boole"):
                s,_=filename.split("_", 1)
                size = 2**int(s)
            elif dir.endswith("alg_random"):
                s,_=filename.split("_", 1)
                size = int(s)
            elif dir.endswith("grupo_abeliano"):

                size = reduce(operator.mul,list( int(x) for x in filename.split("_"))[:-1],1)
                print(size)
            else:
                size = map(int,filename.split("_"))

            results[(dir,size)].append(float(file.readline()[14:]))
        except ValueError:
            print("ERROR in file %s" % f.replace(" ","\ "))
        file.close()
new_results = dict()
for k in results:
    value = 0
    size = len(results[k])
    while results[k]:
        h=results[k].pop()
        value = value+h
    value = value/size
    print(size)
    new_results[k]=value
    print()
    print(k)
    print("Time: %s" % value)

new_new_results = defaultdict(list)
for k in new_results:
    new_new_results[k[0]].append((k[1],new_results[k]))
cantidad = -4
new_new_results2 = {k:sorted(new_new_results[k],key=lambda x: x[0]) for k in new_new_results}

print(new_new_results2)
new_new_results=dict()
for k in new_new_results2:
    if len(new_new_results2[k][cantidad:]) == -cantidad:
        new_new_results[k] = new_new_results2[k][cantidad:]
    else:
        new_new_results[k] = new_new_results2[k]

print(new_new_results)

import matplotlib.pyplot as plt
import numpy as np


ax = plt.subplot(111)
ax.set_ylabel('Time ($s$)')
ax.set_xlabel('Cardinality of Universe')
ax.set_title('Amount of time to decide definability')

#plt.semilogx(2)
plt.loglog(basex=2,basey=2)
#plt.semilogx(basex=2)
print(new_new_results)

k = "alg_random"
label ="Random Algebras"
plt.plot([4,8,16, 32], [i[1] for i in new_new_results[k]], label=label,color="black",marker='v')
k = "grupo_abeliano"
label ="Abelian Groups"
plt.plot([4,8,16, 32], [i[1] for i in new_new_results[k]], label=label,color="black",marker='o')
k = "boole"
label ="Boolean Algebras"
plt.plot([4,8,16, 32], [i[1] for i in new_new_results[k]], label=label,color="black",marker='s')

leg = plt.legend(loc='best', ncol=1) #, shadow=True)#, fancybox=True)
leg.get_frame().set_alpha(0.5)


plt.show()

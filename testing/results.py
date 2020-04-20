import sys

from collections import defaultdict
# filename = sys.argv[1]
# f=open(filename,"r")
# f.readline() # asteriscos basura
# print(float(f.readline()[24:-5])) # hit
# print(float(f.readline()[24:-5]) # minion
# f.close()

import os

results = defaultdict(list)
files = [os.path.join(dp, f) for dp, dn, fn in os.walk("testing") for f in fn]
for i, f in enumerate(files):
    if f.endswith(".hvm"):
        
        _,dir,filename = f.split("/")
        file = open(f,"r")
        try:
            results[dir].append((float(file.readline()[9:-1]),float(file.readline()[9:-1])))
        except ValueError:
            print("ERROR in file %s" % f.replace(" ","\ "))

new_results = dict()
for k in results:
    value = (0,0)
    size = len(results[k])
    while results[k]:
        h,m=results[k].pop()
        value = (value[0]+h,value[1]+m)
    value = (value[0]/size,value[1]/size)
    new_results[k]=value
    print()
    print(k)
    print("Hit:    %s with %s samples" % (value[0],size))
    print("Minion: %s with %s samples" % (value[1], size))
    
import numpy as np
import matplotlib.pyplot as plt


data = [(k,new_results[k][0],new_results[k][1]) for k in new_results]

print(data)

temp_data= sorted(data,key=lambda v:v[2])
data=[]
for n,h,m in temp_data:
    if n == "ret":
        data.append(("Distributive\nLattices",h,m))
    elif n == "grupo_no_abeliano":
        data.append(("Groups",h,m))
    elif n == "grupo_abeliano":
        data.append(("Abelian\nGroups",h,m))
    elif n == "boole":
        data.append(("Boolean\nAlgebras",h,m))
    elif n == "alg_random":
        data.append(("Random\nAlgebras",h,m))
    else:
        data.append((n, h, m))
        

hit_means = tuple(v[1] for v in data)
minion_means = tuple(v[2] for v in data)

ind = np.arange(len(hit_means))  # the x locations for the groups
width = 0.35  # the width of the bars

fig, ax = plt.subplots()
rects1 = ax.bar(ind - width/2, hit_means, width, color='white', hatch="//", edgecolor='black', label='IsoType Strategy')
rects2 = ax.bar(ind + width/2, minion_means, width, color='black', edgecolor='black', label='CPS Strategy')

# Add some text for labels, title and custom x-axis tick labels, etc.
ax.set_ylabel('Time ($s$)')
ax.set_title('Amounts of time by families of algebras')
ax.set_xticks(ind)
ax.set_xticklabels(v[0] for v in data)
ax.legend()
plt.semilogy(np.exp(1/5.0))

def autolabel(rects, xpos='center'):
    """
    Attach a text label above each bar in *rects*, displaying its height.

    *xpos* indicates which side to place the text w.r.t. the center of
    the bar. It can be one of the following {'center', 'right', 'left'}.
    """

    xpos = xpos.lower()  # normalize the case of the parameter
    ha = {'center': 'center', 'right': 'left', 'left': 'right'}
    offset = {'center': 0.5, 'right': 0.57, 'left': 0.43}  # x_txt = x + w*off

    for rect in rects:
        height = rect.get_height()
        ax.text(rect.get_x() + rect.get_width()*offset[xpos], 1.01*height,
                '{}'.format(height), ha=ha[xpos], va='bottom')


#autolabel(rects1, "left")
#autolabel(rects2, "right")

plt.show()
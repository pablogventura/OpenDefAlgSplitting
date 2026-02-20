from itertools import product

universe = 20

print(" ".join(str(e) for e in range(universe)))

target = set()
for a, b, c, d in product(range(universe), repeat=4):
    if a == b or c == d:
        target.add((a, b, c, d))

print(f"T {len(target)} 4")
for t in target:
    print(" ".join(str(t_0) for t_0 in t))

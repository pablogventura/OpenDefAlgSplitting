from first_order import formulas

x,y,z=formulas.variables(*"xyz")

eq = formulas.eq

a=eq(x,x)

b=eq(y,y)

f=formulas.RelSym("f",1)

k=f(z)&a

In [9]: k
Out[9]: f(z)(z,x)

In [10]: a
Out[10]: ⊤(x)

In [11]: b
Out[11]: ⊤(y)

j=f(z)&b

In [13]: k
Out[13]: f(z)(y,z,x)

In [14]: b
Out[14]: ⊤(y)

In [15]: f(z)
Out[15]: f(z)(y,z,x)

l=f(z)

In [17]: l.orphan_vars
Out[17]: {x, y}

In [18]: f(z)
Out[18]: f(z)(y,z,x)

In [19]: f(x)
Out[19]: f(x)(y,x)

In [20]: f(y)
Out[20]: f(y)(y,x)

In [21]: x.free_vars
Out[21]: <bound method Variable.free_vars of x>

In [22]: x.free_vars()
Out[22]: {x}

In [23]: x.free_vars()


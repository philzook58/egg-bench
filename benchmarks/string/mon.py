from z3 import *

T = DeclareSort("T")
mul = Function("mul", T, T, T)
x, y, z, a, b = Consts("x y z a b", T)
ExprRef.__mul__ = lambda self, other: mul(self, other)

eqs = [ForAll([x, y, z], x * (y * z) == (x * y) * z), a * a * a == b * b]
target = a * a * a

s = Solver()
s.add(eqs)
with open("mon.smt2", "w") as f:
    f.write(s.to_smt2())
    f.write(f"\n(optimize {target.serialize()})\n")

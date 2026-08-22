# Seven trees semiring benchmarks

Standalone egglog benchmarks for commutative-semiring identities forced by one
polynomial equation. Run one with:

```sh
egglog 01_seven_trees.egg
```

| file | hypothesis | target | egglog baseline |
|---|---|---|---|
| `01_seven_trees.egg` | `x = 1 + x^2` | `x^7 = x` | passes |
| `02_five_trees.egg` | `x = 1 + x + x^2` | `x^5 = x` | passes |
| `03_four_trees.egg` | `x = 1 + 2x + x^2` | `x^4 = x` | passes |
| `04_quadratic_n4.egg` | `x = 4 + x + x^2` | `x^5 = 16x` | passes |
| `05_cyclotomic_c3.egg` | `x = 3^2 + 4x + x^2` | `x^4 = 27x` | slow |
| `06_quadratic_2_3.egg` | `x = 2 + 3x + x^2` | `x^9 = 16x` | slow |
| `07_cubic_n4.egg` | `x = 4 + x + x^3` | `x^7 = 16x` | passes |
| `08_cubic_phi4.egg` | `x = 1 + 2x + x^2 + x^3` | `x^5 = x` | passes |
| `09_cubic_mixed.egg` | `x = 1 + 3x + 2x^2 + x^3` | `x^7 = x` | passes |
| `10_cubic_candidate1.egg` | `x = 8 + 5x + 2x^2 + x^3` | `x^5 = 16x` | passes |
| `11_cubic_candidate2_hard.egg` | `x = 8 + 9x + 4x^2 + x^3` | `x^7 = 64x` | slow |
| `12_symbolic_phi6.egg` | `cx = c^2 + x^2` | `cx^7 = c^7x` | currently fails |

The first three are the classical seven-, five-, and four-trees examples. The
numeric coefficients are repeated sums of `One`; only the last benchmark has a
genuine second generator `C`. The later examples are intentionally useful as
stress tests. The baseline column records this naive ruleset with a 20-second
wall-clock limit; `slow` means it hit that limit. The symbolic identity is known
to have a short `semi.py` proof, but this egglog encoding reaches its iteration
limit without closing the goal.

The underlying results are related to:

- Andreas Blass, *Seven Trees in One*, https://arxiv.org/abs/math/9405205
- Marcelo Fiore and Tom Leinster, *Objects of Categories as Complex Numbers*,
  https://arxiv.org/abs/math/0212377

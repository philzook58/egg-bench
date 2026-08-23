# SAT

Convert DIMACS CNF files into eq-prog-opt Boolean expressions:

```sh
../../tools/.venv/bin/python ../../tools/dimacs_to_eqbench.py example.cnf -o example.eqbench.lisp
```

The generated optimize term is a nested conjunction of disjunctions:

```lisp
(And (Or ...) (And (Or ...) ...))
```

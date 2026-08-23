# Circuits

Convert AIGER `.aig`/`.aag` files or BLIF `.blif` files into eq-prog-opt Boolean expressions:

```sh
../../.venv/bin/python ../../tools/aig_blif_to_eqbench.py example.aag -o example-aag.eqbench.lisp
../../.venv/bin/python ../../tools/aig_blif_to_eqbench.py example.blif -o example-blif.eqbench.lisp
```

The generated optimize term wraps each named output and conjoins them:

```lisp
(And (Out "o0" ...) (Out "o1" ...))
```

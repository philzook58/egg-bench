# MicroHs functional programs

This directory contains Haskell source programs and their MicroHs combinator
outputs.

From `benchmarks/MicroHs`, build the compiler and evaluator:

```sh
make bin/mhs bin/mhseval
```

Generate the list append combinator file:

```sh
bin/mhs -ilib -isrc -imhs ../functional_prog/microhs/ListAppend.hs -o../functional_prog/microhs/ListAppend.comb
```

Run it with the combinator evaluator:

```sh
bin/mhseval +RTS -r../functional_prog/microhs/ListAppend.comb -RTS
```

Convert the combinator file to the eq-prog-opt benchmark format:

```sh
../../../tools/.venv/bin/python ../../../tools/comb_to_eqbench.py ListAppend.comb -o ListAppend.eqbench.lisp
```


<https://github.com/sampsyo/bril/tree/main/benchmarks>

The `bril` submodule provides benchmark programs. To sample random control-flow
traces and emit eq-prog-opt input:

```sh
../../tools/.venv/bin/python ../../tools/bril_random_traces.py \
  bril/benchmarks/core bril/benchmarks/mem \
  -n 8 --seed 2 --max-steps 30 \
  -o bril-random-traces.eqbench.lisp
```

The generated expressions inline SSA values along the sampled path. Memory and
other effects are threaded explicitly with constructors such as `Alloc`, `Store`,
`Load`, `Free`, `Print`, and `Ret`.

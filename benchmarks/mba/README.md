# MBA deobfuscation

Concrete fixed-width bitvector expression/target pairs in `mba-deobfuscation.egg`.
It defines an AST only; a benchmark runner should provide its chosen rewrite
rules and assert equality of each `$*-source` and `$*-target` pair after its
saturation budget.

## Attribution

- `simplifier-*` pairs were copied or transcribed from
  [mazeworks-security/Simplifier](https://github.com/mazeworks-security/Simplifier),
  specifically `Mba.Simplifier/DSL/simplification.rules` and its README
  semi-linear example. Simplifier is GPL-3.0-or-later.
- `gamba-syntia-*` and `gamba-neureduce-*` pairs were copied from
  [DenuvoSoftwareSolutions/GAMBA](https://github.com/DenuvoSoftwareSolutions/GAMBA),
  `experiments/datasets/syntia.txt` and `experiments/datasets/neureduce.txt`.
  GAMBA is GPL-3.0.

The upstream files retain their respective licenses. This is a small selected
subset, not a replacement for either corpus.

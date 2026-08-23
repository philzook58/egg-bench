# Group Theory

These are executable Egglog word-problem benchmarks. They use a binary
operation `gmul`, an identity element, and either a true inverse function or
an explicit alphabet of inverse letters. Every file ends in `check` commands:
the asserted equality is the expected normal form.
Each program uses a bounded `run ... :until` so it stops as soon as its hardest
target is established instead of saturating indefinitely.

Run one benchmark with:

```sh
egglog group-axioms.egg
```

`group-axioms.egg` contains only the group axioms, oriented to a right-associated
word normal form. Its targets require repeated reassociation and cancellation;
they do not assume commutativity.

`a4-kbmag.egg` is the alternating group A_4 presentation
`< a, b | a^2 = b^3 = (a*b)^3 = 1 >`, using the completed rewriting system
shown in KBMAG's first example. It tests both the original relator and a word
that needs a completion rule.

`heisenberg.egg` is a normal-form slice of KBMAG's Heisenberg example. It
combines free reduction, central-letter collection, and the relation
`y*x = x*y*z`, so the checks require several interacting rule families.

KBMAG is the source for the finite-presentation examples and its distinction
between a group generator and an explicit alphabet letter for its inverse:

- <https://gap-packages.github.io/kbmag/doc/chap2.html>

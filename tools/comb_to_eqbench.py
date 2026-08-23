#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path

from lark import Lark, Token, Transformer


GRAMMAR = r"""
start: VERSION COUNT record*

record: "A" item* LABEL APPLY      -> definition
      | item* END                  -> main_expr

item: ATOM | STRING | APPLY

VERSION: /v[0-9]+(\.[0-9]+)*/
COUNT: /[0-9]+/
LABEL: ":" /[^ \t\r\n@}]+/
APPLY: "@"
END: "}"
ATOM: /[^ \t\r\n@:"}]+/
STRING: ESCAPED_STRING

%import common.ESCAPED_STRING
%import common.WS
%ignore WS
"""


@dataclass(frozen=True)
class Atom:
    value: str


@dataclass(frozen=True)
class App:
    fun: Term
    arg: Term


Term = Atom | App


@dataclass(frozen=True)
class Definition:
    name: str
    body: Term


@dataclass(frozen=True)
class CombFile:
    version: str
    definitions: list[Definition]
    main: Term | None


class CombTransformer(Transformer):
    def start(self, children: list[object]) -> CombFile:
        version = str(children[0])
        definitions = [c for c in children[2:] if isinstance(c, Definition)]
        mains = [c for c in children[2:] if not isinstance(c, Definition)]
        return CombFile(version, definitions, mains[-1] if mains else None)

    def item(self, children: list[object]) -> object:
        return children[0]

    def definition(self, children: list[object]) -> Definition:
        label = children[-2]
        return Definition(str(label)[1:], stack_expr(children[:-2]))

    def main_expr(self, children: list[object]) -> Term | None:
        return stack_expr(children[:-1], allow_trailing_apply=True) if len(children) > 1 else None


def stack_expr(items: list[object], allow_trailing_apply: bool = False) -> Term:
    stack: list[Term] = []
    for item in items:
        if isinstance(item, Token) and item.type == "APPLY":
            if allow_trailing_apply and len(stack) == 1:
                continue
            if len(stack) < 2:
                raise ValueError("application marker needs two preceding terms")
            arg = stack.pop()
            fun = stack.pop()
            stack.append(App(fun, arg))
        elif isinstance(item, Token):
            stack.append(Atom(str(item)))
        else:
            raise TypeError(f"unexpected parse item: {item!r}")
    if len(stack) != 1:
        raise ValueError(f"expected one expression, got {len(stack)} stack entries")
    return stack[0]


def parse_comb(path: Path) -> CombFile:
    parser = Lark(GRAMMAR, parser="lalr", transformer=CombTransformer())
    return parser.parse(collapse_byte_strings(path.read_text(encoding="latin1")))


def collapse_byte_strings(text: str) -> str:
    out: list[str] = []
    i = 0
    while i < len(text):
        if text[i] == "$":
            j = i + 1
            while j < len(text) and text[j].isdigit():
                j += 1
            if j > i + 1 and j < len(text) and text[j] == " ":
                size = int(text[i + 1 : j])
                raw = text[j + 1 : j + 1 + size]
                out.append(string_atom("$" + str(size) + ":" + raw.encode("latin1").hex()))
                i = j + 1 + size
                continue
        out.append(text[i])
        i += 1
    return "".join(out)


def string_atom(text: str) -> str:
    if '"' in text or "\n" in text or "\r" in text:
        text = "hex:" + text.encode("latin1").hex()
    return '"' + text + '"'


def atom_name(text: str) -> str:
    if text.startswith('"') or text.startswith("$"):
        return f"(Lit {string_atom(text)})"
    if text.startswith("#") and re.fullmatch(r"#-?[0-9]+", text):
        return f"(Int {text[1:]})"
    return f"(Ref {string_atom(text)})"


def sexp(term: Term) -> str:
    if isinstance(term, Atom):
        return atom_name(term.value)
    return f"(App {sexp(term.fun)} {sexp(term.arg)})"


def rewrite(name: str, lhs: str, rhs: str) -> str:
    return f"(rewrite {name}\n         {lhs}\n         {rhs})"


def emit_eqbench(comb: CombFile) -> str:
    out = [
        f";; generated from MicroHs combinator file {comb.version}",
        "(sort Comb)",
        "",
        "(function Ref (String) Comb)",
        "(function Lit (String) Comb)",
        "(function Int (i64) Comb)",
        "(function App (Comb Comb) Comb)",
        "",
        ";; Core MicroHs combinator reduction rules.",
        rewrite("comb.S", "(App (App (App (Ref \"S\") x) y) z)", "(App (App x z) (App y z))"),
        rewrite("comb.K", "(App (App (Ref \"K\") x) y)", "x"),
        rewrite("comb.A", "(App (App (Ref \"A\") x) y)", "y"),
        rewrite("comb.I", "(App (Ref \"I\") x)", "x"),
        rewrite("comb.Y", "(App (Ref \"Y\") x)", "(App x (App (Ref \"Y\") x))"),
        rewrite("comb.B", "(App (App (App (Ref \"B\") x) y) z)", "(App x (App y z))"),
        rewrite("comb.C", "(App (App (App (Ref \"C\") x) y) z)", "(App (App x z) y)"),
        rewrite("comb.Sprime", "(App (App (App (App (Ref \"S'\") x) y) z) w)", "(App (App x (App y w)) (App z w))"),
        rewrite("comb.Bprime2", "(App (App (Ref \"B'\") x) y)", "(App (Ref \"B\") (App x y))"),
        rewrite("comb.Bprime4", "(App (App (App (App (Ref \"B'\") x) y) z) w)", "(App (App x y) (App z w))"),
        rewrite("comb.Cprime", "(App (App (App (App (Ref \"C'\") x) y) z) w)", "(App (App x (App y w)) z)"),
        rewrite("comb.P", "(App (App (App (Ref \"P\") x) y) z)", "(App (App z x) y)"),
        rewrite("comb.R2", "(App (App (Ref \"R\") x) y)", "(App (App (Ref \"C\") y) x)"),
        rewrite("comb.R3", "(App (App (App (Ref \"R\") x) y) z)", "(App (App y z) x)"),
        rewrite("comb.O", "(App (App (App (App (Ref \"O\") x) y) z) w)", "(App (App w x) y)"),
        rewrite("comb.U", "(App (App (Ref \"U\") x) y)", "(App y x)"),
        rewrite("comb.Z2", "(App (App (Ref \"Z\") x) y)", "(App (Ref \"K\") (App x y))"),
        rewrite("comb.Z3", "(App (App (App (Ref \"Z\") x) y) z)", "(App x y)"),
        rewrite("comb.J", "(App (App (App (Ref \"J\") x) y) z)", "(App z x)"),
        rewrite("comb.L", "(App (App (App (Ref \"L\") x) y) z)", "(App y x)"),
        rewrite("comb.KK", "(App (App (App (Ref \"KK\") x) y) z)", "y"),
        rewrite("comb.KA", "(App (App (App (Ref \"KA\") x) y) z)", "z"),
        rewrite("comb.K2", "(App (App (App (Ref \"K2\") x) y) z)", "x"),
        rewrite("comb.K3", "(App (App (App (App (Ref \"K3\") x) y) z) w)", "x"),
        rewrite("comb.K4", "(App (App (App (App (App (Ref \"K4\") x) y) z) w) v)", "x"),
        rewrite("comb.CprimeB3", "(App (App (App (Ref \"C'B\") x) y) z)", "(App (App (Ref \"B\") (App x z)) y)"),
        rewrite("comb.CprimeB4", "(App (App (App (App (Ref \"C'B\") x) y) z) w)", "(App (App x z) (App y w))"),
        "",
        ";; Combinator-file definitions.",
    ]
    for definition in comb.definitions:
        out.append(rewrite(f"def.{safe_rewrite_name(definition.name)}", sexp(Atom(definition.name)), sexp(definition.body)))
    if comb.main is not None:
        out.extend(["", ";; Program entry expression.", f"(optimize {sexp(comb.main)})"])
    return "\n".join(out) + "\n"


def safe_rewrite_name(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", name)


def main() -> None:
    argp = argparse.ArgumentParser(description="Convert a MicroHs .comb file to eq-prog-opt S-expressions.")
    argp.add_argument("comb_file", type=Path)
    argp.add_argument("-o", "--output", type=Path)
    args = argp.parse_args()

    text = emit_eqbench(parse_comb(args.comb_file))
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end="")


if __name__ == "__main__":
    main()

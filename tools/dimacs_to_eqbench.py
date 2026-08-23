#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class DimacsCnf:
    variables: int
    clauses: list[list[int]]


def parse_dimacs(text: str) -> DimacsCnf:
    variables: int | None = None
    expected_clauses: int | None = None
    clauses: list[list[int]] = []
    current: list[int] = []

    for line_no, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.strip()
        if not line or line.startswith("c"):
            continue
        if line.startswith("p"):
            parts = line.split()
            if len(parts) != 4 or parts[0] != "p" or parts[1] != "cnf":
                raise ValueError(f"line {line_no}: expected 'p cnf <variables> <clauses>'")
            if variables is not None:
                raise ValueError(f"line {line_no}: duplicate problem line")
            variables = parse_nonnegative(parts[2], line_no, "variable count")
            expected_clauses = parse_nonnegative(parts[3], line_no, "clause count")
            continue

        if variables is None:
            raise ValueError(f"line {line_no}: clause appears before problem line")

        for token in line.split():
            lit = parse_int(token, line_no)
            if lit == 0:
                clauses.append(current)
                current = []
                continue
            if abs(lit) > variables:
                raise ValueError(f"line {line_no}: literal {lit} exceeds variable count {variables}")
            current.append(lit)

    if variables is None or expected_clauses is None:
        raise ValueError("missing problem line")
    if current:
        raise ValueError("unterminated final clause")
    if len(clauses) != expected_clauses:
        raise ValueError(f"expected {expected_clauses} clauses, parsed {len(clauses)}")
    return DimacsCnf(variables, clauses)


def parse_nonnegative(token: str, line_no: int, name: str) -> int:
    value = parse_int(token, line_no)
    if value < 0:
        raise ValueError(f"line {line_no}: negative {name}")
    return value


def parse_int(token: str, line_no: int) -> int:
    try:
        return int(token, 10)
    except ValueError as exc:
        raise ValueError(f"line {line_no}: invalid integer {token!r}") from exc


def emit_eqbench(cnf: DimacsCnf, source: str) -> str:
    out = [
        f";; generated from DIMACS CNF {source}",
        f";; variables: {cnf.variables}, clauses: {len(cnf.clauses)}",
        "(sort BoolExpr)",
        "",
        "(function True () BoolExpr)",
        "(function False () BoolExpr)",
        "(function Var (String) BoolExpr)",
        "(function Not (BoolExpr) BoolExpr)",
        "(function And (BoolExpr BoolExpr) BoolExpr)",
        "(function Or (BoolExpr BoolExpr) BoolExpr)",
        "",
        ";; Boolean simplification rules.",
        "(rewrite and.true.left (And True x) x)",
        "(rewrite and.true.right (And x True) x)",
        "(rewrite and.false.left (And False x) False)",
        "(rewrite and.false.right (And x False) False)",
        "(rewrite or.false.left (Or False x) x)",
        "(rewrite or.false.right (Or x False) x)",
        "(rewrite or.true.left (Or True x) True)",
        "(rewrite or.true.right (Or x True) True)",
        "(rewrite not.true (Not True) False)",
        "(rewrite not.false (Not False) True)",
        "(rewrite not.double (Not (Not x)) x)",
        "",
        f"(optimize {and_expr([or_expr([literal_expr(lit) for lit in clause]) for clause in cnf.clauses])})",
    ]
    return "\n".join(out) + "\n"


def literal_expr(literal: int) -> str:
    var = f'(Var "x{abs(literal)}")'
    return var if literal > 0 else f"(Not {var})"


def or_expr(items: list[str]) -> str:
    return fold_binary("Or", items, "False")


def and_expr(items: list[str]) -> str:
    return fold_binary("And", items, "True")


def fold_binary(name: str, items: list[str], identity: str) -> str:
    if not items:
        return identity
    expr = items[-1]
    for item in reversed(items[:-1]):
        expr = f"({name} {item} {expr})"
    return expr


def main() -> None:
    argp = argparse.ArgumentParser(description="Convert a DIMACS CNF file to an eq-prog-opt Boolean expression.")
    argp.add_argument("dimacs_file", type=Path)
    argp.add_argument("-o", "--output", type=Path)
    args = argp.parse_args()

    cnf = parse_dimacs(args.dimacs_file.read_text(encoding="utf-8"))
    text = emit_eqbench(cnf, str(args.dimacs_file))
    if args.output:
        args.output.write_text(text, encoding="utf-8")
    else:
        print(text, end="")


if __name__ == "__main__":
    main()

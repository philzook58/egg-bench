#!/usr/bin/env python3
from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import aiger
from aiger import aig as aiger_nodes


@dataclass(frozen=True)
class CircuitExpr:
    kind: str
    args: tuple[object, ...] = ()


TRUE = CircuitExpr("true")
FALSE = CircuitExpr("false")


def var(name: str) -> CircuitExpr:
    return CircuitExpr("var", (name,))


def inv(expr: CircuitExpr) -> CircuitExpr:
    if expr == TRUE:
        return FALSE
    if expr == FALSE:
        return TRUE
    if expr.kind == "not":
        return expr.args[0]  # type: ignore[return-value]
    return CircuitExpr("not", (expr,))


def and_expr(items: Iterable[CircuitExpr]) -> CircuitExpr:
    return fold("and", list(items), TRUE)


def or_expr(items: Iterable[CircuitExpr]) -> CircuitExpr:
    return fold("or", list(items), FALSE)


def fold(kind: str, items: list[CircuitExpr], identity: CircuitExpr) -> CircuitExpr:
    if not items:
        return identity
    expr = items[-1]
    for item in reversed(items[:-1]):
        expr = CircuitExpr(kind, (item, expr))
    return expr


def load_circuit(path: Path) -> dict[str, CircuitExpr]:
    suffix = path.suffix.lower()
    if suffix in {".aig", ".aag"}:
        return load_aiger(path)
    if suffix == ".blif":
        return load_blif(path)
    raise ValueError(f"unsupported circuit format: {path.suffix}")


def load_aiger(path: Path) -> dict[str, CircuitExpr]:
    circuit = aiger.load(str(path))
    return {name: node_to_expr(node) for name, node in sorted(circuit.node_map.items())}


def node_to_expr(node: object) -> CircuitExpr:
    if isinstance(node, aiger_nodes.Input):
        return var(node.name)
    if isinstance(node, aiger_nodes.LatchIn):
        return var(f"latch:{node.name}")
    if isinstance(node, aiger_nodes.ConstFalse):
        return FALSE
    if isinstance(node, aiger_nodes.Inverter):
        return inv(node_to_expr(node.input))
    if isinstance(node, aiger_nodes.AndGate):
        return CircuitExpr("and", (node_to_expr(node.left), node_to_expr(node.right)))
    raise TypeError(f"unsupported pyaiger node: {node!r}")


def load_blif(path: Path) -> dict[str, CircuitExpr]:
    inputs: set[str] = set()
    outputs: list[str] = []
    definitions: dict[str, CircuitExpr] = {}
    pending_names: list[str] | None = None
    pending_rows: list[tuple[str, str]] = []

    def finish_names() -> None:
        nonlocal pending_names, pending_rows
        if pending_names is None:
            return
        if not pending_names:
            raise ValueError("empty .names directive")
        *in_names, out_name = pending_names
        terms: list[CircuitExpr] = []
        for pattern, value in pending_rows:
            if value != "1":
                continue
            if len(pattern) != len(in_names):
                raise ValueError(f"cube {pattern!r} has wrong arity for {out_name}")
            literals: list[CircuitExpr] = []
            for bit, in_name in zip(pattern, in_names, strict=True):
                if bit == "-":
                    continue
                atom = definitions.get(in_name, var(in_name))
                if bit == "0":
                    atom = inv(atom)
                elif bit != "1":
                    raise ValueError(f"invalid BLIF cube bit {bit!r}")
                literals.append(atom)
            terms.append(and_expr(literals))
        definitions[out_name] = or_expr(terms)
        pending_names = None
        pending_rows = []

    for raw_line in logical_blif_lines(path.read_text(encoding="utf-8")):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("."):
            finish_names()
            parts = line.split()
            directive = parts[0]
            if directive == ".model":
                continue
            if directive == ".inputs":
                inputs.update(parts[1:])
                continue
            if directive == ".outputs":
                outputs.extend(parts[1:])
                continue
            if directive == ".names":
                pending_names = parts[1:]
                pending_rows = []
                continue
            if directive == ".end":
                break
            raise ValueError(f"unsupported BLIF directive {directive}")
        if pending_names is None:
            raise ValueError(f"truth-table row outside .names: {line}")
        parts = line.split()
        if len(parts) == 1 and len(pending_names) == 1:
            parts = ["", parts[0]]
        if len(parts) != 2:
            raise ValueError(f"expected BLIF truth-table row, got {line!r}")
        pending_rows.append((parts[0], parts[1]))
    else:
        finish_names()

    for name in inputs:
        definitions.setdefault(name, var(name))
    if not outputs:
        outputs = sorted(definitions)
    return {name: definitions.get(name, var(name)) for name in outputs}


def logical_blif_lines(text: str) -> list[str]:
    lines: list[str] = []
    current = ""
    for raw_line in text.splitlines():
        line = raw_line.rstrip()
        if line.endswith("\\"):
            current += line[:-1] + " "
            continue
        lines.append(current + line)
        current = ""
    if current:
        lines.append(current)
    return lines


def emit_eqbench(outputs: dict[str, CircuitExpr], source: str) -> str:
    out = [
        f";; generated from circuit file {source}",
        f";; outputs: {', '.join(outputs) if outputs else '(none)'}",
        "(sort BoolExpr)",
        "",
        "(function True () BoolExpr)",
        "(function False () BoolExpr)",
        "(function Var (String) BoolExpr)",
        "(function Not (BoolExpr) BoolExpr)",
        "(function And (BoolExpr BoolExpr) BoolExpr)",
        "(function Or (BoolExpr BoolExpr) BoolExpr)",
        "(function Out (String BoolExpr) BoolExpr)",
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
        f"(optimize {sexp(and_expr(CircuitExpr('out', (name, expr)) for name, expr in outputs.items()))})",
    ]
    return "\n".join(out) + "\n"


def sexp(expr: CircuitExpr) -> str:
    if expr.kind == "true":
        return "True"
    if expr.kind == "false":
        return "False"
    if expr.kind == "var":
        return f"(Var {string_lit(str(expr.args[0]))})"
    if expr.kind == "not":
        return f"(Not {sexp(expr.args[0])})"
    if expr.kind == "and":
        return f"(And {sexp(expr.args[0])} {sexp(expr.args[1])})"
    if expr.kind == "or":
        return f"(Or {sexp(expr.args[0])} {sexp(expr.args[1])})"
    if expr.kind == "out":
        return f"(Out {string_lit(str(expr.args[0]))} {sexp(expr.args[1])})"
    raise ValueError(f"unknown expression kind {expr.kind}")


def string_lit(text: str) -> str:
    if any(ch in text for ch in ['"', "\n", "\r"]):
        text = "hex:" + text.encode("utf-8").hex()
    return f'"{text}"'


def main() -> None:
    argp = argparse.ArgumentParser(description="Convert AIGER AIG/AAG or BLIF circuits to eq-prog-opt expressions.")
    argp.add_argument("circuit_file", type=Path)
    argp.add_argument("-o", "--output", type=Path)
    args = argp.parse_args()

    text = emit_eqbench(load_circuit(args.circuit_file), str(args.circuit_file))
    if args.output:
        args.output.write_text(text, encoding="utf-8")
    else:
        print(text, end="")


if __name__ == "__main__":
    main()

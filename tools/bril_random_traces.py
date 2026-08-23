#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
BRIL_ROOT = ROOT / "benchmarks" / "cfg" / "bril"
BRIL_TXT = BRIL_ROOT / "bril-txt"


@dataclass(frozen=True)
class Trace:
    source: Path
    function: str
    term: str
    steps: int


def load_bril(path: Path) -> dict[str, Any]:
    if path.suffix == ".json":
        return json.loads(path.read_text())

    env = dict(os.environ)
    env["PYTHONPATH"] = str(BRIL_TXT) + os.pathsep + env.get("PYTHONPATH", "")
    proc = subprocess.run(
        ["bril2json"],
        input=path.read_text(),
        text=True,
        capture_output=True,
        env=env,
        check=True,
    )
    return json.loads(proc.stdout)


def blocks(instrs: list[dict[str, Any]]) -> tuple[list[list[dict[str, Any]]], dict[str, int]]:
    out: list[list[dict[str, Any]]] = []
    labels: dict[str, int] = {}
    current: list[dict[str, Any]] = []

    def finish() -> None:
        nonlocal current
        if current:
            out.append(current)
            current = []

    for instr in instrs:
        if "label" in instr:
            finish()
            labels[instr["label"]] = len(out)
            current = [instr]
        else:
            current.append(instr)
            if instr.get("op") in {"jmp", "br", "ret"}:
                finish()
    finish()
    return out, labels


def atom(text: str) -> str:
    return '"' + text.replace('"', "_").replace("\n", "_") + '"'


def call(name: str, *args: str) -> str:
    return f"({name} {' '.join(args)})" if args else f"({name})"


def var(name: str) -> str:
    return call("Var", atom(name))


def literal(value: Any) -> str:
    if isinstance(value, bool):
        return call("Bool", "1" if value else "0")
    if isinstance(value, int):
        return call("Int", str(value))
    if isinstance(value, float):
        return call("Float", atom(repr(value)))
    if value is None:
        return call("Null")
    return call("Lit", atom(str(value)))


def arg(env: dict[str, str], name: str) -> str:
    return env.get(name, var(name))


VALUE_OPS = {
    "id": ("Id", 1),
    "add": ("Add", 2),
    "mul": ("Mul", 2),
    "sub": ("Sub", 2),
    "div": ("Div", 2),
    "eq": ("Eq", 2),
    "lt": ("Lt", 2),
    "gt": ("Gt", 2),
    "le": ("Le", 2),
    "ge": ("Ge", 2),
    "not": ("Not", 1),
    "and": ("And", 2),
    "or": ("Or", 2),
    "fadd": ("FAdd", 2),
    "fmul": ("FMul", 2),
    "fsub": ("FSub", 2),
    "fdiv": ("FDiv", 2),
    "feq": ("FEq", 2),
    "flt": ("FLt", 2),
    "fgt": ("FGt", 2),
    "fle": ("FLe", 2),
    "fge": ("FGe", 2),
    "ceq": ("CEq", 2),
    "clt": ("CLt", 2),
    "cgt": ("CGt", 2),
    "cle": ("CLe", 2),
    "cge": ("CGe", 2),
    "char2int": ("Char2Int", 1),
    "int2char": ("Int2Char", 1),
    "ptradd": ("PtrAdd", 2),
}


def list_expr(items: list[str]) -> str:
    result = call("Nil")
    for item in reversed(items):
        result = call("Cons", item, result)
    return result


def instr_value(instr: dict[str, Any], env: dict[str, str], mem: str, alloc_id: int) -> tuple[str, str, int]:
    op = instr["op"]
    args = [arg(env, a) for a in instr.get("args", [])]
    if op == "const":
        return literal(instr.get("value")), mem, alloc_id
    if op == "load":
        return call("Load", mem, args[0]), mem, alloc_id
    if op == "alloc":
        ptr = call("Ptr", atom(f"a{alloc_id}"))
        return ptr, call("Alloc", mem, ptr, args[0]), alloc_id + 1
    if op == "call":
        fname = instr.get("funcs", ["unknown"])[0]
        return call("CallRet", atom(fname), mem, list_expr(args)), call("CallMem", atom(fname), mem, list_expr(args)), alloc_id
    if op in VALUE_OPS:
        name, arity = VALUE_OPS[op]
        padded = args + [call("MissingArg")] * max(0, arity - len(args))
        return call(name, *padded[:arity]), mem, alloc_id
    return call("Op", atom(op), list_expr(args)), mem, alloc_id


def trace_function(func: dict[str, Any], source: Path, rng: random.Random, max_steps: int) -> Trace:
    bbs, label_to_block = blocks(func["instrs"])
    env = {a["name"]: var(a["name"]) for a in func.get("args", [])}
    mem = call("Mem0")
    printed: list[str] = []
    block_idx = 0
    steps = 0
    alloc_id = 0
    ret = call("Unit")

    while 0 <= block_idx < len(bbs) and steps < max_steps:
        next_block = block_idx + 1
        jumped = False
        for instr in bbs[block_idx]:
            if "label" in instr:
                continue
            steps += 1
            op = instr.get("op")
            if "dest" in instr:
                value, mem, alloc_id = instr_value(instr, env, mem, alloc_id)
                env[instr["dest"]] = value
                continue
            args = [arg(env, a) for a in instr.get("args", [])]
            if op == "store":
                mem = call("Store", mem, args[0], args[1])
            elif op == "free":
                mem = call("Free", mem, args[0])
            elif op == "print":
                printed.extend(args)
                mem = call("Print", mem, list_expr(args))
            elif op == "call":
                fname = instr.get("funcs", ["unknown"])[0]
                mem = call("CallMem", atom(fname), mem, list_expr(args))
            elif op == "ret":
                ret = args[0] if args else call("Unit")
                return Trace(source, func["name"], call("Ret", mem, ret), steps)
            elif op == "jmp":
                next_block = label_to_block[instr["labels"][0]]
                jumped = True
                break
            elif op == "br":
                choice = rng.randrange(2)
                cond = args[0] if args else call("MissingArg")
                mem = call("Assume", mem, cond if choice == 0 else call("Not", cond))
                next_block = label_to_block[instr["labels"][choice]]
                jumped = True
                break
            if steps >= max_steps:
                break
        block_idx = next_block
        if not jumped and steps >= max_steps:
            break

    if printed:
        ret = printed[-1]
    return Trace(source, func["name"], call("Ret", mem, ret), steps)


def declarations() -> str:
    names = [
        "(sort Expr)",
        "(function Var (String) Expr)",
        "(function Lit (String) Expr)",
        "(function Float (String) Expr)",
        "(function Ptr (String) Expr)",
        "(function Int (i64) Expr)",
        "(function Bool (i64) Expr)",
        "(function Mem0 () Expr)",
        "(function Unit () Expr)",
        "(function Null () Expr)",
        "(function Nil () Expr)",
        "(function MissingArg () Expr)",
        "(function Cons (Expr Expr) Expr)",
        "(function Ret (Expr Expr) Expr)",
        "(function Assume (Expr Expr) Expr)",
        "(function Print (Expr Expr) Expr)",
        "(function Alloc (Expr Expr Expr) Expr)",
        "(function Store (Expr Expr Expr) Expr)",
        "(function Load (Expr Expr) Expr)",
        "(function Free (Expr Expr) Expr)",
        "(function CallRet (String Expr Expr) Expr)",
        "(function CallMem (String Expr Expr) Expr)",
        "(function Op (String Expr) Expr)",
    ]
    for name, arity in sorted(set(VALUE_OPS.values())):
        names.append(f"(function {name} ({' '.join(['Expr'] * arity)}) Expr)")
    return "\n".join(names)


def emit_eqbench(traces: list[Trace]) -> str:
    out = [declarations(), ""]
    for trace in traces:
        rel = trace.source.relative_to(ROOT)
        out.append(f";; {rel}:{trace.function}, {trace.steps} traced instructions")
        out.append(f"(optimize {trace.term})")
        out.append("")
    return "\n".join(out)


def candidate_files(root: Path) -> list[Path]:
    return sorted(p for p in root.rglob("*") if p.suffix in {".bril", ".json"} and "plot.vl.json" not in str(p))


def pick_function(program: dict[str, Any], rng: random.Random) -> dict[str, Any] | None:
    funcs = program.get("functions", [])
    if not funcs:
        return None
    for func in funcs:
        if func.get("name") == "main":
            return func
    return rng.choice(funcs)


def main() -> None:
    ap = argparse.ArgumentParser(description="Randomly trace Bril benchmarks into eq-prog-opt expressions.")
    ap.add_argument("paths", nargs="*", type=Path, default=[BRIL_ROOT / "benchmarks"])
    ap.add_argument("-n", "--count", type=int, default=8)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--max-steps", type=int, default=80)
    ap.add_argument("-o", "--output", type=Path)
    args = ap.parse_args()

    rng = random.Random(args.seed)
    files: list[Path] = []
    for path in args.paths:
        files.extend(candidate_files(path) if path.is_dir() else [path])
    rng.shuffle(files)

    traces: list[Trace] = []
    for path in files:
        if len(traces) >= args.count:
            break
        try:
            program = load_bril(path)
            func = pick_function(program, rng)
            if func is not None:
                traces.append(trace_function(func, path.resolve(), rng, args.max_steps))
        except Exception as exc:
            print(f";; skipped {path}: {exc}", file=sys.stderr)

    text = emit_eqbench(traces)
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end="")


if __name__ == "__main__":
    main()

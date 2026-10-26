#!/usr/bin/env python3
"""Generate the template-based part of the conformance corpus (deterministic).

Writes tests/conformance/gen_*.js. Expected outputs (.out) are produced by
running node over the corpus: see tests/update_golden.sh.
"""
import os
import random

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "conformance")
rng = random.Random(1234)


def num():
    r = rng.random()
    if r < 0.5:
        return str(rng.randint(0, 50))
    if r < 0.7:
        return str(rng.randint(-1000, 100000))
    if r < 0.9:
        return f"{rng.uniform(-100, 100):.3f}"
    return rng.choice(["0", "1", "2147483647", "0.5", "1e3", "3.25"])


def expr(depth):
    if depth == 0 or rng.random() < 0.25:
        return num()
    op = rng.choice(["+", "-", "*", "/", "%", "+", "-", "*", "<", ">=", "===", "!==", "&", "|", "^", "<<", ">>"])
    e = f"({expr(depth - 1)} {op} {expr(depth - 1)})"
    if rng.random() < 0.1:
        e = f"-{e}"
    return e


def gen_arith(i):
    lines = [f"console.log({expr(3)});" for _ in range(6)]
    lines.append(f'console.log("s" + {expr(2)}, {expr(2)} + "t", `${{{expr(2)}}}`);')
    return "\n".join(lines) + "\n"


JIT_OPS = ["+", "-", "*"]


def jit_body(nvars):
    vs = [f"v{k}" for k in range(nvars)]
    stmts = []
    for _ in range(rng.randint(2, 5)):
        tgt = rng.choice(vs)
        a, b = rng.choice(vs + ["a", "b", str(rng.randint(1, 9))]), rng.choice(vs + ["a", "b", str(rng.randint(1, 9))])
        op = rng.choice(JIT_OPS)
        stmts.append(f"{tgt} = {a} {op} {b};")
        if rng.random() < 0.3:
            stmts.append(f"if ({tgt} > {rng.randint(100, 100000)}) {tgt} = {tgt} % {rng.randint(2, 1000)};")
    return stmts


def gen_jit(i):
    nvars = rng.randint(1, 3)
    init = " ".join(f"let v{k} = {rng.randint(0, 5)};" for k in range(nvars))
    body = " ".join(jit_body(nvars))
    cmp = rng.choice(["<", "<=", "!=="])
    ret = " + ".join(f"v{k}" for k in range(nvars))
    fn = f"function f{i}(a, b) {{ {init} for (let i = 0; i {cmp} a; i = i + 1) {{ {body} }} return {ret}; }}"
    calls = []
    calls.append(f"let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f{i}(k % 7, k);")
    calls.append("console.log(acc);")
    # large / fractional / negative arguments exercise deopt paths
    for args in [(5, 3), (20, 2147483), (3, 0.5), (4, -7.25), (12, 1e6), (0, 9), (30, 65536)]:
        calls.append(f"console.log(f{i}({args[0]}, {args[1]}));")
    return fn + "\n" + "\n".join(calls) + "\n"


def gen_rec(i):
    k = rng.randint(2, 4)
    base = rng.randint(1, 3)
    terms = " + ".join(f"r{i}(n - {j + 1})" for j in range(k))
    fn = f"function r{i}(n) {{ if (n < {base + 1}) return n * {rng.randint(1, 3)} + {rng.randint(0, 2)}; return ({terms}) % {rng.randint(1000, 100000)}; }}"
    return fn + "\n" + "\n".join(f"console.log(r{i}({n}));" for n in [0, 3, 8, rng.randint(12, 18)]) + "\n"


WORDS = ["alpha", "beta", "gamma", "delta", "kiln", "fire", "clay", "glaze", "x", ""]


def gen_strings(i):
    w = rng.sample(WORDS, 4)
    s = " ".join(w)
    lines = [f'const s = "{s}";']
    ops = [
        "s.toUpperCase()", "s.length", 's.split(" ")', f's.indexOf("{w[1]}")', "s.slice(2, 9)", "s.slice(-4)",
        's.split(" ").reverse().join("-")', "s.charAt(3)", 's.replace(" ", "_")', 's.includes("a")', "s.substring(5, 1)",
        "s.trim().length", 's.split("").filter(c => c === "a").length', "s.charCodeAt(1)", '[...s].slice(0, 3)',
        's.padStart(30, ".")', "s.repeat(2).length", 's.split(" ").map(x => x.length)', 's.startsWith("al")',
    ]
    for op in rng.sample(ops, 7):
        lines.append(f"console.log({op});")
    return "\n".join(lines) + "\n"


def gen_arrays(i):
    xs = [rng.randint(-20, 50) for _ in range(rng.randint(3, 9))]
    lines = [f"const xs = {xs};"]
    ops = [
        "xs.map(x => x * 2)", "xs.filter(x => x % 2 === 0)", "xs.reduce((a, b) => a + b, 0)", "xs.slice().sort((a, b) => a - b)",
        "xs.slice().sort()", "xs.indexOf(xs[2])", "xs.includes(7)", "xs.some(x => x < 0)", "xs.every(x => x > -50)",
        "xs.find(x => x > 10)", "xs.findIndex(x => x > 10)", "xs.concat([1, 2]).length", "xs.join(':')",
        "xs.map((x, i) => x * i).filter(x => x > 0)", "[...xs, ...xs].length", "xs.slice(1, -1)",
        "Math.max(...xs)", "Math.min(...xs)", "JSON.stringify(xs)", "xs.map(x => [x, x % 3])",
    ]
    for op in rng.sample(ops, 8):
        lines.append(f"console.log({op});")
    lines.append("const ys = xs.slice(); ys.push(100); ys.pop(); ys.unshift(-1); ys.reverse(); console.log(ys, xs.length);")
    return "\n".join(lines) + "\n"


def gen_objects(i):
    keys = rng.sample(["a", "b", "c", "d", "e", "f", "name", "value", "id", "kind"], rng.randint(2, 5))
    lit = ", ".join(f"{k}: {rng.choice([str(rng.randint(0, 99)), repr(rng.choice(WORDS)), '[1, 2]', 'null', 'true'])}" for k in keys)
    lines = [f"const o = {{ {lit} }};", "console.log(o);", "console.log(JSON.stringify(o));", "console.log(Object.keys(o));"]
    k = rng.choice(keys)
    lines.append(f"o.{k} = {rng.randint(0, 9)}; o.extra = 'x'; console.log(o.{k}, o.extra, Object.keys(o).length);")
    lines.append(f"const c = {{ ...o, {k}: 'over' }}; console.log(c.{k}, JSON.stringify(c));")
    cls = f"""class K{i} {{
  constructor(v) {{ this.v = v; this.{k} = v * 2; }}
  get{i}() {{ return this.v + this.{k}; }}
}}
class L{i} extends K{i} {{ get{i}() {{ return super.get{i}() * 10; }} }}
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K{i}(j) : new L{i}(j));
console.log(arr.map(x => x.get{i}()), arr[1], arr[0] instanceof K{i});"""
    lines.append(cls)
    return "\n".join(lines) + "\n"


def gen_closures(i):
    n = rng.randint(2, 6)
    m = rng.randint(1, 5)
    return f"""function make{i}(start) {{
  let count = start;
  const log = [];
  return {{
    inc: (d) => {{ count += d; log.push(count); return count; }},
    get: () => count,
    history: () => log.slice(),
  }};
}}
const cs = [];
for (let j = 0; j < {n}; j++) cs.push(make{i}(j * {m}));
for (let r = 0; r < 3; r++) for (const c of cs) c.inc(r + 1);
console.log(cs.map(c => c.get()));
console.log(cs[0].history(), cs[{n - 1}].history().length);
const adders = [];
for (let j = 0; j < {n}; j++) {{ const base = j * j; adders.push(x => x + base); }}
console.log(adders.map(f => f({m})));
"""


def gen_exceptions(i):
    t = rng.randint(1, 8)
    return f"""function risky{i}(n) {{
  if (n === {t}) throw new Error("bad " + n);
  if (n > {t + 3}) throw new RangeError("range " + n);
  return n * 2;
}}
const out = [];
for (let j = 0; j < {t + 6}; j++) {{
  try {{ out.push(risky{i}(j)); }}
  catch (e) {{ out.push(e.name + ":" + e.message); }}
  finally {{ if (j % 3 === 0) out.push("f"); }}
}}
console.log(out.join(","));
function nested{i}() {{ try {{ try {{ risky{i}({t}); }} catch (e) {{ throw new TypeError("wrapped " + e.message); }} }} catch (e2) {{ return e2.name + "/" + e2.message; }} }}
console.log(nested{i}());
"""


GENS = [
    ("arith", gen_arith, 60),
    ("jit", gen_jit, 60),
    ("rec", gen_rec, 20),
    ("strings", gen_strings, 30),
    ("arrays", gen_arrays, 40),
    ("objects", gen_objects, 30),
    ("closures", gen_closures, 20),
    ("exceptions", gen_exceptions, 20),
]


def main():
    os.makedirs(OUT, exist_ok=True)
    for f in os.listdir(OUT):
        if f.startswith("gen_"):
            os.remove(os.path.join(OUT, f))
    n = 0
    for name, g, count in GENS:
        for i in range(count):
            with open(os.path.join(OUT, f"gen_{name}_{i:03}.js"), "w") as fh:
                fh.write(g(i))
            n += 1
    print(f"generated {n} scripts")


if __name__ == "__main__":
    main()

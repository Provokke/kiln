<div align="center">

# kiln

A small JavaScript-subset engine written from scratch in Rust (no parser or engine crates; the only dependency is `libc` for `mmap`/`mprotect`). The design borrows ideas from V8 and swc: a span-tracking lexer, a Pratt parser, a code formatter that...

![License](https://img.shields.io/badge/LICENSE-MIT-blue?style=for-the-badge)  ![Language](https://img.shields.io/badge/RUST-dea584?style=for-the-badge&logo=rust&logoColor=white)

Refer to [LICENSE](LICENSE) for the legal bits.

</div>

## 🚀 Quick start

```
cargo build --release
./target/release/kiln run [--no-jit] [--jit-threshold N] [--gc-stress] [--stats] [--disasm] file.js
./target/release/kiln fmt file.js      # parse -> pretty print
./target/release/kiln ast file.js      # dump AST
./target/release/kiln check file.js    # parse + resolve
```

Sample (real output):

```
$ cat s.js
function f(a,b){return a*b+1}
const o={x:1,y:[1,2]}
console.log(f(2,3),o)
$ kiln fmt s.js
function f(a, b) {
  return a * b + 1;
}
const o = { x: 1, y: [1, 2] };
console.log(f(2, 3), o);

$ kiln run e.js          # e.js: let x = (1 +;
e.js:1:13: SyntaxError: unexpected ';'
  let x = (1 +;
              ^

$ kiln run --stats benches/js/mandelbrot.js
2826473
--- kiln stats (43.82 ms) ---
gc: 0 minor, 0 major, 110 allocated, 0 promoted, 0 swept, 0 old-gen live
jit: 2 compiled, 0 not eligible, 59833 native entries, 1 deopts
inline caches: 1 sites, 0 hits, 1 misses, hit rate 0.0%
```

`--stats` also lists the hottest inline-cache sites with their state
(monomorphic / polymorphic / megamorphic) and hit/miss counts.

## Supported language

`let`/`const`/`var` (hoisting, block scope, per-iteration `for (let …)`
bindings), function declarations/expressions, closures, arrow functions
(lexical `this`), classes (constructor, methods, static methods, `extends`,
`super(...)`, `super.m()`), objects (shorthand, computed keys, methods,
spread), arrays (spread), member/index access, `new`, `if`/`while`/`do`/`for`/
`for-of`/`for-in`, `break`/`continue`, `return`, `try`/`catch`/`finally`,
`throw`, `typeof`, `instanceof`, `in`, `==`/`===`, logical and nullish
operators (and their assignment forms on identifiers), `**`, bitwise ops,
template literals, unicode identifiers.

Builtins: `console.log`, `Math.*`, `JSON.stringify` (with indent), `Object.keys/
values/entries/assign`, `Array.isArray`, Array `push pop shift unshift slice
splice concat join reverse indexOf includes fill map filter forEach some every
find findIndex reduce sort`, String `charAt charCodeAt indexOf slice substring
split trim toUpperCase replace padStart repeat …`, `Number`, `String`,
`parseInt`, `parseFloat`, `Error`/`TypeError`/`RangeError`/`ReferenceError`.
`console.log` formats values like Node's `util.inspect` (including its
column grouping for long arrays). `gc()` forces a full collection.

## Implementation notes

* **Shapes / ICs** (`src/heap.rs`, `src/vm.rs`): objects hold a shape id and a
  slot vector; shapes form a transition tree. Every `GetProp`/`SetProp`/
  `GetMethod` site owns an inline cache with up to 4 entries (own-property
  loads, depth-1 prototype loads validated by the holder's shape, and
  transitioning stores), then goes megamorphic.
* **GC** (`src/heap.rs`): `GcRef` is an index whose top bit selects the
  generation. Minor GC is Cheney-style: evacuate roots, scan the remembered
  set, scan promoted objects, reset the nursery. Major GC = minor + mark-sweep
  of the old generation. All mutation goes through `Heap::get_mut`, which is
  the write barrier. Collections only run at safepoints between bytecode
  instructions; builtins that call back into JS keep their values in
  `vm.extra_roots` (and e.g. `sort` sorts indices) so moving collections are safe.
  `--gc-stress` requests a collection after every allocation.
* **JIT** (`src/jit.rs`): eligible functions use only locals, number
  constants, `+ - * / %`, unary minus, comparisons fused with branches, and
  calls to global functions. An abstract interpretation computes the stack
  shape at each pc; codegen keeps locals and the operand stack in a flat slot
  array, so a deopt is "return a deopt id; copy slots into a new interpreter
  frame; resume at the recorded pc". The int32 tier guards overflow (`jo`),
  `-0` results and negative/zero `%` operands; after 8 deopts the function is
  recompiled in the float tier. Calls go through a Rust helper, with a direct
  fast path for self-recursion. Non-number call results deopt *after* the call.

## 🧪 Testing

```
cargo test --release      # everything below (also passes in debug)
cargo bench               # interpreter vs JIT timings
```

| suite | what | count |
|---|---|---|
| unit (`src/*`) | lexer, parser (incl. error messages with line:col), printer round trips, resolver, number formatting, GC (copy/forwarding, write barrier, cycle sweeping, slot reuse), shapes, x86 encoder + executing generated code | 25 tests |
| `tests/conformance.rs` | 307 JS scripts with golden `.out` files (produced by node), each run in 4 configs: interpreter, JIT (threshold 0), GC-stress+JIT, GC-stress+interpreter — all must match exactly | 1228 runs |
| `tests/differential.rs` | runs `node` live on the whole corpus and compares with kiln's default (tiered) mode | 307 scripts |
| `tests/jit.rs` | compilation, entries, int-overflow/-0/modulo deopts, deopt in the middle of an expression inside a loop, int→float re-tiering, after-call deopts, exceptions through JIT frames, tier-up threshold, IC hit-rate/state | 13 tests |
| `tests/proptests.rs` | random ASTs: print→parse equality (400 cases); parser fuzzing on random bytes and token soup; random numeric programs agree across interpreter / JIT / GC-stress (150 cases) | 4 properties |

27 corpus scripts are hand-written; 280 are produced by `tests/gen_corpus.py`
(deterministic seed; arithmetic, JIT loops with deopt-triggering arguments,
recursion, strings, arrays, objects/classes, closures, exceptions).
`tests/update_golden.sh` regenerates the golden files with node.

## ⏱️ Benchmarks

`cargo bench` on the shared 4-core build machine (best of 3, release build):

| benchmark | interpreter | JIT | speedup | notes |
|---|---|---|---|---|
| fib(30) | 223 ms | 62 ms | 3.6x | 2.7M native entries, self-recursion fast path |
| loops (20 × 1M iterations) | 2124 ms | 1347 ms | 1.6x | int tier overflows → 9 deopts that finish in the interpreter, then float tier |
| mandelbrot 300×200, 200 iters | 562 ms | 41 ms | 13.8x | float tier |
| objects (classes + arrays) | 314 ms | 297 ms | 1.1x | not JIT-eligible; IC hit rate 100%, 170 minor + 1 major GC |

For comparison node runs these in tens of milliseconds; kiln is a teaching-size engine.

## ⚠️ Honest limits

* No destructuring, rest parameters, getters/setters, generators, async,
  regexes, labels, `delete`, `switch`, Map/Set, Symbols, prototypes on object
  literals (`{}` has a null prototype internally), or array holes.
* `return`/`break` inside `try` with a `finally` skips the `finally` block.
* No TDZ: reading a `let` before its declaration yields `undefined`.
* Strings are Rust UTF-8 strings indexed by code point (`length` is UTF-16
  correct, indexing of astral characters is not).
* `console.log` of `Error` objects prints `Name: message` (no stack).
* JIT: only numeric functions; no OSR (a long loop in a function called once
  stays interpreted for that call; back edges only count towards tier-up of
  later calls); calls to non-self functions go through a Rust helper; the
  helper aliasing of `&mut Vm` while JIT code runs is not checked by miri.
* GC is not incremental; survivors are promoted immediately (no aging);
  strings live outside the GC heap (reference counted).
* Not fuzzed with cargo-fuzz (needs nightly); parser fuzzing is proptest-based.

//! JIT-specific tests: compilation, int/float tiers, deopt paths and stats.

use kiln::vm::{Options, Vm};

fn run(src: &str, opts: Options) -> (String, kiln::vm::JitStats) {
    let src = src.to_string();
    kiln::with_big_stack(move || {
        let mut vm = Vm::new(opts);
        kiln::exec(&mut vm, &src, "t.js").unwrap();
        (std::mem::take(&mut vm.out), vm.jit_stats.clone())
    })
}

fn jit() -> Options {
    Options {
        jit: true,
        jit_threshold: 0,
        ..Options::default()
    }
}

#[test]
fn numeric_function_is_compiled_and_runs_natively() {
    let (out, st) = run(
        "function f(n) { let s = 0; for (let i = 0; i < n; i++) s = s + i; return s; }\nconsole.log(f(1000));",
        jit(),
    );
    assert_eq!(out, "499500\n");
    assert_eq!(st.compiled, 1);
    assert_eq!(st.entries, 1);
    assert_eq!(st.deopts, 0);
}

#[test]
fn recursion_through_jit_call_helper() {
    let (out, st) = run(
        "function fib(n) { if (n < 2) return n; return fib(n - 1) + fib(n - 2); }\nconsole.log(fib(20));",
        jit(),
    );
    assert_eq!(out, "6765\n");
    assert_eq!(st.entries, 21891, "every call of fib runs native code");
    assert_eq!(st.deopts, 0);
}

#[test]
fn int_overflow_deopts_and_resumes_in_interpreter() {
    let (out, st) = run(
        "function m(a, b) { return a * b; }\nconsole.log(m(3, 4), m(65536, 65536), m(-2147483648, -1));",
        jit(),
    );
    assert_eq!(out, "12 4294967296 2147483648\n");
    assert_eq!(st.deopts, 2);
}

#[test]
fn negative_zero_deopts() {
    let (out, st) = run(
        "function m(a, b) { return a * b; }\nfunction n(a) { return -a; }\nconsole.log(m(0, -3), n(0), 1 / m(0, -3), 1 / n(0));",
        jit(),
    );
    assert_eq!(out, "-0 -0 -Infinity -Infinity\n");
    assert_eq!(st.deopts, 4);
}

#[test]
fn deopt_mid_loop_preserves_locals_and_operand_stack() {
    // overflow happens in iteration 31 with partially evaluated expression `1 + x * 2`
    let src = "function f(n) { let x = 1; let k = 0; for (let i = 0; i < n; i++) { x = 1 + x * 2; k = k + 1; } return x + k; }\nconsole.log(f(10), f(40));";
    let (out, st) = run(src, jit());
    assert_eq!(out, "2057 2199023255591\n");
    assert_eq!(st.deopts, 1);
}

#[test]
fn repeated_int_deopts_switch_to_float_variant() {
    let src = "function add(a, b) { return a + b; }\nlet s = 0; for (let i = 0; i < 50; i++) s = s + add(2147483647, i);\nconsole.log(s);";
    let (out, st) = run(src, jit());
    assert_eq!(out, "107374183575\n");
    assert!(st.deopts >= 1 && st.deopts <= 10, "deopts = {}", st.deopts);
    assert_eq!(st.compiled, 2, "int variant then float variant");
}

#[test]
fn float_mode_division_and_nan_comparisons() {
    let src = "function d(a, b) { if (a / b > 1) return 1; if (a / b <= 1) return 2; return 3; }\nconsole.log(d(3, 2), d(1, 2), d(0, 0), d(1.5, 0.5));";
    let (out, st) = run(src, jit());
    assert_eq!(out, "1 2 3 1\n");
    assert!(st.compiled >= 1);
}

#[test]
fn non_numeric_call_result_deopts_after_call() {
    let src = "function g(n) { return n > 2 ? \"big\" : n; }\nfunction f(n) { return g(n) + 1; }\nconsole.log(f(1), f(5));";
    let (out, st) = run(src, jit());
    assert_eq!(out, "2 big1\n");
    assert_eq!(st.deopts, 1);
}

#[test]
fn falling_off_the_end_deopts_to_return_undefined() {
    let (out, st) = run("function f(n) { n = n + 1; }\nconsole.log(f(1));", jit());
    assert_eq!(out, "undefined\n");
    assert_eq!(st.deopts, 1);
}

#[test]
fn exceptions_propagate_through_jit_frames() {
    let src = "function t(n) { if (n > 3) throw new Error(\"x\" + n); return n; }\nfunction f(n) { return t(n) * 2; }\ntry { console.log(f(2)); f(7); } catch (e) { console.log(e.message); }";
    let (out, _) = run(src, jit());
    assert_eq!(out, "4\nx7\n");
}

#[test]
fn ineligible_functions_stay_interpreted() {
    let (out, st) = run(
        "function f(o) { return o.x; }\nconsole.log(f({ x: 1 }));\nfunction g(a) { return [a]; }\nconsole.log(g(1));",
        jit(),
    );
    assert_eq!(out, "1\n[ 1 ]\n");
    assert_eq!(st.compiled, 0);
    assert_eq!(st.entries, 0);
}

#[test]
fn tier_up_respects_threshold() {
    let src = "function f(a) { return a + 1; }\nlet s = 0; for (let i = 0; i < 100; i++) s = s + f(i);\nconsole.log(s);";
    let (out, st) = run(
        src,
        Options {
            jit: true,
            jit_threshold: 60,
            ..Options::default()
        },
    );
    assert_eq!(out, "5050\n");
    assert_eq!(st.entries, 40);
    let (_, st) = run(
        src,
        Options {
            jit: false,
            ..Options::default()
        },
    );
    assert_eq!(st.entries, 0);
}

#[test]
fn inline_cache_hit_rates() {
    let src = "function gx(o) { return o.x; }\nlet s = 0; const o = { x: 1, y: 2 };\nfor (let i = 0; i < 1000; i++) s = s + gx(o);\nconsole.log(s);";
    let out = kiln::with_big_stack(move || {
        let mut vm = Vm::new(Options::default());
        kiln::exec(&mut vm, src, "t").unwrap();
        let site = vm.ic_sites.iter().position(|s| &*s.name == "x").unwrap();
        let ic = &vm.ics[site];
        (ic.hits, ic.misses, ic.state())
    });
    assert_eq!(out, (999, 1, "monomorphic"));
    let src = "function gx(o) { return o.x; }\nconst os = [{ x: 1 }, { a: 0, x: 2 }, { b: 0, x: 3 }];\nlet s = 0; for (let i = 0; i < 300; i++) s = s + gx(os[i % 3]);\nconsole.log(s);";
    let out = kiln::with_big_stack(move || {
        let mut vm = Vm::new(Options::default());
        kiln::exec(&mut vm, src, "t").unwrap();
        let site = vm.ic_sites.iter().position(|s| &*s.name == "x").unwrap();
        let ic = &vm.ics[site];
        (ic.hits, ic.misses, ic.state())
    });
    assert_eq!(out, (297, 3, "polymorphic"));
}

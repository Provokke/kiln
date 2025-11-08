//! `cargo bench`: runs each benchmark script with the interpreter only and with
//! the JIT, checks both print the same result, and reports wall-clock times.

use kiln::vm::{Options, Vm};
use std::time::Instant;

fn time(src: &str, opts: Options) -> (f64, String, String) {
    let src = src.to_string();
    kiln::with_big_stack(move || {
        let mut best = f64::MAX;
        let mut out = String::new();
        let mut stats = String::new();
        for _ in 0..3 {
            let mut vm = Vm::new(opts.clone());
            let t = Instant::now();
            kiln::exec(&mut vm, &src, "bench").unwrap();
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
            out = std::mem::take(&mut vm.out);
            let (h, m) = vm.ics.iter().fold((0, 0), |(h, m), c| (h + c.hits, m + c.misses));
            stats = format!(
                "jit entries {}, deopts {}, IC hit rate {:.1}%, minor GCs {}, major GCs {}",
                vm.jit_stats.entries,
                vm.jit_stats.deopts,
                if h + m > 0 { h as f64 * 100.0 / (h + m) as f64 } else { 0.0 },
                vm.heap.stats.minor_gcs,
                vm.heap.stats.major_gcs
            );
        }
        (best, out, stats)
    })
}

fn main() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/benches/js");
    println!("{:<12} {:>12} {:>12} {:>8}", "benchmark", "interp (ms)", "jit (ms)", "speedup");
    for name in ["fib", "loops", "mandelbrot", "objects"] {
        let src = std::fs::read_to_string(format!("{dir}/{name}.js")).unwrap();
        let (ti, oi, _) = time(
            &src,
            Options {
                jit: false,
                ..Options::default()
            },
        );
        let (tj, oj, sj) = time(&src, Options::default());
        assert_eq!(oi, oj, "interpreter and JIT disagree on {name}");
        println!("{name:<12} {ti:>12.1} {tj:>12.1} {:>7.1}x   [{sj}]", ti / tj);
    }
}


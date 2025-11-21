use kiln::vm::{Options, Vm};
use std::process::ExitCode;

const USAGE: &str = "usage:
  kiln run [--no-jit] [--jit-threshold N] [--gc-stress] [--stats] [--disasm] FILE.js
  kiln fmt FILE.js            pretty-print (parse -> print)
  kiln ast FILE.js            dump the AST
  kiln check FILE.js          parse + resolve only";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    kiln::with_big_stack(move || real_main(args))
}

fn real_main(args: Vec<String>) -> ExitCode {
    let Some(cmd) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut opts = Options {
        echo_stdout: true,
        ..Options::default()
    };
    let mut stats = false;
    let mut disasm = false;
    let mut file = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--no-jit" => opts.jit = false,
            "--gc-stress" => opts.gc_stress = true,
            "--stats" => stats = true,
            "--disasm" => disasm = true,
            "--jit-threshold" => {
                i += 1;
                opts.jit_threshold = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
            f => file = Some(f.to_string()),
        }
        i += 1;
    }
    let Some(file) = file else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kiln: cannot read {file}: {e}");
            return ExitCode::from(2);
        }
    };
    match cmd.as_str() {
        "fmt" | "ast" | "check" => match kiln::parser::parse(&src) {
            Ok(p) => {
                if let Err(e) = kiln::resolver::resolve(&p) {
                    eprintln!("{}", e.render(&src, &file));
                    return ExitCode::from(1);
                }
                match cmd.as_str() {
                    "fmt" => print!("{}", kiln::printer::print_program(&p)),
                    "ast" => println!("{:#?}", p.body),
                    _ => println!("ok"),
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", e.render(&src, &file));
                ExitCode::from(1)
            }
        },
        "run" => {
            if disasm {
                if let Ok(p) = kiln::parser::parse(&src) {
                    if let Ok(r) = kiln::resolver::resolve(&p) {
                        if let Ok((proto, _)) = kiln::compiler::compile(&p, &r, &src, 0) {
                            eprint!("{}", proto.disassemble());
                        }
                    }
                }
            }
            let mut vm = Vm::new(opts);
            let t = std::time::Instant::now();
            let r = kiln::exec(&mut vm, &src, &file);
            let elapsed = t.elapsed();
            if stats {
                print_stats(&vm, elapsed);
            }
            match r {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn print_stats(vm: &Vm, elapsed: std::time::Duration) {
    eprintln!("--- kiln stats ({:.2} ms) ---", elapsed.as_secs_f64() * 1000.0);
    let g = &vm.heap.stats;
    eprintln!(
        "gc: {} minor, {} major, {} allocated, {} promoted, {} swept, {} old-gen live",
        g.minor_gcs,
        g.major_gcs,
        g.allocated,
        g.promoted,
        g.swept,
        vm.heap.old_live()
    );
    let j = &vm.jit_stats;
    eprintln!(
        "jit: {} compiled, {} not eligible, {} native entries, {} deopts",
        j.compiled, j.compile_failures, j.entries, j.deopts
    );
    let (mut hits, mut misses) = (0u64, 0u64);
    for ic in &vm.ics {
        hits += ic.hits;
        misses += ic.misses;
    }
    let total = hits + misses;
    eprintln!(
        "inline caches: {} sites, {} hits, {} misses, hit rate {:.1}%",
        vm.ics.len(),
        hits,
        misses,
        if total > 0 { hits as f64 * 100.0 / total as f64 } else { 0.0 }
    );
    let mut sites: Vec<(usize, &kiln::vm::InlineCache)> = vm.ics.iter().enumerate().filter(|(_, c)| c.hits + c.misses > 0).collect();
    sites.sort_by_key(|(_, c)| std::cmp::Reverse(c.hits + c.misses));
    for (i, c) in sites.iter().take(10) {
        let site = &vm.ic_sites[*i];
        eprintln!(
            "  line {:4} .{:<12} {:<14} hits {:>10} misses {:>6}",
            site.line,
            site.name,
            c.state(),
            c.hits,
            c.misses
        );
    }
}

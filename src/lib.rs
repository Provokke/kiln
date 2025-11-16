//! kiln: a from-scratch JavaScript-subset engine.
//!
//! Pipeline: [`lexer`] -> [`parser`] (AST in [`ast`]) -> [`resolver`] (scope
//! analysis) -> [`compiler`] (stack bytecode in [`bytecode`]) -> [`vm`]
//! (interpreter with hidden classes + inline caches, generational GC in
//! [`heap`]) -> [`jit`] (x86-64 baseline JIT for numeric functions).

pub mod ast;
pub mod builtins;
pub mod bytecode;
pub mod compiler;
pub mod heap;
pub mod jit;
pub mod lexer;
pub mod parser;
pub mod printer;
pub mod resolver;
pub mod value;
pub mod vm;

use vm::{Options, Vm};

/// Outcome of running a script.
pub struct RunResult {
    /// Everything printed via console.log.
    pub output: String,
    /// `Some(message)` for a syntax error or uncaught exception.
    pub error: Option<String>,
}

/// Compile `src` and run it in `vm`. Returns an error message on failure.
pub fn exec(vm: &mut Vm, src: &str, file: &str) -> Result<(), String> {
    let prog = parser::parse(src).map_err(|e| e.render(src, file))?;
    let res = resolver::resolve(&prog).map_err(|e| e.render(src, file))?;
    let (proto, sites) = compiler::compile(&prog, &res, src, vm.ics.len() as u32).map_err(|e| e.render(src, file))?;
    match vm.run_script(proto, sites) {
        Ok(_) => Ok(()),
        Err(exc) => {
            let msg = match &exc {
                value::Value::Obj(r) if vm.is_error(*r) => vm.to_string(&exc),
                other => builtins::inspect(vm, other, 0, 0),
            };
            Err(format!("Uncaught {msg}"))
        }
    }
}

/// Run a script with the given options, capturing console output.
pub fn run_source(src: &str, opts: Options) -> RunResult {
    let mut vm = Vm::new(Options {
        echo_stdout: false,
        ..opts
    });
    let error = exec(&mut vm, src, "<input>").err();
    RunResult {
        output: std::mem::take(&mut vm.out),
        error,
    }
}

/// Run `f` on a thread with a large stack (deep JS recursion uses the native stack
/// when the interpreter re-enters itself from builtins or JIT code).
pub fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("engine thread panicked")
}

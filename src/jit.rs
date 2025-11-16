//! Baseline x86-64 JIT for numeric-only functions.
//!
//! A function is eligible when its bytecode only uses locals (no closures,
//! cells, `this` or `var`), number constants, arithmetic, comparisons fused
//! with conditional jumps, and calls to global functions. Two variants exist:
//!
//! * **int mode**: values are speculated to be int32 and kept as sign-extended
//!   64-bit integers. Overflow, `-0` results and unsupported `%` operands
//!   trigger a *deopt*: the machine code returns a deopt-point id and the
//!   runtime rebuilds an interpreter frame (locals + operand stack) and
//!   resumes at the exact bytecode pc.
//! * **float mode**: values are IEEE doubles held in SSE registers.
//!
//! The JIT frame is a flat array of 8-byte slots: locals first, then the
//! operand stack, so materializing interpreter state at a deopt point is
//! just a copy. Code is emitted into an `mmap`ed buffer that is written
//! while RW and then flipped to RX (never writable and executable at once).

use crate::bytecode::{FuncProto, Op};
use crate::heap::GcRef;
use crate::value::Value;
use crate::vm::{VResult, Vm, MAX_FRAMES};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Int,
    Float,
}

#[derive(Default)]
pub enum Variant {
    #[default]
    NotTried,
    Compiled(Rc<JitCode>),
    Failed,
}

#[derive(Default)]
pub struct JitSlot {
    pub int: Variant,
    pub float: Variant,
    pub int_deopts: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ty {
    Num,
    /// A global function about to be called (constant index of its name).
    Callee(u32),
    /// The `undefined` receiver of a plain call.
    UndefThis,
}

#[derive(Debug, Clone)]
pub struct DeoptPoint {
    pub pc: u32,
    stack: Vec<Ty>,
    /// The deopt happens after a call returned a non-numeric value (held in the context).
    after_call: bool,
}

struct CallSite {
    name_const: u32,
    deopt: usize,
}

pub struct JitCode {
    mem: ExecMem,
    nlocals: usize,
    pub nslots: usize,
    pub mode: Mode,
    pub deopts: Vec<DeoptPoint>,
    calls: Vec<CallSite>,
    pub code_size: usize,
}

/// Runtime context passed to the machine code (opaque to it; used by helpers).
pub struct JitCtx {
    vm: *mut Vm,
    func: *const FuncProto,
    code: *const JitCode,
    pending: Value,
}

type EntryFn = unsafe extern "C" fn(*mut u64, *mut JitCtx) -> u32;

const STATUS_OK: u32 = 0;
const STATUS_EXC: u32 = 1;
const STATUS_DEOPT_BASE: u32 = 16;

// ------------------------------------------------------------------ executable memory

struct ExecMem {
    ptr: *mut u8,
    len: usize,
}

impl ExecMem {
    fn new(code: &[u8]) -> Option<ExecMem> {
        let len = code.len().div_ceil(4096) * 4096;
        // SAFETY: anonymous private mapping with no address hint; checked for MAP_FAILED.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return None;
        }
        let ptr = ptr as *mut u8;
        // SAFETY: `ptr` points to `len >= code.len()` writable bytes we just mapped.
        unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len()) };
        // SAFETY: flipping our own mapping from RW to RX (W^X); range is page aligned.
        if unsafe { libc::mprotect(ptr as *mut libc::c_void, len, libc::PROT_READ | libc::PROT_EXEC) } != 0 {
            // SAFETY: unmapping the region mapped above.
            unsafe { libc::munmap(ptr as *mut libc::c_void, len) };
            return None;
        }
        Some(ExecMem { ptr, len })
    }
}

impl Drop for ExecMem {
    fn drop(&mut self) {
        // SAFETY: ptr/len describe a mapping created in `ExecMem::new` and not yet unmapped.
        unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.len) };
    }
}

// ------------------------------------------------------------------ assembler

#[derive(Default)]
pub struct Asm {
    pub code: Vec<u8>,
}

// register numbers
const RAX: u8 = 0;
const RCX: u8 = 1;
const RDX: u8 = 2;

impl Asm {
    fn b(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }
    fn d32(&mut self, v: i32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }
    /// ModRM for `[rbx + disp32]` with the given reg field.
    fn m_rbx(&mut self, reg: u8, slot: usize) {
        self.code.push(0x80 | (reg << 3) | 3);
        self.d32((slot * 8) as i32);
    }
    pub fn prologue(&mut self) {
        self.b(&[0x55]); // push rbp
        self.b(&[0x48, 0x89, 0xE5]); // mov rbp, rsp
        self.b(&[0x53]); // push rbx
        self.b(&[0x41, 0x54]); // push r12
        self.b(&[0x48, 0x89, 0xFB]); // mov rbx, rdi
        self.b(&[0x49, 0x89, 0xF4]); // mov r12, rsi
    }
    pub fn epilogue(&mut self) {
        self.b(&[0x41, 0x5C]); // pop r12
        self.b(&[0x5B]); // pop rbx
        self.b(&[0x5D]); // pop rbp
        self.b(&[0xC3]); // ret
    }
    /// mov r64, [rbx + 8*slot]
    pub fn load64(&mut self, reg: u8, slot: usize) {
        self.b(&[0x48, 0x8B]);
        self.m_rbx(reg, slot);
    }
    /// mov [rbx + 8*slot], r64
    pub fn store64(&mut self, reg: u8, slot: usize) {
        self.b(&[0x48, 0x89]);
        self.m_rbx(reg, slot);
    }
    /// mov r32, [rbx + 8*slot]
    pub fn load32(&mut self, reg: u8, slot: usize) {
        self.b(&[0x8B]);
        self.m_rbx(reg, slot);
    }
    /// <op> eax, [rbx+8*slot] for add (0x03), sub (0x2B), cmp (0x3B), or (0x0B)
    fn alu32_mem(&mut self, opcode: u8, reg: u8, slot: usize) {
        self.b(&[opcode]);
        self.m_rbx(reg, slot);
    }
    fn imul32_mem(&mut self, reg: u8, slot: usize) {
        self.b(&[0x0F, 0xAF]);
        self.m_rbx(reg, slot);
    }
    /// movsxd rax, r32
    fn movsxd_rax(&mut self, src: u8) {
        self.b(&[0x48, 0x63, 0xC0 | src]);
    }
    pub fn mov_rax_imm64(&mut self, v: u64) {
        self.b(&[0x48, 0xB8]);
        self.code.extend_from_slice(&v.to_le_bytes());
    }
    fn mov_r32_imm(&mut self, reg: u8, v: u32) {
        self.b(&[0xB8 + reg]);
        self.code.extend_from_slice(&v.to_le_bytes());
    }
    /// movsd xmm, [rbx+8*slot]
    fn movsd_load(&mut self, xmm: u8, slot: usize) {
        self.b(&[0xF2, 0x0F, 0x10]);
        self.m_rbx(xmm, slot);
    }
    fn movsd_store(&mut self, xmm: u8, slot: usize) {
        self.b(&[0xF2, 0x0F, 0x11]);
        self.m_rbx(xmm, slot);
    }
    /// addsd(0x58)/mulsd(0x59)/subsd(0x5C)/divsd(0x5E) xmm, [rbx+8*slot]
    fn sse_mem(&mut self, op: u8, xmm: u8, slot: usize) {
        self.b(&[0xF2, 0x0F, op]);
        self.m_rbx(xmm, slot);
    }
    /// ucomisd xmm_a, xmm_b
    fn ucomisd(&mut self, a: u8, b: u8) {
        self.b(&[0x66, 0x0F, 0x2E, 0xC0 | (a << 3) | b]);
    }
    /// jcc rel32 with placeholder; returns offset of the rel32 field.
    pub fn jcc(&mut self, cc: u8) -> usize {
        self.b(&[0x0F, cc]);
        let at = self.code.len();
        self.d32(0);
        at
    }
    pub fn jmp(&mut self) -> usize {
        self.b(&[0xE9]);
        let at = self.code.len();
        self.d32(0);
        at
    }
    pub fn patch(&mut self, at: usize, target: usize) {
        let rel = target as i64 - (at as i64 + 4);
        self.code[at..at + 4].copy_from_slice(&(rel as i32).to_le_bytes());
    }
    fn call_abs(&mut self, addr: u64) {
        self.mov_rax_imm64(addr);
        self.b(&[0xFF, 0xD0]); // call rax
    }
}

// condition codes (second byte of 0F 8x)
const JO: u8 = 0x80;
const JB: u8 = 0x82;
const JAE: u8 = 0x83;
const JE: u8 = 0x84;
const JNE: u8 = 0x85;
const JBE: u8 = 0x86;
const JA: u8 = 0x87;
const JS: u8 = 0x88;
const JP: u8 = 0x8A;
const JL: u8 = 0x8C;
const JGE: u8 = 0x8D;
const JLE: u8 = 0x8E;
const JG: u8 = 0x8F;

fn is_int32(n: f64) -> bool {
    n.fract() == 0.0 && n >= i32::MIN as f64 && n <= i32::MAX as f64 && !(n == 0.0 && n.is_sign_negative())
}

// ------------------------------------------------------------------ analysis + codegen

fn is_cmp(op: &Op) -> bool {
    matches!(
        op,
        Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::StrictEq | Op::StrictNe | Op::Eq | Op::Ne
    )
}

/// Abstract interpretation: stack shape before each pc. Returns None if the function is not eligible.
fn analyze(func: &FuncProto, mode: Mode) -> Option<Vec<Option<Vec<Ty>>>> {
    if func.uses_var || func.is_class || func.is_arrow || !func.captures.is_empty() {
        return None;
    }
    let code = &func.code;
    let mut states: Vec<Option<Vec<Ty>>> = vec![None; code.len() + 1];
    let mut work = vec![(0usize, Vec::new())];
    let nl = func.nlocals;
    while let Some((pc, st)) = work.pop() {
        if pc >= code.len() {
            return None;
        }
        if let Some(prev) = &states[pc] {
            if *prev != st {
                return None;
            }
            continue;
        }
        states[pc] = Some(st.clone());
        let mut st = st;
        let pop_num = |st: &mut Vec<Ty>| -> Option<()> {
            match st.pop() {
                Some(Ty::Num) => Some(()),
                _ => None,
            }
        };
        let mut next = vec![pc + 1];
        match code[pc] {
            Op::Const(c) => match func.consts[c as usize] {
                Value::Num(n) if mode == Mode::Float || is_int32(n) => st.push(Ty::Num),
                _ => return None,
            },
            Op::GetLocal(i) if i >= 1 && i < nl => st.push(Ty::Num),
            Op::SetLocal(i) if i >= 1 && i < nl => {
                if st.last() != Some(&Ty::Num) {
                    return None;
                }
            }
            Op::Pop => {
                st.pop()?;
            }
            Op::Dup => {
                if st.last() != Some(&Ty::Num) {
                    return None;
                }
                st.push(Ty::Num);
            }
            Op::Add | Op::Sub | Op::Mul | Op::Mod => {
                pop_num(&mut st)?;
                pop_num(&mut st)?;
                st.push(Ty::Num);
            }
            Op::Div if mode == Mode::Float => {
                pop_num(&mut st)?;
                pop_num(&mut st)?;
                st.push(Ty::Num);
            }
            Op::Neg | Op::ToNumber => {
                pop_num(&mut st)?;
                st.push(Ty::Num);
            }
            ref op if is_cmp(op) => {
                pop_num(&mut st)?;
                pop_num(&mut st)?;
                match code.get(pc + 1) {
                    Some(Op::JumpIfFalse(t)) | Some(Op::JumpIfTrue(t)) => next = vec![pc + 2, *t as usize],
                    _ => return None,
                }
                // the fused jump instruction's own state
                states[pc + 1] = Some(st.clone());
            }
            Op::JumpIfFalse(t) | Op::JumpIfTrue(t) => {
                pop_num(&mut st)?;
                next = vec![pc + 1, t as usize];
            }
            Op::Jump(t) => next = vec![t as usize],
            Op::Return => {
                pop_num(&mut st)?;
                next = vec![];
            }
            Op::GetGlobal(c) => {
                if code.get(pc + 1) != Some(&Op::Undef) {
                    return None;
                }
                st.push(Ty::Callee(c));
                st.push(Ty::UndefThis);
                states[pc + 1] = Some(st[..st.len() - 1].to_vec());
                next = vec![pc + 2];
            }
            Op::Undef => {
                // A deopt point: the interpreter will materialize `undefined`.
                next = vec![];
            }
            Op::Call(n) => {
                let n = n as usize;
                if st.len() < n + 2 {
                    return None;
                }
                for _ in 0..n {
                    pop_num(&mut st)?;
                }
                if st.pop()? != Ty::UndefThis {
                    return None;
                }
                if !matches!(st.pop()?, Ty::Callee(_)) {
                    return None;
                }
                st.push(Ty::Num);
            }
            _ => return None,
        }
        for n in next {
            work.push((n, st.clone()));
        }
    }
    Some(states)
}

pub fn compile(func: &FuncProto, mode: Mode) -> Option<JitCode> {
    let states = analyze(func, mode)?;
    let nlocals = func.nlocals as usize - 1; // slot 0 (`this`) is not materialized
    let max_depth = states.iter().flatten().map(|s| s.len()).max().unwrap_or(0) + 1;
    let nslots = nlocals + max_depth;
    let local = |i: u32| i as usize - 1;
    let sslot = |d: usize| nlocals + d;

    let mut a = Asm::default();
    a.prologue();
    let mut labels = vec![usize::MAX; func.code.len()];
    let mut fixups: Vec<(usize, usize)> = Vec::new(); // (rel32 pos, target pc)
    let mut deopts: Vec<DeoptPoint> = Vec::new();
    let mut deopt_fixups: Vec<(usize, usize)> = Vec::new(); // (rel32 pos, deopt idx)
    let mut exit_fixups: Vec<usize> = Vec::new();
    let mut calls: Vec<CallSite> = Vec::new();

    let mut pc = 0;
    while pc < func.code.len() {
        labels[pc] = a.code.len();
        let Some(st) = &states[pc] else {
            pc += 1;
            continue; // unreachable code
        };
        let d = st.len();
        let new_deopt = |deopts: &mut Vec<DeoptPoint>, after_call: bool, pc_resume: usize, stack: Vec<Ty>| {
            deopts.push(DeoptPoint {
                pc: pc_resume as u32,
                stack,
                after_call,
            });
            deopts.len() - 1
        };
        let op = func.code[pc];
        match op {
            Op::Const(c) => {
                let Value::Num(n) = func.consts[c as usize] else { unreachable!() };
                let bits = if mode == Mode::Int { n as i64 as u64 } else { n.to_bits() };
                a.mov_rax_imm64(bits);
                a.store64(RAX, sslot(d));
            }
            Op::GetLocal(i) => {
                a.load64(RAX, local(i));
                a.store64(RAX, sslot(d));
            }
            Op::SetLocal(i) => {
                a.load64(RAX, sslot(d - 1));
                a.store64(RAX, local(i));
            }
            Op::Pop => {}
            Op::ToNumber => {}
            Op::Dup => {
                a.load64(RAX, sslot(d - 1));
                a.store64(RAX, sslot(d));
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod => {
                let (x, y) = (sslot(d - 2), sslot(d - 1));
                match mode {
                    Mode::Int => {
                        let k = new_deopt(&mut deopts, false, pc, st.clone());
                        a.load32(RAX, x);
                        match op {
                            Op::Add => a.alu32_mem(0x03, RAX, y),
                            Op::Sub => a.alu32_mem(0x2B, RAX, y),
                            Op::Mul => a.imul32_mem(RAX, y),
                            Op::Mod => {
                                // dividend must be >= 0 and divisor > 0, else deopt
                                a.b(&[0x85, 0xC0]); // test eax, eax
                                deopt_fixups.push((a.jcc(JS), k));
                                a.load32(RCX, y);
                                a.b(&[0x85, 0xC9]); // test ecx, ecx
                                deopt_fixups.push((a.jcc(JLE), k));
                                a.b(&[0x99]); // cdq
                                a.b(&[0xF7, 0xF9]); // idiv ecx
                                a.movsxd_rax(RDX);
                                a.store64(RAX, x);
                                pc += 1;
                                continue;
                            }
                            _ => unreachable!(),
                        }
                        deopt_fixups.push((a.jcc(JO), k));
                        if op == Op::Mul {
                            // 0 result with a negative operand would be -0: deopt
                            a.b(&[0x85, 0xC0]); // test eax, eax
                            let skip = a.jcc(JNE);
                            a.load32(RCX, x);
                            a.alu32_mem(0x0B, RCX, y); // or ecx, [y]
                            deopt_fixups.push((a.jcc(JS), k));
                            let here = a.code.len();
                            a.patch(skip, here);
                        }
                        a.movsxd_rax(RAX);
                        a.store64(RAX, x);
                    }
                    Mode::Float => {
                        a.movsd_load(0, x);
                        if op == Op::Mod {
                            a.movsd_load(1, y);
                            a.call_abs(helper_fmod as *const () as u64);
                        } else {
                            let sse = match op {
                                Op::Add => 0x58,
                                Op::Mul => 0x59,
                                Op::Sub => 0x5C,
                                _ => 0x5E,
                            };
                            a.sse_mem(sse, 0, y);
                        }
                        a.movsd_store(0, x);
                    }
                }
            }
            Op::Neg => {
                let x = sslot(d - 1);
                match mode {
                    Mode::Int => {
                        let k = new_deopt(&mut deopts, false, pc, st.clone());
                        a.load32(RAX, x);
                        a.b(&[0x85, 0xC0]);
                        deopt_fixups.push((a.jcc(JE), k)); // -0
                        a.b(&[0xF7, 0xD8]); // neg eax
                        deopt_fixups.push((a.jcc(JO), k));
                        a.movsxd_rax(RAX);
                        a.store64(RAX, x);
                    }
                    Mode::Float => {
                        a.load64(RAX, x);
                        a.b(&[0x48, 0x0F, 0xBA, 0xF8, 0x3F]); // btc rax, 63
                        a.store64(RAX, x);
                    }
                }
            }
            ref cmp if is_cmp(cmp) => {
                let (x, y) = (sslot(d - 2), sslot(d - 1));
                let (jump_when, target) = match func.code[pc + 1] {
                    Op::JumpIfTrue(t) => (true, t as usize),
                    Op::JumpIfFalse(t) => (false, t as usize),
                    _ => unreachable!(),
                };
                let eq_like = matches!(cmp, Op::StrictEq | Op::Eq | Op::StrictNe | Op::Ne);
                let negate = matches!(cmp, Op::StrictNe | Op::Ne);
                match mode {
                    Mode::Int => {
                        a.load32(RAX, x);
                        a.alu32_mem(0x3B, RAX, y);
                        let (t, f) = match cmp {
                            Op::Lt => (JL, JGE),
                            Op::Le => (JLE, JG),
                            Op::Gt => (JG, JLE),
                            Op::Ge => (JGE, JL),
                            Op::StrictEq | Op::Eq => (JE, JNE),
                            _ => (JNE, JE),
                        };
                        fixups.push((a.jcc(if jump_when { t } else { f }), target));
                    }
                    Mode::Float => {
                        a.movsd_load(0, x);
                        a.movsd_load(1, y);
                        if eq_like {
                            a.ucomisd(0, 1);
                            // equal  <=> ZF=1 && PF=0
                            let jump_if_equal = jump_when != negate;
                            if jump_if_equal {
                                let skip = a.jcc(JP);
                                fixups.push((a.jcc(JE), target));
                                let here = a.code.len();
                                a.patch(skip, here);
                            } else {
                                fixups.push((a.jcc(JP), target));
                                fixups.push((a.jcc(JNE), target));
                            }
                        } else {
                            let (l, r, t, f) = match cmp {
                                Op::Lt => (1, 0, JA, JBE),
                                Op::Le => (1, 0, JAE, JB),
                                Op::Gt => (0, 1, JA, JBE),
                                _ => (0, 1, JAE, JB),
                            };
                            a.ucomisd(l, r);
                            fixups.push((a.jcc(if jump_when { t } else { f }), target));
                        }
                    }
                }
                labels[pc + 1] = a.code.len();
                pc += 2;
                continue;
            }
            Op::JumpIfFalse(t) | Op::JumpIfTrue(t) => {
                let x = sslot(d - 1);
                let jump_on_true = matches!(op, Op::JumpIfTrue(_));
                match mode {
                    Mode::Int => {
                        a.load64(RAX, x);
                        a.b(&[0x48, 0x85, 0xC0]); // test rax, rax
                        fixups.push((a.jcc(if jump_on_true { JNE } else { JE }), t as usize));
                    }
                    Mode::Float => {
                        a.movsd_load(0, x);
                        a.b(&[0x66, 0x0F, 0x57, 0xC9]); // xorpd xmm1, xmm1
                        a.ucomisd(0, 1);
                        // falsy <=> ZF=1 (equal to zero, or NaN)
                        fixups.push((a.jcc(if jump_on_true { JNE } else { JE }), t as usize));
                    }
                }
            }
            Op::Jump(t) => fixups.push((a.jmp(), t as usize)),
            Op::Return => {
                a.load64(RAX, sslot(d - 1));
                a.store64(RAX, 0);
                a.mov_r32_imm(RAX, STATUS_OK);
                exit_fixups.push(a.jmp());
            }
            Op::GetGlobal(_) => {
                // Nothing to materialize: the callee is looked up by the call helper.
                labels[pc + 1] = a.code.len();
                pc += 2;
                continue;
            }
            Op::Undef => {
                let k = new_deopt(&mut deopts, false, pc, st.clone());
                deopt_fixups.push((a.jmp(), k));
            }
            Op::Call(n) => {
                let n = n as usize;
                let callee = d - n - 2;
                let Ty::Callee(name) = st[callee] else { unreachable!() };
                let mut after = st[..callee].to_vec();
                after.push(Ty::Num);
                let k = new_deopt(&mut deopts, true, pc + 1, after);
                calls.push(CallSite {
                    name_const: name,
                    deopt: k,
                });
                a.b(&[0x4C, 0x89, 0xE7]); // mov rdi, r12
                a.b(&[0x48, 0x8D, 0xB3]); // lea rsi, [rbx + disp32]
                a.d32((sslot(callee) * 8) as i32);
                a.mov_r32_imm(RDX, n as u32);
                a.mov_r32_imm(RCX, calls.len() as u32 - 1);
                a.call_abs(helper_call as *const () as u64);
                a.b(&[0x83, 0xF8, 0x01]); // cmp eax, 1
                exit_fixups.push(a.jcc(JE)); // exception: eax == 1 already
                deopt_fixups.push((a.jcc(JA), k));
            }
            _ => return None,
        }
        pc += 1;
    }
    // deopt stubs
    let mut stub_addrs = Vec::new();
    for k in 0..deopts.len() {
        stub_addrs.push(a.code.len());
        a.mov_r32_imm(RAX, STATUS_DEOPT_BASE + k as u32);
        exit_fixups.push(a.jmp());
    }
    let exit = a.code.len();
    a.epilogue();
    for (at, t) in fixups {
        let target = labels[t];
        if target == usize::MAX {
            return None;
        }
        a.patch(at, target);
    }
    for (at, k) in deopt_fixups {
        a.patch(at, stub_addrs[k]);
    }
    for at in exit_fixups {
        a.patch(at, exit);
    }
    let code_size = a.code.len();
    let mem = ExecMem::new(&a.code)?;
    Some(JitCode {
        mem,
        nlocals,
        nslots,
        mode,
        deopts,
        calls,
        code_size,
    })
}

extern "C" fn helper_fmod(a: f64, b: f64) -> f64 {
    a % b
}

/// Called from machine code for `Call(n)`. `slots` points at the callee slot; the
/// arguments follow at `slots[2..2+argc]`. The result is written into `slots[0]`.
/// Returns 0 on success, 1 on exception, 2 if the result is not representable.
extern "C" fn helper_call(ctx: *mut JitCtx, slots: *mut u64, argc: u32, site: u32) -> u32 {
    // SAFETY: `ctx` is the live context created by `run_code` for this activation; the VM
    // pointer is valid and not otherwise borrowed while machine code runs.
    let ctx = unsafe { &mut *ctx };
    let vm = unsafe { &mut *ctx.vm };
    let func = unsafe { &*ctx.func };
    let code = unsafe { &*ctx.code };
    let cs = &code.calls[site as usize];
    let Value::Str(name) = &func.consts[cs.name_const as usize] else {
        return 1;
    };
    let callee = match vm.globals.get(name) {
        Some(v) => v.clone(),
        None => {
            ctx.pending = vm.make_error("ReferenceError", &format!("{name} is not defined"));
            return 1;
        }
    };
    // Fast path: direct self-recursion in the same mode re-enters this machine code
    // with a fresh slot window on the native stack, skipping the generic call path.
    if let Value::Obj(r) = &callee {
        if let crate::heap::ObjKind::Closure { func: f2, .. } = &vm.heap.get(*r).kind {
            if Rc::as_ptr(f2) == ctx.func && argc == func.nparams && code.nslots <= 64 && vm.native_depth < MAX_FRAMES {
                let mut buf = [0u64; 64];
                for (i, b) in buf.iter_mut().enumerate().take(argc as usize) {
                    // SAFETY: argument slots lie inside the caller's slot buffer (see below).
                    *b = unsafe { *slots.add(2 + i) };
                }
                let mut sub = JitCtx {
                    vm: ctx.vm,
                    func: ctx.func,
                    code: ctx.code,
                    pending: Value::Undefined,
                };
                // SAFETY: same compiled code and ABI as the current activation; `buf` holds
                // at least `nslots` entries.
                let entry: EntryFn = unsafe { std::mem::transmute::<*mut u8, EntryFn>(code.mem.ptr) };
                vm.native_depth += 1;
                vm.jit_stats.entries += 1;
                let st = unsafe { entry(buf.as_mut_ptr(), &mut sub) };
                vm.native_depth -= 1;
                let res = match st {
                    STATUS_OK => {
                        // SAFETY: slot 0 of the call window is inside the caller's buffer.
                        unsafe { *slots = buf[0] };
                        return 0;
                    }
                    STATUS_EXC => Err(std::mem::replace(&mut sub.pending, Value::Undefined)),
                    s => {
                        vm.jit_stats.deopts += 1;
                        let pending = std::mem::replace(&mut sub.pending, Value::Undefined);
                        // SAFETY: ctx.func is kept alive by the caller's Rc for the whole activation.
                        let frc = unsafe {
                            Rc::increment_strong_count(ctx.func);
                            Rc::from_raw(ctx.func)
                        };
                        deopt(vm, &frc, code, &buf, (s - STATUS_DEOPT_BASE) as usize, pending)
                    }
                };
                return match res {
                    Ok(Value::Num(n)) if code.mode == Mode::Float || is_int32(n) => {
                        // SAFETY: as above.
                        unsafe { *slots = encode(n, code.mode) };
                        0
                    }
                    Ok(other) => {
                        ctx.pending = other;
                        2
                    }
                    Err(e) => {
                        ctx.pending = e;
                        1
                    }
                };
            }
        }
    }
    let mut args = Vec::with_capacity(argc as usize);
    for i in 0..argc as usize {
        // SAFETY: the slot buffer has `nslots` entries and the analysis guarantees that
        // the callee slot + 2 + argc is within the operand stack region.
        let raw = unsafe { *slots.add(2 + i) };
        args.push(Value::Num(decode(raw, code.mode)));
    }
    match vm.call_value(&callee, Value::Undefined, &args) {
        Ok(Value::Num(n)) if code.mode == Mode::Float || is_int32(n) => {
            // SAFETY: slot 0 of the call window is inside the buffer (see above).
            unsafe { *slots = encode(n, code.mode) };
            let _ = cs.deopt;
            0
        }
        Ok(other) => {
            ctx.pending = other;
            2
        }
        Err(e) => {
            ctx.pending = e;
            1
        }
    }
}

fn decode(raw: u64, mode: Mode) -> f64 {
    match mode {
        Mode::Int => raw as i64 as f64,
        Mode::Float => f64::from_bits(raw),
    }
}

fn encode(n: f64, mode: Mode) -> u64 {
    match mode {
        Mode::Int => n as i64 as u64,
        Mode::Float => n.to_bits(),
    }
}

/// Try to run `func` with numeric `args` as JIT code. Returns None if the function
/// is not (yet) JIT-compiled for these argument types; otherwise the call's result
/// (finishing in the interpreter after a deopt if necessary).
pub fn try_call(vm: &mut Vm, func: &Rc<FuncProto>, args: &[Value]) -> Option<VResult<Value>> {
    let calls = func.calls.get().saturating_add(1);
    func.calls.set(calls);
    if calls <= vm.opts.jit_threshold || args.len() != func.nparams as usize {
        return None;
    }
    let mut all_int = true;
    for a in args {
        match a {
            Value::Num(n) => all_int &= is_int32(*n),
            _ => return None,
        }
    }
    let mode = {
        let slot = func.jit.borrow();
        if all_int && !matches!(slot.int, Variant::Failed) {
            Mode::Int
        } else {
            Mode::Float
        }
    };
    let code = {
        let mut slot = func.jit.borrow_mut();
        let v = if mode == Mode::Int { &mut slot.int } else { &mut slot.float };
        if let Variant::NotTried = v {
            *v = match compile(func, mode) {
                Some(c) => {
                    vm.jit_stats.compiled += 1;
                    Variant::Compiled(Rc::new(c))
                }
                None => {
                    vm.jit_stats.compile_failures += 1;
                    Variant::Failed
                }
            };
        }
        match v {
            Variant::Compiled(c) => c.clone(),
            _ => return None,
        }
    };
    if vm.native_depth >= MAX_FRAMES {
        return Some(Err(vm.make_error("RangeError", "Maximum call stack size exceeded")));
    }
    Some(run_code(vm, func, &code, args))
}

fn run_code(vm: &mut Vm, func: &Rc<FuncProto>, code: &Rc<JitCode>, args: &[Value]) -> VResult<Value> {
    vm.jit_stats.entries += 1;
    let mut slots = vec![0u64; code.nslots];
    for (i, a) in args.iter().enumerate() {
        let Value::Num(n) = a else { unreachable!() };
        slots[i] = encode(*n, code.mode);
    }
    let mut ctx = JitCtx {
        vm: vm as *mut Vm,
        func: Rc::as_ptr(func),
        code: Rc::as_ptr(code),
        pending: Value::Undefined,
    };
    // SAFETY: `code.mem` holds machine code generated by `compile` for this function with
    // the System V signature `EntryFn`; `slots` has `nslots` entries as the code expects;
    // `ctx` outlives the call.
    let entry: EntryFn = unsafe { std::mem::transmute::<*mut u8, EntryFn>(code.mem.ptr) };
    vm.native_depth += 1;
    let status = unsafe { entry(slots.as_mut_ptr(), &mut ctx) };
    vm.native_depth -= 1;
    match status {
        STATUS_OK => Ok(Value::Num(decode(slots[0], code.mode))),
        STATUS_EXC => Err(std::mem::replace(&mut ctx.pending, Value::Undefined)),
        s => {
            let k = (s - STATUS_DEOPT_BASE) as usize;
            vm.jit_stats.deopts += 1;
            if code.mode == Mode::Int {
                let mut slot = func.jit.borrow_mut();
                slot.int_deopts += 1;
                if slot.int_deopts > 8 {
                    slot.int = Variant::Failed;
                }
            }
            deopt(vm, func, code, &slots, k, std::mem::replace(&mut ctx.pending, Value::Undefined))
        }
    }
}

/// Rebuild an interpreter frame from the JIT slots and resume at the deopt point's pc.
fn deopt(vm: &mut Vm, func: &Rc<FuncProto>, code: &JitCode, slots: &[u64], k: usize, pending: Value) -> VResult<Value> {
    let dp = &code.deopts[k];
    let base = vm.stack.len();
    let fp = vm.function_proto.as_obj();
    let closure: GcRef = vm.alloc(
        crate::heap::ObjKind::Closure {
            func: func.clone(),
            cells: vec![],
        },
        fp,
    );
    vm.stack.push(Value::Obj(closure));
    vm.stack.push(Value::Undefined); // this
    for s in slots.iter().take(code.nlocals) {
        vm.stack.push(Value::Num(decode(*s, code.mode)));
    }
    for (d, ty) in dp.stack.iter().enumerate() {
        let is_top = d + 1 == dp.stack.len();
        let v = match ty {
            _ if is_top && dp.after_call => pending.clone(),
            Ty::Num => Value::Num(decode(slots[code.nlocals + d], code.mode)),
            Ty::UndefThis => Value::Undefined,
            Ty::Callee(c) => {
                let Value::Str(name) = &func.consts[*c as usize] else {
                    unreachable!()
                };
                vm.globals.get(name).cloned().unwrap_or(Value::Undefined)
            }
        };
        vm.stack.push(v);
    }
    let depth = vm.frames.len();
    vm.frames.push(crate::vm::Frame {
        closure,
        func: func.clone(),
        pc: dp.pc as usize,
        base,
        construct: false,
    });
    vm.native_depth += 1;
    let r = vm.run(depth);
    vm.native_depth -= 1;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_bytes() {
        let mut a = Asm::default();
        a.load64(RAX, 1);
        assert_eq!(a.code, vec![0x48, 0x8B, 0x83, 8, 0, 0, 0]);
        let mut a = Asm::default();
        a.store64(RCX, 2);
        assert_eq!(a.code, vec![0x48, 0x89, 0x8B, 16, 0, 0, 0]);
        let mut a = Asm::default();
        a.ucomisd(1, 0);
        assert_eq!(a.code, vec![0x66, 0x0F, 0x2E, 0xC8]);
        let mut a = Asm::default();
        let j = a.jmp();
        a.patch(j, 0);
        assert_eq!(a.code, vec![0xE9, 0xFB, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn executes_generated_code() {
        // f(slots) { slots[0] = slots[0] + slots[1] (int add); return 0 }
        let mut a = Asm::default();
        a.prologue();
        a.load32(RAX, 0);
        a.alu32_mem(0x03, RAX, 1);
        a.movsxd_rax(RAX);
        a.store64(RAX, 0);
        a.mov_r32_imm(RAX, 0);
        a.epilogue();
        let mem = ExecMem::new(&a.code).unwrap();
        let mut slots = [40u64, 2u64];
        // SAFETY: the code above follows the EntryFn ABI and only touches slots[0..2].
        let f: EntryFn = unsafe { std::mem::transmute::<*mut u8, EntryFn>(mem.ptr) };
        let st = unsafe { f(slots.as_mut_ptr(), std::ptr::null_mut()) };
        assert_eq!(st, 0);
        assert_eq!(slots[0], 42);
    }
}

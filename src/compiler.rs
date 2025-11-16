//! AST -> bytecode compiler.

use crate::ast::*;
use crate::bytecode::{Capture, FuncProto, Op};
use crate::lexer::{Span, SyntaxError};
use crate::resolver::{FnKey, Resolution, SCRIPT_KEY};
use crate::value::{Str, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

struct Local {
    name: String,
    slot: u32,
    cell: bool,
    kind: VarKind,
}

#[derive(Default)]
struct LoopCtx {
    breaks: Vec<usize>,
    continues: Vec<usize>,
    handler_depth: u32,
}

struct FnState {
    key: FnKey,
    name: String,
    code: Vec<Op>,
    lines: Vec<u32>,
    consts: Vec<Value>,
    const_map: HashMap<String, u32>,
    funcs: Vec<Rc<FuncProto>>,
    captures: Vec<(String, Capture)>,
    nlocals: u32,
    nparams: u32,
    scopes: Vec<Vec<Local>>,
    loops: Vec<LoopCtx>,
    handler_depth: u32,
    is_script: bool,
    is_arrow: bool,
    is_class: bool,
    uses_var: bool,
}

enum VarRef {
    Local(u32, bool, VarKind),
    Upval(u32, VarKind),
    Global(String),
}

/// Information about an inline cache site, for statistics output.
#[derive(Clone, Debug)]
pub struct IcSite {
    pub name: Str,
    pub line: u32,
}

pub struct Compiler<'a> {
    fns: Vec<FnState>,
    res: &'a Resolution,
    line_starts: Vec<u32>,
    cur_line: u32,
    source: Rc<str>,
    global_kinds: HashMap<String, VarKind>,
    pub ic_sites: Vec<IcSite>,
    ic_base: u32,
}

type CResult<T> = Result<T, SyntaxError>;

/// Compile a program into a script function. `ic_base` is the index of the first
/// inline cache this compilation may allocate.
pub fn compile(p: &Program, res: &Resolution, src: &str, ic_base: u32) -> CResult<(Rc<FuncProto>, Vec<IcSite>)> {
    let mut line_starts = vec![0u32];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i as u32 + 1);
        }
    }
    let mut c = Compiler {
        fns: Vec::new(),
        res,
        line_starts,
        cur_line: 1,
        source: Rc::from(src),
        global_kinds: HashMap::new(),
        ic_sites: Vec::new(),
        ic_base,
    };
    c.begin_fn(SCRIPT_KEY, "<script>".into(), false, false);
    c.fns[0].is_script = true;
    // slot 0: this
    c.declare_local("this", VarKind::Var);
    c.fns[0].scopes.push(Vec::new()); // global scope marker
    let mut vars = Vec::new();
    crate::resolver::collect_vars(&p.body, &mut vars);
    for v in vars {
        c.global_kinds.entry(v).or_insert(VarKind::Var);
    }
    for s in &p.body {
        match s {
            Stmt::Var(k, d, _) if *k != VarKind::Var => {
                for (n, _) in d {
                    c.global_kinds.insert(n.clone(), *k);
                }
            }
            Stmt::Function(f) => {
                c.global_kinds.insert(f.name.clone().unwrap(), VarKind::Let);
            }
            Stmt::Class(cl) => {
                c.global_kinds.insert(cl.name.clone().unwrap(), VarKind::Let);
            }
            _ => {}
        }
    }
    // hoist top-level function declarations
    for s in &p.body {
        if let Stmt::Function(f) = s {
            c.function_value(f)?;
            let n = c.name_const(f.name.as_ref().unwrap());
            c.emit(Op::SetGlobal(n));
            c.emit(Op::Pop);
        }
    }
    for s in &p.body {
        if !matches!(s, Stmt::Function(_)) {
            c.stmt(s)?;
        }
    }
    c.emit(Op::Undef);
    c.emit(Op::Return);
    let proto = c.end_fn();
    Ok((proto, c.ic_sites))
}

fn binop(op: &str) -> Op {
    match op {
        "+" => Op::Add,
        "-" => Op::Sub,
        "*" => Op::Mul,
        "/" => Op::Div,
        "%" => Op::Mod,
        "**" => Op::Pow,
        "&" => Op::BitAnd,
        "|" => Op::BitOr,
        "^" => Op::BitXor,
        "<<" => Op::Shl,
        ">>" => Op::Shr,
        ">>>" => Op::UShr,
        "<" => Op::Lt,
        "<=" => Op::Le,
        ">" => Op::Gt,
        ">=" => Op::Ge,
        "==" => Op::Eq,
        "!=" => Op::Ne,
        "===" => Op::StrictEq,
        "!==" => Op::StrictNe,
        "instanceof" => Op::InstanceOf,
        "in" => Op::In,
        _ => unreachable!("{op}"),
    }
}

impl<'a> Compiler<'a> {
    fn f(&mut self) -> &mut FnState {
        self.fns.last_mut().unwrap()
    }

    fn set_span(&mut self, sp: Span) {
        let off = sp.start;
        self.cur_line = match self.line_starts.binary_search(&off) {
            Ok(i) => i as u32 + 1,
            Err(i) => i as u32,
        };
    }

    fn emit(&mut self, op: Op) -> usize {
        let line = self.cur_line;
        let f = self.f();
        f.code.push(op);
        f.lines.push(line);
        f.code.len() - 1
    }

    fn here(&mut self) -> usize {
        self.f().code.len()
    }

    fn patch(&mut self, at: usize) {
        let target = self.here() as u32;
        self.patch_to(at, target);
    }

    fn patch_to(&mut self, at: usize, target: u32) {
        let f = self.f();
        f.code[at] = match f.code[at] {
            Op::Jump(_) => Op::Jump(target),
            Op::JumpIfFalse(_) => Op::JumpIfFalse(target),
            Op::JumpIfTrue(_) => Op::JumpIfTrue(target),
            Op::JumpIfFalseKeep(_) => Op::JumpIfFalseKeep(target),
            Op::JumpIfTrueKeep(_) => Op::JumpIfTrueKeep(target),
            Op::JumpIfNotNullishKeep(_) => Op::JumpIfNotNullishKeep(target),
            Op::PushHandler(_) => Op::PushHandler(target),
            other => panic!("cannot patch {other:?}"),
        };
    }

    fn constant(&mut self, v: Value) -> u32 {
        let key = match &v {
            Value::Num(n) => format!("n{}", n.to_bits()),
            Value::Str(s) => format!("s{s}"),
            _ => unreachable!(),
        };
        if let Some(i) = self.f().const_map.get(&key) {
            return *i;
        }
        let f = self.f();
        f.consts.push(v);
        let i = f.consts.len() as u32 - 1;
        f.const_map.insert(key, i);
        i
    }

    fn name_const(&mut self, s: &str) -> u32 {
        self.constant(Value::str(s))
    }

    fn new_ic(&mut self, name: &str) -> u32 {
        self.ic_sites.push(IcSite {
            name: Rc::from(name),
            line: self.cur_line,
        });
        self.ic_base + self.ic_sites.len() as u32 - 1
    }

    fn begin_fn(&mut self, key: FnKey, name: String, is_arrow: bool, is_class: bool) {
        self.fns.push(FnState {
            key,
            name,
            code: Vec::new(),
            lines: Vec::new(),
            consts: Vec::new(),
            const_map: HashMap::new(),
            funcs: Vec::new(),
            captures: Vec::new(),
            nlocals: 0,
            nparams: 0,
            scopes: vec![Vec::new()],
            loops: Vec::new(),
            handler_depth: 0,
            is_script: false,
            is_arrow,
            is_class,
            uses_var: false,
        });
    }

    fn end_fn(&mut self) -> Rc<FuncProto> {
        let f = self.fns.pop().unwrap();
        Rc::new(FuncProto {
            name: f.name,
            nparams: f.nparams,
            nlocals: f.nlocals,
            code: f.code,
            consts: f.consts,
            funcs: f.funcs,
            captures: f.captures.into_iter().map(|(_, c)| c).collect(),
            lines: f.lines,
            is_class: f.is_class,
            is_arrow: f.is_arrow,
            uses_var: f.uses_var,
            max_stack: 0,
            source: self.source.clone(),
            calls: Cell::new(0),
            jit: RefCell::new(Default::default()),
        })
    }

    /// Declare a local in the innermost scope; returns (slot, is_cell).
    fn declare_local(&mut self, name: &str, kind: VarKind) -> (u32, bool) {
        let cell = self.res.is_captured(self.f().key, name);
        let f = self.f();
        let slot = f.nlocals;
        f.nlocals += 1;
        f.scopes.last_mut().unwrap().push(Local {
            name: name.to_string(),
            slot,
            cell,
            kind,
        });
        (slot, cell)
    }

    fn temp_local(&mut self) -> u32 {
        let f = self.f();
        f.nlocals += 1;
        f.nlocals - 1
    }

    /// Are we at the script's global scope (declarations become globals)?
    fn at_global_scope(&self) -> bool {
        self.fns.len() == 1 && self.fns[0].scopes.len() == 2
    }

    fn resolve_in(&mut self, fi: usize, name: &str) -> VarRef {
        let f = &self.fns[fi];
        let skip_global = if f.is_script { 1 } else { usize::MAX };
        for (si, scope) in f.scopes.iter().enumerate().rev() {
            if si == skip_global {
                continue;
            }
            if let Some(l) = scope.iter().rev().find(|l| l.name == name) {
                return VarRef::Local(l.slot, l.cell, l.kind);
            }
        }
        if let Some(i) = f.captures.iter().position(|(n, _)| n == name) {
            let kind = self.capture_kind(fi, name);
            return VarRef::Upval(i as u32, kind);
        }
        if fi == 0 {
            return VarRef::Global(name.to_string());
        }
        let (cap, kind) = match self.resolve_in(fi - 1, name) {
            VarRef::Global(n) => return VarRef::Global(n),
            VarRef::Local(slot, cell, kind) => {
                assert!(cell, "resolver missed capture of '{name}'");
                (Capture::Local(slot), kind)
            }
            VarRef::Upval(i, kind) => (Capture::Upval(i), kind),
        };
        let f = &mut self.fns[fi];
        f.captures.push((name.to_string(), cap));
        VarRef::Upval(f.captures.len() as u32 - 1, kind)
    }

    fn capture_kind(&mut self, fi: usize, name: &str) -> VarKind {
        if fi == 0 {
            return VarKind::Var;
        }
        match self.resolve_in(fi - 1, name) {
            VarRef::Local(_, _, k) | VarRef::Upval(_, k) => k,
            VarRef::Global(_) => VarKind::Var,
        }
    }

    fn resolve(&mut self, name: &str) -> VarRef {
        let fi = self.fns.len() - 1;
        self.resolve_in(fi, name)
    }

    fn load_var(&mut self, name: &str) {
        match self.resolve(name) {
            VarRef::Local(s, false, _) => self.emit(Op::GetLocal(s)),
            VarRef::Local(s, true, _) => self.emit(Op::GetLocalCell(s)),
            VarRef::Upval(i, _) => self.emit(Op::GetUpval(i)),
            VarRef::Global(n) => {
                let c = self.name_const(&n);
                self.emit(Op::GetGlobal(c))
            }
        };
    }

    /// Store top of stack into a variable (value stays on the stack).
    fn store_var(&mut self, name: &str, is_init: bool) {
        let r = self.resolve(name);
        let kind = match &r {
            VarRef::Local(_, _, k) | VarRef::Upval(_, k) => *k,
            VarRef::Global(n) => self.global_kinds.get(n).copied().unwrap_or(VarKind::Var),
        };
        if kind == VarKind::Const && !is_init {
            let c = self.name_const(name);
            self.emit(Op::ThrowConstAssign(c));
            return;
        }
        match r {
            VarRef::Local(s, false, _) => self.emit(Op::SetLocal(s)),
            VarRef::Local(s, true, _) => self.emit(Op::SetLocalCell(s)),
            VarRef::Upval(i, _) => self.emit(Op::SetUpval(i)),
            VarRef::Global(n) => {
                let c = self.name_const(&n);
                self.emit(Op::SetGlobal(c))
            }
        };
    }

    // ------------------------------------------------------------ statements

    fn push_scope(&mut self) {
        self.f().scopes.push(Vec::new());
    }
    fn pop_scope(&mut self) {
        self.f().scopes.pop();
    }

    /// Declare the lexical bindings of a block up front (creating fresh cells
    /// for captured ones) and hoist function declarations.
    fn enter_block(&mut self, body: &[Stmt]) -> CResult<()> {
        if self.at_global_scope() {
            return Ok(());
        }
        for s in body {
            let names: Vec<(String, VarKind)> = match s {
                Stmt::Var(k @ (VarKind::Let | VarKind::Const), d, _) => d.iter().map(|(n, _)| (n.clone(), *k)).collect(),
                Stmt::Function(f) => vec![(f.name.clone().unwrap(), VarKind::Let)],
                Stmt::Class(c) => vec![(c.name.clone().unwrap(), VarKind::Let)],
                _ => vec![],
            };
            for (n, k) in names {
                let (slot, cell) = self.declare_local(&n, k);
                if cell {
                    self.emit(Op::Undef);
                    self.emit(Op::SetLocal(slot));
                    self.emit(Op::Pop);
                    self.emit(Op::BoxLocal(slot));
                }
            }
        }
        for s in body {
            if let Stmt::Function(f) = s {
                self.function_value(f)?;
                self.store_var(f.name.as_ref().unwrap(), true);
                self.emit(Op::Pop);
            }
        }
        Ok(())
    }

    fn block(&mut self, body: &[Stmt]) -> CResult<()> {
        self.push_scope();
        self.enter_block(body)?;
        for s in body {
            if !matches!(s, Stmt::Function(_)) {
                self.stmt(s)?;
            }
        }
        self.pop_scope();
        Ok(())
    }

    fn sub_stmt(&mut self, s: &Stmt) -> CResult<()> {
        match s {
            Stmt::Block(b, _) => self.block(b),
            Stmt::Function(_) => self.block(std::slice::from_ref(s)),
            other => {
                self.push_scope();
                let r = self.stmt(other);
                self.pop_scope();
                r
            }
        }
    }

    fn stmt(&mut self, s: &Stmt) -> CResult<()> {
        self.set_span(s.span());
        match s {
            Stmt::Expr(e, _) => {
                self.expr(e)?;
                self.emit(Op::Pop);
            }
            Stmt::Var(k, decls, _) => {
                for (n, init) in decls {
                    if self.at_global_scope() {
                        match init {
                            Some(e) => self.expr_named(e, n)?,
                            None if *k == VarKind::Var => continue,
                            None => {
                                self.emit(Op::Undef);
                            }
                        }
                        let c = self.name_const(n);
                        self.emit(Op::SetGlobal(c));
                        self.emit(Op::Pop);
                        continue;
                    }
                    match init {
                        Some(e) => self.expr_named(e, n)?,
                        None if *k == VarKind::Var => continue,
                        None => {
                            self.emit(Op::Undef);
                        }
                    }
                    self.store_var(n, true);
                    self.emit(Op::Pop);
                }
            }
            Stmt::Function(_) => {} // hoisted by enter_block
            Stmt::Class(c) => {
                self.class(c)?;
                let n = c.name.as_ref().unwrap();
                if self.at_global_scope() {
                    let k = self.name_const(n);
                    self.emit(Op::SetGlobal(k));
                } else {
                    self.store_var(n, true);
                }
                self.emit(Op::Pop);
            }
            Stmt::Return(e, _) => {
                match e {
                    Some(e) => self.expr(e)?,
                    None => {
                        self.emit(Op::Undef);
                    }
                }
                self.emit(Op::Return);
            }
            Stmt::If(c, t, e, _) => {
                let jf = self.jump_if_false(c)?;
                self.sub_stmt(t)?;
                if let Some(e) = e {
                    let j = self.emit(Op::Jump(0));
                    for x in jf {
                        self.patch(x);
                    }
                    self.sub_stmt(e)?;
                    self.patch(j);
                } else {
                    for x in jf {
                        self.patch(x);
                    }
                }
            }
            Stmt::While(c, b, _) => {
                let start = self.here() as u32;
                let exits = self.jump_if_false(c)?;
                self.begin_loop();
                self.sub_stmt(b)?;
                self.emit(Op::Jump(start));
                self.end_loop(exits, start);
            }
            Stmt::DoWhile(b, c, _) => {
                let start = self.here() as u32;
                self.begin_loop();
                self.sub_stmt(b)?;
                let cont = self.here() as u32;
                self.set_span(c.span());
                let exits = self.jump_if_false(c)?;
                self.emit(Op::Jump(start));
                self.end_loop(exits, cont);
            }
            Stmt::For(init, test, update, body, _) => {
                self.push_scope();
                let mut loop_cells = Vec::new();
                match init {
                    Some(ForInit::Var(k, d)) => {
                        for (n, e) in d {
                            match e {
                                Some(e) => self.expr_named(e, n)?,
                                None => {
                                    self.emit(Op::Undef);
                                }
                            }
                            if *k == VarKind::Var {
                                self.store_var(n, true);
                                self.emit(Op::Pop);
                            } else {
                                let (slot, cell) = self.declare_local(n, *k);
                                self.emit(Op::SetLocal(slot));
                                self.emit(Op::Pop);
                                if cell {
                                    self.emit(Op::BoxLocal(slot));
                                    loop_cells.push(slot);
                                }
                            }
                        }
                    }
                    Some(ForInit::Expr(e)) => {
                        self.expr(e)?;
                        self.emit(Op::Pop);
                    }
                    None => {}
                }
                let start = self.here() as u32;
                let exits = match test {
                    Some(t) => self.jump_if_false(t)?,
                    None => vec![],
                };
                self.begin_loop();
                self.sub_stmt(body)?;
                let cont = self.here() as u32;
                for s in &loop_cells {
                    self.emit(Op::RefreshCell(*s));
                }
                if let Some(u) = update {
                    self.set_span(u.span());
                    self.expr(u)?;
                    self.emit(Op::Pop);
                }
                self.emit(Op::Jump(start));
                self.end_loop(exits, cont);
                self.pop_scope();
            }
            Stmt::ForOf(k, n, iter, body, _) | Stmt::ForIn(k, n, iter, body, _) => {
                self.push_scope();
                self.expr(iter)?;
                if matches!(s, Stmt::ForIn(..)) {
                    self.emit(Op::Keys);
                }
                let arr = self.temp_local();
                let idx = self.temp_local();
                self.emit(Op::SetLocal(arr));
                self.emit(Op::Pop);
                let zero = self.constant(Value::Num(0.0));
                self.emit(Op::Const(zero));
                self.emit(Op::SetLocal(idx));
                self.emit(Op::Pop);
                let start = self.here() as u32;
                self.emit(Op::GetLocal(idx));
                self.emit(Op::GetLocal(arr));
                let lc = self.name_const("length");
                let ic = self.new_ic("length");
                self.emit(Op::GetProp(lc, ic));
                self.emit(Op::Lt);
                let exit = self.emit(Op::JumpIfFalse(0));
                self.begin_loop();
                self.push_scope();
                self.emit(Op::GetLocal(arr));
                self.emit(Op::GetLocal(idx));
                self.emit(Op::GetElem);
                if *k == VarKind::Var {
                    self.store_var(n, true);
                    self.emit(Op::Pop);
                } else {
                    let (slot, cell) = self.declare_local(n, *k);
                    self.emit(Op::SetLocal(slot));
                    self.emit(Op::Pop);
                    if cell {
                        self.emit(Op::BoxLocal(slot));
                    }
                }
                self.sub_stmt(body)?;
                self.pop_scope();
                let cont = self.here() as u32;
                let one = self.constant(Value::Num(1.0));
                self.emit(Op::GetLocal(idx));
                self.emit(Op::Const(one));
                self.emit(Op::Add);
                self.emit(Op::SetLocal(idx));
                self.emit(Op::Pop);
                self.emit(Op::Jump(start));
                self.end_loop(vec![exit], cont);
                self.pop_scope();
            }
            Stmt::Break(sp) | Stmt::Continue(sp) => {
                let is_break = matches!(s, Stmt::Break(_));
                let depth = self.f().handler_depth;
                let Some(l) = self.f().loops.last() else {
                    return Err(SyntaxError::new("break/continue outside loop", *sp));
                };
                for _ in l.handler_depth..depth {
                    self.emit(Op::PopHandler);
                }
                let j = self.emit(Op::Jump(0));
                let l = self.f().loops.last_mut().unwrap();
                if is_break {
                    l.breaks.push(j)
                } else {
                    l.continues.push(j)
                }
            }
            Stmt::Throw(e, _) => {
                self.expr(e)?;
                self.emit(Op::Throw);
            }
            Stmt::Try(body, handler, finalizer, _) => {
                let outer = if finalizer.is_some() {
                    self.f().handler_depth += 1;
                    Some(self.emit(Op::PushHandler(0)))
                } else {
                    None
                };
                if let Some((param, hbody)) = handler {
                    self.f().handler_depth += 1;
                    let h = self.emit(Op::PushHandler(0));
                    self.block(body)?;
                    self.emit(Op::PopHandler);
                    self.f().handler_depth -= 1;
                    let j = self.emit(Op::Jump(0));
                    self.patch(h);
                    self.push_scope();
                    match param {
                        Some(p) => {
                            let (slot, cell) = self.declare_local(p, VarKind::Let);
                            self.emit(Op::SetLocal(slot));
                            self.emit(Op::Pop);
                            if cell {
                                self.emit(Op::BoxLocal(slot));
                            }
                        }
                        None => {
                            self.emit(Op::Pop);
                        }
                    }
                    self.block(hbody)?;
                    self.pop_scope();
                    self.patch(j);
                } else {
                    self.block(body)?;
                }
                if let (Some(outer), Some(fin)) = (outer, finalizer) {
                    self.emit(Op::PopHandler);
                    self.f().handler_depth -= 1;
                    self.block(fin)?;
                    let done = self.emit(Op::Jump(0));
                    self.patch(outer);
                    let tmp = self.temp_local();
                    self.emit(Op::SetLocal(tmp));
                    self.emit(Op::Pop);
                    self.block(fin)?;
                    self.emit(Op::GetLocal(tmp));
                    self.emit(Op::Throw);
                    self.patch(done);
                }
            }
            Stmt::Block(b, _) => self.block(b)?,
            Stmt::Empty(_) => {}
        }
        Ok(())
    }

    fn begin_loop(&mut self) {
        let d = self.f().handler_depth;
        self.f().loops.push(LoopCtx {
            handler_depth: d,
            ..Default::default()
        });
    }

    fn end_loop(&mut self, exits: Vec<usize>, cont: u32) {
        let l = self.f().loops.pop().unwrap();
        for c in l.continues {
            self.patch_to(c, cont);
        }
        for b in l.breaks.into_iter().chain(exits) {
            self.patch(b);
        }
    }

    /// Compile a condition, emitting jumps taken when it is false. Returns the jumps to patch.
    fn jump_if_false(&mut self, e: &Expr) -> CResult<Vec<usize>> {
        match e {
            Expr::Unary("!", x, _) => self.jump_if_true(x),
            Expr::Logical("&&", a, b, _) => {
                let mut j = self.jump_if_false(a)?;
                j.extend(self.jump_if_false(b)?);
                Ok(j)
            }
            Expr::Logical("||", a, b, _) => {
                let t = self.jump_if_true(a)?;
                let f = self.jump_if_false(b)?;
                for x in t {
                    self.patch(x);
                }
                Ok(f)
            }
            _ => {
                self.expr(e)?;
                Ok(vec![self.emit(Op::JumpIfFalse(0))])
            }
        }
    }

    fn jump_if_true(&mut self, e: &Expr) -> CResult<Vec<usize>> {
        match e {
            Expr::Unary("!", x, _) => self.jump_if_false(x),
            Expr::Logical("||", a, b, _) => {
                let mut j = self.jump_if_true(a)?;
                j.extend(self.jump_if_true(b)?);
                Ok(j)
            }
            Expr::Logical("&&", a, b, _) => {
                let f = self.jump_if_false(a)?;
                let t = self.jump_if_true(b)?;
                for x in f {
                    self.patch(x);
                }
                Ok(t)
            }
            _ => {
                self.expr(e)?;
                Ok(vec![self.emit(Op::JumpIfTrue(0))])
            }
        }
    }

    // ------------------------------------------------------------ functions & classes

    fn compile_function(&mut self, f: &Function, key: FnKey, name: String, is_class: bool) -> CResult<u32> {
        self.begin_fn(key, name, f.is_arrow, is_class);
        if !f.is_arrow {
            self.declare_local("this", VarKind::Var);
        } else {
            self.temp_local(); // slot 0 unused (this comes from the enclosing function)
        }
        for p in &f.params {
            self.declare_local(p, VarKind::Var);
        }
        self.f().nparams = f.params.len() as u32;
        // box captured this/params
        let to_box: Vec<u32> = self.f().scopes[0].iter().filter(|l| l.cell).map(|l| l.slot).collect();
        for s in to_box {
            self.emit(Op::BoxLocal(s));
        }
        let mut vars = Vec::new();
        crate::resolver::collect_vars(&f.body, &mut vars);
        if !vars.is_empty() {
            self.f().uses_var = true;
        }
        for v in vars {
            if self.f().scopes[0].iter().any(|l| l.name == v) {
                continue;
            }
            let (slot, cell) = self.declare_local(&v, VarKind::Var);
            if cell {
                self.emit(Op::Undef);
                self.emit(Op::SetLocal(slot));
                self.emit(Op::Pop);
                self.emit(Op::BoxLocal(slot));
            }
        }
        self.block(&f.body)?;
        self.emit(Op::Undef);
        self.emit(Op::Return);
        let proto = self.end_fn();
        let parent = self.f();
        parent.funcs.push(proto);
        Ok(parent.funcs.len() as u32 - 1)
    }

    fn function_value(&mut self, f: &Function) -> CResult<()> {
        self.function_value_named(f, f.name.clone().unwrap_or_default())
    }

    fn function_value_named(&mut self, f: &Function, name: String) -> CResult<()> {
        let idx = self.compile_function(f, f.span.start, name, false)?;
        self.emit(Op::Closure(idx));
        Ok(())
    }

    /// Compile an initializer expression; anonymous functions take the binding's name.
    fn expr_named(&mut self, e: &Expr, name: &str) -> CResult<()> {
        match e {
            Expr::Function(f) if f.name.is_none() => self.function_value_named(f, name.to_string()),
            Expr::Class(c) if c.name.is_none() => {
                let mut c2 = (**c).clone();
                c2.name = Some(name.to_string());
                self.class_inner(&c2, false)
            }
            _ => self.expr(e),
        }
    }

    fn class(&mut self, c: &Class) -> CResult<()> {
        self.class_inner(c, true)
    }

    fn class_inner(&mut self, c: &Class, _named: bool) -> CResult<()> {
        let name = c.name.clone().unwrap_or_default();
        self.push_scope();
        let (sup, sup_cell) = self.declare_local("%super", VarKind::Const);
        match &c.extends {
            Some(e) => self.expr(e)?,
            None => {
                self.emit(Op::Null);
            }
        }
        self.emit(Op::SetLocal(sup));
        self.emit(Op::Pop);
        if sup_cell {
            self.emit(Op::BoxLocal(sup));
        }
        // parent prototype
        if c.extends.is_some() {
            self.load_var("%super");
            let pc = self.name_const("prototype");
            let ic = self.new_ic("prototype");
            self.emit(Op::GetProp(pc, ic));
        } else {
            self.emit(Op::Null);
        }
        let ctor = match &c.constructor {
            Some(k) => k.clone(),
            None => {
                let sp = Span::new(c.span.start as usize, c.span.start as usize);
                let params: Vec<String> = if c.extends.is_some() {
                    (0..6).map(|i| format!("%a{i}")).collect()
                } else {
                    vec![]
                };
                let body = if c.extends.is_some() {
                    vec![Stmt::Expr(
                        Expr::SuperCall(params.iter().map(|p| Expr::Ident(p.clone(), sp)).collect(), sp),
                        sp,
                    )]
                } else {
                    vec![]
                };
                Function {
                    name: Some(name.clone()),
                    params,
                    body,
                    is_arrow: false,
                    expr_body: false,
                    span: sp,
                }
            }
        };
        let key = if c.constructor.is_some() { ctor.span.start } else { u32::MAX - 1 };
        let idx = self.compile_function(&ctor, key, name, true)?;
        self.emit(Op::Closure(idx));
        self.emit(Op::MakeClass);
        for (mname, m, is_static) in &c.methods {
            let idx = self.compile_function(m, m.span.start, mname.clone(), false)?;
            self.emit(Op::Closure(idx));
            let n = self.name_const(mname);
            if *is_static {
                let ic = self.new_ic(mname);
                self.emit(Op::InitProp(n, ic));
            } else {
                self.emit(Op::DefineMethod(n));
            }
        }
        self.pop_scope();
        Ok(())
    }

    // ------------------------------------------------------------ expressions

    fn args_have_spread(args: &[Expr]) -> bool {
        args.iter().any(|a| matches!(a, Expr::Spread(..)))
    }

    /// Build an array from a list possibly containing spreads.
    fn spread_array(&mut self, items: &[Expr]) -> CResult<()> {
        self.emit(Op::Array(0));
        for a in items {
            match a {
                Expr::Spread(x, _) => {
                    self.expr(x)?;
                    self.emit(Op::ArrayExtend);
                }
                _ => {
                    self.expr(a)?;
                    self.emit(Op::ArrayPush);
                }
            }
        }
        Ok(())
    }

    fn call_args(&mut self, args: &[Expr]) -> CResult<Option<u32>> {
        if Self::args_have_spread(args) {
            self.spread_array(args)?;
            Ok(None)
        } else {
            for a in args {
                self.expr(a)?;
            }
            Ok(Some(args.len() as u32))
        }
    }

    fn load_this(&mut self) {
        if self.f().is_arrow {
            self.load_var("this");
        } else {
            match self.resolve("this") {
                VarRef::Local(s, true, _) => self.emit(Op::GetLocalCell(s)),
                _ => self.emit(Op::GetLocal(0)),
            };
        }
    }

    pub fn expr(&mut self, e: &Expr) -> CResult<()> {
        match e {
            Expr::Num(n, _) => {
                let c = self.constant(Value::Num(*n));
                self.emit(Op::Const(c));
            }
            Expr::Str(s, _) => {
                let c = self.name_const(s);
                self.emit(Op::Const(c));
            }
            Expr::Template(q, es, _) => {
                let c = self.name_const(&q[0]);
                self.emit(Op::Const(c));
                for (i, x) in es.iter().enumerate() {
                    self.expr(x)?;
                    self.emit(Op::Add);
                    if !q[i + 1].is_empty() {
                        let c = self.name_const(&q[i + 1]);
                        self.emit(Op::Const(c));
                        self.emit(Op::Add);
                    }
                }
            }
            Expr::Bool(b, _) => {
                self.emit(if *b { Op::True } else { Op::False });
            }
            Expr::Null(_) => {
                self.emit(Op::Null);
            }
            Expr::Undefined(_) => {
                self.emit(Op::Undef);
            }
            Expr::Ident(n, _) => self.load_var(n),
            Expr::This(_) => self.load_this(),
            Expr::Array(items, _) => {
                if Self::args_have_spread(items) {
                    self.spread_array(items)?;
                } else {
                    for x in items {
                        self.expr(x)?;
                    }
                    self.emit(Op::Array(items.len() as u32));
                }
            }
            Expr::Object(props, _) => {
                self.emit(Op::NewObject);
                for p in props {
                    match p {
                        Prop::KeyValue(k, v) => self.init_prop(k, |c| c.expr_named(v, &key_name(k)))?,
                        Prop::Shorthand(n) => {
                            self.load_var(n);
                            let c = self.name_const(n);
                            let ic = self.new_ic(n);
                            self.emit(Op::InitProp(c, ic));
                        }
                        Prop::Method(k, f) => self.init_prop(k, |c| c.function_value_named(f, key_name(k)))?,
                        Prop::Spread(x) => {
                            self.expr(x)?;
                            self.emit(Op::ObjectSpread);
                        }
                    }
                }
            }
            Expr::Function(f) => {
                if let (Some(n), false) = (&f.name, f.is_arrow) {
                    // named function expression: bind its own name
                    self.push_scope();
                    let (slot, cell) = self.declare_local(n, VarKind::Const);
                    if cell {
                        self.emit(Op::Undef);
                        self.emit(Op::SetLocal(slot));
                        self.emit(Op::Pop);
                        self.emit(Op::BoxLocal(slot));
                    }
                    self.function_value(f)?;
                    self.emit(if cell { Op::SetLocalCell(slot) } else { Op::SetLocal(slot) });
                    self.pop_scope();
                } else {
                    self.function_value(f)?;
                }
            }
            Expr::Class(c) => self.class(c)?,
            Expr::Unary(op, x, _) => {
                if *op == "typeof" {
                    if let Expr::Ident(n, _) = &**x {
                        if let VarRef::Global(g) = self.resolve(n) {
                            let c = self.name_const(&g);
                            self.emit(Op::TypeOfGlobal(c));
                            return Ok(());
                        }
                    }
                }
                self.expr(x)?;
                match *op {
                    "-" => self.emit(Op::Neg),
                    "+" => self.emit(Op::ToNumber),
                    "!" => self.emit(Op::Not),
                    "~" => self.emit(Op::BitNot),
                    "typeof" => self.emit(Op::TypeOf),
                    "void" => {
                        self.emit(Op::Pop);
                        self.emit(Op::Undef)
                    }
                    _ => unreachable!(),
                };
            }
            Expr::Update { op, prefix, target, .. } => {
                let arith = if *op == "++" { Op::Add } else { Op::Sub };
                let one = self.constant(Value::Num(1.0));
                match &**target {
                    Expr::Ident(n, _) => {
                        self.load_var(n);
                        self.emit(Op::ToNumber);
                        if !prefix {
                            self.emit(Op::Dup);
                        }
                        self.emit(Op::Const(one));
                        self.emit(arith);
                        self.store_var(n, false);
                        if !prefix {
                            self.emit(Op::Pop);
                        }
                    }
                    Expr::Member(o, name, _) => {
                        self.expr(o)?;
                        self.emit(Op::Dup);
                        let c = self.name_const(name);
                        let ic = self.new_ic(name);
                        self.emit(Op::GetProp(c, ic));
                        self.emit(Op::ToNumber);
                        let tmp = if !prefix {
                            let t = self.temp_local();
                            self.emit(Op::SetLocal(t));
                            Some(t)
                        } else {
                            None
                        };
                        self.emit(Op::Const(one));
                        self.emit(arith);
                        let ic2 = self.new_ic(name);
                        self.emit(Op::SetProp(c, ic2));
                        if let Some(t) = tmp {
                            self.emit(Op::Pop);
                            self.emit(Op::GetLocal(t));
                        }
                    }
                    Expr::Index(o, k, _) => {
                        self.expr(o)?;
                        self.expr(k)?;
                        self.emit(Op::Dup2);
                        self.emit(Op::GetElem);
                        self.emit(Op::ToNumber);
                        let tmp = if !prefix {
                            let t = self.temp_local();
                            self.emit(Op::SetLocal(t));
                            Some(t)
                        } else {
                            None
                        };
                        self.emit(Op::Const(one));
                        self.emit(arith);
                        self.emit(Op::SetElem);
                        if let Some(t) = tmp {
                            self.emit(Op::Pop);
                            self.emit(Op::GetLocal(t));
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Expr::Binary(op, l, r, _) => {
                self.expr(l)?;
                self.expr(r)?;
                self.emit(binop(op));
            }
            Expr::Logical(op, l, r, _) => {
                self.expr(l)?;
                let j = self.emit(match *op {
                    "&&" => Op::JumpIfFalseKeep(0),
                    "||" => Op::JumpIfTrueKeep(0),
                    _ => Op::JumpIfNotNullishKeep(0),
                });
                self.expr(r)?;
                self.patch(j);
            }
            Expr::Assign(op, target, value, sp) => self.assign(op, target, value, *sp)?,
            Expr::Cond(c, t, f, _) => {
                let jf = self.jump_if_false(c)?;
                self.expr(t)?;
                let j = self.emit(Op::Jump(0));
                for x in jf {
                    self.patch(x);
                }
                self.expr(f)?;
                self.patch(j);
            }
            Expr::Call(callee, args, sp) => {
                self.set_span(*sp);
                match &**callee {
                    Expr::Member(o, name, _) => {
                        self.expr(o)?;
                        let c = self.name_const(name);
                        let ic = self.new_ic(name);
                        self.emit(Op::GetMethod(c, ic));
                    }
                    Expr::Index(o, k, _) => {
                        self.expr(o)?;
                        self.expr(k)?;
                        self.emit(Op::GetElemMethod);
                    }
                    Expr::SuperMember(name, _) => {
                        self.load_var("%super");
                        let pc = self.name_const("prototype");
                        let ic = self.new_ic("prototype");
                        self.emit(Op::GetProp(pc, ic));
                        let c = self.name_const(name);
                        let ic = self.new_ic(name);
                        self.emit(Op::GetProp(c, ic));
                        self.load_this();
                    }
                    other => {
                        self.expr(other)?;
                        self.emit(Op::Undef);
                    }
                }
                self.set_span(*sp);
                match self.call_args(args)? {
                    Some(n) => self.emit(Op::Call(n)),
                    None => self.emit(Op::CallSpread),
                };
            }
            Expr::SuperCall(args, sp) => {
                self.load_var("%super");
                self.load_this();
                if Self::args_have_spread(args) {
                    return Err(SyntaxError::new("spread in super() is not supported", *sp));
                }
                for a in args {
                    self.expr(a)?;
                }
                self.emit(Op::CallSuper(args.len() as u32));
            }
            Expr::SuperMember(name, _) => {
                self.load_var("%super");
                let pc = self.name_const("prototype");
                let ic = self.new_ic("prototype");
                self.emit(Op::GetProp(pc, ic));
                let c = self.name_const(name);
                let ic = self.new_ic(name);
                self.emit(Op::GetProp(c, ic));
            }
            Expr::New(callee, args, sp) => {
                self.expr(callee)?;
                self.set_span(*sp);
                match self.call_args(args)? {
                    Some(n) => self.emit(Op::New(n)),
                    None => self.emit(Op::NewSpread),
                };
            }
            Expr::Member(o, name, sp) => {
                self.expr(o)?;
                self.set_span(*sp);
                let c = self.name_const(name);
                let ic = self.new_ic(name);
                self.emit(Op::GetProp(c, ic));
            }
            Expr::Index(o, k, _) => {
                self.expr(o)?;
                self.expr(k)?;
                self.emit(Op::GetElem);
            }
            Expr::Spread(_, sp) => return Err(SyntaxError::new("unexpected spread", *sp)),
            Expr::Seq(xs, _) => {
                for (i, x) in xs.iter().enumerate() {
                    self.expr(x)?;
                    if i + 1 < xs.len() {
                        self.emit(Op::Pop);
                    }
                }
            }
        }
        Ok(())
    }

    fn init_prop(&mut self, k: &PropKey, value: impl FnOnce(&mut Self) -> CResult<()>) -> CResult<()> {
        match k {
            PropKey::Computed(kx) => {
                self.expr(kx)?;
                value(self)?;
                self.emit(Op::InitElem);
            }
            _ => {
                let name = key_name(k);
                value(self)?;
                let c = self.name_const(&name);
                let ic = self.new_ic(&name);
                self.emit(Op::InitProp(c, ic));
            }
        }
        Ok(())
    }

    fn assign(&mut self, op: &str, target: &Expr, value: &Expr, sp: Span) -> CResult<()> {
        let logical = matches!(op, "&&=" | "||=" | "??=");
        let bin = if op == "=" || logical {
            None
        } else {
            Some(binop(&op[..op.len() - 1]))
        };
        match target {
            Expr::Ident(n, _) => {
                if logical {
                    self.load_var(n);
                    let j = self.emit(match op {
                        "&&=" => Op::JumpIfFalseKeep(0),
                        "||=" => Op::JumpIfTrueKeep(0),
                        _ => Op::JumpIfNotNullishKeep(0),
                    });
                    self.expr_named(value, n)?;
                    self.store_var(n, false);
                    self.patch(j);
                    return Ok(());
                }
                if let Some(b) = bin {
                    self.load_var(n);
                    self.expr(value)?;
                    self.emit(b);
                } else {
                    self.expr_named(value, n)?;
                }
                self.store_var(n, false);
            }
            _ if logical => return Err(SyntaxError::new("logical assignment to a property is not supported", sp)),
            Expr::Member(o, name, _) => {
                self.expr(o)?;
                let c = self.name_const(name);
                if let Some(b) = bin {
                    self.emit(Op::Dup);
                    let ic = self.new_ic(name);
                    self.emit(Op::GetProp(c, ic));
                    self.expr(value)?;
                    self.emit(b);
                } else {
                    self.expr(value)?;
                }
                let ic = self.new_ic(name);
                self.emit(Op::SetProp(c, ic));
            }
            Expr::Index(o, k, _) => {
                self.expr(o)?;
                self.expr(k)?;
                if let Some(b) = bin {
                    self.emit(Op::Dup2);
                    self.emit(Op::GetElem);
                    self.expr(value)?;
                    self.emit(b);
                } else {
                    self.expr(value)?;
                }
                self.emit(Op::SetElem);
            }
            _ => return Err(SyntaxError::new("invalid assignment target", sp)),
        }
        Ok(())
    }
}

fn key_name(k: &PropKey) -> String {
    match k {
        PropKey::Ident(s) | PropKey::Str(s) => s.clone(),
        PropKey::Num(n) => crate::value::num_to_string(*n),
        PropKey::Computed(_) => String::new(),
    }
}

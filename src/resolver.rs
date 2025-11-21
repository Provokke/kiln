//! Scope resolution pass.
//!
//! Walks the AST with a stack of function and block scopes, and computes for
//! every function the set of its local variable names that are captured by
//! a nested closure (those are stored in heap cells by the compiler; all
//! other locals live directly in VM stack slots). It also reports early
//! errors such as duplicate `let`/`const` declarations.

use crate::ast::*;
use crate::lexer::{Span, SyntaxError};
use std::collections::{HashMap, HashSet};

/// Key identifying a function: its span start (the script uses `u32::MAX`).
pub type FnKey = u32;
pub const SCRIPT_KEY: FnKey = u32::MAX;

#[derive(Default, Debug)]
pub struct Resolution {
    pub captured: HashMap<FnKey, HashSet<String>>,
}

impl Resolution {
    pub fn is_captured(&self, f: FnKey, name: &str) -> bool {
        self.captured.get(&f).is_some_and(|s| s.contains(name))
    }
}

struct FnScope {
    key: FnKey,
    blocks: Vec<HashMap<String, VarKind>>,
    is_arrow: bool,
}

struct Resolver {
    fns: Vec<FnScope>,
    res: Resolution,
}

pub fn resolve(p: &Program) -> Result<Resolution, SyntaxError> {
    let mut r = Resolver {
        fns: vec![FnScope {
            key: SCRIPT_KEY,
            blocks: vec![HashMap::new()],
            is_arrow: false,
        }],
        res: Resolution::default(),
    };
    r.declare("this", VarKind::Var, Span::default())?;
    let mut vars = Vec::new();
    collect_vars(&p.body, &mut vars);
    for v in vars {
        r.declare(&v, VarKind::Var, Span::default())?;
    }
    r.block(&p.body)?;
    Ok(r.res)
}

/// Collect `var`-declared names in a function body (not descending into nested functions).
pub fn collect_vars(body: &[Stmt], out: &mut Vec<String>) {
    for s in body {
        collect_vars_stmt(s, out);
    }
}

fn collect_vars_stmt(s: &Stmt, out: &mut Vec<String>) {
    match s {
        Stmt::Var(VarKind::Var, d, _) => out.extend(d.iter().map(|(n, _)| n.clone())),
        Stmt::If(_, t, e, _) => {
            collect_vars_stmt(t, out);
            if let Some(e) = e {
                collect_vars_stmt(e, out)
            }
        }
        Stmt::While(_, b, _) | Stmt::DoWhile(b, _, _) => collect_vars_stmt(b, out),
        Stmt::For(init, _, _, b, _) => {
            if let Some(ForInit::Var(VarKind::Var, d)) = init {
                out.extend(d.iter().map(|(n, _)| n.clone()));
            }
            collect_vars_stmt(b, out)
        }
        Stmt::ForOf(k, n, _, b, _) | Stmt::ForIn(k, n, _, b, _) => {
            if *k == VarKind::Var {
                out.push(n.clone());
            }
            collect_vars_stmt(b, out)
        }
        Stmt::Try(b, h, f, _) => {
            collect_vars(b, out);
            if let Some((_, hb)) = h {
                collect_vars(hb, out)
            }
            if let Some(fb) = f {
                collect_vars(fb, out)
            }
        }
        Stmt::Block(b, _) => collect_vars(b, out),
        _ => {}
    }
}

impl Resolver {
    fn declare(&mut self, name: &str, kind: VarKind, span: Span) -> Result<(), SyntaxError> {
        let scope = self.fns.last_mut().unwrap().blocks.last_mut().unwrap();
        if let Some(prev) = scope.get(name) {
            if kind != VarKind::Var || *prev != VarKind::Var {
                return Err(SyntaxError::new(format!("Identifier '{name}' has already been declared"), span));
            }
        }
        scope.insert(name.to_string(), kind);
        Ok(())
    }

    fn reference(&mut self, name: &str) {
        let n = self.fns.len();
        for fi in (0..n).rev() {
            let found = self.fns[fi].blocks.iter().any(|b| b.contains_key(name));
            if found {
                if fi != n - 1 {
                    // Script top-level block declarations are globals, never captured.
                    let f = &self.fns[fi];
                    let is_global = fi == 0 && f.blocks.iter().rposition(|b| b.contains_key(name)) == Some(0) && name != "this";
                    if !is_global {
                        self.res.captured.entry(f.key).or_default().insert(name.to_string());
                    }
                }
                return;
            }
        }
    }

    fn hoist_block(&mut self, body: &[Stmt]) -> Result<(), SyntaxError> {
        for s in body {
            match s {
                Stmt::Var(k @ (VarKind::Let | VarKind::Const), d, sp) => {
                    for (n, _) in d {
                        self.declare(n, *k, *sp)?;
                    }
                }
                Stmt::Function(f) => self.declare(f.name.as_ref().unwrap(), VarKind::Let, f.span)?,
                Stmt::Class(c) => self.declare(c.name.as_ref().unwrap(), VarKind::Let, c.span)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn block(&mut self, body: &[Stmt]) -> Result<(), SyntaxError> {
        self.hoist_block(body)?;
        for s in body {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn push_block(&mut self) {
        self.fns.last_mut().unwrap().blocks.push(HashMap::new());
    }
    fn pop_block(&mut self) {
        self.fns.last_mut().unwrap().blocks.pop();
    }

    fn scoped_block(&mut self, body: &[Stmt]) -> Result<(), SyntaxError> {
        self.push_block();
        let r = self.block(body);
        self.pop_block();
        r
    }

    fn stmt(&mut self, s: &Stmt) -> Result<(), SyntaxError> {
        match s {
            Stmt::Expr(e, _) | Stmt::Throw(e, _) => self.expr(e),
            Stmt::Var(_, d, _) => {
                for (n, init) in d {
                    self.reference(n);
                    if let Some(e) = init {
                        self.expr(e)?;
                    }
                }
                Ok(())
            }
            Stmt::Function(f) => self.function(f),
            Stmt::Class(c) => self.class(c),
            Stmt::Return(e, _) => e.as_ref().map_or(Ok(()), |e| self.expr(e)),
            Stmt::If(c, t, e, _) => {
                self.expr(c)?;
                self.sub_stmt(t)?;
                if let Some(e) = e {
                    self.sub_stmt(e)?;
                }
                Ok(())
            }
            Stmt::While(c, b, _) | Stmt::DoWhile(b, c, _) => {
                self.expr(c)?;
                self.sub_stmt(b)
            }
            Stmt::For(init, test, update, body, sp) => {
                self.push_block();
                match init {
                    Some(ForInit::Var(k, d)) => {
                        for (n, e) in d {
                            if let Some(e) = e {
                                self.expr(e)?;
                            }
                            if *k != VarKind::Var {
                                self.declare(n, *k, *sp)?;
                            }
                            self.reference(n);
                        }
                    }
                    Some(ForInit::Expr(e)) => self.expr(e)?,
                    None => {}
                }
                if let Some(t) = test {
                    self.expr(t)?;
                }
                if let Some(u) = update {
                    self.expr(u)?;
                }
                self.sub_stmt(body)?;
                self.pop_block();
                Ok(())
            }
            Stmt::ForOf(k, n, e, b, sp) | Stmt::ForIn(k, n, e, b, sp) => {
                self.expr(e)?;
                self.push_block();
                if *k != VarKind::Var {
                    self.declare(n, *k, *sp)?;
                }
                self.reference(n);
                self.sub_stmt(b)?;
                self.pop_block();
                Ok(())
            }
            Stmt::Try(b, h, f, sp) => {
                self.scoped_block(b)?;
                if let Some((p, hb)) = h {
                    self.push_block();
                    if let Some(p) = p {
                        self.declare(p, VarKind::Let, *sp)?;
                    }
                    self.block(hb)?;
                    self.pop_block();
                }
                if let Some(fb) = f {
                    self.scoped_block(fb)?;
                }
                Ok(())
            }
            Stmt::Block(b, _) => self.scoped_block(b),
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Empty(_) => Ok(()),
        }
    }

    fn sub_stmt(&mut self, s: &Stmt) -> Result<(), SyntaxError> {
        self.push_block();
        let r = match s {
            Stmt::Block(b, _) => self.block(b),
            other => self.stmt(other),
        };
        self.pop_block();
        r
    }

    pub fn function(&mut self, f: &Function) -> Result<(), SyntaxError> {
        self.fns.push(FnScope {
            key: f.span.start,
            blocks: vec![HashMap::new()],
            is_arrow: f.is_arrow,
        });
        if !f.is_arrow {
            self.declare("this", VarKind::Var, f.span)?;
        }
        for p in &f.params {
            self.declare(p, VarKind::Var, f.span)?;
        }
        let mut vars = Vec::new();
        collect_vars(&f.body, &mut vars);
        for v in vars {
            self.declare(&v, VarKind::Var, f.span)?;
        }
        self.push_block();
        let r = self.block(&f.body);
        self.pop_block();
        self.fns.pop();
        r
    }

    fn class(&mut self, c: &Class) -> Result<(), SyntaxError> {
        if let Some(e) = &c.extends {
            self.expr(e)?;
        }
        self.push_block();
        self.declare("%super", VarKind::Const, c.span)?;
        if let Some(k) = &c.constructor {
            self.function(k)?;
        }
        if c.extends.is_some() && c.constructor.is_none() {
            // the synthesized default constructor calls super(...)
            self.res
                .captured
                .entry(self.fns.last().unwrap().key)
                .or_default()
                .insert("%super".into());
        }
        for (_, m, _) in &c.methods {
            self.function(m)?;
        }
        self.pop_block();
        Ok(())
    }

    fn expr(&mut self, e: &Expr) -> Result<(), SyntaxError> {
        match e {
            Expr::Ident(n, _) => {
                self.reference(n);
                Ok(())
            }
            Expr::This(_) => {
                let _ = self.fns.last().unwrap().is_arrow;
                self.reference("this");
                Ok(())
            }
            Expr::SuperCall(args, _) => {
                self.reference("%super");
                self.reference("this");
                for a in args {
                    self.expr(a)?;
                }
                Ok(())
            }
            Expr::SuperMember(_, _) => {
                self.reference("%super");
                self.reference("this");
                Ok(())
            }
            Expr::Num(..) | Expr::Str(..) | Expr::Bool(..) | Expr::Null(_) | Expr::Undefined(_) => Ok(()),
            Expr::Template(_, es, _) | Expr::Array(es, _) | Expr::Seq(es, _) => {
                for x in es {
                    self.expr(x)?;
                }
                Ok(())
            }
            Expr::Object(props, _) => {
                for p in props {
                    match p {
                        Prop::KeyValue(k, v) => {
                            if let PropKey::Computed(k) = k {
                                self.expr(k)?;
                            }
                            self.expr(v)?;
                        }
                        Prop::Shorthand(n) => self.reference(n),
                        Prop::Method(k, f) => {
                            if let PropKey::Computed(k) = k {
                                self.expr(k)?;
                            }
                            self.function(f)?;
                        }
                        Prop::Spread(e) => self.expr(e)?,
                    }
                }
                Ok(())
            }
            Expr::Function(f) => {
                // A named function expression can refer to itself.
                if let (Some(n), false) = (&f.name, f.is_arrow) {
                    self.push_block();
                    self.declare(n, VarKind::Const, f.span)?;
                    self.function(f)?;
                    self.pop_block();
                    Ok(())
                } else {
                    self.function(f)
                }
            }
            Expr::Class(c) => self.class(c),
            Expr::Unary(_, x, _) | Expr::Spread(x, _) | Expr::Member(x, _, _) => self.expr(x),
            Expr::Update { target, .. } => self.expr(target),
            Expr::Binary(_, l, r, _) | Expr::Logical(_, l, r, _) | Expr::Assign(_, l, r, _) | Expr::Index(l, r, _) => {
                self.expr(l)?;
                self.expr(r)
            }
            Expr::Cond(a, b, c, _) => {
                self.expr(a)?;
                self.expr(b)?;
                self.expr(c)
            }
            Expr::Call(c, args, _) | Expr::New(c, args, _) => {
                self.expr(c)?;
                for a in args {
                    self.expr(a)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn captured_variables() {
        let src = "function outer(a, b) { let c = 1; let d = 2; return () => a + c + this.x; }";
        let p = parse(src).unwrap();
        let r = resolve(&p).unwrap();
        let key = 0;
        assert!(r.is_captured(key, "a"));
        assert!(r.is_captured(key, "c"));
        assert!(r.is_captured(key, "this"));
        assert!(!r.is_captured(key, "b"));
        assert!(!r.is_captured(key, "d"));
    }

    #[test]
    fn globals_are_not_captured() {
        let p = parse("let g = 1; function f() { return g; }").unwrap();
        let r = resolve(&p).unwrap();
        assert!(!r.is_captured(SCRIPT_KEY, "g"));
        let p = parse("{ let h = 1; function f() { return h; } }").unwrap();
        let r = resolve(&p).unwrap();
        assert!(r.is_captured(SCRIPT_KEY, "h"));
    }

    #[test]
    fn duplicate_declarations() {
        let e = resolve(&parse("let a = 1; let a = 2;").unwrap()).unwrap_err();
        assert_eq!(e.message, "Identifier 'a' has already been declared");
        assert!(resolve(&parse("var a = 1; var a = 2;").unwrap()).is_ok());
        assert!(resolve(&parse("let a = 1; { let a = 2; }").unwrap()).is_ok());
        assert!(resolve(&parse("const a = 1; var a;").unwrap()).is_err());
    }
}

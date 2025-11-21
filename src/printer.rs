//! `kiln fmt`: pretty-prints an AST back to JavaScript source.
//!
//! The printer inserts the minimal parentheses required by operator
//! precedence, so `parse(print(ast)) == ast` (checked by round-trip tests).

use crate::ast::*;
use crate::value::num_to_string;

pub fn print_program(p: &Program) -> String {
    let mut pr = Printer {
        out: String::new(),
        indent: 0,
    };
    for s in &p.body {
        pr.stmt(s);
    }
    pr.out
}

pub fn print_expr(e: &Expr) -> String {
    let mut pr = Printer {
        out: String::new(),
        indent: 0,
    };
    pr.expr(e, 0);
    pr.out
}

struct Printer {
    out: String,
    indent: usize,
}

fn bin_prec(op: &str) -> u8 {
    match op {
        "??" => 4,
        "||" => 5,
        "&&" => 6,
        "|" => 7,
        "^" => 8,
        "&" => 9,
        "==" | "!=" | "===" | "!==" => 10,
        "<" | ">" | "<=" | ">=" | "instanceof" | "in" => 11,
        "<<" | ">>" | ">>>" => 12,
        "+" | "-" => 13,
        "*" | "/" | "%" => 14,
        "**" => 15,
        _ => unreachable!("binary op {op}"),
    }
}

const PREC_UNARY: u8 = 16;
const PREC_POSTFIX: u8 = 17;
const PREC_CALL: u8 = 18;

fn prec(e: &Expr) -> u8 {
    match e {
        Expr::Seq(..) => 0,
        Expr::Assign(..) | Expr::Spread(..) => 2,
        Expr::Function(f) if f.is_arrow => 2,
        Expr::Cond(..) => 3,
        Expr::Binary(op, ..) | Expr::Logical(op, ..) => bin_prec(op),
        Expr::Unary(..) => PREC_UNARY,
        Expr::Update { prefix: true, .. } => PREC_UNARY,
        Expr::Update { prefix: false, .. } => PREC_POSTFIX,
        Expr::Call(..) | Expr::New(..) | Expr::Member(..) | Expr::Index(..) | Expr::SuperCall(..) | Expr::SuperMember(..) => PREC_CALL,
        _ => 19,
    }
}

fn has_call_head(e: &Expr) -> bool {
    match e {
        Expr::Call(..) | Expr::New(..) => true,
        Expr::Member(o, _, _) | Expr::Index(o, _, _) => has_call_head(o),
        _ => false,
    }
}

pub fn quote_str(s: &str, q: char) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push(q);
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            '\0' => out.push_str("\\0"),
            c if c == q => {
                out.push('\\');
                out.push(c)
            }
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(q);
    out
}

impl Printer {
    fn nl(&mut self) {
        self.out.push('\n');
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
    }
    fn w(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn block(&mut self, body: &[Stmt]) {
        self.w("{");
        self.indent += 1;
        for s in body {
            self.nl();
            self.stmt_inner(s);
        }
        self.indent -= 1;
        if !body.is_empty() {
            self.nl();
        }
        self.w("}");
    }

    fn stmt(&mut self, s: &Stmt) {
        self.stmt_inner(s);
        self.out.push('\n');
    }

    fn decls(&mut self, kind: VarKind, decls: &[(String, Option<Expr>)]) {
        self.w(kind.as_str());
        self.w(" ");
        for (i, (n, init)) in decls.iter().enumerate() {
            if i > 0 {
                self.w(", ");
            }
            self.w(n);
            if let Some(e) = init {
                self.w(" = ");
                self.expr(e, 2);
            }
        }
    }

    /// Body of if/while/for: blocks print inline, other statements on the same line.
    fn body(&mut self, s: &Stmt) {
        match s {
            Stmt::Block(b, _) => {
                self.w(" ");
                self.block(b)
            }
            _ => {
                self.indent += 1;
                self.nl();
                self.stmt_inner(s);
                self.indent -= 1;
            }
        }
    }

    fn stmt_inner(&mut self, s: &Stmt) {
        match s {
            Stmt::Expr(e, _) => {
                let mut sub = Printer {
                    out: String::new(),
                    indent: self.indent,
                };
                sub.expr(e, 0);
                let text = sub.out;
                if text.starts_with('{') || text.starts_with("function") || text.starts_with("class") || text.starts_with("let [") {
                    self.w("(");
                    self.w(&text);
                    self.w(")");
                } else {
                    self.w(&text);
                }
                self.w(";");
            }
            Stmt::Var(k, d, _) => {
                self.decls(*k, d);
                self.w(";");
            }
            Stmt::Function(f) => self.function(f, true),
            Stmt::Class(c) => self.class(c),
            Stmt::Return(e, _) => {
                self.w("return");
                if let Some(e) = e {
                    self.w(" ");
                    self.expr(e, 0);
                }
                self.w(";");
            }
            Stmt::If(c, t, e, _) => {
                self.w("if (");
                self.expr(c, 0);
                self.w(")");
                self.body(t);
                if let Some(e) = e {
                    if matches!(**t, Stmt::Block(..)) {
                        self.w(" ");
                    } else {
                        self.nl();
                    }
                    self.w("else");
                    if matches!(**e, Stmt::If(..)) {
                        self.w(" ");
                        self.stmt_inner(e);
                    } else {
                        self.body(e);
                    }
                }
            }
            Stmt::While(c, b, _) => {
                self.w("while (");
                self.expr(c, 0);
                self.w(")");
                self.body(b);
            }
            Stmt::DoWhile(b, c, _) => {
                self.w("do");
                self.body(b);
                if matches!(**b, Stmt::Block(..)) {
                    self.w(" ");
                } else {
                    self.nl();
                }
                self.w("while (");
                self.expr(c, 0);
                self.w(");");
            }
            Stmt::For(init, test, update, b, _) => {
                self.w("for (");
                match init {
                    Some(ForInit::Var(k, d)) => self.decls(*k, d),
                    Some(ForInit::Expr(e)) => self.expr(e, 0),
                    None => {}
                }
                self.w(";");
                if let Some(t) = test {
                    self.w(" ");
                    self.expr(t, 0);
                }
                self.w(";");
                if let Some(u) = update {
                    self.w(" ");
                    self.expr(u, 0);
                }
                self.w(")");
                self.body(b);
            }
            Stmt::ForOf(k, n, e, b, _) | Stmt::ForIn(k, n, e, b, _) => {
                let word = if matches!(s, Stmt::ForOf(..)) { "of" } else { "in" };
                self.w(&format!("for ({} {n} {word} ", k.as_str()));
                self.expr(e, 2);
                self.w(")");
                self.body(b);
            }
            Stmt::Break(_) => self.w("break;"),
            Stmt::Continue(_) => self.w("continue;"),
            Stmt::Throw(e, _) => {
                self.w("throw ");
                self.expr(e, 0);
                self.w(";");
            }
            Stmt::Try(b, h, f, _) => {
                self.w("try ");
                self.block(b);
                if let Some((p, hb)) = h {
                    self.w(" catch ");
                    if let Some(p) = p {
                        self.w(&format!("({p}) "));
                    }
                    self.block(hb);
                }
                if let Some(fb) = f {
                    self.w(" finally ");
                    self.block(fb);
                }
            }
            Stmt::Block(b, _) => self.block(b),
            Stmt::Empty(_) => self.w(";"),
        }
    }

    fn params(&mut self, ps: &[String]) {
        self.w("(");
        self.w(&ps.join(", "));
        self.w(")");
    }

    fn function(&mut self, f: &Function, with_keyword: bool) {
        if f.is_arrow {
            self.params(&f.params);
            self.w(" => ");
            if f.expr_body {
                let Some(Stmt::Return(Some(e), _)) = f.body.first() else {
                    unreachable!()
                };
                if matches!(e, Expr::Object(..)) {
                    self.w("(");
                    self.expr(e, 2);
                    self.w(")");
                } else {
                    self.expr(e, 2);
                }
            } else {
                self.block(&f.body);
            }
            return;
        }
        if with_keyword {
            self.w("function");
            if let Some(n) = &f.name {
                self.w(" ");
                self.w(n);
            }
        } else if let Some(n) = &f.name {
            self.w(n);
        }
        self.params(&f.params);
        self.w(" ");
        self.block(&f.body);
    }

    fn class(&mut self, c: &Class) {
        self.w("class");
        if let Some(n) = &c.name {
            self.w(" ");
            self.w(n);
        }
        if let Some(e) = &c.extends {
            self.w(" extends ");
            self.expr(e, PREC_CALL);
        }
        self.w(" {");
        self.indent += 1;
        if let Some(k) = &c.constructor {
            self.nl();
            self.function(k, false);
        }
        for (_, m, is_static) in &c.methods {
            self.nl();
            if *is_static {
                self.w("static ");
            }
            self.function(m, false);
        }
        self.indent -= 1;
        if c.constructor.is_some() || !c.methods.is_empty() {
            self.nl();
        }
        self.w("}");
    }

    fn args(&mut self, args: &[Expr]) {
        self.w("(");
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                self.w(", ");
            }
            self.expr(a, 2);
        }
        self.w(")");
    }

    fn prop_key(&mut self, k: &PropKey) {
        match k {
            PropKey::Ident(s) => self.w(s),
            PropKey::Str(s) => self.w(&quote_str(s, '"')),
            PropKey::Num(n) => self.w(&num_to_string(*n)),
            PropKey::Computed(e) => {
                self.w("[");
                self.expr(e, 2);
                self.w("]");
            }
        }
    }

    pub fn expr(&mut self, e: &Expr, min: u8) {
        let p = prec(e);
        if p < min {
            self.w("(");
            self.expr(e, 0);
            self.w(")");
            return;
        }
        match e {
            Expr::Num(n, _) => self.w(&num_to_string(*n)),
            Expr::Str(s, _) => self.w(&quote_str(s, '"')),
            Expr::Template(q, es, _) => {
                self.w("`");
                for (i, part) in q.iter().enumerate() {
                    let mut esc = String::new();
                    let chars: Vec<char> = part.chars().collect();
                    for (j, c) in chars.iter().enumerate() {
                        match c {
                            '`' => esc.push_str("\\`"),
                            '\\' => esc.push_str("\\\\"),
                            '$' if chars.get(j + 1) == Some(&'{') => esc.push_str("\\$"),
                            '\r' => esc.push_str("\\r"),
                            c => esc.push(*c),
                        }
                    }
                    self.w(&esc);
                    if let Some(x) = es.get(i) {
                        self.w("${");
                        self.expr(x, 0);
                        self.w("}");
                    }
                }
                self.w("`");
            }
            Expr::Bool(b, _) => self.w(if *b { "true" } else { "false" }),
            Expr::Null(_) => self.w("null"),
            Expr::Undefined(_) => self.w("undefined"),
            Expr::Ident(n, _) => self.w(n),
            Expr::This(_) => self.w("this"),
            Expr::Array(items, _) => {
                self.w("[");
                for (i, a) in items.iter().enumerate() {
                    if i > 0 {
                        self.w(", ");
                    }
                    self.expr(a, 2);
                }
                self.w("]");
            }
            Expr::Object(props, _) => {
                if props.is_empty() {
                    self.w("{}");
                    return;
                }
                self.w("{ ");
                for (i, p) in props.iter().enumerate() {
                    if i > 0 {
                        self.w(", ");
                    }
                    match p {
                        Prop::KeyValue(k, v) => {
                            self.prop_key(k);
                            self.w(": ");
                            self.expr(v, 2);
                        }
                        Prop::Shorthand(n) => self.w(n),
                        Prop::Method(k, f) => {
                            self.prop_key(k);
                            self.params(&f.params);
                            self.w(" ");
                            self.block(&f.body);
                        }
                        Prop::Spread(e) => {
                            self.w("...");
                            self.expr(e, 2);
                        }
                    }
                }
                self.w(" }");
            }
            Expr::Function(f) => self.function(f, true),
            Expr::Class(c) => self.class(c),
            Expr::Unary(op, x, _) => {
                self.w(op);
                let mut sub = Printer {
                    out: String::new(),
                    indent: self.indent,
                };
                sub.expr(x, PREC_UNARY);
                let needs_space = op.chars().all(|c| c.is_alphabetic())
                    || ((*op == "-" || *op == "+") && (sub.out.starts_with('-') || sub.out.starts_with('+')));
                if needs_space {
                    self.w(" ");
                }
                self.w(&sub.out);
            }
            Expr::Update { op, prefix, target, .. } => {
                if *prefix {
                    self.w(op);
                    self.expr(target, PREC_CALL);
                } else {
                    self.expr(target, PREC_CALL);
                    self.w(op);
                }
            }
            Expr::Binary(op, l, r, _) | Expr::Logical(op, l, r, _) => {
                let p = bin_prec(op);
                let (lmin, rmin) = if *op == "**" { (PREC_POSTFIX, p) } else { (p, p + 1) };
                self.expr(l, lmin);
                self.w(" ");
                self.w(op);
                self.w(" ");
                self.expr(r, rmin);
            }
            Expr::Assign(op, t, v, _) => {
                self.expr(t, PREC_CALL);
                self.w(" ");
                self.w(op);
                self.w(" ");
                self.expr(v, 2);
            }
            Expr::Cond(c, t, f, _) => {
                self.expr(c, 4);
                self.w(" ? ");
                self.expr(t, 2);
                self.w(" : ");
                self.expr(f, 2);
            }
            Expr::Call(c, a, _) => {
                self.expr(c, PREC_CALL);
                self.args(a);
            }
            Expr::New(c, a, _) => {
                self.w("new ");
                if has_call_head(c) || prec(c) < PREC_CALL {
                    self.w("(");
                    self.expr(c, 0);
                    self.w(")");
                } else {
                    self.expr(c, PREC_CALL);
                }
                self.args(a);
            }
            Expr::Member(o, n, _) => {
                if matches!(**o, Expr::Num(..)) {
                    self.w("(");
                    self.expr(o, 0);
                    self.w(")");
                } else {
                    self.expr(o, PREC_CALL);
                }
                self.w(".");
                self.w(n);
            }
            Expr::Index(o, i, _) => {
                self.expr(o, PREC_CALL);
                self.w("[");
                self.expr(i, 0);
                self.w("]");
            }
            Expr::SuperCall(a, _) => {
                self.w("super");
                self.args(a);
            }
            Expr::SuperMember(n, _) => {
                self.w("super.");
                self.w(n);
            }
            Expr::Spread(x, _) => {
                self.w("...");
                self.expr(x, 2);
            }
            Expr::Seq(xs, _) => {
                for (i, x) in xs.iter().enumerate() {
                    if i > 0 {
                        self.w(", ");
                    }
                    self.expr(x, 2);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn roundtrip(src: &str) {
        let a = parse(src).unwrap_or_else(|e| panic!("{}", e.render(src, "in")));
        let printed = print_program(&a);
        let b = parse(&printed).unwrap_or_else(|e| panic!("reparse failed: {}\n{printed}", e.render(&printed, "out")));
        assert_eq!(a, b, "AST mismatch after printing:\n{printed}");
        // printing is a fixpoint
        assert_eq!(printed, print_program(&b));
    }

    #[test]
    fn roundtrips() {
        for src in [
            "let x = (1 + 2) * 3 - -4 - (-5) ** 2;",
            "a = b ? c : d ? e : f; (a, b);",
            "const f = (a, b) => ({ a, b }); let g = x => y => x + y;",
            "function fib(n) { if (n < 2) return n; return fib(n - 1) + fib(n - 2); }",
            "for (let i = 0, j = 10; i < j; i++, j--) { if (i % 2 === 0) continue; else break; }",
            "class A extends B { constructor(x) { super(x); this.x = x; } get() { return super.get() + this.x; } static make() { return new A(1); } }",
            "try { throw new Error(\"boom\"); } catch (e) { console.log(e.message); } finally { done(); }",
            "let s = `a${b + `c${d}`}\\`e\\${f}`; let o = { 'k-1': 1, 2: [1, 2, ...xs], [k]: v, m(a) { return a; } };",
            "(function () { return 1; })(); new (f())(); new a.b.C(1).d; (1).toString(); x = typeof y === \"string\";",
            "a - (b - c); a - b - c; (a ** b) ** c; a ** b ** c; !(a && b) || c ?? d; -(-x); +(+x); - +x;",
            "do x++; while (x < 10); while (true) { break; } for (const v of [1, 2]) console.log(v); for (let k in o) { }",
            "x += 1; x **= 2; x &&= y; a.b.c = d[e][f] = g;",
        ] {
            roundtrip(src);
        }
    }
}

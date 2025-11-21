//! Recursive-descent statement parser with a Pratt (precedence-climbing)
//! expression parser.

use crate::ast::*;
use crate::lexer::{Lexer, Span, SyntaxError, Tok, Token};

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    in_function: bool,
    loop_depth: u32,
}

type PResult<T> = Result<T, SyntaxError>;

/// Parse a whole program.
pub fn parse(src: &str) -> PResult<Program> {
    let toks = Lexer::new(src).tokenize()?;
    let mut p = Parser {
        toks,
        pos: 0,
        in_function: false,
        loop_depth: 0,
    };
    let mut body = Vec::new();
    while !p.at_eof() {
        body.push(p.statement()?);
    }
    Ok(Program { body })
}

/// Parse a single expression (used for template substitutions and tests).
pub fn parse_expr_at(src: &str, base: u32) -> PResult<Expr> {
    let toks = Lexer::with_base(src, base).tokenize()?;
    let mut p = Parser {
        toks,
        pos: 0,
        in_function: false,
        loop_depth: 0,
    };
    let e = p.expression()?;
    if !p.at_eof() {
        return Err(p.unexpected());
    }
    Ok(e)
}

fn binary_prec(op: &str) -> Option<(u8, bool)> {
    // (precedence, right-associative)
    Some(match op {
        "??" => (1, false),
        "||" => (2, false),
        "&&" => (3, false),
        "|" => (4, false),
        "^" => (5, false),
        "&" => (6, false),
        "==" | "!=" | "===" | "!==" => (7, false),
        "<" | ">" | "<=" | ">=" | "instanceof" | "in" => (8, false),
        "<<" | ">>" | ">>>" => (9, false),
        "+" | "-" => (10, false),
        "*" | "/" | "%" => (11, false),
        "**" => (12, true),
        _ => return None,
    })
}

const ASSIGN_OPS: &[&str] = &[
    "=", "+=", "-=", "*=", "/=", "%=", "**=", "<<=", ">>=", ">>>=", "&=", "|=", "^=", "&&=", "||=", "??=",
];

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }
    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }
    fn span(&self) -> Span {
        self.toks[self.pos].span
    }
    fn prev_span(&self) -> Span {
        self.toks[self.pos.saturating_sub(1)].span
    }
    fn at_eof(&self) -> bool {
        matches!(self.peek(), Tok::Eof)
    }
    fn advance(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn is_punct(&self, p: &str) -> bool {
        matches!(self.peek(), Tok::Punct(q) if *q == p)
    }
    fn is_kw(&self, k: &str) -> bool {
        matches!(self.peek(), Tok::Keyword(q) if *q == k)
    }
    fn eat_punct(&mut self, p: &str) -> bool {
        if self.is_punct(p) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, k: &str) -> bool {
        if self.is_kw(k) {
            self.advance();
            true
        } else {
            false
        }
    }
    fn unexpected(&self) -> SyntaxError {
        SyntaxError::new(format!("unexpected {}", self.peek()), self.span())
    }
    fn expect_punct(&mut self, p: &str) -> PResult<()> {
        if self.eat_punct(p) {
            Ok(())
        } else {
            Err(SyntaxError::new(format!("expected '{p}' but found {}", self.peek()), self.span()))
        }
    }
    fn ident(&mut self) -> PResult<String> {
        match self.peek().clone() {
            Tok::Ident(s) => {
                self.advance();
                Ok(s)
            }
            // contextual keywords usable as identifiers
            Tok::Keyword(k @ ("of" | "undefined")) if k == "of" => {
                self.advance();
                Ok(k.to_string())
            }
            other => Err(SyntaxError::new(format!("expected identifier but found {other}"), self.span())),
        }
    }
    /// Property names may be any identifier including reserved words.
    fn prop_name(&mut self) -> PResult<String> {
        match self.peek().clone() {
            Tok::Ident(s) => {
                self.advance();
                Ok(s)
            }
            Tok::Keyword(k) => {
                self.advance();
                Ok(k.to_string())
            }
            other => Err(SyntaxError::new(format!("expected property name but found {other}"), self.span())),
        }
    }
    fn semi(&mut self) -> PResult<()> {
        if self.eat_punct(";") || self.is_punct("}") || self.at_eof() || self.toks[self.pos].nl_before {
            Ok(())
        } else {
            Err(SyntaxError::new(format!("expected ';' but found {}", self.peek()), self.span()))
        }
    }

    // ---------------------------------------------------------------- statements

    fn block_body(&mut self) -> PResult<Vec<Stmt>> {
        self.expect_punct("{")?;
        let mut v = Vec::new();
        while !self.is_punct("}") {
            if self.at_eof() {
                return Err(SyntaxError::new("unexpected end of input, expected '}'", self.span()));
            }
            v.push(self.statement()?);
        }
        self.advance();
        Ok(v)
    }

    fn var_decls(&mut self, kind: VarKind, allow_no_init: bool) -> PResult<Vec<(String, Option<Expr>)>> {
        let mut decls = Vec::new();
        loop {
            let name_span = self.span();
            let name = self.ident()?;
            let init = if self.eat_punct("=") {
                Some(self.assign()?)
            } else {
                if kind == VarKind::Const && !allow_no_init {
                    return Err(SyntaxError::new("missing initializer in const declaration", name_span));
                }
                None
            };
            decls.push((name, init));
            if !self.eat_punct(",") {
                break;
            }
        }
        Ok(decls)
    }

    fn var_kind(&self) -> Option<VarKind> {
        match self.peek() {
            Tok::Keyword("let") => Some(VarKind::Let),
            Tok::Keyword("const") => Some(VarKind::Const),
            Tok::Keyword("var") => Some(VarKind::Var),
            _ => None,
        }
    }

    fn loop_body(&mut self) -> PResult<Stmt> {
        self.loop_depth += 1;
        let b = self.statement();
        self.loop_depth -= 1;
        b
    }

    pub fn statement(&mut self) -> PResult<Stmt> {
        let start = self.span();
        if let Some(kind) = self.var_kind() {
            self.advance();
            let decls = self.var_decls(kind, false)?;
            self.semi()?;
            return Ok(Stmt::Var(kind, decls, start.to(self.prev_span())));
        }
        match self.peek().clone() {
            Tok::Punct("{") => {
                let b = self.block_body()?;
                Ok(Stmt::Block(b, start.to(self.prev_span())))
            }
            Tok::Punct(";") => {
                self.advance();
                Ok(Stmt::Empty(start))
            }
            Tok::Keyword("function") => {
                let f = self.function(false)?;
                if f.name.is_none() {
                    return Err(SyntaxError::new("function statement requires a name", start));
                }
                Ok(Stmt::Function(f))
            }
            Tok::Keyword("class") => {
                let c = self.class()?;
                if c.name.is_none() {
                    return Err(SyntaxError::new("class statement requires a name", start));
                }
                Ok(Stmt::Class(c))
            }
            Tok::Keyword("return") => {
                if !self.in_function {
                    return Err(SyntaxError::new("illegal return statement outside of function", start));
                }
                self.advance();
                let val = if self.is_punct(";") || self.is_punct("}") || self.at_eof() || self.toks[self.pos].nl_before {
                    None
                } else {
                    Some(self.expression()?)
                };
                self.semi()?;
                Ok(Stmt::Return(val, start.to(self.prev_span())))
            }
            Tok::Keyword("if") => {
                self.advance();
                self.expect_punct("(")?;
                let c = self.expression()?;
                self.expect_punct(")")?;
                let t = self.statement()?;
                let e = if self.eat_kw("else") {
                    Some(Box::new(self.statement()?))
                } else {
                    None
                };
                Ok(Stmt::If(c, Box::new(t), e, start.to(self.prev_span())))
            }
            Tok::Keyword("while") => {
                self.advance();
                self.expect_punct("(")?;
                let c = self.expression()?;
                self.expect_punct(")")?;
                let b = self.loop_body()?;
                Ok(Stmt::While(c, Box::new(b), start.to(self.prev_span())))
            }
            Tok::Keyword("do") => {
                self.advance();
                let b = self.loop_body()?;
                if !self.eat_kw("while") {
                    return Err(SyntaxError::new(format!("expected 'while' but found {}", self.peek()), self.span()));
                }
                self.expect_punct("(")?;
                let c = self.expression()?;
                self.expect_punct(")")?;
                self.eat_punct(";");
                Ok(Stmt::DoWhile(Box::new(b), c, start.to(self.prev_span())))
            }
            Tok::Keyword("for") => self.for_stmt(),
            Tok::Keyword("break") | Tok::Keyword("continue") => {
                let is_break = self.is_kw("break");
                self.advance();
                if self.loop_depth == 0 {
                    let w = if is_break { "break" } else { "continue" };
                    return Err(SyntaxError::new(format!("illegal {w} statement outside of a loop"), start));
                }
                self.semi()?;
                Ok(if is_break { Stmt::Break(start) } else { Stmt::Continue(start) })
            }
            Tok::Keyword("throw") => {
                self.advance();
                if self.toks[self.pos].nl_before {
                    return Err(SyntaxError::new("illegal newline after throw", self.span()));
                }
                let e = self.expression()?;
                self.semi()?;
                Ok(Stmt::Throw(e, start.to(self.prev_span())))
            }
            Tok::Keyword("try") => {
                self.advance();
                let body = self.block_body()?;
                let mut handler = None;
                let mut finalizer = None;
                if self.eat_kw("catch") {
                    let param = if self.eat_punct("(") {
                        let n = self.ident()?;
                        self.expect_punct(")")?;
                        Some(n)
                    } else {
                        None
                    };
                    handler = Some((param, self.block_body()?));
                }
                if self.eat_kw("finally") {
                    finalizer = Some(self.block_body()?);
                }
                if handler.is_none() && finalizer.is_none() {
                    return Err(SyntaxError::new("missing catch or finally after try", self.span()));
                }
                Ok(Stmt::Try(body, handler, finalizer, start.to(self.prev_span())))
            }
            _ => {
                let e = self.expression()?;
                self.semi()?;
                Ok(Stmt::Expr(e, start.to(self.prev_span())))
            }
        }
    }

    fn for_stmt(&mut self) -> PResult<Stmt> {
        let start = self.span();
        self.advance();
        self.expect_punct("(")?;
        let mut init = None;
        if let Some(kind) = self.var_kind() {
            // for (let x of e) / for (let x in e)
            if matches!(self.peek_at(1), Tok::Ident(_)) && matches!(self.peek_at(2), Tok::Keyword("of") | Tok::Keyword("in")) {
                self.advance();
                let name = self.ident()?;
                let is_of = self.eat_kw("of");
                if !is_of {
                    self.advance();
                }
                let iter = self.assign()?;
                self.expect_punct(")")?;
                let body = self.loop_body()?;
                let sp = start.to(self.prev_span());
                return Ok(if is_of {
                    Stmt::ForOf(kind, name, iter, Box::new(body), sp)
                } else {
                    Stmt::ForIn(kind, name, iter, Box::new(body), sp)
                });
            }
            self.advance();
            init = Some(ForInit::Var(kind, self.var_decls(kind, false)?));
        } else if !self.is_punct(";") {
            init = Some(ForInit::Expr(self.expression()?));
        }
        self.expect_punct(";")?;
        let test = if self.is_punct(";") { None } else { Some(self.expression()?) };
        self.expect_punct(";")?;
        let update = if self.is_punct(")") { None } else { Some(self.expression()?) };
        self.expect_punct(")")?;
        let body = self.loop_body()?;
        Ok(Stmt::For(init, test, update, Box::new(body), start.to(self.prev_span())))
    }

    fn params(&mut self) -> PResult<Vec<String>> {
        self.expect_punct("(")?;
        let mut ps = Vec::new();
        while !self.is_punct(")") {
            let sp = self.span();
            let n = self.ident()?;
            if ps.contains(&n) {
                return Err(SyntaxError::new(format!("duplicate parameter name '{n}'"), sp));
            }
            ps.push(n);
            if !self.eat_punct(",") {
                break;
            }
        }
        self.expect_punct(")")?;
        Ok(ps)
    }

    fn function_body(&mut self) -> PResult<Vec<Stmt>> {
        let (sf, sl) = (self.in_function, self.loop_depth);
        self.in_function = true;
        self.loop_depth = 0;
        let b = self.block_body();
        self.in_function = sf;
        self.loop_depth = sl;
        b
    }

    fn function(&mut self, _expr: bool) -> PResult<Function> {
        let start = self.span();
        self.advance(); // function
        let name = if matches!(self.peek(), Tok::Ident(_)) {
            Some(self.ident()?)
        } else {
            None
        };
        let params = self.params()?;
        let body = self.function_body()?;
        Ok(Function {
            name,
            params,
            body,
            is_arrow: false,
            expr_body: false,
            span: start.to(self.prev_span()),
        })
    }

    fn method(&mut self, name: String, start: Span) -> PResult<Function> {
        let params = self.params()?;
        let body = self.function_body()?;
        Ok(Function {
            name: Some(name),
            params,
            body,
            is_arrow: false,
            expr_body: false,
            span: start.to(self.prev_span()),
        })
    }

    fn class(&mut self) -> PResult<Class> {
        let start = self.span();
        self.advance(); // class
        let name = if matches!(self.peek(), Tok::Ident(_)) {
            Some(self.ident()?)
        } else {
            None
        };
        let extends = if self.eat_kw("extends") {
            Some(Box::new(self.unary_postfix()?))
        } else {
            None
        };
        self.expect_punct("{")?;
        let mut constructor = None;
        let mut methods = Vec::new();
        while !self.eat_punct("}") {
            if self.eat_punct(";") {
                continue;
            }
            let ms = self.span();
            let is_static = matches!(self.peek(), Tok::Ident(s) if s == "static") && !matches!(self.peek_at(1), Tok::Punct("("));
            if is_static {
                self.advance();
            }
            let mname = self.prop_name()?;
            let f = self.method(mname.clone(), ms)?;
            if mname == "constructor" && !is_static {
                if constructor.is_some() {
                    return Err(SyntaxError::new("a class may only have one constructor", ms));
                }
                constructor = Some(f);
            } else {
                methods.push((mname, f, is_static));
            }
        }
        Ok(Class {
            name,
            extends,
            constructor,
            methods,
            span: start.to(self.prev_span()),
        })
    }

    // ---------------------------------------------------------------- expressions

    pub fn expression(&mut self) -> PResult<Expr> {
        let start = self.span();
        let e = self.assign()?;
        if self.is_punct(",") {
            let mut v = vec![e];
            while self.eat_punct(",") {
                v.push(self.assign()?);
            }
            return Ok(Expr::Seq(v, start.to(self.prev_span())));
        }
        Ok(e)
    }

    /// Is the parser at the start of an arrow function?
    fn at_arrow(&self) -> bool {
        match self.peek() {
            Tok::Ident(_) => matches!(self.peek_at(1), Tok::Punct("=>")),
            Tok::Punct("(") => {
                let mut depth = 0;
                let mut i = self.pos;
                while i < self.toks.len() {
                    match &self.toks[i].tok {
                        Tok::Punct("(") => depth += 1,
                        Tok::Punct(")") => {
                            depth -= 1;
                            if depth == 0 {
                                return matches!(self.toks.get(i + 1).map(|t| &t.tok), Some(Tok::Punct("=>")))
                                    && !self.toks[i + 1].nl_before;
                            }
                        }
                        Tok::Eof => return false,
                        _ => {}
                    }
                    i += 1;
                }
                false
            }
            _ => false,
        }
    }

    fn arrow(&mut self) -> PResult<Expr> {
        let start = self.span();
        let params = if matches!(self.peek(), Tok::Ident(_)) {
            vec![self.ident()?]
        } else {
            self.params()?
        };
        self.expect_punct("=>")?;
        let (body, expr_body) = if self.is_punct("{") {
            (self.function_body()?, false)
        } else {
            let (sf, sl) = (self.in_function, self.loop_depth);
            self.in_function = true;
            self.loop_depth = 0;
            let e = self.assign();
            self.in_function = sf;
            self.loop_depth = sl;
            let e = e?;
            let sp = e.span();
            (vec![Stmt::Return(Some(e), sp)], true)
        };
        Ok(Expr::Function(Box::new(Function {
            name: None,
            params,
            body,
            is_arrow: true,
            expr_body,
            span: start.to(self.prev_span()),
        })))
    }

    pub fn assign(&mut self) -> PResult<Expr> {
        if self.at_arrow() {
            return self.arrow();
        }
        let start = self.span();
        let lhs = self.conditional()?;
        if let Tok::Punct(op) = self.peek().clone() {
            if ASSIGN_OPS.contains(&op) {
                if !matches!(lhs, Expr::Ident(..) | Expr::Member(..) | Expr::Index(..)) {
                    return Err(SyntaxError::new("invalid left-hand side in assignment", lhs.span()));
                }
                self.advance();
                let rhs = self.assign()?;
                return Ok(Expr::Assign(op, Box::new(lhs), Box::new(rhs), start.to(self.prev_span())));
            }
        }
        Ok(lhs)
    }

    fn conditional(&mut self) -> PResult<Expr> {
        let start = self.span();
        let c = self.binary(0)?;
        if self.eat_punct("?") {
            let t = self.assign()?;
            self.expect_punct(":")?;
            let e = self.assign()?;
            return Ok(Expr::Cond(Box::new(c), Box::new(t), Box::new(e), start.to(self.prev_span())));
        }
        Ok(c)
    }

    fn binary(&mut self, min_prec: u8) -> PResult<Expr> {
        let start = self.span();
        let mut lhs = self.unary_postfix()?;
        loop {
            let op: &'static str = match self.peek() {
                Tok::Punct(p) => p,
                Tok::Keyword(k @ ("instanceof" | "in")) => k,
                _ => break,
            };
            let Some((prec, right)) = binary_prec(op) else { break };
            if prec < min_prec {
                break;
            }
            let prev_is_paren = matches!(self.toks[self.pos.saturating_sub(1)].tok, Tok::Punct(")"));
            if op == "**" && matches!(lhs, Expr::Unary(..)) && !prev_is_paren {
                return Err(SyntaxError::new("unary operator before '**' must be parenthesized", self.span()));
            }
            self.advance();
            let rhs = self.binary(if right { prec } else { prec + 1 })?;
            let sp = start.to(self.prev_span());
            lhs = if matches!(op, "&&" | "||" | "??") {
                Expr::Logical(op, Box::new(lhs), Box::new(rhs), sp)
            } else {
                Expr::Binary(op, Box::new(lhs), Box::new(rhs), sp)
            };
        }
        Ok(lhs)
    }

    fn unary_postfix(&mut self) -> PResult<Expr> {
        let start = self.span();
        let op: Option<&'static str> = match self.peek() {
            Tok::Punct(p @ ("!" | "-" | "+" | "~")) => Some(p),
            Tok::Keyword(k @ ("typeof" | "void")) => Some(k),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let e = self.unary_postfix()?;
            return Ok(Expr::Unary(op, Box::new(e), start.to(self.prev_span())));
        }
        if let Tok::Punct(op @ ("++" | "--")) = self.peek().clone() {
            self.advance();
            let e = self.unary_postfix()?;
            if !matches!(e, Expr::Ident(..) | Expr::Member(..) | Expr::Index(..)) {
                return Err(SyntaxError::new("invalid operand for prefix update", e.span()));
            }
            return Ok(Expr::Update {
                op,
                prefix: true,
                target: Box::new(e),
                span: start.to(self.prev_span()),
            });
        }
        let e = self.call_member()?;
        if let Tok::Punct(op @ ("++" | "--")) = self.peek().clone() {
            if !self.toks[self.pos].nl_before {
                if !matches!(e, Expr::Ident(..) | Expr::Member(..) | Expr::Index(..)) {
                    return Err(SyntaxError::new("invalid operand for postfix update", e.span()));
                }
                self.advance();
                return Ok(Expr::Update {
                    op,
                    prefix: false,
                    target: Box::new(e),
                    span: start.to(self.prev_span()),
                });
            }
        }
        Ok(e)
    }

    fn args(&mut self) -> PResult<Vec<Expr>> {
        self.expect_punct("(")?;
        let mut v = Vec::new();
        while !self.is_punct(")") {
            let s = self.span();
            if self.eat_punct("...") {
                let e = self.assign()?;
                v.push(Expr::Spread(Box::new(e), s.to(self.prev_span())));
            } else {
                v.push(self.assign()?);
            }
            if !self.eat_punct(",") {
                break;
            }
        }
        self.expect_punct(")")?;
        Ok(v)
    }

    fn call_member(&mut self) -> PResult<Expr> {
        let start = self.span();
        let mut e = if self.eat_kw("new") {
            let callee = self.new_callee()?;
            let args = if self.is_punct("(") { self.args()? } else { Vec::new() };
            Expr::New(Box::new(callee), args, start.to(self.prev_span()))
        } else {
            self.primary()?
        };
        loop {
            if self.eat_punct(".") {
                let n = self.prop_name()?;
                e = Expr::Member(Box::new(e), n, start.to(self.prev_span()));
            } else if self.eat_punct("[") {
                let i = self.expression()?;
                self.expect_punct("]")?;
                e = Expr::Index(Box::new(e), Box::new(i), start.to(self.prev_span()));
            } else if self.is_punct("(") {
                let a = self.args()?;
                e = Expr::Call(Box::new(e), a, start.to(self.prev_span()));
            } else if let Tok::Template(..) = self.peek() {
                return Err(SyntaxError::new("tagged templates are not supported", self.span()));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn new_callee(&mut self) -> PResult<Expr> {
        let start = self.span();
        let mut e = if self.is_kw("new") { self.call_member()? } else { self.primary()? };
        loop {
            if self.eat_punct(".") {
                let n = self.prop_name()?;
                e = Expr::Member(Box::new(e), n, start.to(self.prev_span()));
            } else if self.eat_punct("[") {
                let i = self.expression()?;
                self.expect_punct("]")?;
                e = Expr::Index(Box::new(e), Box::new(i), start.to(self.prev_span()));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let start = self.span();
        let t = self.advance();
        Ok(match t.tok {
            Tok::Num(n) => Expr::Num(n, start),
            Tok::Str(s) => Expr::Str(s, start),
            Tok::Template(quasis, srcs) => {
                let mut exprs = Vec::new();
                for (src, base) in srcs {
                    exprs.push(parse_expr_at(&src, base)?);
                }
                Expr::Template(quasis, exprs, start)
            }
            Tok::Ident(s) => Expr::Ident(s, start),
            Tok::Keyword("of") => Expr::Ident("of".into(), start),
            Tok::Keyword("true") => Expr::Bool(true, start),
            Tok::Keyword("false") => Expr::Bool(false, start),
            Tok::Keyword("null") => Expr::Null(start),
            Tok::Keyword("undefined") => Expr::Undefined(start),
            Tok::Keyword("this") => Expr::This(start),
            Tok::Keyword("super") => {
                if self.is_punct("(") {
                    let a = self.args()?;
                    Expr::SuperCall(a, start.to(self.prev_span()))
                } else if self.eat_punct(".") {
                    let n = self.prop_name()?;
                    Expr::SuperMember(n, start.to(self.prev_span()))
                } else {
                    return Err(SyntaxError::new("'super' keyword unexpected here", start));
                }
            }
            Tok::Keyword("function") => {
                self.pos -= 1;
                Expr::Function(Box::new(self.function(true)?))
            }
            Tok::Keyword("class") => {
                self.pos -= 1;
                Expr::Class(Box::new(self.class()?))
            }
            Tok::Punct("(") => {
                let e = self.expression()?;
                self.expect_punct(")")?;
                e
            }
            Tok::Punct("[") => {
                let mut v = Vec::new();
                while !self.is_punct("]") {
                    let s = self.span();
                    if self.is_punct(",") {
                        return Err(SyntaxError::new("array holes are not supported", s));
                    }
                    if self.eat_punct("...") {
                        let e = self.assign()?;
                        v.push(Expr::Spread(Box::new(e), s.to(self.prev_span())));
                    } else {
                        v.push(self.assign()?);
                    }
                    if !self.eat_punct(",") {
                        break;
                    }
                }
                self.expect_punct("]")?;
                Expr::Array(v, start.to(self.prev_span()))
            }
            Tok::Punct("{") => {
                let mut props = Vec::new();
                while !self.is_punct("}") {
                    let ps = self.span();
                    if self.eat_punct("...") {
                        props.push(Prop::Spread(self.assign()?));
                    } else {
                        let key = match self.peek().clone() {
                            Tok::Str(s) => {
                                self.advance();
                                PropKey::Str(s)
                            }
                            Tok::Num(n) => {
                                self.advance();
                                PropKey::Num(n)
                            }
                            Tok::Punct("[") => {
                                self.advance();
                                let e = self.assign()?;
                                self.expect_punct("]")?;
                                PropKey::Computed(e)
                            }
                            _ => PropKey::Ident(self.prop_name()?),
                        };
                        if self.eat_punct(":") {
                            props.push(Prop::KeyValue(key, self.assign()?));
                        } else if self.is_punct("(") {
                            let name = match &key {
                                PropKey::Ident(s) | PropKey::Str(s) => s.clone(),
                                _ => String::new(),
                            };
                            let f = self.method(name, ps)?;
                            props.push(Prop::Method(key, f));
                        } else {
                            match key {
                                PropKey::Ident(n) if !crate::lexer::KEYWORDS.contains(&n.as_str()) => props.push(Prop::Shorthand(n)),
                                _ => return Err(SyntaxError::new(format!("expected ':' but found {}", self.peek()), self.span())),
                            }
                        }
                    }
                    if !self.eat_punct(",") {
                        break;
                    }
                }
                self.expect_punct("}")?;
                Expr::Object(props, start.to(self.prev_span()))
            }
            other => {
                self.pos -= 1;
                return Err(SyntaxError::new(format!("unexpected {other}"), start));
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Program {
        parse(s).unwrap_or_else(|e| panic!("{}", e.render(s, "t")))
    }

    fn expr(s: &str) -> Expr {
        parse_expr_at(s, 0).unwrap()
    }

    #[test]
    fn precedence() {
        match expr("1 + 2 * 3") {
            Expr::Binary("+", l, r, _) => {
                assert!(matches!(*l, Expr::Num(..)));
                assert!(matches!(*r, Expr::Binary("*", ..)));
            }
            e => panic!("{e:?}"),
        }
        // ** is right associative
        match expr("2 ** 3 ** 2") {
            Expr::Binary("**", _, r, _) => assert!(matches!(*r, Expr::Binary("**", ..))),
            e => panic!("{e:?}"),
        }
        // - is left associative
        match expr("a - b - c") {
            Expr::Binary("-", l, _, _) => assert!(matches!(*l, Expr::Binary("-", ..))),
            e => panic!("{e:?}"),
        }
        assert!(matches!(expr("a || b && c"), Expr::Logical("||", _, _, _)));
        assert!(matches!(expr("a = b = c"), Expr::Assign("=", _, r, _) if matches!(*r, Expr::Assign(..))));
        assert!(matches!(expr("a ? b : c ? d : e"), Expr::Cond(_, _, e, _) if matches!(*e, Expr::Cond(..))));
    }

    #[test]
    fn arrows() {
        assert!(matches!(expr("x => x + 1"), Expr::Function(f) if f.is_arrow && f.expr_body));
        assert!(matches!(expr("(a, b) => { return a }"), Expr::Function(f) if f.params.len() == 2 && !f.expr_body));
        assert!(matches!(expr("() => 1"), Expr::Function(_)));
        assert!(matches!(expr("(a + b) * c"), Expr::Binary("*", ..)));
    }

    #[test]
    fn statements() {
        let prog = p("let a = 1, b; const c = 2; for (let i = 0; i < 3; i++) { if (i) continue; else break; }\n\
                      for (const x of xs) {} while (a) a--; do { } while (0)\n\
                      try { throw 1 } catch (e) { } finally { }\n\
                      function f(a, b) { return a } class A extends B { constructor() { super(1) } m() { return super.m() } static s() {} }");
        assert_eq!(prog.body.len(), 9);
        assert!(matches!(prog.body[8], Stmt::Class(ref c) if c.methods.len() == 2 && c.constructor.is_some()));
    }

    #[test]
    fn asi_and_return() {
        let prog = p("function f() { return\n 1 }\nlet x = 1\nx++\n");
        match &prog.body[0] {
            Stmt::Function(f) => assert!(matches!(f.body[0], Stmt::Return(None, _))),
            _ => panic!(),
        }
        assert_eq!(prog.body.len(), 3);
    }

    #[test]
    fn objects_and_members() {
        let e = expr("{ a: 1, 'b': 2, 3: c, [k]: 4, d, m() { return 1 }, ...o }.a.b[0](1, ...r)");
        assert!(matches!(e, Expr::Call(..)));
        assert!(matches!(expr("new A.B(1).c"), Expr::Member(o, _, _) if matches!(*o, Expr::New(..))));
        assert!(matches!(expr("new A"), Expr::New(_, a, _) if a.is_empty()));
    }

    fn err(s: &str) -> String {
        let e = parse(s).unwrap_err();
        let (l, c) = crate::lexer::line_col(s, e.span.start);
        format!("{l}:{c} {}", e.message)
    }

    #[test]
    fn error_messages() {
        assert_eq!(err("let x = ;"), "1:9 unexpected ';'");
        assert_eq!(err("foo(1, 2\nbar"), "2:1 expected ')' but found identifier 'bar'");
        assert_eq!(err("if (x) {\n  y = 1;\n"), "3:1 unexpected end of input, expected '}'");
        assert_eq!(err("1 = 2"), "1:1 invalid left-hand side in assignment");
        assert_eq!(err("const z;"), "1:7 missing initializer in const declaration");
        assert_eq!(err("return 5"), "1:1 illegal return statement outside of function");
        assert_eq!(err("break;"), "1:1 illegal break statement outside of a loop");
        assert_eq!(err("function f(a, a) {}"), "1:15 duplicate parameter name 'a'");
        assert_eq!(err("let a = 1 let b = 2"), "1:11 expected ';' but found keyword 'let'");
        assert_eq!(err("x = `a${1 +}`"), "1:12 unexpected end of input");
    }
}

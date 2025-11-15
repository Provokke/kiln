//! AST for the kiln JavaScript subset. Spans compare equal regardless of
//! value (see [`crate::lexer::Span`]), so `==` on AST is structural equality.

use crate::lexer::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    Var,
    Let,
    Const,
}

impl VarKind {
    pub fn as_str(self) -> &'static str {
        match self {
            VarKind::Var => "var",
            VarKind::Let => "let",
            VarKind::Const => "const",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    /// Arrow function with an expression body is stored as `[Return(expr)]` with this flag set.
    pub is_arrow: bool,
    pub expr_body: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Class {
    pub name: Option<String>,
    pub extends: Option<Box<Expr>>,
    pub constructor: Option<Function>,
    pub methods: Vec<(String, Function, bool)>, // (name, fn, is_static)
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropKey {
    Ident(String),
    Str(String),
    Num(f64),
    Computed(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Prop {
    KeyValue(PropKey, Expr),
    Shorthand(String),
    Method(PropKey, Function),
    Spread(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64, Span),
    Str(String, Span),
    Template(Vec<String>, Vec<Expr>, Span),
    Bool(bool, Span),
    Null(Span),
    Undefined(Span),
    Ident(String, Span),
    This(Span),
    Array(Vec<Expr>, Span),
    Object(Vec<Prop>, Span),
    Function(Box<Function>),
    Class(Box<Class>),
    Unary(&'static str, Box<Expr>, Span),
    Update {
        op: &'static str,
        prefix: bool,
        target: Box<Expr>,
        span: Span,
    },
    Binary(&'static str, Box<Expr>, Box<Expr>, Span),
    Logical(&'static str, Box<Expr>, Box<Expr>, Span),
    Assign(&'static str, Box<Expr>, Box<Expr>, Span),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    New(Box<Expr>, Vec<Expr>, Span),
    Member(Box<Expr>, String, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    SuperCall(Vec<Expr>, Span),
    SuperMember(String, Span),
    Spread(Box<Expr>, Span),
    Seq(Vec<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Num(_, s)
            | Expr::Str(_, s)
            | Expr::Template(_, _, s)
            | Expr::Bool(_, s)
            | Expr::Null(s)
            | Expr::Undefined(s)
            | Expr::Ident(_, s)
            | Expr::This(s)
            | Expr::Array(_, s)
            | Expr::Object(_, s)
            | Expr::Unary(_, _, s)
            | Expr::Update { span: s, .. }
            | Expr::Binary(_, _, _, s)
            | Expr::Logical(_, _, _, s)
            | Expr::Assign(_, _, _, s)
            | Expr::Cond(_, _, _, s)
            | Expr::Call(_, _, s)
            | Expr::New(_, _, s)
            | Expr::Member(_, _, s)
            | Expr::Index(_, _, s)
            | Expr::SuperCall(_, s)
            | Expr::SuperMember(_, s)
            | Expr::Spread(_, s)
            | Expr::Seq(_, s) => *s,
            Expr::Function(f) => f.span,
            Expr::Class(c) => c.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ForInit {
    Var(VarKind, Vec<(String, Option<Expr>)>),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Expr(Expr, Span),
    Var(VarKind, Vec<(String, Option<Expr>)>, Span),
    Function(Function),
    Class(Class),
    Return(Option<Expr>, Span),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>, Span),
    While(Expr, Box<Stmt>, Span),
    DoWhile(Box<Stmt>, Expr, Span),
    For(Option<ForInit>, Option<Expr>, Option<Expr>, Box<Stmt>, Span),
    ForOf(VarKind, String, Expr, Box<Stmt>, Span),
    ForIn(VarKind, String, Expr, Box<Stmt>, Span),
    Break(Span),
    Continue(Span),
    Throw(Expr, Span),
    Try(Vec<Stmt>, Option<(Option<String>, Vec<Stmt>)>, Option<Vec<Stmt>>, Span),
    Block(Vec<Stmt>, Span),
    Empty(Span),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Expr(_, s)
            | Stmt::Var(_, _, s)
            | Stmt::Return(_, s)
            | Stmt::If(_, _, _, s)
            | Stmt::While(_, _, s)
            | Stmt::DoWhile(_, _, s)
            | Stmt::For(_, _, _, _, s)
            | Stmt::ForOf(_, _, _, _, s)
            | Stmt::ForIn(_, _, _, _, s)
            | Stmt::Break(s)
            | Stmt::Continue(s)
            | Stmt::Throw(_, s)
            | Stmt::Try(_, _, _, s)
            | Stmt::Block(_, s)
            | Stmt::Empty(s) => *s,
            Stmt::Function(f) => f.span,
            Stmt::Class(c) => c.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub body: Vec<Stmt>,
}

//! Stack-machine bytecode.

use crate::value::Value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Const(u32),
    Undef,
    Null,
    True,
    False,
    Pop,
    Dup,
    /// Duplicate the top two values (a b -> a b a b).
    Dup2,
    /// Local variable `i` (slot 0 is `this`, then parameters, then other locals).
    GetLocal(u32),
    SetLocal(u32),
    /// Local holding a cell (captured variable).
    GetLocalCell(u32),
    SetLocalCell(u32),
    /// Replace local `i` with a fresh cell containing its current value.
    BoxLocal(u32),
    /// Replace the cell in local `i` with a fresh cell holding the same value (per-iteration `let`).
    RefreshCell(u32),
    GetUpval(u32),
    SetUpval(u32),
    GetGlobal(u32),
    SetGlobal(u32),
    /// `typeof name` for a possibly undeclared global.
    TypeOfGlobal(u32),
    /// GetProp(name constant, inline-cache index)
    GetProp(u32, u32),
    /// obj val -> val
    SetProp(u32, u32),
    /// obj -> method obj (callee and receiver for a following Call)
    GetMethod(u32, u32),
    GetElem,
    SetElem,
    /// Same as GetElem but leaves the receiver: obj key -> fn obj
    GetElemMethod,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    StrictEq,
    StrictNe,
    InstanceOf,
    In,
    Neg,
    ToNumber,
    Not,
    BitNot,
    TypeOf,
    Jump(u32),
    JumpIfFalse(u32),
    JumpIfTrue(u32),
    /// Jump if top is falsy without popping it, else pop.
    JumpIfFalseKeep(u32),
    JumpIfTrueKeep(u32),
    JumpIfNotNullishKeep(u32),
    /// Stack: callee this arg0..argN-1
    Call(u32),
    /// Like Call but permitted on class constructors (used for `super(...)`).
    CallSuper(u32),
    /// Stack: callee this argsArray
    CallSpread,
    /// Stack: callee arg0..argN-1
    New(u32),
    NewSpread,
    Return,
    Closure(u32),
    Array(u32),
    /// array value -> array (push)
    ArrayPush,
    /// array iterable -> array (append all elements)
    ArrayExtend,
    NewObject,
    /// obj value -> obj
    InitProp(u32, u32),
    /// obj key value -> obj
    InitElem,
    /// obj source -> obj
    ObjectSpread,
    Throw,
    /// Throws a TypeError about assignment to a constant (name constant).
    ThrowConstAssign(u32),
    PushHandler(u32),
    PopHandler,
    /// Keys of an object as an array (for-in).
    Keys,
    /// Set the `prototype` of the closure on top to a fresh object whose proto is the value below it.
    /// Stack: parentProto ctor -> ctor
    MakeClass,
    /// ctor fn -> ctor (defines fn as non-static method on ctor.prototype, by name constant)
    DefineMethod(u32),
}

/// How a closure captures a variable from the enclosing function.
#[derive(Clone, Copy, Debug)]
pub enum Capture {
    /// The enclosing function's local slot (which holds a cell).
    Local(u32),
    /// The enclosing function's own upvalue.
    Upval(u32),
}

#[derive(Default, Debug, Clone, Copy)]
pub struct JitCounters {
    pub calls: u32,
}

pub struct FuncProto {
    pub name: String,
    pub nparams: u32,
    pub nlocals: u32,
    pub code: Vec<Op>,
    pub consts: Vec<Value>,
    pub funcs: Vec<Rc<FuncProto>>,
    pub captures: Vec<Capture>,
    /// Source line of each op (for error messages).
    pub lines: Vec<u32>,
    pub is_class: bool,
    pub is_arrow: bool,
    pub uses_var: bool,
    pub max_stack: u32,
    pub source: Rc<str>,
    /// Call counter for tier-up.
    pub calls: Cell<u32>,
    pub jit: RefCell<crate::jit::JitSlot>,
}

impl std::fmt::Debug for FuncProto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<function {}>", self.name)
    }
}

impl FuncProto {
    /// Human readable disassembly (used by `kiln run --disasm`).
    pub fn disassemble(&self) -> String {
        let mut s = format!(
            "== {} (params={}, locals={}, max_stack={}) ==\n",
            if self.name.is_empty() { "<anon>" } else { &self.name },
            self.nparams,
            self.nlocals,
            self.max_stack
        );
        for (i, op) in self.code.iter().enumerate() {
            let extra = match op {
                Op::Const(c)
                | Op::GetGlobal(c)
                | Op::SetGlobal(c)
                | Op::GetProp(c, _)
                | Op::SetProp(c, _)
                | Op::GetMethod(c, _)
                | Op::InitProp(c, _) => {
                    format!("    ; {:?}", self.consts[*c as usize])
                }
                _ => String::new(),
            };
            s += &format!("{i:5}  {op:?}{extra}\n");
        }
        for f in &self.funcs {
            s += &f.disassemble();
        }
        s
    }
}

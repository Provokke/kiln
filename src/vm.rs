//! Bytecode interpreter with inline caches, exception handling and GC safepoints.

use crate::bytecode::{Capture, FuncProto, Op};
use crate::compiler::IcSite;
use crate::heap::{GcRef, Heap, NativeFn, Obj, ObjKind};
use crate::value::{num_to_string, str_to_num, strict_eq, to_int32, Str, Value};
use std::collections::HashMap;
use std::rc::Rc;

pub const MAX_FRAMES: usize = 9000;

pub struct Frame {
    pub closure: GcRef,
    pub func: Rc<FuncProto>,
    pub pc: usize,
    pub base: usize,
    pub construct: bool,
}

struct Handler {
    frame: usize,
    sp: usize,
    pc: usize,
}

#[derive(Clone, Copy, Debug)]
struct IcEntry {
    shape: u32,
    slot: u32,
    /// For loads from the prototype: the holder's shape. For transitioning stores: the new shape.
    aux: u32,
    kind: u8, // 0 = own, 1 = proto load, 2 = store-add transition
}

#[derive(Default, Clone, Debug)]
pub struct InlineCache {
    entries: Vec<IcEntry>,
    pub hits: u64,
    pub misses: u64,
    pub megamorphic: bool,
}

impl InlineCache {
    pub fn state(&self) -> &'static str {
        if self.megamorphic {
            "megamorphic"
        } else {
            match self.entries.len() {
                0 => "uninitialized",
                1 => "monomorphic",
                _ => "polymorphic",
            }
        }
    }
    fn add(&mut self, e: IcEntry) {
        if self.megamorphic {
            return;
        }
        if self.entries.len() >= 4 {
            self.megamorphic = true;
            self.entries.clear();
        } else {
            self.entries.push(e);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub jit: bool,
    pub jit_threshold: u32,
    pub gc_stress: bool,
    pub echo_stdout: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            jit: true,
            jit_threshold: 500,
            gc_stress: false,
            echo_stdout: false,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct JitStats {
    pub compiled: u64,
    pub compile_failures: u64,
    pub entries: u64,
    pub deopts: u64,
}

pub struct Vm {
    pub heap: Heap,
    pub stack: Vec<Value>,
    pub frames: Vec<Frame>,
    handlers: Vec<Handler>,
    pub globals: HashMap<Str, Value>,
    pub ics: Vec<InlineCache>,
    pub ic_sites: Vec<IcSite>,
    pub extra_roots: Vec<Value>,
    pub array_proto: Value,
    pub string_proto: Value,
    pub number_proto: Value,
    pub function_proto: Value,
    pub error_proto: Value,
    pub out: String,
    pub opts: Options,
    pub jit_stats: JitStats,
    /// Nesting depth of native (Rust) re-entry into JS, bounded to protect the native stack.
    pub native_depth: usize,
}

pub type VResult<T> = Result<T, Value>;

pub fn is_array_index(s: &str) -> Option<usize> {
    if s.is_empty() || s.len() > 10 || (s.len() > 1 && s.starts_with('0')) || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

impl Vm {
    pub fn new(opts: Options) -> Vm {
        let mut heap = Heap::default();
        heap.stress = opts.gc_stress;
        let mut vm = Vm {
            heap,
            stack: Vec::with_capacity(1024),
            frames: Vec::new(),
            handlers: Vec::new(),
            globals: HashMap::new(),
            ics: Vec::new(),
            ic_sites: Vec::new(),
            extra_roots: Vec::new(),
            array_proto: Value::Undefined,
            string_proto: Value::Undefined,
            number_proto: Value::Undefined,
            function_proto: Value::Undefined,
            error_proto: Value::Undefined,
            out: String::new(),
            opts,
            jit_stats: JitStats::default(),
            native_depth: 0,
        };
        crate::builtins::install(&mut vm);
        vm
    }

    // ------------------------------------------------------------ allocation helpers

    pub fn alloc(&mut self, kind: ObjKind, proto: Option<GcRef>) -> GcRef {
        self.heap.alloc(Obj::new(kind, proto))
    }

    pub fn new_object(&mut self) -> GcRef {
        self.alloc(ObjKind::Plain, None)
    }

    pub fn new_array(&mut self, items: Vec<Value>) -> Value {
        let p = self.array_proto.as_obj();
        Value::Obj(self.alloc(ObjKind::Array(items), p))
    }

    pub fn new_native(&mut self, name: &'static str, f: NativeFn) -> Value {
        let p = self.function_proto.as_obj();
        Value::Obj(self.alloc(ObjKind::Native { f, name }, p))
    }

    pub fn make_error(&mut self, kind: &str, msg: &str) -> Value {
        let proto = match self.globals.get(kind).cloned() {
            Some(Value::Obj(ctor)) => match self.get_own(ctor, "prototype") {
                Some(Value::Obj(p)) => Some(p),
                _ => None,
            },
            _ => None,
        };
        let o = self.alloc(ObjKind::Plain, proto);
        self.set_own(o, &Rc::from("message"), Value::str(msg));
        Value::Obj(o)
    }

    pub fn type_error<T>(&mut self, msg: impl AsRef<str>) -> VResult<T> {
        Err(self.make_error("TypeError", msg.as_ref()))
    }

    // ------------------------------------------------------------ properties

    pub fn get_own(&self, r: GcRef, key: &str) -> Option<Value> {
        let o = self.heap.get(r);
        self.heap.shape_lookup(o.shape, key).map(|s| o.slots[s as usize].clone())
    }

    pub fn set_own(&mut self, r: GcRef, key: &Str, v: Value) {
        let shape = self.heap.get(r).shape;
        match self.heap.shape_lookup(shape, key) {
            Some(s) => self.heap.get_mut(r).slots[s as usize] = v,
            None => {
                let ns = self.heap.shape_add(shape, key);
                let o = self.heap.get_mut(r);
                o.shape = ns;
                o.slots.push(v);
            }
        }
    }

    /// Own enumerable keys in insertion order (array indices first for arrays).
    pub fn own_keys(&self, r: GcRef) -> Vec<Str> {
        let o = self.heap.get(r);
        let mut keys: Vec<Str> = Vec::new();
        if let ObjKind::Array(items) = &o.kind {
            keys.extend((0..items.len()).map(|i| Rc::from(i.to_string())));
        }
        keys.extend(self.prop_keys(r));
        keys
    }

    /// Named property keys in JS enumeration order: integer-like keys ascending, then insertion order.
    pub fn prop_keys(&self, r: GcRef) -> Vec<Str> {
        let o = self.heap.get(r);
        let all = &self.heap.shapes[o.shape as usize].keys;
        let mut idx: Vec<(usize, Str)> = all.iter().filter_map(|k| is_array_index(k).map(|i| (i, k.clone()))).collect();
        idx.sort_by_key(|(i, _)| *i);
        let mut out: Vec<Str> = idx.into_iter().map(|(_, k)| k).collect();
        out.extend(all.iter().filter(|k| is_array_index(k).is_none()).cloned());
        out
    }

    fn proto_of_primitive(&self, v: &Value) -> Option<GcRef> {
        match v {
            Value::Str(_) => self.string_proto.as_obj(),
            Value::Num(_) => self.number_proto.as_obj(),
            _ => None,
        }
    }

    /// Generic property load (with optional inline cache).
    pub fn get_prop(&mut self, obj: &Value, key: &Str, ic: Option<u32>) -> VResult<Value> {
        let r = match obj {
            Value::Obj(r) => *r,
            Value::Str(s) => {
                if &**key == "length" {
                    return Ok(Value::Num(s.encode_utf16().count() as f64));
                }
                if let Some(i) = is_array_index(key) {
                    return Ok(s.chars().nth(i).map(|c| Value::str(&c.to_string())).unwrap_or(Value::Undefined));
                }
                return Ok(self.lookup_chain(self.proto_of_primitive(obj), key));
            }
            Value::Undefined | Value::Null => {
                let what = if matches!(obj, Value::Null) { "null" } else { "undefined" };
                return self.type_error(format!("Cannot read properties of {what} (reading '{key}')"));
            }
            _ => return Ok(self.lookup_chain(self.proto_of_primitive(obj), key)),
        };
        // Arrays: length and indices
        if let ObjKind::Array(items) = &self.heap.get(r).kind {
            if &**key == "length" {
                return Ok(Value::Num(items.len() as f64));
            }
            if let Some(i) = is_array_index(key) {
                return Ok(items.get(i).cloned().unwrap_or(Value::Undefined));
            }
        }
        match &self.heap.get(r).kind {
            ObjKind::Closure { func, .. } if matches!(&**key, "name" | "length") => {
                if self.heap.shape_lookup(self.heap.get(r).shape, key).is_none() {
                    return Ok(if &**key == "name" {
                        Value::str(&func.name)
                    } else {
                        Value::Num(func.nparams as f64)
                    });
                }
            }
            ObjKind::Native { name, .. } if &**key == "name" => return Ok(Value::str(name)),
            _ => {}
        }
        if let Some(ic) = ic {
            let o = self.heap.get(r);
            let cache = &self.ics[ic as usize];
            for e in &cache.entries {
                if e.shape == o.shape {
                    if e.kind == 0 {
                        let v = o.slots[e.slot as usize].clone();
                        self.ics[ic as usize].hits += 1;
                        return Ok(v);
                    } else if e.kind == 1 {
                        if let Some(p) = o.proto {
                            let po = self.heap.get(p);
                            if po.shape == e.aux {
                                let v = po.slots[e.slot as usize].clone();
                                self.ics[ic as usize].hits += 1;
                                return Ok(v);
                            }
                        }
                    }
                }
            }
            self.ics[ic as usize].misses += 1;
            // slow path + cache fill
            let o = self.heap.get(r);
            if let Some(slot) = self.heap.shape_lookup(o.shape, key) {
                let shape = o.shape;
                let v = o.slots[slot as usize].clone();
                self.ics[ic as usize].add(IcEntry {
                    shape,
                    slot,
                    aux: 0,
                    kind: 0,
                });
                return Ok(v);
            }
            if let Some(p) = o.proto {
                let po = self.heap.get(p);
                if let Some(slot) = self.heap.shape_lookup(po.shape, key) {
                    let e = IcEntry {
                        shape: o.shape,
                        slot,
                        aux: po.shape,
                        kind: 1,
                    };
                    let v = po.slots[slot as usize].clone();
                    self.ics[ic as usize].add(e);
                    return Ok(v);
                }
            }
        }
        Ok(self.lookup_chain(Some(r), key))
    }

    fn lookup_chain(&self, mut cur: Option<GcRef>, key: &str) -> Value {
        let mut depth = 0;
        while let Some(r) = cur {
            let o = self.heap.get(r);
            if let Some(s) = self.heap.shape_lookup(o.shape, key) {
                return o.slots[s as usize].clone();
            }
            cur = o.proto;
            depth += 1;
            if depth > 1000 {
                break;
            }
        }
        Value::Undefined
    }

    pub fn set_prop(&mut self, obj: &Value, key: &Str, v: Value, ic: Option<u32>) -> VResult<()> {
        let r = match obj {
            Value::Obj(r) => *r,
            Value::Undefined | Value::Null => {
                let what = if matches!(obj, Value::Null) { "null" } else { "undefined" };
                return self.type_error(format!("Cannot set properties of {what} (setting '{key}')"));
            }
            _ => return Ok(()), // silently ignored on primitives (sloppy mode)
        };
        if let ObjKind::Array(_) = &self.heap.get(r).kind {
            if &**key == "length" {
                let n = match v {
                    Value::Num(n) if n >= 0.0 && n.fract() == 0.0 => n as usize,
                    _ => return Err(self.make_error("RangeError", "Invalid array length")),
                };
                if let ObjKind::Array(items) = &mut self.heap.get_mut(r).kind {
                    items.resize(n, Value::Undefined);
                }
                return Ok(());
            }
            if let Some(i) = is_array_index(key) {
                if let ObjKind::Array(items) = &mut self.heap.get_mut(r).kind {
                    if i >= items.len() {
                        if i > items.len() + 10_000_000 {
                            return Ok(());
                        }
                        items.resize(i + 1, Value::Undefined);
                    }
                    items[i] = v;
                }
                return Ok(());
            }
        }
        if let Some(ic) = ic {
            let shape = self.heap.get(r).shape;
            let hit = self.ics[ic as usize]
                .entries
                .iter()
                .find(|e| e.shape == shape && e.kind != 1)
                .copied();
            if let Some(e) = hit {
                self.ics[ic as usize].hits += 1;
                let o = self.heap.get_mut(r);
                if e.kind == 0 {
                    o.slots[e.slot as usize] = v;
                } else {
                    o.shape = e.aux;
                    o.slots.push(v);
                }
                return Ok(());
            }
            self.ics[ic as usize].misses += 1;
            match self.heap.shape_lookup(shape, key) {
                Some(slot) => {
                    self.ics[ic as usize].add(IcEntry {
                        shape,
                        slot,
                        aux: 0,
                        kind: 0,
                    });
                    self.heap.get_mut(r).slots[slot as usize] = v;
                }
                None => {
                    let ns = self.heap.shape_add(shape, key);
                    let slot = self.heap.get(r).slots.len() as u32;
                    self.ics[ic as usize].add(IcEntry {
                        shape,
                        slot,
                        aux: ns,
                        kind: 2,
                    });
                    let o = self.heap.get_mut(r);
                    o.shape = ns;
                    o.slots.push(v);
                }
            }
            return Ok(());
        }
        self.set_own(r, key, v);
        Ok(())
    }

    pub fn key_of(&mut self, k: &Value) -> Str {
        match k {
            Value::Str(s) => s.clone(),
            Value::Num(n) => Rc::from(num_to_string(*n)),
            other => {
                let s = self.to_string(other);
                Rc::from(s)
            }
        }
    }

    fn get_elem(&mut self, obj: &Value, key: &Value) -> VResult<Value> {
        if let (Value::Obj(r), Value::Num(n)) = (obj, key) {
            if let ObjKind::Array(items) = &self.heap.get(*r).kind {
                if *n >= 0.0 && n.fract() == 0.0 {
                    return Ok(items.get(*n as usize).cloned().unwrap_or(Value::Undefined));
                }
            }
        }
        let k = self.key_of(key);
        self.get_prop(obj, &k, None)
    }

    fn set_elem(&mut self, obj: &Value, key: &Value, v: Value) -> VResult<()> {
        if let (Value::Obj(r), Value::Num(n)) = (obj, key) {
            let n = *n;
            if let ObjKind::Array(items) = &mut self.heap.get_mut(*r).kind {
                if n >= 0.0 && n.fract() == 0.0 && (n as usize) < items.len() {
                    items[n as usize] = v;
                    return Ok(());
                }
            }
        }
        let k = self.key_of(key);
        self.set_prop(obj, &k, v, None)
    }

    // ------------------------------------------------------------ conversions

    pub fn to_number(&mut self, v: &Value) -> f64 {
        match v {
            Value::Undefined => f64::NAN,
            Value::Null => 0.0,
            Value::Bool(b) => *b as u8 as f64,
            Value::Num(n) => *n,
            Value::Str(s) => str_to_num(s),
            Value::Obj(r) => {
                if let ObjKind::Array(_) = self.heap.get(*r).kind {
                    let s = self.to_string(v);
                    str_to_num(&s)
                } else {
                    f64::NAN
                }
            }
        }
    }

    pub fn to_string(&mut self, v: &Value) -> String {
        match v {
            Value::Undefined => "undefined".into(),
            Value::Null => "null".into(),
            Value::Bool(b) => b.to_string(),
            Value::Num(n) => num_to_string(*n),
            Value::Str(s) => s.to_string(),
            Value::Obj(r) => {
                let r = *r;
                match &self.heap.get(r).kind {
                    ObjKind::Array(items) => {
                        let items = items.clone();
                        let parts: Vec<String> = items
                            .iter()
                            .map(|x| if x.is_nullish() { String::new() } else { self.to_string(x) })
                            .collect();
                        parts.join(",")
                    }
                    ObjKind::Closure { func, .. } => {
                        if func.is_class {
                            format!("class {} {{ }}", func.name)
                        } else {
                            format!("function {}() {{ [code] }}", func.name)
                        }
                    }
                    ObjKind::Native { name, .. } => format!("function {name}() {{ [native code] }}"),
                    _ => {
                        if let Some(f) = self.user_to_string(r) {
                            if let Ok(res) = self.call_value(&f, v.clone(), &[]) {
                                if !matches!(res, Value::Obj(_)) {
                                    return self.to_string(&res);
                                }
                            }
                        }
                        if self.is_error(r) {
                            let name = self.lookup_chain(Some(r), "name");
                            let msg = self.lookup_chain(Some(r), "message");
                            let name = self.to_string(&name);
                            let msg = self.to_string(&msg);
                            if msg.is_empty() {
                                name
                            } else {
                                format!("{name}: {msg}")
                            }
                        } else {
                            "[object Object]".into()
                        }
                    }
                }
            }
        }
    }

    pub fn is_error(&self, r: GcRef) -> bool {
        let Some(ep) = self.error_proto.as_obj() else { return false };
        let mut cur = self.heap.get(r).proto;
        while let Some(p) = cur {
            if p == ep {
                return true;
            }
            cur = self.heap.get(p).proto;
        }
        false
    }

    pub fn type_of(&self, v: &Value) -> &'static str {
        match v {
            Value::Undefined => "undefined",
            Value::Null => "object",
            Value::Bool(_) => "boolean",
            Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::Obj(r) => match self.heap.get(*r).kind {
                ObjKind::Closure { .. } | ObjKind::Native { .. } => "function",
                _ => "object",
            },
        }
    }

    pub fn loose_eq(&mut self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Undefined | Value::Null, Value::Undefined | Value::Null) => true,
            (Value::Undefined | Value::Null, _) | (_, Value::Undefined | Value::Null) => false,
            (Value::Num(_), Value::Str(_)) | (Value::Str(_), Value::Num(_)) | (Value::Bool(_), _) | (_, Value::Bool(_)) => {
                if let (Value::Obj(_), _) | (_, Value::Obj(_)) = (a, b) {
                    let x = self.primitive_of(a);
                    let y = self.primitive_of(b);
                    return self.loose_eq(&x, &y);
                }
                self.to_number(a) == self.to_number(b)
            }
            (Value::Obj(_), Value::Obj(_)) => strict_eq(a, b),
            (Value::Obj(_), _) | (_, Value::Obj(_)) => {
                let x = self.primitive_of(a);
                let y = self.primitive_of(b);
                self.loose_eq(&x, &y)
            }
            _ => strict_eq(a, b),
        }
    }

    fn primitive_of(&mut self, v: &Value) -> Value {
        match v {
            Value::Obj(_) => Value::str(&self.to_string(v)),
            other => other.clone(),
        }
    }

    /// A user-defined (bytecode) `toString` reachable from an ordinary object, if any.
    fn user_to_string(&self, r: GcRef) -> Option<Value> {
        if !matches!(self.heap.get(r).kind, ObjKind::Plain) {
            return None;
        }
        let f = self.lookup_chain(Some(r), "toString");
        match f {
            Value::Obj(fr) if matches!(self.heap.get(fr).kind, ObjKind::Closure { .. }) => Some(f),
            _ => None,
        }
    }

    fn add(&mut self, a: Value, b: Value) -> Value {
        if let (Value::Num(x), Value::Num(y)) = (&a, &b) {
            return Value::Num(x + y);
        }
        // Root both operands: converting one may run user code (toString) and trigger a GC.
        let base = self.extra_roots.len();
        self.extra_roots.push(a);
        self.extra_roots.push(b);
        let a = self.extra_roots[base].clone();
        let a = self.primitive_of(&a);
        let b = self.extra_roots[base + 1].clone();
        let b = self.primitive_of(&b);
        self.extra_roots.truncate(base);
        if matches!(a, Value::Str(_)) || matches!(b, Value::Str(_)) {
            let mut s = self.to_string(&a);
            s.push_str(&self.to_string(&b));
            Value::str(&s)
        } else {
            Value::Num(self.to_number(&a) + self.to_number(&b))
        }
    }

    fn compare(&mut self, a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
        let a = self.primitive_of(a);
        let b = self.primitive_of(b);
        if let (Value::Str(x), Value::Str(y)) = (&a, &b) {
            return Some(x.encode_utf16().cmp(y.encode_utf16()));
        }
        let x = self.to_number(&a);
        let y = self.to_number(&b);
        x.partial_cmp(&y)
    }

    fn instance_of(&mut self, v: &Value, ctor: &Value) -> VResult<bool> {
        let Value::Obj(c) = ctor else {
            return self.type_error("Right-hand side of 'instanceof' is not callable");
        };
        let Some(Value::Obj(proto)) = self.get_own(*c, "prototype") else {
            return Ok(false);
        };
        let Value::Obj(mut o) = v else { return Ok(false) };
        loop {
            match self.heap.get(o).proto {
                Some(p) if p == proto => return Ok(true),
                Some(p) => o = p,
                None => return Ok(false),
            }
        }
    }

    // ------------------------------------------------------------ GC

    pub fn collect(&mut self, force_major: bool) {
        let mut heap = std::mem::take(&mut self.heap);
        let major = force_major || heap.wants_major();
        {
            let mut roots = |visit: &mut dyn FnMut(&mut GcRef)| {
                for v in self
                    .stack
                    .iter_mut()
                    .chain(self.globals.values_mut())
                    .chain(self.extra_roots.iter_mut())
                {
                    if let Value::Obj(r) = v {
                        visit(r)
                    }
                }
                for f in self.frames.iter_mut() {
                    visit(&mut f.closure)
                }
                for v in [
                    &mut self.array_proto,
                    &mut self.string_proto,
                    &mut self.number_proto,
                    &mut self.function_proto,
                    &mut self.error_proto,
                ] {
                    if let Value::Obj(r) = v {
                        visit(r)
                    }
                }
            };
            if major {
                heap.major_gc(&mut roots);
            } else {
                heap.minor_gc(&mut roots);
            }
        }
        self.heap = heap;
    }

    // ------------------------------------------------------------ calls

    /// Run a compiled script.
    pub fn run_script(&mut self, proto: Rc<FuncProto>, sites: Vec<IcSite>) -> VResult<Value> {
        self.ics.resize(self.ics.len() + sites.len(), InlineCache::default());
        self.ic_sites.extend(sites);
        let fp = self.function_proto.as_obj();
        let clo = self.alloc(
            ObjKind::Closure {
                func: proto.clone(),
                cells: vec![],
            },
            fp,
        );
        let base = self.stack.len();
        self.stack.push(Value::Obj(clo));
        self.stack.push(Value::Undefined);
        for _ in 1..proto.nlocals {
            self.stack.push(Value::Undefined);
        }
        let depth = self.frames.len();
        self.frames.push(Frame {
            closure: clo,
            func: proto,
            pc: 0,
            base,
            construct: false,
        });
        self.run(depth)
    }

    /// Call any callable value from Rust (re-entrant).
    pub fn call_value(&mut self, f: &Value, this: Value, args: &[Value]) -> VResult<Value> {
        if self.native_depth > MAX_FRAMES {
            return Err(self.make_error("RangeError", "Maximum call stack size exceeded"));
        }
        let Value::Obj(r) = f else {
            let s = self.to_string(f);
            return self.type_error(format!("{s} is not a function"));
        };
        match &self.heap.get(*r).kind {
            ObjKind::Native { f: nf, .. } => {
                let nf = *nf;
                self.native_depth += 1;
                let res = nf(self, this, args);
                self.native_depth -= 1;
                res
            }
            ObjKind::Closure { func, .. } => {
                let func = func.clone();
                if func.is_class {
                    return self.type_error(format!("Class constructor {} cannot be invoked without 'new'", func.name));
                }
                if self.opts.jit {
                    if let Some(res) = crate::jit::try_call(self, &func, args) {
                        return res;
                    }
                }
                let base = self.stack.len();
                self.stack.push(f.clone());
                self.stack.push(this);
                self.stack.extend_from_slice(args);
                self.push_frame(*r, func, base, args.len(), false)?;
                let depth = self.frames.len() - 1;
                self.native_depth += 1;
                let res = self.run(depth);
                self.native_depth -= 1;
                res
            }
            _ => {
                let s = self.to_string(f);
                self.type_error(format!("{s} is not a function"))
            }
        }
    }

    fn push_frame(&mut self, closure: GcRef, func: Rc<FuncProto>, base: usize, argc: usize, construct: bool) -> VResult<()> {
        if self.frames.len() >= MAX_FRAMES {
            self.stack.truncate(base);
            return Err(self.make_error("RangeError", "Maximum call stack size exceeded"));
        }
        let np = func.nparams as usize;
        if argc > np {
            self.stack.truncate(base + 2 + np);
        }
        let want = base + 1 + func.nlocals as usize;
        self.stack.resize(want, Value::Undefined);
        self.frames.push(Frame {
            closure,
            func,
            pc: 0,
            base,
            construct,
        });
        Ok(())
    }

    fn closure_cell(&self, closure: GcRef, i: u32) -> GcRef {
        match &self.heap.get(closure).kind {
            ObjKind::Closure { cells, .. } => cells[i as usize],
            _ => unreachable!(),
        }
    }

    fn cell_get(&self, cell: GcRef) -> Value {
        match &self.heap.get(cell).kind {
            ObjKind::Cell(v) => v.clone(),
            _ => unreachable!("not a cell"),
        }
    }

    fn cell_set(&mut self, cell: GcRef, v: Value) {
        if let ObjKind::Cell(c) = &mut self.heap.get_mut(cell).kind {
            *c = v;
        }
    }

    fn new_cell(&mut self, v: Value) -> Value {
        Value::Obj(self.alloc(ObjKind::Cell(v), None))
    }

    /// Unwind to the nearest handler at or above `stop_depth`; Err if none.
    fn throw(&mut self, exc: Value, stop_depth: usize) -> VResult<()> {
        if let Some(h) = self.handlers.pop_if(|h| h.frame >= stop_depth) {
            self.frames.truncate(h.frame + 1);
            self.stack.truncate(h.sp);
            self.stack.push(exc);
            self.frames[h.frame].pc = h.pc;
            return Ok(());
        }
        let base = self.frames[stop_depth].base;
        self.frames.truncate(stop_depth);
        self.stack.truncate(base);
        Err(exc)
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("stack underflow")
    }

    fn construct_this(&mut self, ctor: GcRef) -> Value {
        let proto = match self.get_own(ctor, "prototype") {
            Some(Value::Obj(p)) => p,
            _ => {
                let p = self.new_object();
                self.set_own(p, &Rc::from("constructor"), Value::Obj(ctor));
                self.set_own(ctor, &Rc::from("prototype"), Value::Obj(p));
                p
            }
        };
        Value::Obj(self.alloc(ObjKind::Plain, Some(proto)))
    }

    /// Main interpreter loop. Runs until the frame at index `stop_depth` returns.
    pub fn run(&mut self, stop_depth: usize) -> VResult<Value> {
        'frame: loop {
            let fi = self.frames.len() - 1;
            let func = self.frames[fi].func.clone();
            let base = self.frames[fi].base;
            let lbase = base + 1;
            let mut pc = self.frames[fi].pc;
            macro_rules! throw {
                ($e:expr) => {{
                    let exc = $e;
                    self.frames[fi].pc = pc;
                    self.throw(exc, stop_depth)?;
                    continue 'frame;
                }};
            }
            macro_rules! tri {
                ($e:expr) => {
                    match $e {
                        Ok(v) => v,
                        Err(exc) => throw!(exc),
                    }
                };
            }
            macro_rules! num_binop {
                ($f:expr) => {{
                    let b = self.pop();
                    let a = self.pop();
                    let r = match (&a, &b) {
                        (Value::Num(x), Value::Num(y)) => $f(*x, *y),
                        _ => {
                            let x = self.to_number(&a);
                            let y = self.to_number(&b);
                            $f(x, y)
                        }
                    };
                    self.stack.push(Value::Num(r));
                }};
            }
            macro_rules! int_binop {
                ($f:expr) => {{
                    let b = self.pop();
                    let a = self.pop();
                    let x = self.to_number(&a);
                    let y = self.to_number(&b);
                    self.stack.push(Value::Num($f(to_int32(x), to_int32(y))));
                }};
            }
            macro_rules! cmp_op {
                ($pat:pat) => {{
                    let b = self.pop();
                    let a = self.pop();
                    let r = match (&a, &b) {
                        (Value::Num(x), Value::Num(y)) => x.partial_cmp(y),
                        _ => self.compare(&a, &b),
                    };
                    self.stack.push(Value::Bool(matches!(r, $pat)));
                }};
            }
            loop {
                if self.heap.gc_pending {
                    self.frames[fi].pc = pc;
                    self.collect(false);
                }
                let op = func.code[pc];
                pc += 1;
                match op {
                    Op::Const(c) => self.stack.push(func.consts[c as usize].clone()),
                    Op::Undef => self.stack.push(Value::Undefined),
                    Op::Null => self.stack.push(Value::Null),
                    Op::True => self.stack.push(Value::Bool(true)),
                    Op::False => self.stack.push(Value::Bool(false)),
                    Op::Pop => {
                        self.stack.pop();
                    }
                    Op::Dup => {
                        let v = self.stack.last().unwrap().clone();
                        self.stack.push(v);
                    }
                    Op::Dup2 => {
                        let n = self.stack.len();
                        let a = self.stack[n - 2].clone();
                        let b = self.stack[n - 1].clone();
                        self.stack.push(a);
                        self.stack.push(b);
                    }
                    Op::GetLocal(i) => {
                        let v = self.stack[lbase + i as usize].clone();
                        self.stack.push(v);
                    }
                    Op::SetLocal(i) => {
                        let v = self.stack.last().unwrap().clone();
                        self.stack[lbase + i as usize] = v;
                    }
                    Op::GetLocalCell(i) => {
                        let Value::Obj(c) = self.stack[lbase + i as usize] else {
                            unreachable!()
                        };
                        let v = self.cell_get(c);
                        self.stack.push(v);
                    }
                    Op::SetLocalCell(i) => {
                        let Value::Obj(c) = self.stack[lbase + i as usize] else {
                            unreachable!()
                        };
                        let v = self.stack.last().unwrap().clone();
                        self.cell_set(c, v);
                    }
                    Op::BoxLocal(i) => {
                        let v = self.stack[lbase + i as usize].clone();
                        let c = self.new_cell(v);
                        self.stack[lbase + i as usize] = c;
                    }
                    Op::RefreshCell(i) => {
                        let Value::Obj(c) = self.stack[lbase + i as usize] else {
                            unreachable!()
                        };
                        let v = self.cell_get(c);
                        let c = self.new_cell(v);
                        self.stack[lbase + i as usize] = c;
                    }
                    Op::GetUpval(i) => {
                        let c = self.closure_cell(self.frames[fi].closure, i);
                        let v = self.cell_get(c);
                        self.stack.push(v);
                    }
                    Op::SetUpval(i) => {
                        let c = self.closure_cell(self.frames[fi].closure, i);
                        let v = self.stack.last().unwrap().clone();
                        self.cell_set(c, v);
                    }
                    Op::GetGlobal(c) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        match self.globals.get(name) {
                            Some(v) => self.stack.push(v.clone()),
                            None => {
                                let e = self.make_error("ReferenceError", &format!("{name} is not defined"));
                                throw!(e)
                            }
                        }
                    }
                    Op::SetGlobal(c) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        let v = self.stack.last().unwrap().clone();
                        self.globals.insert(name.clone(), v);
                    }
                    Op::TypeOfGlobal(c) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        let t = match self.globals.get(name) {
                            Some(v) => self.type_of(v),
                            None => "undefined",
                        };
                        self.stack.push(Value::str(t));
                    }
                    Op::GetProp(c, ic) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        let o = self.pop();
                        let v = tri!(self.get_prop(&o, name, Some(ic)));
                        self.stack.push(v);
                    }
                    Op::SetProp(c, ic) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        let v = self.pop();
                        let o = self.pop();
                        tri!(self.set_prop(&o, name, v.clone(), Some(ic)));
                        self.stack.push(v);
                    }
                    Op::GetMethod(c, ic) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        let o = self.pop();
                        let v = tri!(self.get_prop(&o, name, Some(ic)));
                        if !matches!(v, Value::Obj(_)) {
                            let desc = match &o {
                                Value::Obj(_) => format!("{}.{name} is not a function", self.describe(&o)),
                                _ => format!("{}.{name} is not a function", self.describe(&o)),
                            };
                            let e = self.make_error("TypeError", &desc);
                            throw!(e)
                        }
                        self.stack.push(v);
                        self.stack.push(o);
                    }
                    Op::GetElem => {
                        let k = self.pop();
                        let o = self.pop();
                        let v = tri!(self.get_elem(&o, &k));
                        self.stack.push(v);
                    }
                    Op::GetElemMethod => {
                        let k = self.pop();
                        let o = self.pop();
                        let v = tri!(self.get_elem(&o, &k));
                        self.stack.push(v);
                        self.stack.push(o);
                    }
                    Op::SetElem => {
                        let v = self.pop();
                        let k = self.pop();
                        let o = self.pop();
                        tri!(self.set_elem(&o, &k, v.clone()));
                        self.stack.push(v);
                    }
                    Op::Add => {
                        let b = self.pop();
                        let a = self.pop();
                        let r = self.add(a, b);
                        self.stack.push(r);
                    }
                    Op::Sub => num_binop!(|x, y| x - y),
                    Op::Mul => num_binop!(|x, y| x * y),
                    Op::Div => num_binop!(|x, y| x / y),
                    Op::Mod => num_binop!(|x: f64, y: f64| x % y),
                    Op::Pow => num_binop!(js_pow),
                    Op::BitAnd => int_binop!(|x: i32, y: i32| (x & y) as f64),
                    Op::BitOr => int_binop!(|x: i32, y: i32| (x | y) as f64),
                    Op::BitXor => int_binop!(|x: i32, y: i32| (x ^ y) as f64),
                    Op::Shl => int_binop!(|x: i32, y: i32| x.wrapping_shl(y as u32 & 31) as f64),
                    Op::Shr => int_binop!(|x: i32, y: i32| (x >> (y as u32 & 31)) as f64),
                    Op::UShr => int_binop!(|x: i32, y: i32| ((x as u32) >> (y as u32 & 31)) as f64),
                    Op::Lt => cmp_op!(Some(std::cmp::Ordering::Less)),
                    Op::Le => cmp_op!(Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
                    Op::Gt => cmp_op!(Some(std::cmp::Ordering::Greater)),
                    Op::Ge => cmp_op!(Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
                    Op::Eq | Op::Ne => {
                        let b = self.pop();
                        let a = self.pop();
                        let r = self.loose_eq(&a, &b);
                        self.stack.push(Value::Bool(r == matches!(op, Op::Eq)));
                    }
                    Op::StrictEq | Op::StrictNe => {
                        let b = self.pop();
                        let a = self.pop();
                        let r = strict_eq(&a, &b);
                        self.stack.push(Value::Bool(r == matches!(op, Op::StrictEq)));
                    }
                    Op::InstanceOf => {
                        let b = self.pop();
                        let a = self.pop();
                        let r = tri!(self.instance_of(&a, &b));
                        self.stack.push(Value::Bool(r));
                    }
                    Op::In => {
                        let o = self.pop();
                        let k = self.pop();
                        let Value::Obj(r) = o else {
                            let e = self.make_error("TypeError", "Cannot use 'in' operator to search for a key in a primitive");
                            throw!(e)
                        };
                        let k = self.key_of(&k);
                        let has = match &self.heap.get(r).kind {
                            ObjKind::Array(items) if &*k == "length" || is_array_index(&k).is_some_and(|i| i < items.len()) => true,
                            _ => {
                                let mut cur = Some(r);
                                let mut found = false;
                                while let Some(c) = cur {
                                    let ob = self.heap.get(c);
                                    if self.heap.shape_lookup(ob.shape, &k).is_some() {
                                        found = true;
                                        break;
                                    }
                                    cur = ob.proto;
                                }
                                found
                            }
                        };
                        self.stack.push(Value::Bool(has));
                    }
                    Op::Neg => {
                        let a = self.pop();
                        let n = self.to_number(&a);
                        self.stack.push(Value::Num(-n));
                    }
                    Op::ToNumber => {
                        if !matches!(self.stack.last(), Some(Value::Num(_))) {
                            let a = self.pop();
                            let n = self.to_number(&a);
                            self.stack.push(Value::Num(n));
                        }
                    }
                    Op::Not => {
                        let a = self.pop();
                        self.stack.push(Value::Bool(!a.truthy()));
                    }
                    Op::BitNot => {
                        let a = self.pop();
                        let n = self.to_number(&a);
                        self.stack.push(Value::Num(!to_int32(n) as f64));
                    }
                    Op::TypeOf => {
                        let a = self.pop();
                        let t = self.type_of(&a);
                        self.stack.push(Value::str(t));
                    }
                    Op::Jump(t) => {
                        if (t as usize) < pc {
                            // back edge: loop iterations count towards tier-up
                            func.calls.set(func.calls.get().saturating_add(1));
                        }
                        pc = t as usize
                    }
                    Op::JumpIfFalse(t) => {
                        if !self.pop().truthy() {
                            pc = t as usize;
                        }
                    }
                    Op::JumpIfTrue(t) => {
                        if self.pop().truthy() {
                            pc = t as usize;
                        }
                    }
                    Op::JumpIfFalseKeep(t) => {
                        if !self.stack.last().unwrap().truthy() {
                            pc = t as usize;
                        } else {
                            self.stack.pop();
                        }
                    }
                    Op::JumpIfTrueKeep(t) => {
                        if self.stack.last().unwrap().truthy() {
                            pc = t as usize;
                        } else {
                            self.stack.pop();
                        }
                    }
                    Op::JumpIfNotNullishKeep(t) => {
                        if !self.stack.last().unwrap().is_nullish() {
                            pc = t as usize;
                        } else {
                            self.stack.pop();
                        }
                    }
                    Op::Call(argc) | Op::CallSuper(argc) => {
                        let argc = argc as usize;
                        let cidx = self.stack.len() - argc - 2;
                        self.frames[fi].pc = pc;
                        match self.call_at(cidx, argc, matches!(op, Op::CallSuper(_)), false) {
                            Ok(true) => continue 'frame,
                            Ok(false) => {}
                            Err(e) => throw!(e),
                        }
                    }
                    Op::CallSpread => {
                        let arr = self.pop();
                        let args = self.array_items(&arr);
                        let argc = args.len();
                        self.stack.extend(args);
                        let cidx = self.stack.len() - argc - 2;
                        self.frames[fi].pc = pc;
                        match self.call_at(cidx, argc, false, false) {
                            Ok(true) => continue 'frame,
                            Ok(false) => {}
                            Err(e) => throw!(e),
                        }
                    }
                    Op::New(_) | Op::NewSpread => {
                        let argc = match op {
                            Op::New(n) => n as usize,
                            _ => {
                                let arr = self.pop();
                                let args = self.array_items(&arr);
                                let n = args.len();
                                self.stack.extend(args);
                                n
                            }
                        };
                        let cidx = self.stack.len() - argc - 1;
                        // insert `this` placeholder after callee
                        self.stack.insert(cidx + 1, Value::Undefined);
                        self.frames[fi].pc = pc;
                        match self.call_at(cidx, argc, true, true) {
                            Ok(true) => continue 'frame,
                            Ok(false) => {}
                            Err(e) => throw!(e),
                        }
                    }
                    Op::Return => {
                        let mut result = self.pop();
                        let frame = self.frames.pop().unwrap();
                        while self.handlers.last().is_some_and(|h| h.frame >= self.frames.len()) {
                            self.handlers.pop();
                        }
                        if frame.construct && !matches!(result, Value::Obj(_)) {
                            result = self.stack[frame.base + 1].clone();
                            // `this` may have been boxed into a cell because a closure captured it
                            if let Value::Obj(c) = result {
                                if let ObjKind::Cell(v) = &self.heap.get(c).kind {
                                    result = v.clone();
                                }
                            }
                        }
                        self.stack.truncate(frame.base);
                        if self.frames.len() == stop_depth {
                            return Ok(result);
                        }
                        self.stack.push(result);
                        continue 'frame;
                    }
                    Op::Closure(idx) => {
                        let proto = func.funcs[idx as usize].clone();
                        let closure = self.frames[fi].closure;
                        let mut cells = Vec::with_capacity(proto.captures.len());
                        for c in &proto.captures {
                            cells.push(match c {
                                Capture::Local(s) => match self.stack[lbase + *s as usize] {
                                    Value::Obj(r) => r,
                                    _ => unreachable!("captured local is not a cell"),
                                },
                                Capture::Upval(j) => self.closure_cell(closure, *j),
                            });
                        }
                        let fp = self.function_proto.as_obj();
                        let r = self.alloc(ObjKind::Closure { func: proto, cells }, fp);
                        self.stack.push(Value::Obj(r));
                    }
                    Op::Array(n) => {
                        let items = self.stack.split_off(self.stack.len() - n as usize);
                        let a = self.new_array(items);
                        self.stack.push(a);
                    }
                    Op::ArrayPush => {
                        let v = self.pop();
                        let Some(Value::Obj(a)) = self.stack.last() else { unreachable!() };
                        if let ObjKind::Array(items) = &mut self.heap.get_mut(*a).kind {
                            items.push(v);
                        }
                    }
                    Op::ArrayExtend => {
                        let src = self.pop();
                        let items = match &src {
                            Value::Str(s) => s.chars().map(|c| Value::str(&c.to_string())).collect(),
                            Value::Obj(r) if matches!(self.heap.get(*r).kind, ObjKind::Array(_)) => self.array_items(&src),
                            _ => {
                                let d = self.describe(&src);
                                let e = self.make_error("TypeError", &format!("{d} is not iterable"));
                                throw!(e)
                            }
                        };
                        let Some(Value::Obj(a)) = self.stack.last() else { unreachable!() };
                        if let ObjKind::Array(dst) = &mut self.heap.get_mut(*a).kind {
                            dst.extend(items);
                        }
                    }
                    Op::NewObject => {
                        let o = self.new_object();
                        self.stack.push(Value::Obj(o));
                    }
                    Op::InitProp(c, ic) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        let v = self.pop();
                        let o = self.stack.last().unwrap().clone();
                        tri!(self.set_prop(&o, name, v, Some(ic)));
                    }
                    Op::InitElem => {
                        let v = self.pop();
                        let k = self.pop();
                        let o = self.stack.last().unwrap().clone();
                        tri!(self.set_elem(&o, &k, v));
                    }
                    Op::ObjectSpread => {
                        let src = self.pop();
                        let Some(Value::Obj(dst)) = self.stack.last().cloned() else {
                            unreachable!()
                        };
                        if let Value::Obj(s) = src {
                            for k in self.own_keys(s) {
                                let v = tri!(self.get_prop(&src, &k, None));
                                self.set_own(dst, &k, v);
                            }
                        }
                    }
                    Op::Throw => {
                        let e = self.pop();
                        throw!(e)
                    }
                    Op::ThrowConstAssign(_) => {
                        let e = self.make_error("TypeError", "Assignment to constant variable.");
                        throw!(e)
                    }
                    Op::PushHandler(t) => {
                        self.handlers.push(Handler {
                            frame: fi,
                            sp: self.stack.len(),
                            pc: t as usize,
                        });
                    }
                    Op::PopHandler => {
                        self.handlers.pop();
                    }
                    Op::Keys => {
                        let o = self.pop();
                        let keys: Vec<Value> = match o {
                            Value::Obj(r) => self.own_keys(r).into_iter().map(Value::Str).collect(),
                            Value::Str(ref s) => (0..s.chars().count()).map(|i| Value::str(&i.to_string())).collect(),
                            _ => vec![],
                        };
                        let a = self.new_array(keys);
                        self.stack.push(a);
                    }
                    Op::MakeClass => {
                        let ctor = self.pop();
                        let parent = self.pop();
                        let Value::Obj(c) = ctor else { unreachable!() };
                        let pp = match parent {
                            Value::Obj(p) => Some(p),
                            Value::Null => None,
                            _ => {
                                let e = self.make_error("TypeError", "Class extends value is not a constructor or null");
                                throw!(e)
                            }
                        };
                        let proto = self.alloc(ObjKind::Plain, pp);
                        self.set_own(proto, &Rc::from("constructor"), Value::Obj(c));
                        self.set_own(c, &Rc::from("prototype"), Value::Obj(proto));
                        self.stack.push(Value::Obj(c));
                    }
                    Op::DefineMethod(c) => {
                        let Value::Str(name) = &func.consts[c as usize] else {
                            unreachable!()
                        };
                        let m = self.pop();
                        let Some(Value::Obj(ctor)) = self.stack.last().cloned() else {
                            unreachable!()
                        };
                        if let Some(Value::Obj(p)) = self.get_own(ctor, "prototype") {
                            self.set_own(p, name, m);
                        }
                    }
                }
            }
        }
    }

    /// Length of an array value (0 for non-arrays), without copying.
    pub fn array_len(&self, v: &Value) -> usize {
        match v {
            Value::Obj(r) => match &self.heap.get(*r).kind {
                ObjKind::Array(items) => items.len(),
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Element `i` of an array value, without copying the array.
    pub fn array_get(&self, v: &Value, i: usize) -> Value {
        match v {
            Value::Obj(r) => match &self.heap.get(*r).kind {
                ObjKind::Array(items) => items.get(i).cloned().unwrap_or(Value::Undefined),
                _ => Value::Undefined,
            },
            _ => Value::Undefined,
        }
    }

    pub fn array_items(&self, v: &Value) -> Vec<Value> {
        match v {
            Value::Obj(r) => match &self.heap.get(*r).kind {
                ObjKind::Array(items) => items.clone(),
                _ => vec![],
            },
            _ => vec![],
        }
    }

    /// Short description of a value for error messages.
    pub fn describe(&mut self, v: &Value) -> String {
        match v {
            Value::Str(s) => format!("\"{s}\""),
            Value::Obj(r) => match &self.heap.get(*r).kind {
                ObjKind::Array(_) => "array".into(),
                ObjKind::Closure { func, .. } => format!("function {}", func.name),
                _ => "object".into(),
            },
            other => self.to_string(other),
        }
    }

    /// Perform a call whose callee is at `stack[cidx]`, `this` at `cidx+1` and args after.
    /// Returns Ok(true) if a new interpreter frame was pushed, Ok(false) if the result
    /// was already pushed (native or JIT call).
    fn call_at(&mut self, cidx: usize, argc: usize, allow_class: bool, construct: bool) -> VResult<bool> {
        let callee = self.stack[cidx].clone();
        let Value::Obj(r) = callee else {
            self.stack.truncate(cidx);
            let d = self.describe(&callee);
            return self.type_error(format!("{d} is not a {}", if construct { "constructor" } else { "function" }));
        };
        match &self.heap.get(r).kind {
            ObjKind::Closure { func, .. } => {
                let func = func.clone();
                if func.is_class && !allow_class {
                    self.stack.truncate(cidx);
                    return self.type_error(format!("Class constructor {} cannot be invoked without 'new'", func.name));
                }
                if construct {
                    if func.is_arrow {
                        self.stack.truncate(cidx);
                        return self.type_error(format!("{} is not a constructor", func.name));
                    }
                    let this = self.construct_this(r);
                    self.stack[cidx + 1] = this;
                } else if self.opts.jit && !func.is_class {
                    let args: Vec<Value> = self.stack[cidx + 2..].to_vec();
                    if let Some(res) = crate::jit::try_call(self, &func, &args) {
                        self.stack.truncate(cidx);
                        self.stack.push(res?);
                        return Ok(false);
                    }
                }
                self.push_frame(r, func, cidx, argc, construct)?;
                Ok(true)
            }
            ObjKind::Native { f, .. } => {
                let f = *f;
                if construct {
                    let this = self.construct_this(r);
                    self.stack[cidx + 1] = this;
                }
                let args: Vec<Value> = self.stack[cidx + 2..].to_vec();
                let this = self.stack[cidx + 1].clone();
                self.native_depth += 1;
                let res = f(self, this.clone(), &args);
                self.native_depth -= 1;
                self.stack.truncate(cidx);
                let mut v = res?;
                if construct && !matches!(v, Value::Obj(_)) {
                    v = this;
                }
                self.stack.push(v);
                Ok(false)
            }
            _ => {
                self.stack.truncate(cidx);
                let d = self.describe(&callee);
                self.type_error(format!("{d} is not a {}", if construct { "constructor" } else { "function" }))
            }
        }
    }

    /// Line number of the instruction currently executing in the innermost frame.
    pub fn current_line(&self) -> u32 {
        self.frames
            .last()
            .map(|f| f.func.lines.get(f.pc.saturating_sub(1)).copied().unwrap_or(0))
            .unwrap_or(0)
    }

    pub fn print_line(&mut self, s: &str) {
        if self.opts.echo_stdout {
            println!("{s}");
        } else {
            self.out.push_str(s);
            self.out.push('\n');
        }
    }
}

pub fn js_pow(x: f64, y: f64) -> f64 {
    if y.is_nan() || ((x == 1.0 || x == -1.0) && y.is_infinite()) {
        return f64::NAN;
    }
    x.powf(y)
}

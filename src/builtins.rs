//! Built-in objects: console, Math, JSON, Array/String/Number prototypes, Error types.

use crate::heap::{NativeFn, ObjKind};
use crate::printer::quote_str;
use crate::value::{num_to_string, Str, Value};
use crate::vm::{VResult, Vm};
use std::rc::Rc;

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Undefined)
}

fn def(vm: &mut Vm, target: &Value, name: &'static str, f: NativeFn) {
    let n = vm.new_native(name, f);
    let _ = vm.set_prop(target, &Rc::from(name), n, None);
}

fn set(vm: &mut Vm, target: &Value, name: &str, v: Value) {
    let _ = vm.set_prop(target, &Rc::from(name), v, None);
}

pub fn install(vm: &mut Vm) {
    let fproto = Value::Obj(vm.new_object());
    vm.function_proto = fproto;
    let aproto = Value::Obj(vm.new_object());
    vm.array_proto = aproto.clone();
    let sproto = Value::Obj(vm.new_object());
    vm.string_proto = sproto.clone();
    let nproto = Value::Obj(vm.new_object());
    vm.number_proto = nproto.clone();

    // console
    let console = Value::Obj(vm.new_object());
    def(vm, &console, "log", console_log);
    def(vm, &console, "error", console_log);
    vm.globals.insert(Rc::from("console"), console);

    // Math
    let math = Value::Obj(vm.new_object());
    set(vm, &math, "PI", Value::Num(std::f64::consts::PI));
    set(vm, &math, "E", Value::Num(std::f64::consts::E));
    macro_rules! math1 {
        ($name:literal, $f:expr) => {
            def(vm, &math, $name, |vm, _, a| {
                let x = vm.to_number(&arg(a, 0));
                Ok(Value::Num($f(x)))
            });
        };
    }
    math1!("abs", f64::abs);
    math1!("floor", f64::floor);
    math1!("ceil", f64::ceil);
    math1!("sqrt", f64::sqrt);
    math1!("cbrt", f64::cbrt);
    math1!("sin", f64::sin);
    math1!("cos", f64::cos);
    math1!("tan", f64::tan);
    math1!("atan", f64::atan);
    math1!("exp", f64::exp);
    math1!("log", f64::ln);
    math1!("log2", f64::log2);
    math1!("log10", f64::log10);
    math1!("trunc", f64::trunc);
    math1!("round", |x: f64| if x.is_finite() { (x + 0.5).floor() } else { x });
    math1!("sign", |x: f64| if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    });
    def(vm, &math, "pow", |vm, _, a| {
        let x = vm.to_number(&arg(a, 0));
        let y = vm.to_number(&arg(a, 1));
        Ok(Value::Num(crate::vm::js_pow(x, y)))
    });
    def(vm, &math, "atan2", |vm, _, a| {
        let y = vm.to_number(&arg(a, 0));
        let x = vm.to_number(&arg(a, 1));
        Ok(Value::Num(y.atan2(x)))
    });
    def(vm, &math, "hypot", |vm, _, a| {
        let mut s = 0.0;
        for v in a {
            let x = vm.to_number(v);
            s += x * x;
        }
        Ok(Value::Num(s.sqrt()))
    });
    def(vm, &math, "max", |vm, _, a| {
        let mut m = f64::NEG_INFINITY;
        for v in a {
            let x = vm.to_number(v);
            if x.is_nan() {
                return Ok(Value::Num(f64::NAN));
            }
            if x > m {
                m = x;
            }
        }
        Ok(Value::Num(m))
    });
    def(vm, &math, "min", |vm, _, a| {
        let mut m = f64::INFINITY;
        for v in a {
            let x = vm.to_number(v);
            if x.is_nan() {
                return Ok(Value::Num(f64::NAN));
            }
            if x < m {
                m = x;
            }
        }
        Ok(Value::Num(m))
    });
    def(vm, &math, "random", |_, _, _| {
        // xorshift seeded from the clock; not used by the deterministic test corpus
        use std::cell::Cell;
        thread_local!(static SEED: Cell<u64> = const { Cell::new(0) });
        let v = SEED.with(|s| {
            let mut x = s.get();
            if x == 0 {
                x = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(1)
                    | 1;
            }
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            s.set(x);
            x
        });
        Ok(Value::Num((v >> 11) as f64 / (1u64 << 53) as f64))
    });
    vm.globals.insert(Rc::from("Math"), math);

    // JSON
    let json = Value::Obj(vm.new_object());
    def(vm, &json, "stringify", json_stringify);
    vm.globals.insert(Rc::from("JSON"), json);

    // Object
    let object = vm.new_native("Object", |vm, _, _| Ok(Value::Obj(vm.new_object())));
    def(vm, &object, "keys", |vm, _, a| {
        let keys = match arg(a, 0) {
            Value::Obj(r) => vm.own_keys(r).into_iter().map(Value::Str).collect(),
            _ => vec![],
        };
        Ok(vm.new_array(keys))
    });
    def(vm, &object, "values", |vm, _, a| {
        let o = arg(a, 0);
        let mut vals = vec![];
        if let Value::Obj(r) = o {
            for k in vm.own_keys(r) {
                vals.push(vm.get_prop(&o, &k, None)?);
            }
        }
        Ok(vm.new_array(vals))
    });
    def(vm, &object, "entries", |vm, _, a| {
        let o = arg(a, 0);
        let mut pairs = vec![];
        if let Value::Obj(r) = o {
            for k in vm.own_keys(r) {
                let v = vm.get_prop(&o, &k, None)?;
                pairs.push(vm.new_array(vec![Value::Str(k), v]));
            }
        }
        Ok(vm.new_array(pairs))
    });
    def(vm, &object, "assign", |vm, _, a| {
        let target = arg(a, 0);
        for src in a.iter().skip(1) {
            if let Value::Obj(r) = src {
                for k in vm.own_keys(*r) {
                    let v = vm.get_prop(src, &k, None)?;
                    vm.set_prop(&target, &k, v, None)?;
                }
            }
        }
        Ok(target)
    });
    vm.globals.insert(Rc::from("Object"), object);

    // Array
    let array = vm.new_native("Array", |vm, _, a| {
        if a.len() == 1 {
            if let Value::Num(n) = a[0] {
                return Ok(vm.new_array(vec![Value::Undefined; n as usize]));
            }
        }
        Ok(vm.new_array(a.to_vec()))
    });
    set(vm, &array, "prototype", aproto.clone());
    def(vm, &array, "isArray", |vm, _, a| {
        Ok(Value::Bool(
            matches!(arg(a, 0), Value::Obj(r) if matches!(vm.heap.get(r).kind, ObjKind::Array(_))),
        ))
    });
    vm.globals.insert(Rc::from("Array"), array);
    install_array_proto(vm, &aproto);
    install_string_proto(vm, &sproto);

    def(vm, &nproto, "toFixed", |vm, this, a| {
        let x = vm.to_number(&this);
        let d = vm.to_number(&arg(a, 0)) as usize;
        Ok(Value::str(&format!("{x:.d$}")))
    });
    def(vm, &nproto, "toString", |vm, this, a| {
        let x = vm.to_number(&this);
        let radix = match arg(a, 0) {
            Value::Undefined => 10,
            v => vm.to_number(&v) as u32,
        };
        if radix == 10 || x.fract() != 0.0 || !(2..=36).contains(&radix) {
            return Ok(Value::str(&num_to_string(x)));
        }
        let neg = x < 0.0;
        let mut n = x.abs() as u64;
        let mut digits = vec![];
        loop {
            digits.push(std::char::from_digit((n % radix as u64) as u32, radix).unwrap());
            n /= radix as u64;
            if n == 0 {
                break;
            }
        }
        if neg {
            digits.push('-');
        }
        Ok(Value::str(&digits.iter().rev().collect::<String>()))
    });

    // conversions
    let string_fn = vm.new_native("String", |vm, _, a| {
        let v = arg(a, 0);
        Ok(Value::str(&if a.is_empty() { String::new() } else { vm.to_string(&v) }))
    });
    set(vm, &string_fn, "prototype", sproto);
    vm.globals.insert(Rc::from("String"), string_fn);
    let number_fn = vm.new_native("Number", |vm, _, a| {
        Ok(Value::Num(if a.is_empty() { 0.0 } else { vm.to_number(&a[0]) }))
    });
    set(vm, &number_fn, "prototype", nproto);
    set(vm, &number_fn, "MAX_SAFE_INTEGER", Value::Num(9007199254740991.0));
    set(vm, &number_fn, "EPSILON", Value::Num(f64::EPSILON));
    def(vm, &number_fn, "isInteger", |_, _, a| {
        Ok(Value::Bool(matches!(arg(a, 0), Value::Num(n) if n.is_finite() && n.fract() == 0.0)))
    });
    vm.globals.insert(Rc::from("Number"), number_fn);
    let boolean_fn = vm.new_native("Boolean", |_, _, a| Ok(Value::Bool(arg(a, 0).truthy())));
    vm.globals.insert(Rc::from("Boolean"), boolean_fn);
    let f = vm.new_native("parseInt", |vm, _, a| {
        let s = vm.to_string(&arg(a, 0));
        let radix = match arg(a, 1) {
            Value::Undefined => 0,
            v => vm.to_number(&v) as u32,
        };
        let t = s.trim();
        let (neg, t) = match t.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, t.strip_prefix('+').unwrap_or(t)),
        };
        let (radix, t) = if (radix == 16 || radix == 0) && (t.starts_with("0x") || t.starts_with("0X")) {
            (16, &t[2..])
        } else {
            (if radix == 0 { 10 } else { radix }, t)
        };
        if !(2..=36).contains(&radix) {
            return Ok(Value::Num(f64::NAN));
        }
        let mut v = 0f64;
        let mut any = false;
        for c in t.chars() {
            match c.to_digit(radix) {
                Some(d) => {
                    v = v * radix as f64 + d as f64;
                    any = true
                }
                None => break,
            }
        }
        Ok(Value::Num(if !any {
            f64::NAN
        } else if neg {
            -v
        } else {
            v
        }))
    });
    vm.globals.insert(Rc::from("parseInt"), f);
    let f = vm.new_native("parseFloat", |vm, _, a| {
        let s = vm.to_string(&arg(a, 0));
        let t = s.trim_start();
        let mut end = 0;
        let b = t.as_bytes();
        let mut seen_dot = false;
        let mut seen_e = false;
        while end < b.len() {
            let c = b[end];
            if c.is_ascii_digit() || (end == 0 && (c == b'-' || c == b'+')) {
            } else if c == b'.' && !seen_dot && !seen_e {
                seen_dot = true;
            } else if (c == b'e' || c == b'E') && !seen_e && end > 0 {
                seen_e = true;
                if end + 1 < b.len() && (b[end + 1] == b'-' || b[end + 1] == b'+') {
                    end += 1;
                }
            } else {
                break;
            }
            end += 1;
        }
        let mut sub = &t[..end];
        while !sub.is_empty() && sub.parse::<f64>().is_err() {
            sub = &sub[..sub.len() - 1];
        }
        if t.starts_with("Infinity") {
            return Ok(Value::Num(f64::INFINITY));
        }
        Ok(Value::Num(sub.parse().unwrap_or(f64::NAN)))
    });
    vm.globals.insert(Rc::from("parseFloat"), f);
    let f = vm.new_native("isNaN", |vm, _, a| Ok(Value::Bool(vm.to_number(&arg(a, 0)).is_nan())));
    vm.globals.insert(Rc::from("isNaN"), f);
    let f = vm.new_native("isFinite", |vm, _, a| Ok(Value::Bool(vm.to_number(&arg(a, 0)).is_finite())));
    vm.globals.insert(Rc::from("isFinite"), f);
    vm.globals.insert(Rc::from("NaN"), Value::Num(f64::NAN));
    vm.globals.insert(Rc::from("Infinity"), Value::Num(f64::INFINITY));

    // Errors
    let error_ctor: NativeFn = |vm, this, a| {
        let msg = arg(a, 0);
        if !matches!(msg, Value::Undefined) {
            let m = vm.to_string(&msg);
            vm.set_prop(&this, &Rc::from("message"), Value::str(&m), None)?;
        }
        Ok(this)
    };
    let error = vm.new_native("Error", error_ctor);
    let eproto = Value::Obj(vm.new_object());
    set(vm, &eproto, "name", Value::str("Error"));
    set(vm, &eproto, "message", Value::str(""));
    set(vm, &eproto, "constructor", error.clone());
    def(vm, &eproto, "toString", |vm, this, _| Ok(Value::str(&vm.to_string(&this))));
    set(vm, &error, "prototype", eproto.clone());
    vm.error_proto = eproto.clone();
    vm.globals.insert(Rc::from("Error"), error);
    for name in ["TypeError", "RangeError", "ReferenceError", "SyntaxError"] {
        let ctor = vm.new_native(name, error_ctor);
        let p = vm.alloc(ObjKind::Plain, eproto.as_obj());
        let p = Value::Obj(p);
        set(vm, &p, "name", Value::str(name));
        set(vm, &p, "constructor", ctor.clone());
        set(vm, &ctor, "prototype", p);
        vm.globals.insert(Rc::from(name), ctor);
    }

    // Engine introspection helpers (used by tests)
    let f = vm.new_native("gc", |vm, _, _| {
        // SAFETY of collecting here: natives are only invoked from the interpreter's Call
        // path where every live value is still on the VM stack or in extra_roots.
        vm.collect(true);
        Ok(Value::Undefined)
    });
    vm.globals.insert(Rc::from("gc"), f);
}

// ---------------------------------------------------------------- console.log / inspect

fn console_log(vm: &mut Vm, _this: Value, args: &[Value]) -> VResult<Value> {
    let parts: Vec<String> = args
        .iter()
        .map(|a| match a {
            Value::Str(s) => s.to_string(),
            other => inspect(vm, other, 0, 0),
        })
        .collect();
    let line = parts.join(" ");
    vm.print_line(&line);
    Ok(Value::Undefined)
}

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(ch) if ch.is_alphabetic() || ch == '_' || ch == '$')
        && c.all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '$')
}

fn inspect_str(s: &str) -> String {
    if s.contains('\'') && !s.contains('"') {
        quote_str(s, '"')
    } else {
        quote_str(s, '\'')
    }
}

/// Format a value like Node's `util.inspect` (subset).
pub fn inspect(vm: &mut Vm, v: &Value, depth: usize, indent: usize) -> String {
    match v {
        Value::Str(s) => inspect_str(s),
        Value::Num(n) if *n == 0.0 && n.is_sign_negative() => "-0".into(),
        Value::Obj(r) => {
            let r = *r;
            let (kind_name, is_array) = match &vm.heap.get(r).kind {
                ObjKind::Closure { func, .. } => {
                    if func.is_class {
                        let ext = {
                            // find parent class via prototype chain
                            let proto = vm.get_own(r, "prototype");
                            let parent = proto
                                .and_then(|p| p.as_obj())
                                .and_then(|p| vm.heap.get(p).proto)
                                .and_then(|pp| vm.get_own(pp, "constructor"));
                            match parent {
                                Some(Value::Obj(c)) => match &vm.heap.get(c).kind {
                                    ObjKind::Closure { func, .. } => format!(" extends {}", func.name),
                                    ObjKind::Native { name, .. } => format!(" extends {name}"),
                                    _ => String::new(),
                                },
                                _ => String::new(),
                            }
                        };
                        return format!("[class {}{ext}]", if func.name.is_empty() { "(anonymous)" } else { &func.name });
                    }
                    return if func.name.is_empty() {
                        "[Function (anonymous)]".into()
                    } else {
                        format!("[Function: {}]", func.name)
                    };
                }
                ObjKind::Native { name, .. } => return format!("[Function: {name}]"),
                ObjKind::Array(_) => (String::new(), true),
                ObjKind::Cell(_) => return "<cell>".into(),
                ObjKind::Plain => {
                    if vm.is_error(r) {
                        let s = vm.to_string(v);
                        return if depth == 0 { s } else { format!("[{s}]") };
                    }
                    let proto = vm.heap.get(r).proto;
                    let name = match proto.and_then(|p| vm.get_own(p, "constructor")) {
                        Some(Value::Obj(c)) => match &vm.heap.get(c).kind {
                            ObjKind::Closure { func, .. } => func.name.clone(),
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    (name, false)
                }
            };
            if depth > 2 {
                return if is_array {
                    "[Array]".into()
                } else if kind_name.is_empty() {
                    "[Object]".into()
                } else {
                    format!("[{kind_name}]")
                };
            }
            let mut out = Vec::new();
            let mut all_numbers = true;
            if is_array {
                let items = vm.array_items(v);
                let mut holes = 0;
                for it in &items {
                    if !matches!(it, Value::Num(_)) {
                        all_numbers = false;
                    }
                    let _ = &mut holes;
                    out.push(inspect(vm, it, depth + 1, indent + 2));
                    if out.len() >= 100 {
                        break;
                    }
                }
                if items.len() > 100 {
                    out.push(format!("... {} more items", items.len() - 100));
                }
            }
            let keys = vm.prop_keys(r);
            for k in keys {
                let val = vm.get_own(r, &k).unwrap_or(Value::Undefined);
                let ks = if is_ident(&k) { k.to_string() } else { inspect_str(&k) };
                let vs = inspect(vm, &val, depth + 1, indent + 2);
                out.push(format!("{ks}: {vs}"));
            }
            let (open, close) = if is_array { ("[", "]") } else { ("{", "}") };
            let base = kind_name;
            if out.is_empty() {
                return if base.is_empty() {
                    format!("{open}{close}")
                } else {
                    format!("{base} {open}{close}")
                };
            }
            let entries = out.len();
            if is_array && entries > 6 {
                out = group_array_elements(&out, indent, all_numbers);
            }
            let prefix = if base.is_empty() { String::new() } else { format!("{base} ") };
            if entries == out.len() {
                let start = out.len() + indent + open.len() + base.len() + 10;
                let total: usize = out.iter().map(|s| s.chars().count()).sum::<usize>() + out.len() + start;
                if total <= 80 && !out.iter().any(|s| s.contains('\n')) {
                    return format!("{prefix}{open} {} {close}", out.join(", "));
                }
            }
            let ind = " ".repeat(indent);
            format!("{prefix}{open}\n{ind}  {}\n{ind}{close}", out.join(&format!(",\n{ind}  ")))
        }
        other => vm.to_string(other),
    }
}

/// Node's column grouping for arrays with more than six entries.
fn group_array_elements(output: &[String], indent: usize, all_numbers: bool) -> Vec<String> {
    let mut total_length = 0usize;
    let mut max_length = 0usize;
    let sep = 2usize;
    let data_len: Vec<usize> = output.iter().map(|s| s.chars().count()).collect();
    for &len in &data_len {
        total_length += len + sep;
        max_length = max_length.max(len);
    }
    let actual_max = max_length + sep;
    if actual_max * 3 + indent < 80 && (total_length as f64 / actual_max as f64 > 5.0 || max_length <= 6) {
        let average_bias = (actual_max as f64 - total_length as f64 / output.len() as f64).sqrt();
        let biased_max = (actual_max as f64 - 3.0 - average_bias).max(1.0);
        let columns = ((2.5 * biased_max * output.len() as f64).sqrt() / biased_max)
            .round()
            .min(((80 - indent) / actual_max) as f64)
            .min(12.0)
            .min(15.0) as usize;
        if columns <= 1 {
            return output.to_vec();
        }
        let mut max_line_length = Vec::new();
        for i in 0..columns {
            let mut line_length = 0;
            let mut j = i;
            while j < output.len() {
                line_length = line_length.max(data_len[j]);
                j += columns;
            }
            max_line_length.push(line_length + sep);
        }
        let mut tmp = Vec::new();
        let mut i = 0;
        while i < output.len() {
            let max = (i + columns).min(output.len());
            let mut s = String::new();
            let mut j = i;
            while j < max - 1 {
                let cell = format!("{}, ", output[j]);
                let w = max_line_length[j - i];
                if all_numbers {
                    s += &format!("{cell:>w$}");
                } else {
                    s += &format!("{cell:<w$}");
                }
                j += 1;
            }
            if all_numbers {
                let w = max_line_length[j - i] - sep;
                s += &format!("{:>w$}", output[j]);
            } else {
                s += &output[j];
            }
            tmp.push(s);
            i += columns;
        }
        return tmp;
    }
    output.to_vec()
}

// ---------------------------------------------------------------- JSON

fn json_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_ser(vm: &mut Vm, v: &Value, indent: &str, cur: &str, depth: usize) -> VResult<Option<String>> {
    if depth > 200 {
        return Err(vm.make_error("TypeError", "Converting circular structure to JSON"));
    }
    Ok(Some(match v {
        Value::Undefined => return Ok(None),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Num(n) => {
            if n.is_finite() {
                num_to_string(*n)
            } else {
                "null".into()
            }
        }
        Value::Str(s) => json_quote(s),
        Value::Obj(r) => {
            let r = *r;
            let inner = format!("{cur}{indent}");
            let (open, close, sep_nl) = if indent.is_empty() { ("", "", "") } else { ("\n", "\n", "\n") };
            match &vm.heap.get(r).kind {
                ObjKind::Closure { .. } | ObjKind::Native { .. } => return Ok(None),
                ObjKind::Array(items) => {
                    let items = items.clone();
                    if items.is_empty() {
                        return Ok(Some("[]".into()));
                    }
                    let mut parts = vec![];
                    for it in &items {
                        parts.push(json_ser(vm, it, indent, &inner, depth + 1)?.unwrap_or_else(|| "null".into()));
                    }
                    let parts: Vec<String> = parts
                        .into_iter()
                        .map(|p| format!("{}{p}", if indent.is_empty() { "" } else { &inner }))
                        .collect();
                    format!(
                        "[{open}{}{close}{}]",
                        parts.join(&format!(",{sep_nl}")),
                        if indent.is_empty() { "" } else { cur }
                    )
                }
                _ => {
                    let keys = vm.own_keys(r);
                    let mut parts = vec![];
                    for k in keys {
                        let val = vm.get_prop(v, &k, None)?;
                        if let Some(s) = json_ser(vm, &val, indent, &inner, depth + 1)? {
                            let colon = if indent.is_empty() { ":" } else { ": " };
                            parts.push(format!(
                                "{}{}{colon}{s}",
                                if indent.is_empty() { "" } else { &inner },
                                json_quote(&k)
                            ));
                        }
                    }
                    if parts.is_empty() {
                        return Ok(Some("{}".into()));
                    }
                    format!(
                        "{{{open}{}{close}{}}}",
                        parts.join(&format!(",{sep_nl}")),
                        if indent.is_empty() { "" } else { cur }
                    )
                }
            }
        }
    }))
}

fn json_stringify(vm: &mut Vm, _this: Value, args: &[Value]) -> VResult<Value> {
    let indent = match arg(args, 2) {
        Value::Num(n) => " ".repeat((n.max(0.0) as usize).min(10)),
        Value::Str(s) => s.chars().take(10).collect(),
        _ => String::new(),
    };
    Ok(match json_ser(vm, &arg(args, 0), &indent, "", 0)? {
        Some(s) => Value::str(&s),
        None => Value::Undefined,
    })
}

// ---------------------------------------------------------------- Array.prototype

fn this_items(vm: &mut Vm, this: &Value) -> VResult<Vec<Value>> {
    match this {
        Value::Obj(r) if matches!(vm.heap.get(*r).kind, ObjKind::Array(_)) => Ok(vm.array_items(this)),
        _ => vm.type_error("Array.prototype method called on non-array"),
    }
}

fn with_array<R>(vm: &mut Vm, this: &Value, f: impl FnOnce(&mut Vec<Value>) -> R) -> VResult<R> {
    match this {
        Value::Obj(r) => match &mut vm.heap.get_mut(*r).kind {
            ObjKind::Array(items) => Ok(f(items)),
            _ => vm.type_error("Array.prototype method called on non-array"),
        },
        _ => vm.type_error("Array.prototype method called on non-array"),
    }
}

/// Run `f` with values rooted in `vm.extra_roots` so they survive GCs triggered by callbacks.
/// Inside `f`, read them back via `vm.extra_roots[base + i]`.
fn with_roots<R>(vm: &mut Vm, vals: Vec<Value>, f: impl FnOnce(&mut Vm, usize) -> R) -> R {
    let base = vm.extra_roots.len();
    vm.extra_roots.extend(vals);
    let r = f(vm, base);
    vm.extra_roots.truncate(base);
    r
}

fn rel_index(vm: &mut Vm, v: &Value, len: usize, default: usize) -> usize {
    match v {
        Value::Undefined => default,
        v => {
            let n = vm.to_number(v);
            let n = if n.is_nan() { 0.0 } else { n.trunc() };
            if n < 0.0 {
                (len as f64 + n).max(0.0) as usize
            } else {
                (n as usize).min(len)
            }
        }
    }
}

/// Iterate an array calling `cb(elem, index, array)`; `step` sees each result and may stop early.
fn iterate(
    vm: &mut Vm,
    this: Value,
    cb: Value,
    extra: Value,
    mut step: impl FnMut(&mut Vm, usize, Value, Value, usize) -> VResult<bool>,
) -> VResult<Value> {
    if !matches!(&cb, Value::Obj(r) if matches!(vm.heap.get(*r).kind, ObjKind::Closure{..} | ObjKind::Native{..})) {
        let d = vm.describe(&cb);
        return vm.type_error(format!("{d} is not a function"));
    }
    this_items(vm, &this)?;
    with_roots(vm, vec![this, cb, extra], |vm, base| {
        let mut i = 0;
        loop {
            let arr = vm.extra_roots[base].clone();
            let len = vm.array_len(&arr);
            if i >= len {
                break;
            }
            let item = vm.array_get(&arr, i);
            let cb = vm.extra_roots[base + 1].clone();
            let r = vm.call_value(&cb, Value::Undefined, &[item, Value::Num(i as f64), arr])?;
            // re-read the element: the callback may have triggered a moving GC
            let item = vm.array_get(&vm.extra_roots[base], i);
            if !step(vm, base, item, r, i)? {
                break;
            }
            i += 1;
        }
        Ok(vm.extra_roots[base + 2].clone())
    })
}

fn push_root_array(vm: &mut Vm, base: usize, v: Value) {
    if let Value::Obj(r) = vm.extra_roots[base + 2] {
        if let ObjKind::Array(items) = &mut vm.heap.get_mut(r).kind {
            items.push(v);
        }
    }
}

fn install_array_proto(vm: &mut Vm, p: &Value) {
    def(vm, p, "push", |vm, this, a| {
        let a = a.to_vec();
        let n = with_array(vm, &this, |items| {
            items.extend(a);
            items.len()
        })?;
        Ok(Value::Num(n as f64))
    });
    def(vm, p, "pop", |vm, this, _| {
        Ok(with_array(vm, &this, |items| items.pop())?.unwrap_or(Value::Undefined))
    });
    def(vm, p, "shift", |vm, this, _| {
        Ok(with_array(vm, &this, |items| if items.is_empty() { None } else { Some(items.remove(0)) })?.unwrap_or(Value::Undefined))
    });
    def(vm, p, "unshift", |vm, this, a| {
        let a = a.to_vec();
        let n = with_array(vm, &this, |items| {
            items.splice(0..0, a);
            items.len()
        })?;
        Ok(Value::Num(n as f64))
    });
    def(vm, p, "slice", |vm, this, a| {
        let items = this_items(vm, &this)?;
        let s = rel_index(vm, &arg(a, 0), items.len(), 0);
        let e = rel_index(vm, &arg(a, 1), items.len(), items.len());
        Ok(vm.new_array(if s < e { items[s..e].to_vec() } else { vec![] }))
    });
    def(vm, p, "splice", |vm, this, a| {
        let len = this_items(vm, &this)?.len();
        let s = rel_index(vm, &arg(a, 0), len, 0);
        let dc = match arg(a, 1) {
            Value::Undefined if a.len() < 2 => len - s,
            v => (vm.to_number(&v).max(0.0) as usize).min(len - s),
        };
        let ins: Vec<Value> = a.iter().skip(2).cloned().collect();
        let removed = with_array(vm, &this, |items| items.splice(s..s + dc, ins).collect::<Vec<_>>())?;
        Ok(vm.new_array(removed))
    });
    def(vm, p, "concat", |vm, this, a| {
        let mut items = this_items(vm, &this)?;
        for x in a {
            match x {
                Value::Obj(r) if matches!(vm.heap.get(*r).kind, ObjKind::Array(_)) => items.extend(vm.array_items(x)),
                other => items.push(other.clone()),
            }
        }
        Ok(vm.new_array(items))
    });
    def(vm, p, "join", |vm, this, a| {
        let items = this_items(vm, &this)?;
        let sep = match arg(a, 0) {
            Value::Undefined => ",".to_string(),
            v => vm.to_string(&v),
        };
        let parts: Vec<String> = items
            .iter()
            .map(|x| if x.is_nullish() { String::new() } else { vm.to_string(x) })
            .collect();
        Ok(Value::str(&parts.join(&sep)))
    });
    def(vm, p, "toString", |vm, this, _| Ok(Value::str(&vm.to_string(&this))));
    def(vm, p, "reverse", |vm, this, _| {
        with_array(vm, &this, |items| items.reverse())?;
        Ok(this)
    });
    def(vm, p, "indexOf", |vm, this, a| {
        let items = this_items(vm, &this)?;
        let x = arg(a, 0);
        Ok(Value::Num(
            items
                .iter()
                .position(|v| crate::value::strict_eq(v, &x))
                .map(|i| i as f64)
                .unwrap_or(-1.0),
        ))
    });
    def(vm, p, "lastIndexOf", |vm, this, a| {
        let items = this_items(vm, &this)?;
        let x = arg(a, 0);
        Ok(Value::Num(
            items
                .iter()
                .rposition(|v| crate::value::strict_eq(v, &x))
                .map(|i| i as f64)
                .unwrap_or(-1.0),
        ))
    });
    def(vm, p, "includes", |vm, this, a| {
        let items = this_items(vm, &this)?;
        let x = arg(a, 0);
        Ok(Value::Bool(items.iter().any(|v| {
            crate::value::strict_eq(v, &x) || matches!((v, &x), (Value::Num(p), Value::Num(q)) if p.is_nan() && q.is_nan())
        })))
    });
    def(vm, p, "fill", |vm, this, a| {
        let v = arg(a, 0);
        with_array(vm, &this, |items| items.iter_mut().for_each(|x| *x = v.clone()))?;
        Ok(this)
    });
    def(vm, p, "map", |vm, this, a| {
        let out = vm.new_array(vec![]);
        iterate(vm, this, arg(a, 0), out, |vm, base, _, r, _| {
            push_root_array(vm, base, r);
            Ok(true)
        })
    });
    def(vm, p, "filter", |vm, this, a| {
        let out = vm.new_array(vec![]);
        iterate(vm, this, arg(a, 0), out, |vm, base, item, r, _| {
            if r.truthy() {
                push_root_array(vm, base, item);
            }
            Ok(true)
        })
    });
    def(vm, p, "forEach", |vm, this, a| {
        iterate(vm, this, arg(a, 0), Value::Undefined, |_, _, _, _, _| Ok(true))?;
        Ok(Value::Undefined)
    });
    def(vm, p, "some", |vm, this, a| {
        let mut found = false;
        iterate(vm, this, arg(a, 0), Value::Undefined, |_, _, _, r, _| {
            found = r.truthy();
            Ok(!found)
        })?;
        Ok(Value::Bool(found))
    });
    def(vm, p, "every", |vm, this, a| {
        let mut all = true;
        iterate(vm, this, arg(a, 0), Value::Undefined, |_, _, _, r, _| {
            all = r.truthy();
            Ok(all)
        })?;
        Ok(Value::Bool(all))
    });
    def(vm, p, "find", |vm, this, a| {
        iterate(vm, this, arg(a, 0), Value::Undefined, |vm, base, item, r, _| {
            if r.truthy() {
                vm.extra_roots[base + 2] = item;
                return Ok(false);
            }
            Ok(true)
        })
    });
    def(vm, p, "findIndex", |vm, this, a| {
        let mut idx = -1.0;
        iterate(vm, this, arg(a, 0), Value::Undefined, |_, _, _, r, i| {
            if r.truthy() {
                idx = i as f64;
            }
            Ok(idx < 0.0)
        })?;
        Ok(Value::Num(idx))
    });
    def(vm, p, "reduce", |vm, this, a| {
        let items = this_items(vm, &this)?;
        let cb = arg(a, 0);
        let (acc, start) = if a.len() >= 2 {
            (a[1].clone(), 0)
        } else if items.is_empty() {
            return vm.type_error("Reduce of empty array with no initial value");
        } else {
            (items[0].clone(), 1)
        };
        with_roots(vm, vec![this, cb, acc], |vm, base| {
            let mut i = start;
            loop {
                let arr = vm.extra_roots[base].clone();
                if i >= vm.array_len(&arr) {
                    break;
                }
                let item = vm.array_get(&arr, i);
                let cb = vm.extra_roots[base + 1].clone();
                let acc = vm.extra_roots[base + 2].clone();
                let r = vm.call_value(&cb, Value::Undefined, &[acc, item, Value::Num(i as f64), arr])?;
                vm.extra_roots[base + 2] = r;
                i += 1;
            }
            Ok(vm.extra_roots[base + 2].clone())
        })
    });
    def(vm, p, "sort", |vm, this, a| {
        let n = this_items(vm, &this)?.len();
        let cmp = arg(a, 0);
        // Stable merge sort over *indices*, so no heap reference is held across a
        // comparator call (which may trigger a moving GC).
        with_roots(vm, vec![this, cmp], |vm, base| -> VResult<Value> {
            let mut idx: Vec<usize> = (0..n).collect();
            let mut width = 1;
            while width < n {
                let mut i = 0;
                while i < n {
                    let mid = (i + width).min(n);
                    let hi = (i + 2 * width).min(n);
                    let (mut l, mut r) = (i, mid);
                    let mut merged = Vec::with_capacity(hi - i);
                    while l < mid && r < hi {
                        let arr = &vm.extra_roots[base];
                        let (x, y) = (vm.array_get(arr, idx[l]), vm.array_get(arr, idx[r]));
                        if sort_gt(vm, base, &x, &y)? {
                            merged.push(idx[r]);
                            r += 1;
                        } else {
                            merged.push(idx[l]);
                            l += 1;
                        }
                    }
                    merged.extend_from_slice(&idx[l..mid]);
                    merged.extend_from_slice(&idx[r..hi]);
                    idx.splice(i..hi, merged);
                    i += 2 * width;
                }
                width *= 2;
            }
            let this = vm.extra_roots[base].clone();
            let cur = vm.array_items(&this);
            if cur.len() == n {
                let result: Vec<Value> = idx.iter().map(|&i| cur[i].clone()).collect();
                with_array(vm, &this, |items| *items = result)?;
            }
            Ok(this)
        })
    });
}

/// Should `y` come before `x`? (i.e. compare(x, y) > 0)
fn sort_gt(vm: &mut Vm, base: usize, x: &Value, y: &Value) -> VResult<bool> {
    let cmp = vm.extra_roots[base + 1].clone();
    if matches!(cmp, Value::Undefined) {
        if x.is_nullish() || y.is_nullish() {
            return Ok(matches!(x, Value::Undefined) && !matches!(y, Value::Undefined));
        }
        let a = vm.to_string(x);
        let b = vm.to_string(y);
        return Ok(a.encode_utf16().cmp(b.encode_utf16()) == std::cmp::Ordering::Greater);
    }
    let r = vm.call_value(&cmp, Value::Undefined, &[x.clone(), y.clone()])?;
    Ok(vm.to_number(&r) > 0.0)
}

// ---------------------------------------------------------------- String.prototype

fn this_str(vm: &mut Vm, this: &Value) -> Str {
    match this {
        Value::Str(s) => s.clone(),
        other => Rc::from(vm.to_string(other)),
    }
}

fn install_string_proto(vm: &mut Vm, p: &Value) {
    def(vm, p, "charAt", |vm, this, a| {
        let s = this_str(vm, &this);
        let i = vm.to_number(&arg(a, 0));
        let i = if i.is_nan() { 0.0 } else { i };
        Ok(Value::str(&if i < 0.0 {
            String::new()
        } else {
            s.chars().nth(i as usize).map(|c| c.to_string()).unwrap_or_default()
        }))
    });
    def(vm, p, "charCodeAt", |vm, this, a| {
        let s = this_str(vm, &this);
        let i = vm.to_number(&arg(a, 0));
        let i = if i.is_nan() { 0.0 } else { i };
        let u: Vec<u16> = s.encode_utf16().collect();
        Ok(Value::Num(if i < 0.0 {
            f64::NAN
        } else {
            u.get(i as usize).map(|c| *c as f64).unwrap_or(f64::NAN)
        }))
    });
    def(vm, p, "indexOf", |vm, this, a| {
        let s = this_str(vm, &this);
        let n = vm.to_string(&arg(a, 0));
        Ok(Value::Num(s.find(&n).map(|b| s[..b].chars().count() as f64).unwrap_or(-1.0)))
    });
    def(vm, p, "lastIndexOf", |vm, this, a| {
        let s = this_str(vm, &this);
        let n = vm.to_string(&arg(a, 0));
        Ok(Value::Num(s.rfind(&n).map(|b| s[..b].chars().count() as f64).unwrap_or(-1.0)))
    });
    def(vm, p, "includes", |vm, this, a| {
        let s = this_str(vm, &this);
        let n = vm.to_string(&arg(a, 0));
        Ok(Value::Bool(s.contains(&n)))
    });
    def(vm, p, "startsWith", |vm, this, a| {
        let s = this_str(vm, &this);
        let n = vm.to_string(&arg(a, 0));
        Ok(Value::Bool(s.starts_with(&n)))
    });
    def(vm, p, "endsWith", |vm, this, a| {
        let s = this_str(vm, &this);
        let n = vm.to_string(&arg(a, 0));
        Ok(Value::Bool(s.ends_with(&n)))
    });
    fn slice_impl(vm: &mut Vm, this: Value, a: &[Value], substring: bool) -> VResult<Value> {
        let s = this_str(vm, &this);
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        let (mut st, mut en) = if substring {
            let f = |vm: &mut Vm, v: &Value, d: usize| match v {
                Value::Undefined => d,
                v => {
                    let n = vm.to_number(v);
                    if n.is_nan() || n < 0.0 {
                        0
                    } else {
                        (n as usize).min(len)
                    }
                }
            };
            (f(vm, &arg(a, 0), 0), f(vm, &arg(a, 1), len))
        } else {
            (rel_index(vm, &arg(a, 0), len, 0), rel_index(vm, &arg(a, 1), len, len))
        };
        if substring && st > en {
            std::mem::swap(&mut st, &mut en);
        }
        Ok(Value::str(&if st < en {
            chars[st..en].iter().collect::<String>()
        } else {
            String::new()
        }))
    }
    def(vm, p, "slice", |vm, this, a| slice_impl(vm, this, a, false));
    def(vm, p, "substring", |vm, this, a| slice_impl(vm, this, a, true));
    def(vm, p, "toUpperCase", |vm, this, _| {
        Ok(Value::str(&this_str(vm, &this).to_uppercase()))
    });
    def(vm, p, "toLowerCase", |vm, this, _| {
        Ok(Value::str(&this_str(vm, &this).to_lowercase()))
    });
    def(vm, p, "trim", |vm, this, _| Ok(Value::str(this_str(vm, &this).trim())));
    def(vm, p, "trimStart", |vm, this, _| Ok(Value::str(this_str(vm, &this).trim_start())));
    def(vm, p, "trimEnd", |vm, this, _| Ok(Value::str(this_str(vm, &this).trim_end())));
    def(vm, p, "toString", |vm, this, _| Ok(Value::Str(this_str(vm, &this))));
    def(vm, p, "at", |vm, this, a| {
        let s = this_str(vm, &this);
        let chars: Vec<char> = s.chars().collect();
        let i = vm.to_number(&arg(a, 0)) as i64;
        let i = if i < 0 { chars.len() as i64 + i } else { i };
        Ok(if i < 0 || i as usize >= chars.len() {
            Value::Undefined
        } else {
            Value::str(&chars[i as usize].to_string())
        })
    });
    def(vm, p, "repeat", |vm, this, a| {
        let s = this_str(vm, &this);
        let n = vm.to_number(&arg(a, 0));
        if n < 0.0 || n.is_infinite() {
            return Err(vm.make_error("RangeError", &format!("Invalid count value: {}", num_to_string(n))));
        }
        Ok(Value::str(&s.repeat(n as usize)))
    });
    fn pad(vm: &mut Vm, this: Value, a: &[Value], start: bool) -> VResult<Value> {
        let s = this_str(vm, &this);
        let target = vm.to_number(&arg(a, 0)) as usize;
        let filler = match arg(a, 1) {
            Value::Undefined => " ".to_string(),
            v => vm.to_string(&v),
        };
        let len = s.chars().count();
        if target <= len || filler.is_empty() {
            return Ok(Value::Str(s));
        }
        let padding: String = filler.chars().cycle().take(target - len).collect();
        Ok(Value::str(&if start { format!("{padding}{s}") } else { format!("{s}{padding}") }))
    }
    def(vm, p, "padStart", |vm, this, a| pad(vm, this, a, true));
    def(vm, p, "padEnd", |vm, this, a| pad(vm, this, a, false));
    def(vm, p, "concat", |vm, this, a| {
        let mut s = this_str(vm, &this).to_string();
        for x in a {
            s += &vm.to_string(x);
        }
        Ok(Value::str(&s))
    });
    def(vm, p, "split", |vm, this, a| {
        let s = this_str(vm, &this);
        let parts: Vec<Value> = match arg(a, 0) {
            Value::Undefined => vec![Value::Str(s.clone())],
            sep => {
                let sep = vm.to_string(&sep);
                if sep.is_empty() {
                    s.chars().map(|c| Value::str(&c.to_string())).collect()
                } else {
                    s.split(sep.as_str()).map(Value::str).collect()
                }
            }
        };
        Ok(vm.new_array(parts))
    });
    def(vm, p, "replace", |vm, this, a| {
        let s = this_str(vm, &this);
        let pat = vm.to_string(&arg(a, 0));
        let rep = vm.to_string(&arg(a, 1));
        Ok(Value::str(&s.replacen(&pat, &rep, 1)))
    });
    def(vm, p, "replaceAll", |vm, this, a| {
        let s = this_str(vm, &this);
        let pat = vm.to_string(&arg(a, 0));
        let rep = vm.to_string(&arg(a, 1));
        Ok(Value::str(&s.replace(&pat, &rep)))
    });
}

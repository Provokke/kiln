//! Runtime values and number formatting.

use crate::heap::GcRef;
use std::rc::Rc;

pub type Str = Rc<str>;

#[derive(Clone, Debug)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(Str),
    Obj(GcRef),
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Obj(_) => true,
        }
    }
    pub fn as_obj(&self) -> Option<GcRef> {
        match self {
            Value::Obj(r) => Some(*r),
            _ => None,
        }
    }
    pub fn is_nullish(&self) -> bool {
        matches!(self, Value::Undefined | Value::Null)
    }
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }
}

/// Strict equality (`===`).
pub fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Obj(x), Value::Obj(y)) => x == y,
        _ => false,
    }
}

/// Convert a string to a number following the JS `ToNumber` rules (subset).
pub fn str_to_num(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return i64::from_str_radix(h, 16).map(|v| v as f64).unwrap_or(f64::NAN);
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if t.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')) {
        t.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// Format a number exactly like JavaScript's `Number.prototype.toString()`.
pub fn num_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n == 0.0 {
        return "0".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if n < 0.0 {
        return format!("-{}", num_to_string(-n));
    }
    // Shortest round-trip digits via Rust's formatter in exponential form.
    let e = format!("{n:e}"); // e.g. "1.2345e3"
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n_exp = exp + 1; // position of decimal point relative to digits
    if k <= n_exp && n_exp <= 21 {
        format!("{digits}{}", "0".repeat((n_exp - k) as usize))
    } else if 0 < n_exp && n_exp <= 21 {
        format!("{}.{}", &digits[..n_exp as usize], &digits[n_exp as usize..])
    } else if -6 < n_exp && n_exp <= 0 {
        format!("0.{}{digits}", "0".repeat((-n_exp) as usize))
    } else {
        let sign = if n_exp - 1 < 0 { '-' } else { '+' };
        let ex = (n_exp - 1).abs();
        if k == 1 {
            format!("{digits}e{sign}{ex}")
        } else {
            format!("{}.{}e{sign}{ex}", &digits[..1], &digits[1..])
        }
    }
}

/// ToInt32 as used by bitwise operators.
pub fn to_int32(n: f64) -> i32 {
    if !n.is_finite() {
        return 0;
    }
    let t = n.trunc();
    let m = t.rem_euclid(4294967296.0);
    m as u32 as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn number_formatting_matches_js() {
        let cases = [
            (1.0, "1"),
            (-1.5, "-1.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (123456789012.0, "123456789012"),
            (1e-7, "1e-7"),
            (0.000001, "0.000001"),
            (1.5e-10, "1.5e-10"),
            (2.5e25, "2.5e+25"),
            (f64::NAN, "NaN"),
            (-0.0, "0"),
            (f64::INFINITY, "Infinity"),
            (2.71, "2.71"),
            (100.0, "100"),
        ];
        for (n, s) in cases {
            assert_eq!(num_to_string(n), s, "{n}");
        }
    }
    #[test]
    fn int32() {
        assert_eq!(to_int32(4294967297.0), 1);
        assert_eq!(to_int32(-1.0), -1);
        assert_eq!(to_int32(2147483648.0), -2147483648);
        assert_eq!(to_int32(f64::NAN), 0);
    }
}

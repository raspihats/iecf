//! STRING (Ed.3 6.4.2, 6.6.2.5.11): a `STRING(n)` is `char V_X[n + 1]`,
//! the characters then a 0, one byte each, Latin-1 (docs/dependencies.md).
//!
//! A string expression is a C `const char *`. A function's result is
//! written into a buffer of the statement, `char s3[n + 1]`, whose `n`
//! is known here: the most its inputs can make, so a result is never cut
//! short on its way to the variable it is stored in - only that store
//! can be too long. A string too long, cut to fit, and a length or
//! position out of its string are flagged, not faulted (decided
//! 2026-11-06, runtime/iecf.h).

use crate::derived::Kind;
use crate::names::Scope;
use crate::{Gen, R};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;

const FLAG: &str = "&ctx->flag";

/// The longest STRING (docs/dependencies.md).
pub(crate) const MAX: u32 = 65535;

/// `bytes` as a C string literal: printable ASCII as itself, the rest in
/// octal, so no escape runs into the character after it.
pub(crate) fn c_literal(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' | b'\\' | b'?' => s.push_str(&format!("\\{:03o}", b)),
            0x20..=0x7E => s.push(b as char),
            _ => s.push_str(&format!("\\{:03o}", b)),
        }
    }
    s.push('"');
    s
}

impl Gen<'_> {
    /// Whether `e` is a STRING (not a WSTRING).
    pub(crate) fn is_string(&mut self, e: &Expr) -> R<bool> {
        Ok(matches!(self.ty_of(e)?, Ty::Str { wide: false, .. }))
    }

    /// A buffer of the statement for a result of up to `n` characters.
    fn buffer(&mut self, n: u32) -> String {
        let s = self.fresh("s");
        self.str_temps.push(format!("char {s}[{}];", n as u64 + 1));
        s
    }

    /// `e`, a STRING expression: C `const char *` and the most characters
    /// it can hold.
    pub(crate) fn string(&mut self, e: &Expr, scope: &Scope) -> R<(String, u32)> {
        if let Some(crate::enable::Sub::Str(c, n)) = self.subst.get(&e.span) {
            return Ok((c.clone(), *n));
        }
        match &e.kind {
            ExprKind::Literal(l) if l.kind == LiteralKind::String => {
                match iecf_sema::strings::string_bytes(&l.text) {
                    Ok(b) if b.contains(&0) => {
                        // '$00' ends the string where it stands.
                        let end = b.iter().position(|c| *c == 0).unwrap_or(b.len());
                        Ok((c_literal(&b[..end]), end as u32))
                    }
                    Ok(b) => Ok((c_literal(&b), b.len().min(MAX as usize) as u32)),
                    Err(_) => self.not_yet(e.span, "this string literal"),
                }
            }
            ExprKind::Paren(x) => self.string(x, scope),
            ExprKind::Name(_) | ExprKind::Member(..) | ExprKind::Index(..) => match self.data_path(e, scope)? {
                Some((lv, t)) => match t.kind {
                    Kind::Str(n) => Ok((lv, n)),
                    _ => self.not_yet(e.span, "this string"),
                },
                None => self.not_yet(e.span, "this string"),
            },
            ExprKind::Call { args, .. } => self.string_of_call(e, args, scope),
            _ => self.not_yet(e.span, "this string expression"),
        }
    }

    /// LEFT, RIGHT, MID, CONCAT, INSERT, DELETE, REPLACE (Ed.3 Table 34).
    pub(crate) fn string_of_call(&mut self, e: &Expr, args: &[Arg], scope: &Scope) -> R<(String, u32)> {
        if let Some(r) = self.gated(e, args, scope, true)? {
            return Ok(r);
        }
        let Some(sc) = self.types.std_call(e.span).cloned() else {
            return self.not_yet(e.span, "a FUNCTION of the project giving a STRING");
        };
        if sc.name == "TO_STRING" {
            return self.text_of(e, "", args, scope);
        }
        if let Some(from) = sc.name.strip_suffix("_TO_STRING") {
            return self.text_of(e, from, args, scope);
        }
        if !matches!(sc.name.as_str(), "LEFT" | "RIGHT" | "MID" | "DELETE" | "INSERT" | "REPLACE" | "CONCAT") {
            return self.not_yet(e.span, format!("the standard function {}", sc.name));
        }
        if sc.outputs.iter().any(|(_, s)| s.is_some()) {
            return self.not_yet(e.span, "outputs of a standard function");
        }
        let input =
            |name: &str| sc.inputs.iter().find(|(n, _)| n == name).and_then(|(_, s)| crate::expr::arg_value(args, *s));
        let int = |g: &mut Self, name: &str| -> R<String> {
            match input(name) {
                Some(x) => Ok(format!("(int64_t)({})", g.value_as(x, Elem::Lint, scope)?)),
                None => g.not_yet(e.span, format!("{} without {name}", sc.name)),
            }
        };
        let s = |g: &mut Self, name: &str| -> R<(String, u32)> {
            match input(name) {
                Some(x) => g.string(x, scope),
                None => g.not_yet(e.span, format!("{} without {name}", sc.name)),
            }
        };
        let sum = |a: u32, b: u32| a.saturating_add(b).min(MAX);
        // Too long, or a length or position out of the string: flagged,
        // never a fault (runtime/iecf.h).
        self.may_flag = true;
        let f = sc.name.to_ascii_lowercase();
        match sc.name.as_str() {
            "LEFT" | "RIGHT" => {
                let (i, n) = s(self, "IN")?;
                let l = int(self, "L")?;
                let d = self.buffer(n);
                Ok((format!("iecf_{f}({FLAG}, {d}, {n}u, {i}, {l})"), n))
            }
            "MID" | "DELETE" => {
                let (i, n) = s(self, "IN")?;
                let (l, p) = (int(self, "L")?, int(self, "P")?);
                let d = self.buffer(n);
                Ok((format!("iecf_{f}({FLAG}, {d}, {n}u, {i}, {l}, {p})"), n))
            }
            "INSERT" => {
                let ((a, na), (b, nb)) = (s(self, "IN1")?, s(self, "IN2")?);
                let p = int(self, "P")?;
                let n = sum(na, nb);
                let d = self.buffer(n);
                Ok((format!("iecf_insert({FLAG}, {d}, {n}u, {a}, {b}, {p})"), n))
            }
            "REPLACE" => {
                let ((a, na), (b, nb)) = (s(self, "IN1")?, s(self, "IN2")?);
                let (l, p) = (int(self, "L")?, int(self, "P")?);
                let n = sum(na, nb);
                let d = self.buffer(n);
                Ok((format!("iecf_replace({FLAG}, {d}, {n}u, {a}, {b}, {l}, {p})"), n))
            }
            "CONCAT" => {
                let mut all = Vec::new();
                for (_, span) in &sc.inputs {
                    if let Some(x) = crate::expr::arg_value(args, *span) {
                        if !self.is_string(x)? {
                            return self.not_yet(x.span, "a CHAR in CONCAT");
                        }
                        all.push(self.string(x, scope)?);
                    }
                }
                let Some(((first, mut n), rest)) = all.split_first().map(|(a, r)| (a.clone(), r)) else {
                    return self.not_yet(e.span, "this CONCAT");
                };
                let mut acc = first;
                for (x, nx) in rest {
                    n = sum(n, *nx);
                    let d = self.buffer(n);
                    acc = format!("iecf_concat({FLAG}, {d}, {n}u, {acc}, {x})");
                }
                Ok((acc, n))
            }
            name => self.not_yet(e.span, format!("the standard function {name}")),
        }
    }

    /// `<FROM>_TO_STRING(x)`, `TO_STRING(x)`: the text of `x` in a buffer
    /// of the statement long enough for any value of its type.
    fn text_of(&mut self, e: &Expr, from: &str, args: &[Arg], scope: &Scope) -> R<(String, u32)> {
        let x = match args.first() {
            Some(Arg::Positional(x) | Arg::Input(_, x)) => x,
            _ => return self.not_yet(e.span, "this conversion"),
        };
        let t = match Elem::from_name(from) {
            Some(t) => t,
            None if from.is_empty() => self.elem_of(x)?,
            None => return self.not_yet(e.span, format!("{from}_TO_STRING")),
        };
        // The text each type gives (docs/dependencies.md): decimal for the
        // integers and bit strings, C's %g for the reals, the literal for
        // the durations; `n` the longest.
        let v = self.value_as(x, t, scope)?;
        // `@` stands for the buffer.
        let (code, n) = match t {
            Elem::Bool => (format!("iecf_bool_str(@, {v})"), 5),
            t if t.is_signed_int() => (format!("iecf_s_str(@, (int64_t)({v}))"), 20),
            t if t.is_int() || t.is_bit() => (format!("iecf_u_str(@, (uint64_t)({v}))"), 20),
            Elem::Real => (format!("iecf_real_str(@, (double)({v}), 7u)"), 24),
            Elem::Lreal => (format!("iecf_real_str(@, {v}, 15u)"), 24),
            Elem::Time => (format!("iecf_time_str(@, (int64_t)({v}), 0)"), 40),
            Elem::Ltime => (format!("iecf_time_str(@, (int64_t)({v}), 1)"), 40),
            Elem::Date => (format!("iecf_date_str(@, {v})"), 12),
            Elem::Tod => (format!("iecf_tod_str(@, {v})"), 18),
            Elem::Dt => (format!("iecf_dt_str(@, {v})"), 22),
            _ => return self.not_yet(e.span, format!("{}_TO_STRING", t.name())),
        };
        let d = self.buffer(n);
        Ok((code.replacen('@', &d, 1), n))
    }

    /// `STRING_TO_<T>(s)`, `TO_<T>(s)`: the text read as a literal of `T`;
    /// none, or one `T` cannot hold, gives 0, flagged (runtime/iecf.h).
    pub(crate) fn string_from(&mut self, e: &Expr, name: &str, all: &[&Expr], scope: &Scope) -> R<Option<String>> {
        let to = match name.strip_prefix("STRING_TO_") {
            Some(t) => t,
            None => match name.strip_prefix("TO_") {
                Some(t) if all.len() == 1 && self.is_string(all[0])? => t,
                _ => return Ok(None),
            },
        };
        let Some(t) = Elem::from_name(to) else {
            return self.not_yet(e.span, format!("STRING_TO_{to}"));
        };
        let (s, _) = self.string(all[0], scope)?;
        let ct = crate::c_type(t);
        self.may_flag = true;
        let code = match t {
            Elem::Bool => format!("iecf_str_bool({FLAG}, {s})"),
            t if t.is_signed_int() => {
                let (lo, hi) = t.int_range().unwrap_or_default();
                let lo = if lo == i64::MIN as i128 { "INT64_MIN".to_string() } else { format!("INT64_C({lo})") };
                format!("(({ct})iecf_str_s({FLAG}, {s}, {lo}, INT64_C({hi})))")
            }
            t if t.is_int() || t.is_bit() => {
                let hi = crate::expr::repr(t).and_then(|r| r.int_range()).map_or(0, |(_, hi)| hi);
                format!("(({ct})iecf_str_u({FLAG}, {s}, UINT64_C({hi})))")
            }
            Elem::Real => format!("(({ct})iecf_str_real({FLAG}, {s}, 24u))"),
            Elem::Lreal => format!("iecf_str_real({FLAG}, {s}, 53u)"),
            Elem::Time => format!("(({ct})iecf_str_time({FLAG}, {s}, 0))"),
            Elem::Ltime => format!("(({ct})iecf_str_time({FLAG}, {s}, 1))"),
            Elem::Date => format!("iecf_str_date({FLAG}, {s})"),
            Elem::Tod => format!("iecf_str_tod({FLAG}, {s})"),
            Elem::Dt => format!("iecf_str_dt({FLAG}, {s})"),
            _ => return self.not_yet(e.span, format!("STRING_TO_{to}")),
        };
        Ok(Some(code))
    }

    /// LEN and FIND, which give an INT.
    pub(crate) fn string_int(&mut self, e: &Expr, name: &str, all: &[&Expr], scope: &Scope) -> R<Option<String>> {
        let code = match name {
            "LEN" => {
                if !self.is_string(all[0])? {
                    return Ok(None);
                }
                let (s, _) = self.string(all[0], scope)?;
                format!("iecf_slen({s})")
            }
            "FIND" => {
                if !(self.is_string(all[0])? && self.is_string(all[1])?) {
                    return Ok(None);
                }
                let ((a, _), (b, _)) = (self.string(all[0], scope)?, self.string(all[1], scope)?);
                format!("iecf_find({a}, {b})")
            }
            _ => return Ok(None),
        };
        // At most 65535: an INT holds it when the string is up to 32767.
        let to = self.elem_of(e)?;
        self.convert(e.span, code, Elem::Udint, to).map(Some)
    }

    /// A comparison of strings: C's `iecf_scmp` against 0.
    pub(crate) fn string_compare(&mut self, sym: &str, a: &Expr, b: &Expr, scope: &Scope) -> R<String> {
        let ((x, _), (y, _)) = (self.string(a, scope)?, self.string(b, scope)?);
        Ok(format!("((IEC_BOOL)(iecf_scmp({x}, {y}) {sym} 0))"))
    }
}

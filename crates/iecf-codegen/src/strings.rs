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
            ExprKind::Call { args, .. } => self.string_call(e, args, scope),
            _ => self.not_yet(e.span, "this string expression"),
        }
    }

    /// LEFT, RIGHT, MID, CONCAT, INSERT, DELETE, REPLACE (Ed.3 Table 34).
    fn string_call(&mut self, e: &Expr, args: &[Arg], scope: &Scope) -> R<(String, u32)> {
        let Some(sc) = self.types.std_call(e.span).cloned() else {
            return self.not_yet(e.span, "a FUNCTION of the project giving a STRING");
        };
        if !matches!(sc.name.as_str(), "LEFT" | "RIGHT" | "MID" | "DELETE" | "INSERT" | "REPLACE" | "CONCAT") {
            return self.not_yet(e.span, format!("the standard function {}", sc.name));
        }
        if sc.outputs.iter().any(|(_, s)| s.is_some())
            || args.iter().any(|a| match a {
                Arg::Input(n, _) | Arg::Output { name: n, .. } => n.is("EN") || n.is("ENO"),
                _ => false,
            })
        {
            return self.not_yet(e.span, "EN, ENO or outputs of a standard function");
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

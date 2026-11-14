//! STRING and WSTRING (Ed.3 6.4.2, 6.6.2.5.11): a `STRING(n)` is `char
//! V_X[n + 1]`, the characters then a 0, one byte each, Latin-1; a
//! `WSTRING(n)` `IEC_WCHAR V_X[n + 1]`, UTF-16 units then a 0
//! (docs/dependencies.md). The runtime's functions are the same for both,
//! a WSTRING's named with a `w`: `iecf_concat`, `iecf_wconcat`.
//!
//! A string expression is a C `const char *` or `const IEC_WCHAR *`. A function's result is
//! written into a buffer of the statement, `char s3[n + 1]`, whose `n`
//! is known here: the most its inputs can make, so a result is never cut
//! short on its way to the variable it is stored in - only that store
//! can be too long. A string too long, cut to fit, and a length or
//! position out of its string are flagged, not faulted (decided
//! 2026-11-06, runtime/iecf.h).

use crate::derived::Kind;
use crate::names::{Lv, Scope};
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

/// The C type of a string's unit: `char`, or `IEC_WCHAR` for a WSTRING.
pub(crate) fn unit(wide: bool) -> &'static str {
    if wide { "IEC_WCHAR" } else { "char" }
}

/// The infix of the runtime's functions for a WSTRING: `iecf_wconcat`.
pub(crate) fn infix(wide: bool) -> &'static str {
    if wide { "w" } else { "" }
}

/// The units of a string literal `text`, quotes included: bytes for a
/// STRING, UTF-16 units for a WSTRING (iecf-sema's strings.rs).
pub(crate) fn literal_units(text: &str, wide: bool) -> Result<Vec<u16>, String> {
    if wide {
        iecf_sema::strings::wstring_units(text)
    } else {
        iecf_sema::strings::string_bytes(text).map(|b| b.into_iter().map(u16::from).collect())
    }
}

/// `units` as a C string: a string literal for a STRING, a compound
/// literal for a WSTRING, which C99 has no literal of.
pub(crate) fn c_units(units: &[u16], wide: bool) -> String {
    if !wide {
        let b: Vec<u8> = units.iter().map(|u| *u as u8).collect();
        return c_literal(&b);
    }
    let mut s = String::from("((const IEC_WCHAR[]){");
    for u in units {
        s.push_str(&format!("{u}u, "));
    }
    s.push_str("0u})");
    s
}

/// Where a WSTRING is that is not a C array of its units: in the image,
/// its bytes, any byte, read and written unit by unit (runtime/iecf.h,
/// `iecf_wimg_get`); through a reference, which the run knows to be in C
/// memory or in the image (`iecf_wref_get`).
pub(crate) enum WideAt {
    Image(String),
    Ref(String),
}

/// Where the WSTRING at `lv` is, when it is not a C array.
pub(crate) fn wide_at(lv: &Lv, wide: bool) -> Option<WideAt> {
    match lv {
        Lv::Bytes { area, off, .. } if wide => Some(WideAt::Image(format!("({} + {off})", area.ptr()))),
        Lv::Ref(r) if wide => Some(WideAt::Ref(r.clone())),
        _ => None,
    }
}

impl WideAt {
    /// The WSTRING(n)'s units, an `IEC_WCHAR *`: copied into the buffer
    /// `d` if they must be.
    pub(crate) fn get(&self, d: &str, n: u32) -> String {
        match self {
            WideAt::Image(p) => format!("iecf_wimg_get({d}, {n}u, {p})"),
            WideAt::Ref(r) => format!("iecf_wref_get({d}, {n}u, &ctx->image, {r})"),
        }
    }

    /// A statement writing the n + 1 units at `s` into it.
    pub(crate) fn put(&self, n: u32, s: &str) -> String {
        match self {
            WideAt::Image(p) => format!("iecf_wimg_put({p}, {n}u, {s});"),
            WideAt::Ref(r) => format!("iecf_wref_put(&ctx->image, {r}, {n}u, {s});"),
        }
    }
}

/// `stmt`, a statement storing into the string `@`, for a STRING(n) or
/// WSTRING(n) at `lv`. A WSTRING in the image: into a copy of it, then
/// written back whole; through a reference, the reference taken once,
/// then the same if it is to the image.
pub(crate) fn store_string(lv: &Lv, n: u32, wide: bool, stmt: &str) -> String {
    match wide_at(lv, wide) {
        Some(WideAt::Ref(r)) => {
            let at = WideAt::Ref("wr".into());
            format!(
                "{{ iecf_ref_t wr = {r}; IEC_WCHAR wimg[{}]; IEC_WCHAR *wp = {}; {} {} }}",
                n as u64 + 1,
                at.get("wimg", n),
                stmt.replace('@', "wp"),
                at.put(n, "wp")
            )
        }
        Some(at) => format!(
            "{{ IEC_WCHAR wimg[{}]; {}; {} {} }}",
            n as u64 + 1,
            at.get("wimg", n),
            stmt.replace('@', "wimg"),
            at.put(n, "wimg")
        ),
        None => stmt.replace('@', &lv.string(n, wide)),
    }
}

impl Gen<'_> {
    /// Whether `e` is a STRING or a WSTRING.
    pub(crate) fn is_string(&mut self, e: &Expr) -> R<bool> {
        Ok(matches!(self.ty_of(e)?, Ty::Str { .. }))
    }

    /// Whether `e` is of a double-byte type, a WSTRING or a WCHAR.
    pub(crate) fn is_wide(&mut self, e: &Expr) -> R<bool> {
        Ok(matches!(self.ty_of(e)?, Ty::Str { wide: true, .. } | Ty::Elem(Elem::Wchar)))
    }

    /// A buffer of the statement for a result of up to `n` characters.
    pub(crate) fn buffer(&mut self, n: u32, wide: bool) -> String {
        let s = self.fresh("s");
        self.str_temps.push(format!("{} {s}[{}];", unit(wide), n as u64 + 1));
        s
    }

    /// `e`, a STRING or WSTRING expression: C `const char *` or `const
    /// IEC_WCHAR *`, and the most characters it can hold.
    pub(crate) fn string(&mut self, e: &Expr, scope: &Scope) -> R<(String, u32)> {
        if let Some(crate::enable::Sub::Str(c, n)) = self.subst.get(&e.span) {
            return Ok((c.clone(), *n));
        }
        // A CHAR where a STRING goes, a WCHAR where a WSTRING goes (Ed.3
        // Figure 11: implicit).
        if let Ty::Elem(c @ (Elem::Char | Elem::Wchar)) = self.ty_of(e)? {
            let w = c == Elem::Wchar;
            let v = self.value(e, scope)?;
            let b = self.fresh("c");
            self.str_temps.push(format!("{} {b}[2];", unit(w)));
            return Ok((format!("iecf_{}char_str({b}, {v})", infix(w)), 1));
        }
        let wide = self.is_wide(e)?;
        match &e.kind {
            ExprKind::Literal(l) if matches!(l.kind, LiteralKind::String | LiteralKind::WString) => {
                match literal_units(&l.text, wide) {
                    Ok(u) => {
                        // '$00' ends the string where it stands.
                        let end = u.iter().position(|c| *c == 0).unwrap_or(u.len());
                        Ok((c_units(&u[..end], wide), end.min(MAX as usize) as u32))
                    }
                    Err(_) => self.not_yet(e.span, "this string literal"),
                }
            }
            ExprKind::Paren(x) => self.string(x, scope),
            ExprKind::Name(_) | ExprKind::Member(..) | ExprKind::Index(..) | ExprKind::Deref(_) => {
                match self.data_path(e, scope)? {
                    Some((lv, t)) => match t.kind {
                        Kind::Str(n, w) => match wide_at(&lv, w) {
                            Some(at) => {
                                let b = self.buffer(n, true);
                                Ok((at.get(&b, n), n))
                            }
                            None => Ok((lv.string(n, w), n)),
                        },
                        _ => self.not_yet(e.span, "this string"),
                    },
                    None => self.not_yet(e.span, "this string"),
                }
            }
            ExprKind::Call { args, .. } => self.string_of_call(e, args, scope),
            _ => self.not_yet(e.span, "this string expression"),
        }
    }

    /// `s[i]` read, a CHAR: `STRING_TO_CHAR(MID(s, 1, i))` (Ed.3 6.4.2),
    /// flagged where `i` is not in the string (runtime/iecf.h).
    pub(crate) fn string_char(&mut self, s: &Expr, i: &Expr, scope: &Scope) -> R<String> {
        let w = infix(self.is_wide(s)?);
        let (s, _) = self.string(s, scope)?;
        let p = self.value_as(i, Elem::Lint, scope)?;
        self.may_flag = true;
        Ok(format!("iecf_{w}schar({FLAG}, {s}, (int64_t)({p}))"))
    }

    /// `s[i] := c`, `s` at `lv`: the character replaced; a position not
    /// in the string changes nothing, flagged (runtime/iecf.h).
    pub(crate) fn put_char(&mut self, lv: &str, wide: bool, p: &str, c: &str) -> String {
        format!("iecf_{}sput({FLAG}, {lv}, (int64_t)({p}), {c});", infix(wide))
    }

    /// LEFT, RIGHT, MID, CONCAT, INSERT, DELETE, REPLACE (Ed.3 Table 34).
    pub(crate) fn string_of_call(&mut self, e: &Expr, args: &[Arg], scope: &Scope) -> R<(String, u32)> {
        if let Some(r) = self.gated(e, args, scope, true)? {
            return Ok(r);
        }
        let Some(sc) = self.types.std_call(e.span).cloned() else {
            // The project's FUNCTION or method: its result in a buffer of
            // the statement's, returned (params.rs).
            let ExprKind::Call { callee, .. } = &e.kind else { return self.not_yet(e.span, "this string") };
            let c = self.user_call(e, callee, args, scope)?;
            let n = match self.called_sig(e).and_then(|s| s.ret_data) {
                Some(crate::derived::DTy { kind: Kind::Str(n, _), .. }) => n,
                _ => return self.not_yet(e.span, "a FUNCTION of the project giving a STRING"),
            };
            return Ok((c, n));
        };
        let wide = self.is_wide(e)?;
        if sc.name == "TO_STRING" || sc.name == "TO_WSTRING" {
            return self.text_of(e, "", wide, args, scope);
        }
        if let Some(from) = sc.name.strip_suffix(if wide { "_TO_WSTRING" } else { "_TO_STRING" }) {
            return self.text_of(e, from, wide, args, scope);
        }
        if let Some(r) = self.string_select(e, &sc, args, wide, scope)? {
            return Ok(r);
        }
        if !matches!(sc.name.as_str(), "LEFT" | "RIGHT" | "MID" | "DELETE" | "INSERT" | "REPLACE" | "CONCAT") {
            return self.not_yet(e.span, format!("the standard function {}", sc.called));
        }
        if sc.outputs.iter().any(|(_, s)| s.is_some()) {
            return self.not_yet(e.span, "outputs of a standard function");
        }
        let input = |name: &str| sc.inputs.iter().find(|(n, _)| n == name).and_then(|(_, s)| sc.input(args, *s));
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
        let w = infix(wide);
        let f = format!("{w}{}", sc.name.to_ascii_lowercase());
        match sc.name.as_str() {
            "LEFT" | "RIGHT" => {
                let (i, n) = s(self, "IN")?;
                let l = int(self, "L")?;
                let d = self.buffer(n, wide);
                Ok((format!("iecf_{f}({FLAG}, {d}, {n}u, {i}, {l})"), n))
            }
            "MID" | "DELETE" => {
                let (i, n) = s(self, "IN")?;
                let (l, p) = (int(self, "L")?, int(self, "P")?);
                let d = self.buffer(n, wide);
                Ok((format!("iecf_{f}({FLAG}, {d}, {n}u, {i}, {l}, {p})"), n))
            }
            "INSERT" => {
                let ((a, na), (b, nb)) = (s(self, "IN1")?, s(self, "IN2")?);
                let p = int(self, "P")?;
                let n = sum(na, nb);
                let d = self.buffer(n, wide);
                Ok((format!("iecf_{w}insert({FLAG}, {d}, {n}u, {a}, {b}, {p})"), n))
            }
            "REPLACE" => {
                let ((a, na), (b, nb)) = (s(self, "IN1")?, s(self, "IN2")?);
                let (l, p) = (int(self, "L")?, int(self, "P")?);
                let n = sum(na, nb);
                let d = self.buffer(n, wide);
                Ok((format!("iecf_{w}replace({FLAG}, {d}, {n}u, {a}, {b}, {l}, {p})"), n))
            }
            "CONCAT" => {
                let mut all = Vec::new();
                for (_, span) in &sc.inputs {
                    if let Some(x) = sc.input(args, *span) {
                        all.push(self.string(x, scope)?);
                    }
                }
                let Some(((first, mut n), rest)) = all.split_first().map(|(a, r)| (a.clone(), r)) else {
                    return self.not_yet(e.span, "this CONCAT");
                };
                let mut acc = first;
                for (x, nx) in rest {
                    n = sum(n, *nx);
                    let d = self.buffer(n, wide);
                    acc = format!("iecf_{w}concat({FLAG}, {d}, {n}u, {acc}, {x})");
                }
                Ok((acc, n))
            }
            name => self.not_yet(e.span, format!("the standard function {name}")),
        }
    }

    /// MOVE, SEL, MUX, MIN, MAX and LIMIT of strings (Ed.3 Tables 31, 32):
    /// every input evaluated, as any function's, and one of them given,
    /// where it is; the most it can hold the most of any (runtime/iecf.h).
    fn string_select(
        &mut self,
        e: &Expr,
        sc: &iecf_sema::StdCall,
        args: &[Arg],
        wide: bool,
        scope: &Scope,
    ) -> R<Option<(String, u32)>> {
        let name = sc.name.as_str();
        if !matches!(name, "MOVE" | "SEL" | "MUX" | "MIN" | "MAX" | "LIMIT") {
            return Ok(None);
        }
        let all: Vec<&Expr> = sc.inputs.iter().filter_map(|(_, s)| sc.input(args, *s)).collect();
        let input = |name: &str| sc.inputs.iter().find(|(n, _)| n == name).and_then(|(_, s)| sc.input(args, *s));
        let w = infix(wide);
        let strings = |g: &mut Self, xs: &[&Expr]| -> R<(Vec<String>, u32)> {
            let mut out = Vec::new();
            let mut n = 0;
            for x in xs {
                let (c, k) = g.string(x, scope)?;
                out.push(c);
                n = n.max(k);
            }
            Ok((out, n))
        };
        let r = match name {
            "MOVE" => self.string(all[0], scope)?,
            "SEL" => {
                let (Some(g), Some(a), Some(b)) = (input("G"), input("IN0"), input("IN1")) else {
                    return self.not_yet(e.span, "this SEL");
                };
                let g = self.value_as(g, Elem::Bool, scope)?;
                let (v, n) = strings(self, &[a, b])?;
                (format!("iecf_{w}ssel({g}, {}, {})", v[0], v[1]), n)
            }
            "MIN" | "MAX" => {
                let f = name.to_ascii_lowercase();
                let (v, n) = strings(self, &all)?;
                let mut acc = v[0].clone();
                for x in &v[1..] {
                    acc = format!("iecf_{w}s{f}({acc}, {x})");
                }
                (acc, n)
            }
            "LIMIT" => {
                let (Some(mn), Some(i), Some(mx)) = (input("MN"), input("IN"), input("MX")) else {
                    return self.not_yet(e.span, "this LIMIT");
                };
                let (v, n) = strings(self, &[mn, i, mx])?;
                (format!("iecf_{w}slimit({}, {}, {})", v[0], v[1], v[2]), n)
            }
            _ => {
                let Some((k, ins)) = all.split_first() else { return self.not_yet(e.span, "this MUX") };
                let k = match self.ty_of(k)? {
                    Ty::Elem(Elem::Ulint) => format!("iecf_mux_k((uint64_t)({}))", self.value(k, scope)?),
                    Ty::Elem(t) if t.is_int() => format!("(int64_t)({})", self.value(k, scope)?),
                    _ => self.value_as(k, Elem::Lint, scope)?,
                };
                let (v, n) = strings(self, ins)?;
                self.may_fault = true;
                let u = unit(wide);
                (format!("iecf_{w}smux(&ctx->fault, {k}, {}u, (const {u} *const[]){{{}}})", v.len(), v.join(", ")), n)
            }
        };
        Ok(Some(r))
    }

    /// `<FROM>_TO_STRING(x)`, `TO_STRING(x)`: the text of `x` in a buffer
    /// of the statement long enough for any value of its type; `wide`,
    /// `<FROM>_TO_WSTRING(x)` and `TO_WSTRING(x)`, the same text in
    /// UTF-16. Between the string and character types, Ed.3 Table 27.
    fn text_of(&mut self, e: &Expr, from: &str, wide: bool, args: &[Arg], scope: &Scope) -> R<(String, u32)> {
        let x = match args.first() {
            Some(Arg::Positional(x) | Arg::Input(_, x)) => x,
            _ => return self.not_yet(e.span, "this conversion"),
        };
        let xt = self.ty_of(x)?;
        let t = match (Elem::from_name(from), &xt) {
            (Some(t), _) => t,
            (None, Ty::Str { wide: true, .. }) if from.is_empty() || from == "WSTRING" => Elem::Wstring,
            (None, Ty::Str { wide: false, .. }) if from.is_empty() || from == "STRING" => Elem::String,
            (None, _) if from.is_empty() => self.elem_of(x)?,
            (None, _) => return self.not_yet(e.span, format!("{from}_TO_STRING")),
        };
        match (t, wide) {
            // CHAR_TO_STRING, WCHAR_TO_WSTRING: the character, not a text
            // of its code (Table 27); a string to its own type, itself.
            (Elem::Char | Elem::String, false) | (Elem::Wchar | Elem::Wstring, true) => return self.string(x, scope),
            // WSTRING_TO_STRING: Latin-1 kept, any other character skipped,
            // flagged (runtime/iecf.h).
            (Elem::Wstring, false) => {
                let (s, n) = self.string(x, scope)?;
                let d = self.buffer(n, false);
                self.may_flag = true;
                return Ok((format!("iecf_wstr_str({FLAG}, {d}, {n}u, {s})"), n));
            }
            // STRING_TO_WSTRING, and through it CHAR_TO_WSTRING and the
            // text of any other type: each Latin-1 code its UTF-16 unit.
            (_, true) => {
                let (s, n) = if t == Elem::String || t == Elem::Char {
                    self.string(x, scope)?
                } else {
                    self.text_of(e, t.name(), false, args, scope)?
                };
                let d = self.buffer(n, true);
                return Ok((format!("iecf_str_wstr({d}, {n}u, {s})"), n));
            }
            _ => {}
        }
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
            Elem::Ldate => (format!("iecf_ldate_str(@, {v})"), 13),
            Elem::Ltod => (format!("iecf_ltod_str(@, {v})"), 23),
            Elem::Ldt => (format!("iecf_ldt_str(@, {v})"), 33),
            _ => return self.not_yet(e.span, format!("{}_TO_STRING", t.name())),
        };
        let d = self.buffer(n, false);
        Ok((code.replacen('@', &d, 1), n))
    }

    /// `STRING_TO_<T>(s)`, `TO_<T>(s)`: the text read as a literal of `T`;
    /// none, or one `T` cannot hold, gives 0, flagged (runtime/iecf.h).
    pub(crate) fn string_from(&mut self, e: &Expr, name: &str, all: &[&Expr], scope: &Scope) -> R<Option<String>> {
        let to = match name.strip_prefix("STRING_TO_").or_else(|| name.strip_prefix("WSTRING_TO_")) {
            Some(t) => t,
            None => match name.strip_prefix("TO_") {
                Some(t) if all.len() == 1 && self.is_string(all[0])? => t,
                _ => return Ok(None),
            },
        };
        let Some(t) = Elem::from_name(to) else {
            return self.not_yet(e.span, format!("STRING_TO_{to}"));
        };
        let wide = self.is_wide(all[0])?;
        let (s, n) = self.string(all[0], scope)?;
        let ct = crate::c_type(t);
        self.may_flag = true;
        // WSTRING_TO_WCHAR: the first character (Table 27, row 2).
        if wide && t == Elem::Wchar {
            return Ok(Some(format!("iecf_wstring_char({FLAG}, {s})")));
        }
        // Any other from a WSTRING: its text read as a STRING's, a character
        // a STRING cannot hold read as one no literal has (runtime/iecf.h).
        let s = if wide {
            let d = self.buffer(n, false);
            format!("iecf_wstr_text({d}, {n}u, {s})")
        } else {
            s
        };
        let code = match t {
            Elem::Bool => format!("iecf_str_bool({FLAG}, {s})"),
            // The first character, not a text read (Ed.3 Table 27).
            Elem::Char => format!("iecf_string_char({FLAG}, {s})"),
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
            Elem::Ldate => format!("iecf_str_ldate({FLAG}, {s})"),
            Elem::Ltod => format!("iecf_str_ltod({FLAG}, {s})"),
            Elem::Ldt => format!("iecf_str_ldt({FLAG}, {s})"),
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
                let w = infix(self.is_wide(all[0])?);
                let (s, _) = self.string(all[0], scope)?;
                format!("iecf_{w}slen({s})")
            }
            // IN2 is ANY_CHARS: a CHAR or WCHAR too (Table 34, NOTE 3),
            // a one-character string there (`string`).
            "FIND" => {
                let chars = matches!(self.ty_of(all[1])?, Ty::Elem(Elem::Char | Elem::Wchar));
                if !(self.is_string(all[0])? && (chars || self.is_string(all[1])?)) {
                    return Ok(None);
                }
                let w = infix(self.is_wide(all[0])?);
                let ((a, _), (b, _)) = (self.string(all[0], scope)?, self.string(all[1], scope)?);
                format!("iecf_{w}find({a}, {b})")
            }
            _ => return Ok(None),
        };
        // At most 65535: an INT holds it when the string is up to 32767.
        let to = self.elem_of(e)?;
        self.convert(e.span, code, Elem::Udint, to).map(Some)
    }

    /// A comparison of strings: C's `iecf_scmp` against 0.
    pub(crate) fn string_compare(&mut self, sym: &str, a: &Expr, b: &Expr, scope: &Scope) -> R<String> {
        let w = infix(self.is_wide(a)? || self.is_wide(b)?);
        let ((x, _), (y, _)) = (self.string(a, scope)?, self.string(b, scope)?);
        Ok(format!("((IEC_BOOL)(iecf_{w}scmp({x}, {y}) {sym} 0))"))
    }
}

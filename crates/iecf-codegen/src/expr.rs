//! Expressions: each becomes a C expression of a stated elementary type,
//! with no undefined behaviour - what can overflow, divide by zero or go
//! out of range calls a helper of runtime/iecf.h, which wraps and flags
//! an overflow and faults on division by zero.

use crate::derived::{DTy, Kind};
use crate::names::{Place, Scope};
use crate::{Gen, R, c_type};
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_sema::stdlib::{Dir, unify};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;

const FAULT: &str = "&ctx->fault";
const FLAG: &str = "&ctx->flag";

/// The value of an integer literal as written, without its sign.
fn literal_int(l: &Literal) -> Option<i128> {
    let text = l.text.replace('_', "");
    let text = text.rsplit_once('#').filter(|(b, _)| b.parse::<u32>().is_err()).map_or(text.as_str(), |(_, v)| v);
    let v = if let Some((base, digits)) = text.split_once('#') {
        i128::from_str_radix(digits, base.parse().ok()?).ok()?
    } else {
        match text.to_ascii_uppercase().as_str() {
            "TRUE" => 1,
            "FALSE" => 0,
            t => t.parse().ok()?,
        }
    };
    Some(if l.negative { -v } else { v })
}

/// The value of a constant integer expression: literals, signs, parentheses
/// and + - * / MOD between them.
pub(crate) fn const_int(e: &Expr) -> Option<i128> {
    match &e.kind {
        ExprKind::Literal(l) if matches!(l.kind, LiteralKind::Int | LiteralKind::Based | LiteralKind::Bool) => {
            literal_int(l)
        }
        ExprKind::Paren(x) => const_int(x),
        ExprKind::Unary(UnaryOp::Neg, x) => const_int(x)?.checked_neg(),
        ExprKind::Unary(UnaryOp::Plus, x) => const_int(x),
        ExprKind::Binary(op, a, b) => {
            let (a, b) = (const_int(a)?, const_int(b)?);
            match op {
                BinaryOp::Add => a.checked_add(b),
                BinaryOp::Sub => a.checked_sub(b),
                BinaryOp::Mul => a.checked_mul(b),
                BinaryOp::Div => a.checked_div(b),
                BinaryOp::Mod => Some(if b == 0 { 0 } else { a.checked_rem(b).unwrap_or(0) }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `T#1h2m3.5s` in nanoseconds (Ed.3 6.3.4: d h m s ms us ns, the last
/// one may have a fraction).
pub(crate) fn duration_ns(text: &str) -> Option<i64> {
    let (_, body) = text.split_once('#')?;
    let body = body.replace('_', "").to_ascii_lowercase();
    let (neg, mut rest) = match body.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, body.as_str()),
    };
    let mut total: i128 = 0;
    while !rest.is_empty() {
        let n = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
        let (num, after) = rest.split_at(n);
        let u = after.find(|c: char| c.is_ascii_digit()).unwrap_or(after.len());
        let (unit, after) = after.split_at(u);
        let scale: i128 = match unit {
            "d" => 86_400_000_000_000,
            "h" => 3_600_000_000_000,
            "m" => 60_000_000_000,
            "s" => 1_000_000_000,
            "ms" => 1_000_000,
            "us" => 1_000,
            "ns" => 1,
            _ => return None,
        };
        let (int, frac) = num.split_once('.').unwrap_or((num, ""));
        let mut v = int.parse::<i128>().ok()? * scale;
        let mut div = 1i128;
        for c in frac.chars() {
            div *= 10;
            v += c.to_digit(10)? as i128 * scale / div;
        }
        total += v;
        rest = after;
    }
    i64::try_from(if neg { -total } else { total }).ok()
}

/// The integer type a type is held in, for the helpers of runtime/iecf.h:
/// itself for the integers and bit strings, DINT for TIME (signed 32-bit
/// milliseconds) and LINT for LTIME (signed 64-bit nanoseconds),
/// docs/dependencies.md. `None` for what this slice does not hold.
pub(crate) fn repr(e: Elem) -> Option<Elem> {
    match e {
        Elem::Time => Some(Elem::Dint),
        Elem::Ltime => Some(Elem::Lint),
        // DATE and DT in seconds since 1970, TOD in ms since midnight.
        Elem::Date | Elem::Tod | Elem::Dt => Some(Elem::Udint),
        // LDATE and LDT in ns since 1970, LTOD in ns since midnight.
        Elem::Ldate | Elem::Ltod | Elem::Ldt => Some(Elem::Lint),
        e if is_intlike(e) => Some(e),
        _ => None,
    }
}

/// A C constant of type `e`, if `v` fits.
pub(crate) fn lit(v: i128, e: Elem) -> Option<String> {
    let r = repr(e)?;
    let (lo, hi) = if r == Elem::Bool { (0, 1) } else { r.int_range()? };
    if v < lo || v > hi {
        return None;
    }
    let t = c_type(e);
    Some(if r.is_signed_int() {
        if v == i64::MIN as i128 {
            format!("(({t})(-INT64_C(9223372036854775807) - 1))")
        } else {
            format!("(({t})INT64_C({v}))")
        }
    } else {
        format!("(({t})UINT64_C({v}))")
    })
}

pub(crate) fn zero(e: Elem) -> String {
    match e {
        Elem::Real => "0.0f".into(),
        Elem::Lreal => "0.0".into(),
        _ => lit(0, e).unwrap_or_else(|| "0".into()),
    }
}

/// Whether this slice holds values of the type: the integers, bit
/// strings, durations, DATE, TOD, DT and reals.
/// A duration, date or time of day: held as an integer in its own unit.
pub(crate) fn is_time_or_date(e: Elem) -> bool {
    matches!(e, Elem::Time | Elem::Ltime | Elem::Date | Elem::Ldate | Elem::Tod | Elem::Ltod | Elem::Dt | Elem::Ldt)
}

pub(crate) fn held(e: Elem) -> bool {
    repr(e).is_some() || e.is_real()
}

/// A real literal as a C constant of `e` (REAL or LREAL): the digits as
/// written, which C reads to the nearest value of the type.
fn real_text(text: &str, e: Elem) -> String {
    let t = text.replace('_', "");
    let t = if t.contains(['.', 'e', 'E']) { t } else { format!("{t}.0") };
    if e == Elem::Real { format!("{t}f") } else { t }
}

/// The C math function under each function of Ed.3 Table 28 that has one.
fn math_function(name: &str) -> Option<&'static str> {
    Some(match name {
        "SQRT" => "sqrt",
        "LN" => "log",
        "LOG" => "log10",
        "EXP" => "exp",
        "SIN" => "sin",
        "COS" => "cos",
        "TAN" => "tan",
        "ASIN" => "asin",
        "ACOS" => "acos",
        "ATAN" => "atan",
        _ => return None,
    })
}

/// `TRUNC`, `TRUNC_INT`, `REAL_TRUNC_INT`: the source if named, the target.
fn trunc_names(name: &str) -> Option<(Option<Elem>, Elem)> {
    if name == "TRUNC" {
        return Some((None, Elem::Dint));
    }
    if let Some(to) = name.strip_prefix("TRUNC_") {
        return Some((None, Elem::from_name(to)?));
    }
    let (from, to) = name.split_once("_TRUNC_")?;
    Some((Some(Elem::from_name(from)?), Elem::from_name(to)?))
}

pub(crate) fn arg_value(args: &[Arg], span: Option<Span>) -> Option<&Expr> {
    let span = span?;
    args.iter().find_map(|a| match a {
        Arg::Positional(e) | Arg::Input(_, e) if e.span == span => Some(e),
        _ => None,
    })
}

/// `INT` and `DINT` from `INT_TO_DINT`; `(None, DINT)` from `TO_DINT`.
fn conversion_names(name: &str) -> Option<(Option<Elem>, Elem)> {
    if let Some(to) = name.strip_prefix("TO_") {
        return Some((None, Elem::from_name(to)?));
    }
    let mut from = 0;
    while let Some(i) = name[from..].find("_TO_") {
        let (a, b) = (&name[..from + i], &name[from + i + 4..]);
        if let (Some(a), Some(b)) = (Elem::from_name(a), Elem::from_name(b)) {
            return Some((Some(a), b));
        }
        from += i + 1;
    }
    None
}

fn is_intlike(e: Elem) -> bool {
    e.is_int() || e.is_bit()
}

fn expr_duration(l: &Literal) -> Option<i128> {
    let ns: i128 = duration_ns(&l.text)?.into();
    Some(if l.negative { -ns } else { ns })
}

impl Gen<'_> {
    /// A call of the user's FUNCTION: `F_<NAME>(ctx, inputs...)`, every
    /// input given, by its argument or by its initial value.
    fn user_call(&mut self, e: &Expr, callee: &Expr, args: &[Arg], scope: &Scope) -> R<String> {
        let ExprKind::Name(id) = &callee.kind else { return self.not_yet(callee.span, "this call") };
        let key = id.text.to_ascii_uppercase();
        if let Some(Place::Fb(..)) = scope.vars.get(&key) {
            return self.not_yet(e.span, "a function block called inside an expression");
        }
        let sig = self.fn_sig(&key, id.span)?;
        // Taken before the arguments are written, so a call among them
        // does not take it.
        let want = self.want_eno.take();
        let mut given: Vec<Option<&Expr>> = vec![None; sig.inputs.len()];
        for (i, a) in args.iter().enumerate() {
            let slot = match a {
                Arg::Positional(x) if i < given.len() => Some((i, x)),
                Arg::Input(n, x) => sig.inputs.iter().position(|(k, _, _)| n.is(k)).map(|j| (j, x)),
                _ => None,
            };
            match slot {
                Some((j, x)) => given[j] = Some(x),
                None => return self.not_yet(e.span, "this argument of a FUNCTION (outputs, EN, ENO)"),
            }
        }
        let mut codes = vec!["ctx".to_string()];
        if sig.eno {
            codes.push(match want {
                Some(t) => {
                    self.eno_used = true;
                    format!("&{t}")
                }
                None => "&(IEC_BOOL){1}".into(),
            });
        }
        for ((_, t, default), x) in sig.inputs.iter().zip(given) {
            codes.push(match x {
                Some(x) => self.value_as(x, *t, scope)?,
                None => default.clone(),
            });
        }
        self.may_fault = true;
        Ok(format!("F_{}({})", sig.key, codes.join(", ")))
    }

    /// The type of `e`; an enumerated one as the integer it is held in.
    pub(crate) fn ty_of(&mut self, e: &Expr) -> R<Ty> {
        match self.types.expr(e.span) {
            Some(Ty::Enum(k)) => {
                let k = k.clone();
                Ok(Ty::Elem(self.enum_base(&k, e.span)?))
            }
            Some(t) => Ok(t.clone()),
            None => self.not_yet(e.span, "this expression"),
        }
    }

    /// The enumerated value `e` names, if it names one: `idle`, `mode#idle`.
    pub(crate) fn enum_const(&mut self, e: &Expr) -> R<Option<String>> {
        let name = match &e.kind {
            ExprKind::Name(id) => id,
            ExprKind::EnumValue { value, .. } => value,
            ExprKind::Paren(x) => return self.enum_const(x),
            _ => return Ok(None),
        };
        let Some(Ty::Enum(k)) = self.types.expr(e.span).cloned() else { return Ok(None) };
        self.enum_base(&k, e.span)?;
        match self.enum_value(&k, &name.text) {
            Some(v) => Ok(Some(v)),
            None => self.not_yet(e.span, format!("the value {}", name.text)),
        }
    }

    /// A variable of a derived type, or an element or field of one: its C
    /// lvalue and type. An index is checked where it is used: out of its
    /// bounds it faults and reads element 0 (runtime/iecf.h).
    pub(crate) fn data_path(&mut self, e: &Expr, scope: &Scope) -> R<Option<(String, DTy)>> {
        match &e.kind {
            ExprKind::Name(id) => match scope.vars.get(&id.text.to_ascii_uppercase()) {
                Some(Place::Data(lv, t)) => Ok(Some((lv.clone(), t.clone()))),
                _ => Ok(None),
            },
            ExprKind::Paren(x) => self.data_path(x, scope),
            ExprKind::Member(base, m) => {
                // A function block's input or output of an array, structure
                // or STRING type.
                if let ExprKind::Name(id) = &base.kind
                    && let Some(Place::Block(lv, block)) = scope.vars.get(&id.text.to_ascii_uppercase())
                {
                    let name = m.text.to_ascii_uppercase();
                    let found = self.blocks.get(block).and_then(|b| b.data.iter().find(|(n, _, _)| *n == name));
                    // An elementary input or output is value()'s.
                    return Ok(found.map(|(n, t, _)| (format!("{lv}.V_{n}"), t.clone())));
                }
                let Some((lv, t)) = self.data_path(base, scope)? else { return Ok(None) };
                let Kind::Struct(k) = &t.kind else { return self.not_yet(m.span, "this member") };
                let key = m.text.to_ascii_uppercase();
                match self.structs.get(k).and_then(|fs| fs.iter().find(|(n, _)| *n == key)) {
                    Some((_, ft)) => Ok(Some((format!("{lv}.V_{key}"), ft.clone()))),
                    None => self.not_yet(m.span, format!("the field {}", m.text)),
                }
            }
            ExprKind::Index(base, idx) => {
                let Some((mut lv, t)) = self.data_path(base, scope)? else {
                    return self.not_yet(e.span, "this index");
                };
                let Kind::Array(elem, bounds) = &t.kind else { return self.not_yet(e.span, "this index") };
                if idx.len() != bounds.len() {
                    return self.not_yet(e.span, "an index of another count of dimensions");
                }
                for (i, (lo, hi)) in idx.iter().zip(bounds) {
                    let off = self.index(i, *lo, *hi, scope)?;
                    lv = format!("{lv}[{off}]");
                }
                Ok(Some((lv, (**elem).clone())))
            }
            _ => Ok(None),
        }
    }

    /// The offset of index `i` in `lo..hi`: a constant, or checked.
    pub(crate) fn index(&mut self, i: &Expr, lo: i64, hi: i64, scope: &Scope) -> R<String> {
        if let Some(v) = const_int(i) {
            if v < lo as i128 || v > hi as i128 {
                return self.not_yet(i.span, format!("the index {v}, outside {lo}..{hi}"));
            }
            return Ok(format!("{}u", v - lo as i128));
        }
        let t = self.elem_of(i)?;
        if !t.is_int() {
            return self.not_yet(i.span, format!("an index of type {}", t.name()));
        }
        let v = self.value(i, scope)?;
        self.may_fault = true;
        Ok(if t.is_signed_int() {
            format!("iecf_index({FAULT}, (int64_t)({v}), INT64_C({lo}), INT64_C({hi}))")
        } else {
            format!("iecf_index_u({FAULT}, (uint64_t)({v}), INT64_C({lo}), INT64_C({hi}))")
        })
    }

    /// `e` as a C expression of type `want`, by an implicit conversion
    /// (Ed.3 Figure 11, which only widens) where its own type differs.
    pub(crate) fn value_as(&mut self, e: &Expr, want: Elem, scope: &Scope) -> R<String> {
        if let ExprKind::Binary(BinaryOp::Power, a, b) = &e.kind {
            // A literal base types `2.0 ** n` as a literal; `n` need not be.
            if want.is_real() && matches!(self.ty_of(e)?, Ty::RealLit) {
                return self.power(e.span, want, a, b, scope);
            }
        }
        match self.ty_of(e)? {
            Ty::IntLit(_) | Ty::RealLit if want.is_real() => self.real_const(e, want),
            Ty::IntLit(_) => {
                let Some(v) = const_int(e) else { return self.not_yet(e.span, "this constant expression") };
                match lit(v, want) {
                    Some(c) => Ok(c),
                    None => {
                        self.diags.push(Diagnostic::new(
                            Code::LiteralRange,
                            e.span,
                            format!("{v} is outside {}", want.name()),
                        ));
                        Err(crate::Skip)
                    }
                }
            }
            Ty::Elem(own) => {
                let code = self.value(e, scope)?;
                if own == want {
                    Ok(code)
                } else if (is_intlike(own) && is_intlike(want))
                    || (own.is_int() && want.is_real())
                    || (own == Elem::Real && want == Elem::Lreal)
                {
                    // Figure 11's implicit conversions keep the value: a cast.
                    Ok(format!("(({})({code}))", c_type(want)))
                } else if matches!(
                    (own, want),
                    (Elem::Time, Elem::Ltime)
                        | (Elem::Dt, Elem::Ldt)
                        | (Elem::Date, Elem::Ldate)
                        | (Elem::Tod, Elem::Ltod)
                ) {
                    // Figure 11's implicit widening of a duration or date:
                    // to nanoseconds.
                    self.convert(e.span, code, own, want)
                } else {
                    self.not_yet(e.span, format!("{} as {}", own.name(), want.name()))
                }
            }
            t => self.not_yet(e.span, format!("a value of type {t}")),
        }
    }

    /// The elementary type `e` has, an untyped integer constant counted as
    /// LINT (or ULINT past LINT's range).
    pub(crate) fn elem_of(&mut self, e: &Expr) -> R<Elem> {
        match self.ty_of(e)? {
            Ty::Elem(t) if held(t) => Ok(t),
            Ty::RealLit => Ok(Elem::Lreal),
            Ty::IntLit(_) => Ok(match const_int(e) {
                Some(v) if v > i64::MAX as i128 => Elem::Ulint,
                _ => Elem::Lint,
            }),
            t => self.not_yet(e.span, format!("a value of type {t}")),
        }
    }

    /// `e` as a C expression of its own type.
    pub(crate) fn value(&mut self, e: &Expr, scope: &Scope) -> R<String> {
        if let Some(crate::enable::Sub::Val(c)) = self.subst.get(&e.span) {
            return Ok(c.clone());
        }
        if matches!(self.ty_of(e)?, Ty::IntLit(_) | Ty::RealLit) {
            let t = self.elem_of(e)?;
            return self.value_as(e, t, scope);
        }
        match &e.kind {
            ExprKind::Literal(l) if l.kind == LiteralKind::Time => {
                let t = self.elem_of(e)?;
                if let Some(r) = iecf_sema::datetime::literal(&l.text) {
                    // The checker refused one it cannot hold (E0020).
                    return match r.ok().and_then(|(_, v)| lit(v.into(), t)) {
                        Some(c) => Ok(c),
                        None => Err(crate::Skip),
                    };
                }
                let ns = expr_duration(l);
                let v = match (t, ns) {
                    (Elem::Time, Some(ns)) if ns % 1_000_000 == 0 => Some(ns / 1_000_000),
                    (Elem::Ltime, Some(ns)) => Some(ns),
                    _ => None,
                };
                match v.and_then(|v| lit(v, t)) {
                    Some(c) => Ok(c),
                    None => self.not_yet(e.span, "this duration (TIME counts whole milliseconds)"),
                }
            }
            ExprKind::Literal(l) if l.kind == LiteralKind::Real => {
                let t = self.elem_of(e)?;
                let text = real_text(&l.text, t);
                Ok(if l.negative { format!("(-{text})") } else { text })
            }
            ExprKind::Literal(l) => {
                let t = self.elem_of(e)?;
                match literal_int(l).and_then(|v| lit(v, t)) {
                    Some(c) => Ok(c),
                    None => self.not_yet(e.span, "this literal"),
                }
            }
            ExprKind::Member(..) | ExprKind::Index(..) if self.data_path(e, scope)?.is_some() => {
                match self.data_path(e, scope)? {
                    Some((lv, t)) if t.scalar().is_some() => Ok(lv),
                    _ => self.not_yet(e.span, "a whole array or structure here"),
                }
            }
            // A step's X and T, an action's Q (sfc.rs).
            ExprKind::Member(base, m)
                if matches!(&base.kind, ExprKind::Name(id)
                    if scope.vars.contains_key(&format!("{}.{}", id.text.to_ascii_uppercase(), m.text.to_ascii_uppercase()))) =>
            {
                let ExprKind::Name(id) = &base.kind else { return self.not_yet(e.span, "this member") };
                let k = format!("{}.{}", id.text.to_ascii_uppercase(), m.text.to_ascii_uppercase());
                Ok(scope.vars[&k].load())
            }
            ExprKind::Member(base, m) => {
                let fb = match &base.kind {
                    ExprKind::Name(id) => scope.vars.get(&id.text.to_ascii_uppercase()).and_then(|p| self.iface(p)),
                    _ => None,
                };
                let Some(fb) = fb else { return self.not_yet(e.span, "this member") };
                match fb.param(&m.text) {
                    Some((_, field, _, Dir::Input | Dir::Output)) => Ok(format!("{}.{field}", fb.lv)),
                    _ => self.not_yet(m.span, format!("{}.{}", fb.label, m.text)),
                }
            }
            ExprKind::Paren(x) => self.value(x, scope),
            ExprKind::Name(id) => match scope.vars.get(&id.text.to_ascii_uppercase()) {
                Some(Place::Data(_, t)) if t.scalar().is_none() => {
                    self.not_yet(id.span, "a whole array or structure here")
                }
                Some(p) => Ok(p.load()),
                None => match self.enum_const(e)? {
                    Some(v) => Ok(v),
                    None => self.not_yet(id.span, format!("'{}' here", id.text)),
                },
            },
            ExprKind::EnumValue { value, .. } => match self.enum_const(e)? {
                Some(v) => Ok(v),
                None => self.not_yet(value.span, format!("the value {}", value.text)),
            },
            ExprKind::DirectAddress(a) => {
                let t = self.elem_of(e)?;
                let p: Place = self.located(a, t)?;
                Ok(p.load())
            }
            ExprKind::Unary(op, x) => {
                let t = self.elem_of(e)?;
                let v = self.value_as(x, t, scope)?;
                let ct = c_type(t);
                Ok(match op {
                    UnaryOp::Plus => v,
                    UnaryOp::Neg if t.is_real() => format!("(({ct})(-({v})))"),
                    UnaryOp::Neg if t.is_signed_int() => {
                        self.may_flag = true;
                        format!("iecf_neg_{}({FLAG}, {v})", t.name())
                    }
                    UnaryOp::Neg => return self.not_yet(e.span, format!("negation of {}", t.name())),
                    UnaryOp::Not if t == Elem::Bool => format!("(({ct})!({v}))"),
                    UnaryOp::Not => format!("(({ct})~({v}))"),
                })
            }
            ExprKind::Binary(op, a, b) => self.binary(e, *op, a, b, scope),
            ExprKind::Call { callee, args } => self.call(e, callee, args, scope),
            _ => self.not_yet(e.span, "this expression"),
        }
    }

    fn binary(&mut self, e: &Expr, op: BinaryOp, a: &Expr, b: &Expr, scope: &Scope) -> R<String> {
        use BinaryOp::*;
        if matches!(op, Eq | Ne | Lt | Le | Gt | Ge) {
            return self.compare(e.span, op, &[a, b], scope);
        }
        if matches!(op, Add | Sub)
            && (crate::dates::is_date(self.elem_of(a)?) || crate::dates::is_date(self.elem_of(b)?))
        {
            return self.date_arith(e.span, op, a, b, scope);
        }
        let t = self.elem_of(e)?;
        if t.is_duration() && matches!(op, Mul | Div) {
            return self.duration_scale(e.span, op, t, a, b, scope);
        }
        if op == Power {
            return self.power(e.span, t, a, b, scope);
        }
        let (x, y) = (self.value_as(a, t, scope)?, self.value_as(b, t, scope)?);
        self.arith(e.span, op, t, x, y)
    }

    /// `a op b op c ...` for a comparison: Table 33's extensible GT, GE,
    /// EQ, LE, LT hold when each neighbouring pair does.
    fn compare(&mut self, at: Span, op: BinaryOp, xs: &[&Expr], scope: &Scope) -> R<String> {
        use BinaryOp::*;
        let mut common: Option<Ty> = None;
        for x in xs {
            let t = self.ty_of(x)?;
            common = match common {
                None => Some(t),
                Some(c) => unify(&c, &t),
            };
            if common.is_none() {
                return self.not_yet(at, "this comparison");
            }
        }
        if xs.len() > 2 {
            // The middle ones would be read twice.
            return self.not_yet(at, "a comparison of more than two values");
        }
        let sym = match op {
            Eq => "==",
            Ne => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            _ => ">=",
        };
        if matches!(common, Some(Ty::Str { wide: false, .. })) {
            return self.string_compare(sym, xs[0], xs[1], scope);
        }
        let common = match common {
            Some(Ty::Elem(t)) if held(t) => t,
            Some(Ty::IntLit(_)) => self.elem_of(xs[0])?,
            Some(Ty::RealLit) => Elem::Lreal,
            Some(t) => return self.not_yet(at, format!("comparing {t}")),
            None => return self.not_yet(at, "this comparison"),
        };
        let (x, y) = (self.value_as(xs[0], common, scope)?, self.value_as(xs[1], common, scope)?);
        Ok(format!("((IEC_BOOL)(({x}) {sym} ({y})))"))
    }

    /// `a ** b` and EXPT: a REAL or LREAL `t` raised to any number, by
    /// C's `pow`; an infinite or NaN result is flagged like any real's.
    fn power(&mut self, at: Span, t: Elem, a: &Expr, b: &Expr, scope: &Scope) -> R<String> {
        if !t.is_real() {
            return self.not_yet(at, format!("** on {}", t.name()));
        }
        let x = self.value_as(a, t, scope)?;
        let y = match self.ty_of(b)? {
            Ty::IntLit(_) | Ty::RealLit => self.value_as(b, Elem::Lreal, scope)?,
            Ty::Elem(bt) if bt.is_real() || bt.is_int() => self.value(b, scope)?,
            other => return self.not_yet(b.span, format!("an exponent of {other}")),
        };
        self.math.insert(("IECF_EXPT".to_string(), "pow"));
        self.may_flag = true;
        Ok(if t == Elem::Real {
            format!("iecf_real_of({FLAG}, IECF_EXPT((double)({x}), (double)({y})))")
        } else {
            format!("iecf_lreal_of({FLAG}, IECF_EXPT({x}, (double)({y})))")
        })
    }

    /// `x op y`, both already C of type `t`.
    fn arith(&mut self, at: Span, op: BinaryOp, t: Elem, x: String, y: String) -> R<String> {
        use BinaryOp::*;
        if t.is_duration() && !matches!(op, Add | Sub) {
            return self.not_yet(at, format!("{} * and / on a duration", t.name()));
        }
        let ct = c_type(t);
        if t.is_real() {
            let n = t.name();
            self.may_flag = true;
            return Ok(match op {
                Add => format!("iecf_add_{n}({FLAG}, {x}, {y})"),
                Sub => format!("iecf_sub_{n}({FLAG}, {x}, {y})"),
                Mul => format!("iecf_mul_{n}({FLAG}, {x}, {y})"),
                Div => {
                    self.may_fault = true;
                    format!("iecf_div_{n}({FAULT}, {FLAG}, {x}, {y})")
                }
                _ => return self.not_yet(at, format!("this operator on {n}")),
            });
        }
        let n = repr(t).unwrap_or(t).name();
        let t = repr(t).unwrap_or(t);
        Ok(match op {
            And | Or | Xor => {
                let sym = match op {
                    And => "&",
                    Or => "|",
                    _ => "^",
                };
                format!("(({ct})(({x}) {sym} ({y})))")
            }
            Add | Sub | Mul if t.is_int() => {
                self.may_flag = true;
                let f = match op {
                    Add => "add",
                    Sub => "sub",
                    _ => "mul",
                };
                format!("iecf_{f}_{n}({FLAG}, {x}, {y})")
            }
            Div if t.is_signed_int() => {
                self.may_fault = true;
                self.may_flag = true;
                format!("iecf_div_{n}({FAULT}, {FLAG}, {x}, {y})")
            }
            Div if t.is_int() => {
                self.may_fault = true;
                format!("iecf_div_{n}({FAULT}, {x}, {y})")
            }
            Mod if t.is_int() => format!("iecf_mod_{n}({x}, {y})"),
            _ => return self.not_yet(at, format!("this operator on {n}")),
        })
    }

    pub(crate) fn call(&mut self, e: &Expr, callee: &Expr, args: &[Arg], scope: &Scope) -> R<String> {
        if let Some((r, _)) = self.gated(e, args, scope, false)? {
            return Ok(r);
        }
        let Some(sc) = self.types.std_call(e.span).cloned() else {
            return self.user_call(e, callee, args, scope);
        };
        if sc.outputs.iter().any(|(_, s)| s.is_some()) {
            if let Some(code) = self.date_split(e, &sc, args, scope)? {
                return Ok(code);
            }
            return self.not_yet(e.span, "outputs of a standard function");
        }
        let input = |name: &str| sc.inputs.iter().find(|(n, _)| n == name).and_then(|(_, s)| arg_value(args, *s));
        let all: Vec<&Expr> = sc.inputs.iter().filter_map(|(_, s)| arg_value(args, *s)).collect();
        if let Some(code) = self.string_int(e, &sc.name, &all, scope)? {
            return Ok(code);
        }
        if let Some(code) = self.string_from(e, &sc.name, &all, scope)? {
            return Ok(code);
        }
        if let Some(code) = self.date_call(e.span, &sc.name, &all, scope)? {
            let from = if sc.name == "DAY_OF_WEEK" { Elem::Usint } else { self.elem_of(e)? };
            let to = self.elem_of(e)?;
            return self.convert(e.span, code, from, to);
        }
        let result = self.elem_of(e)?;
        let rn = result.name();
        match sc.name.as_str() {
            "ABS" => {
                let x = self.value_as(all[0], result, scope)?;
                if result.is_real() {
                    Ok(format!("iecf_abs_{rn}({x})"))
                } else if result.is_signed_int() {
                    self.may_flag = true;
                    Ok(format!("iecf_abs_{rn}({FLAG}, {x})"))
                } else {
                    Ok(x)
                }
            }
            "MIN" | "MAX" => {
                let f = sc.name.to_ascii_lowercase();
                let hn = repr(result).unwrap_or(result).name();
                let mut acc = self.value_as(all[0], result, scope)?;
                for x in &all[1..] {
                    let v = self.value_as(x, result, scope)?;
                    acc = format!("iecf_{f}_{hn}({acc}, {v})");
                }
                Ok(acc)
            }
            // Tables 29-31, 33: the operators as functions, ADD, MUL,
            // AND, OR, XOR extensible.
            "MUL_TIME" | "MUL_LTIME" | "DIV_TIME" | "DIV_LTIME" => {
                let op = if sc.name.starts_with("MUL") { BinaryOp::Mul } else { BinaryOp::Div };
                self.duration_scale(e.span, op, result, all[0], all[1], scope)
            }
            "MUL" | "DIV" if result.is_duration() && all.len() == 2 => {
                let op = if sc.name == "MUL" { BinaryOp::Mul } else { BinaryOp::Div };
                self.duration_scale(e.span, op, result, all[0], all[1], scope)
            }
            "ADD" | "MUL" | "AND" | "OR" | "XOR" | "SUB" | "DIV" | "MOD" => {
                let op = match sc.name.as_str() {
                    "ADD" => BinaryOp::Add,
                    "MUL" => BinaryOp::Mul,
                    "AND" => BinaryOp::And,
                    "OR" => BinaryOp::Or,
                    "XOR" => BinaryOp::Xor,
                    "SUB" => BinaryOp::Sub,
                    "DIV" => BinaryOp::Div,
                    _ => BinaryOp::Mod,
                };
                let Some((first, rest)) = all.split_first() else { return self.not_yet(e.span, "this call") };
                let mut acc = self.value_as(first, result, scope)?;
                for x in rest {
                    let v = self.value_as(x, result, scope)?;
                    acc = self.arith(e.span, op, result, acc, v)?;
                }
                Ok(acc)
            }
            "GT" | "GE" | "EQ" | "LE" | "LT" | "NE" => {
                let op = match sc.name.as_str() {
                    "GT" => BinaryOp::Gt,
                    "GE" => BinaryOp::Ge,
                    "EQ" => BinaryOp::Eq,
                    "LE" => BinaryOp::Le,
                    "LT" => BinaryOp::Lt,
                    _ => BinaryOp::Ne,
                };
                self.compare(e.span, op, &all, scope)
            }
            "NOT" => {
                let x = self.value_as(all[0], result, scope)?;
                let ct = c_type(result);
                Ok(if result == Elem::Bool { format!("(({ct})!({x}))") } else { format!("(({ct})~({x}))") })
            }
            "MOVE" => self.value_as(all[0], result, scope),
            "EXPT" => {
                let (Some(a), Some(b)) = (input("IN1"), input("IN2")) else {
                    return self.not_yet(e.span, "this EXPT");
                };
                self.power(e.span, result, a, b, scope)
            }
            "SEL" => {
                let (Some(g), Some(a), Some(b)) = (input("G"), input("IN0"), input("IN1")) else {
                    return self.not_yet(e.span, "this SEL");
                };
                let g = self.value_as(g, Elem::Bool, scope)?;
                let (a, b) = (self.value_as(a, result, scope)?, self.value_as(b, result, scope)?);
                let Some(n) = repr(result).or(Some(result).filter(|t| t.is_real())) else {
                    return self.not_yet(e.span, format!("SEL of {rn}"));
                };
                Ok(format!("iecf_sel_{}({g}, {a}, {b})", n.name()))
            }
            "MUX" => {
                // Every input evaluated, as any function's (Ed.3 Table 32).
                let Some((k, ins)) = all.split_first() else { return self.not_yet(e.span, "this MUX") };
                let Some(n) = repr(result).or(Some(result).filter(|t| t.is_real())) else {
                    return self.not_yet(e.span, format!("MUX of {rn}"));
                };
                let k = match self.ty_of(k)? {
                    Ty::Elem(t) if t.is_int() => {
                        let c = self.value(k, scope)?;
                        if t == Elem::Ulint {
                            format!("iecf_mux_k((uint64_t)({c}))")
                        } else {
                            format!("(int64_t)({c})")
                        }
                    }
                    _ => self.value_as(k, Elem::Lint, scope)?,
                };
                let mut vs = Vec::new();
                for x in ins {
                    vs.push(self.value_as(x, result, scope)?);
                }
                self.may_fault = true;
                Ok(format!(
                    "iecf_mux_{}({FAULT}, {k}, {}u, ({}[]){{{}}})",
                    n.name(),
                    vs.len(),
                    c_type(result),
                    vs.join(", ")
                ))
            }
            "LIMIT" => {
                let (Some(mn), Some(i), Some(mx)) = (input("MN"), input("IN"), input("MX")) else {
                    return self.not_yet(e.span, "this LIMIT");
                };
                let (mn, i, mx) = (
                    self.value_as(mn, result, scope)?,
                    self.value_as(i, result, scope)?,
                    self.value_as(mx, result, scope)?,
                );
                Ok(format!("iecf_limit_{}({mn}, {i}, {mx})", repr(result).unwrap_or(result).name()))
            }
            "SHL" | "SHR" | "ROL" | "ROR" => {
                let (Some(i), Some(n)) = (input("IN"), input("N")) else {
                    return self.not_yet(e.span, "this shift");
                };
                if !result.is_bit() || result == Elem::Bool {
                    return self.not_yet(e.span, format!("{} of {rn}", sc.name));
                }
                let f = sc.name.to_ascii_lowercase();
                let x = self.value_as(i, result, scope)?;
                match (self.ty_of(n)?, const_int(n)) {
                    (Ty::IntLit(_), Some(v)) if v >= 0 => Ok(format!("iecf_{f}_{rn}_u({x}, UINT64_C({v}))")),
                    (Ty::Elem(t), _) if t.is_unsigned_int() => {
                        let c = self.value(n, scope)?;
                        Ok(format!("iecf_{f}_{rn}_u({x}, (uint64_t)({c}))"))
                    }
                    _ => {
                        let c = self.value_as(n, Elem::Lint, scope)?;
                        self.may_fault = true;
                        Ok(format!("iecf_{f}_{rn}({FAULT}, {x}, {c})"))
                    }
                }
            }
            name if math_function(name).is_some() && result.is_real() => {
                let f = math_function(name).unwrap_or_default();
                let macro_ = format!("IECF_{name}");
                self.math.insert((macro_.clone(), f));
                let x = self.value_as(all[0], result, scope)?;
                self.may_flag = true;
                Ok(if result == Elem::Real {
                    format!("iecf_real_of({FLAG}, {macro_}((double)({x})))")
                } else {
                    format!("iecf_lreal_of({FLAG}, {macro_}({x}))")
                })
            }
            name if trunc_names(name).is_some() => {
                let (named_from, to) = trunc_names(name).unwrap_or((None, Elem::Dint));
                let Some(x) = all.first() else { return self.not_yet(e.span, "this TRUNC") };
                let from = match named_from {
                    Some(f) => f,
                    None => self.elem_of(x)?,
                };
                let code = self.value_as(x, from, scope)?;
                self.real_to_int(e.span, code, to, "trunc")
            }
            name => {
                let Some((named_from, to)) = conversion_names(name) else {
                    return self.not_yet(e.span, format!("the standard function {name}"));
                };
                let Some(x) = all.first() else { return self.not_yet(e.span, "this conversion") };
                let from = match (self.ty_of(x)?, named_from) {
                    (Ty::IntLit(_) | Ty::RealLit, None) => return self.value_as(x, to, scope),
                    (Ty::IntLit(_) | Ty::RealLit, Some(f)) => f,
                    (Ty::Elem(t), _) => t,
                    (t, _) => return self.not_yet(x.span, format!("converting {t}")),
                };
                let code = self.value_as(x, from, scope)?;
                self.convert(e.span, code, from, to)
            }
        }
    }

    /// A real `code` to the integer `to`: rounded (`round`) or truncated
    /// (`trunc`). Out of range it is flagged and sets the statement's
    /// `keep`, which leaves the statement undone (stmt.rs).
    pub(crate) fn real_to_int(&mut self, at: Span, code: String, to: Elem, how: &str) -> R<String> {
        if !self.keep_ok {
            return self.not_yet(at, "a real converted to an integer here");
        }
        let k = match &self.keep {
            Some(k) => k.clone(),
            None => {
                let k = self.fresh("k");
                self.keep = Some(k.clone());
                k
            }
        };
        let ct = c_type(to);
        let (lo, hi) = to.int_range().unwrap_or_default();
        self.may_flag = true;
        Ok(if to.is_signed_int() {
            let lo = if lo == i64::MIN as i128 { "INT64_MIN".to_string() } else { format!("INT64_C({lo})") };
            format!("(({ct})iecf_{how}_s({FLAG}, &{k}, (double)({code}), {lo}, INT64_C({hi})))")
        } else {
            format!("(({ct})iecf_{how}_u({FLAG}, &{k}, (double)({code}), UINT64_C({hi})))")
        })
    }

    /// An expression of literals only, in a real type: C's own arithmetic
    /// on constants, as the type.
    fn real_const(&mut self, e: &Expr, want: Elem) -> R<String> {
        let ct = c_type(want);
        match &e.kind {
            ExprKind::Literal(l) if l.kind == LiteralKind::Real => {
                let t = real_text(&l.text, want);
                Ok(if l.negative { format!("(-{t})") } else { t })
            }
            ExprKind::Literal(_) => match const_int(e) {
                Some(v) => Ok(real_text(&v.to_string(), want)),
                None => self.not_yet(e.span, "this literal"),
            },
            ExprKind::Paren(x) | ExprKind::Unary(UnaryOp::Plus, x) => self.real_const(x, want),
            ExprKind::Unary(UnaryOp::Neg, x) => Ok(format!("(({ct})(-{}))", self.real_const(x, want)?)),
            ExprKind::Binary(op @ (BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul), a, b) => {
                let sym = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    _ => "*",
                };
                let (a, b) = (self.real_const(a, want)?, self.real_const(b, want)?);
                Ok(format!("(({ct})({a} {sym} {b}))"))
            }
            _ => self.not_yet(e.span, "this constant expression"),
        }
    }

    /// `code`, of type `from`, as `to`: numbers keep their value or fault
    /// (Ed.3 6.6.2.5.3); a bit string on either side is a binary transfer
    /// (6.6.2.5.4, 6.6.2.5.5).
    pub(crate) fn convert(&mut self, at: Span, code: String, from: Elem, to: Elem) -> R<String> {
        let ct = c_type(to);
        if from == to {
            return Ok(code);
        }
        match (from, to) {
            (f, t) if f.is_real() && t.is_int() => return self.real_to_int(at, code, to, "round"),
            (f, t) if f.is_int() && t.is_real() => return Ok(format!("(({ct})({code}))")),
            (Elem::Real, Elem::Lreal) => return Ok(format!("((IEC_LREAL)({code}))")),
            (Elem::Lreal, Elem::Real) => {
                self.may_flag = true;
                return Ok(format!("iecf_real_of({FLAG}, {code})"));
            }
            (Elem::Dword, Elem::Real) => return Ok(format!("iecf_bits_real({code})")),
            (Elem::Lword, Elem::Lreal) => return Ok(format!("iecf_bits_lreal({code})")),
            (Elem::Real, Elem::Dword) => return Ok(format!("iecf_real_bits({code})")),
            (Elem::Lreal, Elem::Lword) => return Ok(format!("iecf_lreal_bits({code})")),
            (Elem::Dt, Elem::Tod) => return Ok(format!("iecf_dt_tod({code})")),
            (Elem::Dt, Elem::Date) => return Ok(format!("iecf_dt_date({code})")),
            _ => {}
        }
        // The long and short durations and dates (Ed.3 Table 22, Figure
        // 11): widening exact; narrowing rounds down, and one the short
        // type cannot hold gives its default, flagged (runtime/iecf.h).
        let f = match (from, to) {
            (Elem::Time, Elem::Ltime) => Some(("iecf_time_ltime", false)),
            (Elem::Ltime, Elem::Time) => Some(("iecf_ltime_time", true)),
            (Elem::Dt, Elem::Ldt) | (Elem::Date, Elem::Ldate) => Some(("iecf_dt_ldt", false)),
            (Elem::Tod, Elem::Ltod) => Some(("iecf_tod_ltod", false)),
            (Elem::Dt, Elem::Ldate) => Some(("iecf_dt_ldate", false)),
            (Elem::Dt, Elem::Ltod) => Some(("iecf_dt_ltod", false)),
            (Elem::Ldt, Elem::Dt) => Some(("iecf_ldt_dt", true)),
            (Elem::Ldt | Elem::Ldate, Elem::Date) => Some(("iecf_ldt_date", true)),
            (Elem::Ldt, Elem::Ldate) => Some(("iecf_ldt_ldate", true)),
            (Elem::Ldt, Elem::Ltod) => Some(("iecf_ldt_ltod", false)),
            (Elem::Ldt | Elem::Ltod, Elem::Tod) => Some(("iecf_ldt_tod", false)),
            _ => None,
        };
        match f {
            Some((f, true)) => {
                self.may_flag = true;
                return Ok(format!("{f}({FLAG}, {code})"));
            }
            Some((f, false)) => return Ok(format!("{f}({code})")),
            None => {}
        }
        if !(is_intlike(from) && is_intlike(to)) {
            return self.not_yet(at, format!("{}_TO_{}", from.name(), to.name()));
        }
        if from.is_int() && to.is_int() {
            let (flo, fhi) = from.int_range().unwrap_or_default();
            let (tlo, thi) = to.int_range().unwrap_or_default();
            if tlo <= flo && fhi <= thi {
                return Ok(format!("(({ct})({code}))"));
            }
            // Out of range: the low bits, flagged (iecf.h).
            self.may_flag = true;
            let bits = match (from.is_signed_int(), to.is_signed_int()) {
                (true, true) => format!("iecf_s_in({FLAG}, (int64_t)({code}), INT64_C({tlo}), INT64_C({thi}))"),
                (true, false) => format!("iecf_su_in({FLAG}, (int64_t)({code}), UINT64_C({thi}))"),
                (false, false) => format!("iecf_u_in({FLAG}, (uint64_t)({code}), UINT64_C({thi}))"),
                (false, true) => format!("iecf_us_in({FLAG}, (uint64_t)({code}), INT64_C({thi}))"),
            };
            return Ok(match to {
                Elem::Sint => format!("iecf_to_i8((uint8_t)({bits}))"),
                Elem::Int => format!("iecf_to_i16((uint16_t)({bits}))"),
                Elem::Dint => format!("iecf_to_i32((uint32_t)({bits}))"),
                Elem::Lint => format!("iecf_to_i64({bits})"),
                _ => format!("(({ct})({bits}))"),
            });
        }
        // A binary transfer: the low bytes, or zeros added on the left.
        if from == Elem::Bool {
            return Ok(format!("(({ct})({code}))"));
        }
        // A bit string to BOOL: TRUE for any value but 0, as CODESYS and
        // TwinCAT (docs/extensions.md).
        if to == Elem::Bool {
            return Ok(format!("((IEC_BOOL)(({code}) != 0u))"));
        }
        let bits = |e: Elem| match e {
            Elem::Sint | Elem::Usint | Elem::Byte => 8,
            Elem::Int | Elem::Uint | Elem::Word => 16,
            Elem::Dint | Elem::Udint | Elem::Dword => 32,
            _ => 64,
        };
        Ok(if to.is_signed_int() {
            let w = bits(to);
            format!("iecf_to_i{w}((uint{w}_t)({code}))")
        } else {
            format!("(({ct})({code}))")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration_ns("T#10ms"), Some(10_000_000));
        assert_eq!(duration_ns("time#1h2m3s"), Some(3_723_000_000_000));
        assert_eq!(duration_ns("T#1.5s"), Some(1_500_000_000));
        assert_eq!(duration_ns("t#-2_500ms"), Some(-2_500_000_000));
        assert_eq!(duration_ns("T#1d"), Some(86_400_000_000_000));
        assert_eq!(duration_ns("T#5x"), None);
    }

    #[test]
    fn literals() {
        assert_eq!(lit(-5, Elem::Int).as_deref(), Some("((IEC_INT)INT64_C(-5))"));
        assert_eq!(lit(40000, Elem::Int), None);
        assert_eq!(
            lit(i64::MIN as i128, Elem::Lint).as_deref(),
            Some("((IEC_LINT)(-INT64_C(9223372036854775807) - 1))")
        );
        assert_eq!(
            lit(18_000_000_000_000_000_000, Elem::Ulint).as_deref(),
            Some("((IEC_ULINT)UINT64_C(18000000000000000000))")
        );
        assert_eq!(lit(1, Elem::Bool).as_deref(), Some("((IEC_BOOL)UINT64_C(1))"));
    }

    #[test]
    fn conversions_by_name() {
        assert_eq!(conversion_names("INT_TO_DINT"), Some((Some(Elem::Int), Elem::Dint)));
        assert_eq!(conversion_names("TO_UINT"), Some((None, Elem::Uint)));
        assert_eq!(conversion_names("TIME_OF_DAY_TO_DT"), Some((Some(Elem::Tod), Elem::Dt)));
        assert_eq!(conversion_names("SHL"), None);
    }
}

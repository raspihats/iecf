//! Expressions: each becomes a C expression of a stated elementary type,
//! with no undefined behaviour - what can overflow, divide by zero or go
//! out of range calls a helper of runtime/iecf.h that faults instead.

use crate::names::{Place, Scope};
use crate::{Gen, R, c_type};
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_sema::stdlib::{Dir, unify};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;

const FAULT: &str = "&ctx->fault";

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
    lit(0, e).unwrap_or_else(|| "0".into())
}

fn arg_value(args: &[Arg], span: Option<Span>) -> Option<&Expr> {
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
        for ((_, t, default), x) in sig.inputs.iter().zip(given) {
            codes.push(match x {
                Some(x) => self.value_as(x, *t, scope)?,
                None => default.clone(),
            });
        }
        self.may_fault = true;
        Ok(format!("F_{}({})", sig.key, codes.join(", ")))
    }

    fn ty_of(&mut self, e: &Expr) -> R<Ty> {
        match self.types.expr(e.span) {
            Some(t) => Ok(t.clone()),
            None => self.not_yet(e.span, "this expression"),
        }
    }

    /// `e` as a C expression of type `want`, by an implicit conversion
    /// (Ed.3 Figure 11, which only widens) where its own type differs.
    pub(crate) fn value_as(&mut self, e: &Expr, want: Elem, scope: &Scope) -> R<String> {
        match self.ty_of(e)? {
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
                } else if is_intlike(own) && is_intlike(want) {
                    Ok(format!("(({})({code}))", c_type(want)))
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
            Ty::Elem(t) if repr(t).is_some() => Ok(t),
            Ty::IntLit(_) => Ok(match const_int(e) {
                Some(v) if v > i64::MAX as i128 => Elem::Ulint,
                _ => Elem::Lint,
            }),
            t => self.not_yet(e.span, format!("a value of type {t}")),
        }
    }

    /// `e` as a C expression of its own type.
    pub(crate) fn value(&mut self, e: &Expr, scope: &Scope) -> R<String> {
        if matches!(self.ty_of(e)?, Ty::IntLit(_)) {
            let t = self.elem_of(e)?;
            return self.value_as(e, t, scope);
        }
        match &e.kind {
            ExprKind::Literal(l) if l.kind == LiteralKind::Time => {
                let t = self.elem_of(e)?;
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
            ExprKind::Literal(l) => {
                let t = self.elem_of(e)?;
                match literal_int(l).and_then(|v| lit(v, t)) {
                    Some(c) => Ok(c),
                    None => self.not_yet(e.span, "this literal"),
                }
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
                Some(p) => Ok(p.load()),
                None => self.not_yet(id.span, format!("'{}' here", id.text)),
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
                    UnaryOp::Neg if t.is_signed_int() => {
                        self.may_fault = true;
                        format!("iecf_neg_{}({FAULT}, {v})", t.name())
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
            let (ta, tb) = (self.ty_of(a)?, self.ty_of(b)?);
            let common = match unify(&ta, &tb) {
                Some(Ty::Elem(t)) if repr(t).is_some() => t,
                Some(Ty::IntLit(_)) => self.elem_of(a)?,
                Some(t) => return self.not_yet(e.span, format!("comparing {t}")),
                None => return self.not_yet(e.span, "this comparison"),
            };
            let (x, y) = (self.value_as(a, common, scope)?, self.value_as(b, common, scope)?);
            let sym = match op {
                Eq => "==",
                Ne => "!=",
                Lt => "<",
                Le => "<=",
                Gt => ">",
                _ => ">=",
            };
            return Ok(format!("((IEC_BOOL)(({x}) {sym} ({y})))"));
        }
        let t = self.elem_of(e)?;
        if t.is_duration() && !matches!(op, Add | Sub) {
            return self.not_yet(e.span, format!("{} * and / on a duration", t.name()));
        }
        let (x, y) = (self.value_as(a, t, scope)?, self.value_as(b, t, scope)?);
        let ct = c_type(t);
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
            Add | Sub | Mul | Div if t.is_int() => {
                self.may_fault = true;
                let f = match op {
                    Add => "add",
                    Sub => "sub",
                    Mul => "mul",
                    _ => "div",
                };
                format!("iecf_{f}_{n}({FAULT}, {x}, {y})")
            }
            Mod if t.is_int() => format!("iecf_mod_{n}({x}, {y})"),
            _ => return self.not_yet(e.span, format!("this operator on {n}")),
        })
    }

    fn call(&mut self, e: &Expr, callee: &Expr, args: &[Arg], scope: &Scope) -> R<String> {
        let Some(sc) = self.types.std_call(e.span).cloned() else {
            return self.user_call(e, callee, args, scope);
        };
        if args.iter().any(|a| match a {
            Arg::Input(n, _) | Arg::Output { name: n, .. } => n.is("EN") || n.is("ENO"),
            _ => false,
        }) || sc.outputs.iter().any(|(_, s)| s.is_some())
        {
            return self.not_yet(e.span, "EN, ENO or outputs of a standard function");
        }
        let input = |name: &str| sc.inputs.iter().find(|(n, _)| n == name).and_then(|(_, s)| arg_value(args, *s));
        let all: Vec<&Expr> = sc.inputs.iter().filter_map(|(_, s)| arg_value(args, *s)).collect();
        let result = self.elem_of(e)?;
        let rn = result.name();
        match sc.name.as_str() {
            "ABS" => {
                let x = self.value_as(all[0], result, scope)?;
                if result.is_signed_int() {
                    self.may_fault = true;
                    Ok(format!("iecf_abs_{rn}({FAULT}, {x})"))
                } else {
                    Ok(x)
                }
            }
            "MIN" | "MAX" => {
                let f = sc.name.to_ascii_lowercase();
                let mut acc = self.value_as(all[0], result, scope)?;
                for x in &all[1..] {
                    let v = self.value_as(x, result, scope)?;
                    acc = format!("iecf_{f}_{rn}({acc}, {v})");
                }
                Ok(acc)
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
                Ok(format!("iecf_limit_{rn}({mn}, {i}, {mx})"))
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
            name => {
                let Some((named_from, to)) = conversion_names(name) else {
                    return self.not_yet(e.span, format!("the standard function {name}"));
                };
                let Some(x) = all.first() else { return self.not_yet(e.span, "this conversion") };
                let from = match (self.ty_of(x)?, named_from) {
                    (Ty::IntLit(_), None) => return self.value_as(x, to, scope),
                    (Ty::IntLit(_), Some(f)) => f,
                    (Ty::Elem(t), _) => t,
                    (t, _) => return self.not_yet(x.span, format!("converting {t}")),
                };
                let code = self.value_as(x, from, scope)?;
                self.convert(e.span, code, from, to)
            }
        }
    }

    /// `code`, of type `from`, as `to`: numbers keep their value or fault
    /// (Ed.3 6.6.2.5.3); a bit string on either side is a binary transfer
    /// (6.6.2.5.4, 6.6.2.5.5).
    fn convert(&mut self, at: Span, code: String, from: Elem, to: Elem) -> R<String> {
        let ct = c_type(to);
        if from == to {
            return Ok(code);
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
            self.may_fault = true;
            return Ok(match (from.is_signed_int(), to.is_signed_int()) {
                (true, true) => {
                    format!("(({ct})iecf_s_in({FAULT}, (int64_t)({code}), INT64_C({tlo}), INT64_C({thi})))")
                }
                (true, false) => format!("(({ct})iecf_su_in({FAULT}, (int64_t)({code}), UINT64_C({thi})))"),
                (false, false) => format!("(({ct})iecf_u_in({FAULT}, (uint64_t)({code}), UINT64_C({thi})))"),
                (false, true) => format!("(({ct})iecf_us_in({FAULT}, (uint64_t)({code}), INT64_C({thi})))"),
            });
        }
        // A binary transfer: the low bytes, or zeros added on the left.
        if from == Elem::Bool {
            return Ok(format!("(({ct})({code}))"));
        }
        if to == Elem::Bool {
            return Ok(format!("((IEC_BOOL)(({code}) & 1u))"));
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

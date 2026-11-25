//! A chain of REAL or LREAL `+`, `-`, `*` and `/` tested once, at its end
//! (docs/c-output.md §5).
//!
//! Each of `+`, `-` and `*` gives a result that is not finite whenever an
//! operand is not (an infinity, a NaN), so a chain whose last result is
//! finite had no infinite or NaN result anywhere: nothing to flag, and the
//! value is the operators' own, each rounded to the type as runtime/iecf.h
//! rounds it. Only when the last result is not finite is the chain worked
//! out again operator by operator, through iecf.h's flagging helpers, so
//! the flag is the one a test after each operator gives, in its order.
//! Twice is safe because a chain's leaves are variables, their fields and
//! literals, which read the same each time; a call of a FUNCTION ends a
//! chain. A chain with a standard function or an array element in it
//! (below) is worked out once in the test, into a variable declared before
//! the statement (`chN`): C may not merge two calls of `exp`, which may set
//! errno. Such a chain is formed only in a statement's own expression,
//! where a variable can be declared before it.
//!
//! `/` joins a chain when its divisor is a leaf. Its quotient is not
//! finite whenever its dividend is not; by a finite divisor other than 0
//! it is otherwise what the helper gives, unflagged when finite; by an
//! infinite one it is 0 or NaN, as the helper's, which flags the NaN and
//! not the 0. A divisor of 0 faults: the chain divides only when every
//! divisor is other than 0 (no division by zero in the C, §1), else it is
//! worked out operator by operator, which flags what comes before and
//! faults at the division, in order. A divisor that is itself a result,
//! of `+`, `-`, `*`, `/` of leaves and no call, is tested on its own as
//! well, finite and not 0: an infinite one would give 0, hiding its flag,
//! so then the chain is worked out operator by operator too.
//!
//! A standard function or conversion joins a chain where it, too, gives a
//! result that is not finite whenever its argument is not (added
//! 2026-11-25, florin: "do 1,2,3"): `SQRT`, `LN`, `LOG`, `SIN`, `COS`,
//! `TAN`, `ASIN`, `ACOS` and `ABS` of a REAL or LREAL (C's functions keep
//! a NaN a NaN, and take an infinity to an infinity or a NaN), and a REAL
//! converted to an LREAL or back (`iecf_real_raw`: beyond REAL's range,
//! an infinity). Their own infinite or NaN results - `LN(0.0)`,
//! `SQRT(-1.0)` - stay so to the chain's end, and are flagged there as
//! their helpers flag them. `EXP` and `ATAN` take an infinity to a finite
//! value (`EXP(-inf)` is 0): they join only with a leaf for argument,
//! whose own value no operator flagged. A number converted to a REAL or
//! LREAL from a leaf is a leaf, always finite; and so is an element of an
//! array of a leaf, indexed by leaves and literals, which reads the same
//! each time (its index checked each time, a fault the first's).

use crate::expr::strip_paren;
use crate::names::{Place, Scope};
use crate::{Gen, R, c_type};
use iecf_sema::types::Elem;
use iecf_syntax::ast::*;

impl Gen<'_> {
    /// `e`, a `+`, `-` or `*` of type `t`, as a chain tested once - if it
    /// is one, of two operators at least.
    pub(crate) fn real_chain(&mut self, e: &Expr, t: Elem, scope: &Scope) -> R<Option<String>> {
        if self.no_chain || !t.is_real() || self.chain(e, t, scope)?.unwrap_or(0) < 2 {
            return Ok(None);
        }
        // One with a call or an index needs a variable before the
        // statement: only in a statement's own expression.
        if !no_call(e) && !self.pre_ok {
            return Ok(None);
        }
        let mut divisors = Vec::new();
        let raw = self.raw(e, t, scope, &mut divisors)?;
        let was = std::mem::replace(&mut self.no_chain, true);
        let checked = self.value_as(e, t, scope);
        self.no_chain = was;
        let checked = checked?;
        let mut test = Vec::new();
        for d in &divisors {
            if !d.leaf {
                test.push(format!("iecf_finite_{}({})", d.t.name(), d.code));
            }
            test.push(format!("({}) != ({})0", d.code, c_type(d.t)));
        }
        if no_call(e) {
            test.push(format!("iecf_finite_{}({raw})", t.name()));
            return Ok(Some(format!("({} ? ({raw}) : ({checked}))", test.join(" && "))));
        }
        // A call or an index is worked out once: C may not merge two
        // calls of `exp`, which may set errno. Its value goes to a
        // variable declared before the statement and set here, in order.
        let v = self.fresh("ch");
        self.str_temps.push(format!("{} {v} = 0;", c_type(t)));
        test.push(format!("iecf_finite_{}({v} = ({raw}))", t.name()));
        Ok(Some(format!("({} ? {v} : ({checked}))", test.join(" && "))))
    }

    /// The operators of the chain `e` is, 0 for a leaf; None when `e` is
    /// neither.
    fn chain(&mut self, e: &Expr, t: Elem, scope: &Scope) -> R<Option<usize>> {
        let e = strip_paren(e);
        Ok(match &e.kind {
            ExprKind::Binary(BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul, a, b) => {
                if self.elem_of(e)? != t {
                    return Ok(None);
                }
                match (self.chain(a, t, scope)?, self.chain(b, t, scope)?) {
                    (Some(x), Some(y)) => Some(x + y + 1),
                    _ => None,
                }
            }
            // By a leaf, or a result of no call, tested on its own.
            ExprKind::Binary(BinaryOp::Div, a, b) => {
                if self.elem_of(e)? != t {
                    return Ok(None);
                }
                match (self.chain(a, t, scope)?, self.chain(b, t, scope)?) {
                    (Some(x), Some(0)) => Some(x + 1),
                    (Some(x), Some(y)) if no_call(b) => Some(x + y + 1),
                    _ => None,
                }
            }
            ExprKind::Unary(UnaryOp::Neg | UnaryOp::Plus, x) if self.elem_of(e)? == t => self.chain(x, t, scope)?,
            ExprKind::Literal(_) => Some(0),
            ExprKind::Name(_) | ExprKind::Member(..) => read_only(e, scope).then_some(0),
            ExprKind::Index(base, idx) => (read_only(base, scope)
                && idx.iter().all(|i| matches!(strip_paren(i).kind, ExprKind::Literal(_)) || read_only(i, scope)))
            .then_some(0),
            ExprKind::Call { args, .. } => match self.chain_call(e, args, t)? {
                Some(Joins::Passes(x, from)) => self.chain(x, from, scope)?.map(|n| n + 1),
                Some(Joins::LeafOnly(x)) => (self.chain(x, t, scope)? == Some(0)).then_some(1),
                Some(Joins::Leaf(x)) => {
                    (matches!(strip_paren(x).kind, ExprKind::Literal(_)) || read_only(x, scope)).then_some(0)
                }
                None => None,
            },
            _ => None,
        })
    }

    /// How the standard call `e` of type `t`, one argument and neither EN
    /// nor an output, joins a chain: its argument and the type it is
    /// worked out in.
    fn chain_call<'e>(&mut self, e: &'e Expr, args: &'e [Arg], t: Elem) -> R<Option<Joins<'e>>> {
        let Some(sc) = self.types.std_call(e.span).cloned() else { return Ok(None) };
        if self.elem_of(e)? != t
            || sc.inputs.len() != 1
            || !sc.outputs.is_empty()
            || args.len() != 1
            || args.iter().any(|a| matches!(a, Arg::Input(n, _) if n.is("EN")))
        {
            return Ok(None);
        }
        let Some(x) = args.iter().find_map(|a| match a {
            Arg::Positional(x) | Arg::Input(_, x) => Some(x),
            _ => None,
        }) else {
            return Ok(None);
        };
        Ok(match sc.name.as_str() {
            "SQRT" | "LN" | "LOG" | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ABS" => Some(Joins::Passes(x, t)),
            "EXP" | "ATAN" => Some(Joins::LeafOnly(x)),
            name => match crate::expr::conversion_names(name) {
                Some((named, to)) if to == t => {
                    let from = match named {
                        Some(f) => f,
                        None => self.elem_of(x)?,
                    };
                    if from.is_real() {
                        Some(Joins::Passes(x, from))
                    } else if from.is_int() {
                        Some(Joins::Leaf(x))
                    } else {
                        None
                    }
                }
                _ => None,
            },
        })
    }

    /// The chain `e` with no test: each operator's result cast to the type,
    /// as iecf.h's helpers cast it; each divisor added to `divisors`.
    fn raw(&mut self, e: &Expr, t: Elem, scope: &Scope, divisors: &mut Vec<Divisor>) -> R<String> {
        let ct = c_type(t);
        let e = strip_paren(e);
        Ok(match &e.kind {
            ExprKind::Binary(op @ (BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div), a, b) => {
                let sym = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Div => "/",
                    _ => "*",
                };
                let leaf = *op == BinaryOp::Div && self.chain(b, t, scope)? == Some(0);
                let a = self.raw(a, t, scope, divisors)?;
                let b = self.raw(b, t, scope, divisors)?;
                if *op == BinaryOp::Div {
                    divisors.push(Divisor { code: b.clone(), t, leaf });
                }
                format!("(({ct})(({a}) {sym} ({b})))")
            }
            ExprKind::Unary(UnaryOp::Neg, x) => format!("(({ct})(-({})))", self.raw(x, t, scope, divisors)?),
            ExprKind::Unary(UnaryOp::Plus, x) => self.raw(x, t, scope, divisors)?,
            ExprKind::Call { args, .. } => match self.chain_call(e, args, t)? {
                Some(Joins::Passes(x, from)) => {
                    let inner = self.raw(x, from, scope, divisors)?;
                    self.raw_call(e, t, from, &inner)
                }
                Some(Joins::LeafOnly(x)) => {
                    let inner = self.raw(x, t, scope, divisors)?;
                    self.raw_call(e, t, t, &inner)
                }
                Some(Joins::Leaf(_)) | None => self.value_as(e, t, scope)?,
            },
            _ => self.value_as(e, t, scope)?,
        })
    }
}

impl Gen<'_> {
    /// The standard call `e`, of type `t`, of `inner` (of type `from`),
    /// unflagged: what expr.rs writes, without its flagging helper.
    fn raw_call(&mut self, e: &Expr, t: Elem, from: Elem, inner: &str) -> String {
        let name = self.types.std_call(e.span).map(|sc| sc.name.clone()).unwrap_or_default();
        if let Some(f) = crate::expr::math_function(&name) {
            let macro_ = format!("IECF_{name}");
            self.math.insert((macro_.clone(), f));
            return if t == Elem::Real {
                format!("iecf_real_raw({macro_}((double)({inner})))")
            } else {
                format!("((IEC_LREAL){macro_}({inner}))")
            };
        }
        if name == "ABS" {
            return format!("iecf_abs_{}({inner})", t.name());
        }
        match (from, t) {
            (Elem::Lreal, Elem::Real) => format!("iecf_real_raw({inner})"),
            (Elem::Real, Elem::Lreal) => format!("((IEC_LREAL)({inner}))"),
            _ => inner.to_string(),
        }
    }
}

/// How a standard call joins a chain ([`Gen::chain_call`]).
enum Joins<'e> {
    /// Its result is not finite whenever its argument, worked out in the
    /// type given, is not: a function of Table 28, a REAL or LREAL
    /// conversion.
    Passes(&'e Expr, Elem),
    /// `EXP` or `ATAN`: with a leaf for argument only.
    LeafOnly(&'e Expr),
    /// A number converted to a REAL or LREAL: a leaf, when its argument is.
    Leaf(&'e Expr),
}

/// A divisor of a chain: tested other than 0, and finite when it is not
/// a leaf.
struct Divisor {
    code: String,
    t: Elem,
    leaf: bool,
}

/// Whether `e` has no call in it.
fn no_call(e: &Expr) -> bool {
    match &strip_paren(e).kind {
        ExprKind::Binary(_, a, b) => no_call(a) && no_call(b),
        ExprKind::Unary(_, x) => no_call(x),
        ExprKind::Call { .. } | ExprKind::Index(..) => false,
        _ => true,
    }
}

/// Whether reading `e` - a variable, or a field of one - has no effect
/// and gives the same each time: no call, no index, no reference.
fn read_only(e: &Expr, scope: &Scope) -> bool {
    match &strip_paren(e).kind {
        ExprKind::Name(id) => matches!(
            scope.vars.get(&id.text.to_ascii_uppercase()),
            Some(Place::Field(..) | Place::Located { .. } | Place::Block(..) | Place::Fb(..) | Place::Data(..))
        ),
        ExprKind::Member(x, _) => read_only(x, scope),
        _ => false,
    }
}

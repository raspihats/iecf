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
//! literals, which read the same each time; a call or an index ends a
//! chain.
//!
//! `/` joins a chain when its divisor is a leaf. Its quotient is not
//! finite whenever its dividend is not; by a finite divisor other than 0
//! it is otherwise what the helper gives, unflagged when finite; by an
//! infinite one it is 0 or NaN, as the helper's, which flags the NaN and
//! not the 0. A divisor of 0 faults: the chain divides only when every
//! divisor is other than 0 (no division by zero in the C, §1), else it is
//! worked out operator by operator, which flags what comes before and
//! faults at the division, in order. A divisor that is itself a result
//! ends the chain: an infinite one would give 0, hiding its flag.

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
        let mut divisors = Vec::new();
        let raw = self.raw(e, t, scope, &mut divisors)?;
        let was = std::mem::replace(&mut self.no_chain, true);
        let checked = self.value_as(e, t, scope);
        self.no_chain = was;
        let checked = checked?;
        let mut test: Vec<String> = divisors.iter().map(|d| format!("({d}) != ({})0", c_type(t))).collect();
        test.push(format!("iecf_finite_{}({raw})", t.name()));
        Ok(Some(format!("({} ? ({raw}) : ({checked}))", test.join(" && "))))
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
            // By a leaf alone.
            ExprKind::Binary(BinaryOp::Div, a, b) => {
                if self.elem_of(e)? != t {
                    return Ok(None);
                }
                match (self.chain(a, t, scope)?, self.chain(b, t, scope)?) {
                    (Some(x), Some(0)) => Some(x + 1),
                    _ => None,
                }
            }
            ExprKind::Unary(UnaryOp::Neg | UnaryOp::Plus, x) if self.elem_of(e)? == t => self.chain(x, t, scope)?,
            ExprKind::Literal(_) => Some(0),
            ExprKind::Name(_) | ExprKind::Member(..) => read_only(e, scope).then_some(0),
            _ => None,
        })
    }

    /// The chain `e` with no test: each operator's result cast to the type,
    /// as iecf.h's helpers cast it; each divisor added to `divisors`.
    fn raw(&mut self, e: &Expr, t: Elem, scope: &Scope, divisors: &mut Vec<String>) -> R<String> {
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
                let a = self.raw(a, t, scope, divisors)?;
                let b = self.raw(b, t, scope, divisors)?;
                if *op == BinaryOp::Div {
                    divisors.push(b.clone());
                }
                format!("(({ct})(({a}) {sym} ({b})))")
            }
            ExprKind::Unary(UnaryOp::Neg, x) => format!("(({ct})(-({})))", self.raw(x, t, scope, divisors)?),
            ExprKind::Unary(UnaryOp::Plus, x) => self.raw(x, t, scope, divisors)?,
            _ => self.value_as(e, t, scope)?,
        })
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

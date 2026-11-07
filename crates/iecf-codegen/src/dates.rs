//! DATE, TOD and DT (docs/dependencies.md): each a `uint32_t`, DATE and
//! DT in seconds since 1970, TOD in milliseconds since midnight; Ed.3
//! Table 30's arithmetic and the conversions between them through
//! runtime/iecf.h, which wraps a TOD at midnight and wraps and flags a
//! DATE, DT or difference out of range.

use crate::names::Scope;
use crate::{Gen, R};
use iecf_diagnostics::Span;
use iecf_sema::types::Elem;
use iecf_syntax::ast::{BinaryOp, Expr};

const FLAG: &str = "&ctx->flag";
const FAULT: &str = "&ctx->fault";

pub(crate) fn is_date(e: Elem) -> bool {
    matches!(e, Elem::Date | Elem::Tod | Elem::Dt)
}

impl Gen<'_> {
    /// `a op b` where either is a DATE, TOD or DT (Ed.3 Table 30), as the
    /// operator or as ADD_TOD_TIME and the other functions by name.
    pub(crate) fn date_arith(&mut self, at: Span, op: BinaryOp, a: &Expr, b: &Expr, scope: &Scope) -> R<String> {
        let (ta, tb) = (self.elem_of(a)?, self.elem_of(b)?);
        let (x, y) = (self.value(a, scope)?, self.value(b, scope)?);
        use BinaryOp::{Add, Sub};
        use Elem::{Date, Dt, Time, Tod};
        Ok(match (op, ta, tb) {
            (Add, Tod, Time) => format!("iecf_add_TOD({x}, {y})"),
            (Sub, Tod, Time) => format!("iecf_sub_TOD({x}, {y})"),
            (Sub, Tod, Tod) => {
                self.may_flag = true;
                format!("iecf_sub_TOD_TOD({FLAG}, {x}, {y})")
            }
            (Add, Dt, Time) => {
                self.may_flag = true;
                format!("iecf_add_DT({FLAG}, {x}, {y})")
            }
            (Sub, Dt, Time) => {
                self.may_flag = true;
                format!("iecf_sub_DT({FLAG}, {x}, {y})")
            }
            (Sub, Dt, Dt) | (Sub, Date, Date) => {
                self.may_flag = true;
                format!("iecf_sub_DT_DT({FLAG}, {x}, {y})")
            }
            _ => return self.not_yet(at, format!("{} and {} here", ta.name(), tb.name())),
        })
    }

    /// A standard function on DATE, TOD or DT, if `name` is one.
    pub(crate) fn date_call(&mut self, at: Span, name: &str, all: &[&Expr], scope: &Scope) -> R<Option<String>> {
        let op = match name {
            "ADD_TOD_TIME" | "ADD_DT_TIME" => Some(BinaryOp::Add),
            "SUB_DATE_DATE" | "SUB_TOD_TIME" | "SUB_TOD_TOD" | "SUB_DT_TIME" | "SUB_DT_DT" => Some(BinaryOp::Sub),
            "ADD" | "SUB" if all.len() == 2 => {
                let (ta, tb) = (self.elem_of(all[0])?, self.elem_of(all[1])?);
                if !(is_date(ta) || is_date(tb)) {
                    return Ok(None);
                }
                Some(if name == "ADD" { BinaryOp::Add } else { BinaryOp::Sub })
            }
            _ => None,
        };
        if let Some(op) = op {
            return self.date_arith(at, op, all[0], all[1], scope).map(Some);
        }
        Ok(Some(match name {
            "CONCAT_DATE_TOD" => {
                let d = self.value_as(all[0], Elem::Date, scope)?;
                let t = self.value_as(all[1], Elem::Tod, scope)?;
                self.may_flag = true;
                format!("iecf_concat_date_tod({FLAG}, {d}, {t})")
            }
            "DAY_OF_WEEK" => {
                let d = self.value_as(all[0], Elem::Date, scope)?;
                format!("iecf_day_of_week({d})")
            }
            _ => return Ok(None),
        }))
    }
}

impl Gen<'_> {
    /// A TIME or LTIME `t` times or divided by a number (Ed.3 Table 30,
    /// MUL_TIME, DIV_TIME): `d * n`, `n * d`, `d / n`. By an integer,
    /// exact, an overflow wrapped and flagged; by a real, computed in
    /// LREAL and rounded half to even as REAL_TO_DINT is, out of range
    /// flagged and the statement left undone (docs/dependencies.md).
    /// Dividing by 0 or 0.0 faults.
    pub(crate) fn duration_scale(
        &mut self,
        at: Span,
        op: BinaryOp,
        t: Elem,
        a: &Expr,
        b: &Expr,
        scope: &Scope,
    ) -> R<String> {
        let (d, n) = if self.elem_of(a)?.is_duration() { (a, b) } else { (b, a) };
        if op == BinaryOp::Div && !std::ptr::eq(d, a) {
            return self.not_yet(at, "a number divided by a duration");
        }
        let held = if t == Elem::Time { Elem::Dint } else { Elem::Lint };
        let x = self.value_as(d, t, scope)?;
        let nt = self.elem_of(n)?;
        let div = op == BinaryOp::Div;
        self.may_flag = true;
        if nt.is_real() {
            let f = self.value_as(n, Elem::Lreal, scope)?;
            let v = if div {
                self.may_fault = true;
                format!("iecf_div_LREAL({FAULT}, {FLAG}, (double)({x}), {f})")
            } else {
                format!("((double)({x}) * ({f}))")
            };
            let r = self.real_to_int(at, v, held, "round")?;
            return Ok(format!("(({})({r}))", crate::c_type(t)));
        }
        if !nt.is_int() {
            return self.not_yet(n.span, format!("a duration and {}", nt.name()));
        }
        let f = if nt == Elem::Ulint {
            let c = self.value(n, scope)?;
            self.convert(n.span, c, Elem::Ulint, Elem::Lint)?
        } else {
            self.value_as(n, Elem::Lint, scope)?
        };
        let v = if div {
            self.may_fault = true;
            format!("iecf_div_LINT({FAULT}, {FLAG}, (int64_t)({x}), {f})")
        } else {
            format!("iecf_mul_LINT({FLAG}, (int64_t)({x}), {f})")
        };
        Ok(if t == Elem::Time { format!("((IEC_TIME)iecf_fit_DINT({FLAG}, {v}))") } else { v })
    }
}

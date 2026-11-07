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

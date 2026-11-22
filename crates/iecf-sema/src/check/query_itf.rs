//! CODESYS's `__QUERYINTERFACE(a, b)` (the query-interface extension,
//! query_itf.rs): `b ?= a` (Ed.3 6.6.6.7) in an expression, TRUE when
//! `b` then holds an instance - when `a`'s instance's type implements
//! `b`'s interface - and `b` NULL otherwise.

use super::Checker;
use crate::types::{Elem, Ty};
use iecf_diagnostics::{Code, Span};
use iecf_syntax::ast::*;

impl<'a> Checker<'a> {
    pub(super) fn query_itf_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let gives = Ty::Elem(Elem::Bool);
        let [Arg::Positional(from), Arg::Positional(into)] = args else {
            self.error(
                Code::BadCall,
                span,
                "__QUERYINTERFACE takes an interface and an interface variable: __QUERYINTERFACE(a, b)",
            );
            self.args_only(args);
            return gives;
        };
        let got = self.expr(from);
        let want = self.place(into);
        match &got {
            Ty::Itf(_) | Ty::Null | Ty::Unknown => {}
            Ty::Fb(_) => self.error(
                Code::BadCall,
                from.span,
                "__QUERYINTERFACE in iecf takes an interface first, not an instance: whether an instance's type \
                 implements an interface is known as it is compiled; give it with `b := inst`",
            ),
            t => {
                self.error(Code::OperandTypes, from.span, format!("__QUERYINTERFACE takes an interface first, not {t}"))
            }
        }
        if !matches!(want, Ty::Itf(_) | Ty::Unknown) {
            self.error(
                Code::OperandTypes,
                into.span,
                format!("__QUERYINTERFACE stores into a variable of an interface, not {want}"),
            );
        }
        gives
    }
}

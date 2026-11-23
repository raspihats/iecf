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

    /// `__QUERYPOINTER(a, p)` (query-pointer): `p ?= a`, `p` a POINTER TO
    /// a block - a REF_TO it - TRUE when `p` then holds an instance: the
    /// one `a` refers to, if it is of `p`'s block's type or derived from
    /// it; `p` NULL otherwise. CODESYS leaves the type to the program.
    pub(super) fn query_ptr_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let gives = Ty::Elem(Elem::Bool);
        let [Arg::Positional(from), Arg::Positional(into)] = args else {
            self.error(
                Code::BadCall,
                span,
                "__QUERYPOINTER takes an interface and a pointer variable: __QUERYPOINTER(a, p)",
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
                "__QUERYPOINTER in iecf takes an interface first, not an instance: a pointer to an instance is \
                 `p := ADR(inst)`",
            ),
            t => self.error(Code::OperandTypes, from.span, format!("__QUERYPOINTER takes an interface first, not {t}")),
        }
        let block = matches!(&want, Ty::Ref(t) if matches!(**t, Ty::Fb(_)));
        if !block && !want.is_unknown() {
            self.error(
                Code::OperandTypes,
                into.span,
                format!("__QUERYPOINTER stores into a POINTER TO a function block or a class, not {want}"),
            );
        }
        gives
    }
}

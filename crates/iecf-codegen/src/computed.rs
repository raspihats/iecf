//! Initial values worked out as their variables are set up (the
//! initial-expressions extension, docs/extensions.md): a variable's
//! initial value that names variables or calls functions, which the
//! checker allowed and ordered (`Types::init_expr`, E0036).
//!
//! Where the variable is set up, its type's initial value is written
//! and a marker left (`later`); once the POU's variables are all known,
//! the marker becomes the assignment of the value to the variable,
//! compiled as a statement of the POU is (`fill_computed`): converted,
//! clamped and flagged as an assignment is, a fault stopping the set-up
//! where it is (docs/c-output.md §5).

use crate::names::{Place, Scope};
use crate::stmt::Body;
use crate::{Gen, R};
use iecf_sema::Extension;
use iecf_syntax::ast::{Expr, ExprKind, Ident, Stmt, StmtKind};
use std::fmt::Write as _;

/// The line an initial value worked out as its variable is set up
/// leaves, until `fill_computed`.
fn marker(n: usize) -> String {
    format!("/*@INIT{n}@*/")
}

/// Whether `text` has such a line.
pub(crate) fn has_marker(text: &str) -> bool {
    text.contains("/*@INIT")
}

/// The name the variable being set up is reached by while its value is
/// assigned: no IEC name holds `@`.
const TARGET: &str = "@INIT";

impl Gen<'_> {
    /// Whether `x`, an initial value, is worked out as its variable is
    /// set up: the checker allowed it, and it is no constant expression.
    pub(crate) fn computed(&mut self, x: &Expr) -> bool {
        self.types.has(Extension::InitialExpressions) && self.types.init_expr(x.span) && self.not_constant(x).is_some()
    }

    /// What E0032 adds where the initial value `x` would be worked out
    /// as its variable is set up, were the extension on.
    pub(crate) fn computed_hint(&self, x: &Expr) -> &'static str {
        if !self.types.has(Extension::InitialExpressions) && self.types.init_expr(x.span) {
            "; the extension initial-expressions works it out as the variable is set up"
        } else {
            ""
        }
    }

    /// The marker of `x` given to the variable at `place`, into `out`.
    pub(crate) fn later(&mut self, place: Place, x: &Expr, out: &mut String) {
        let n = self.computed_inits.len();
        self.computed_inits.push((place, x.clone()));
        let _ = writeln!(out, "    {}", marker(n));
    }

    /// The markers left in `text`, each the assignment of its value,
    /// now that `scope` has the POU's variables: a statement of the POU
    /// `pou`, which leaves by `ret` if it faults.
    pub(crate) fn fill_computed(&mut self, text: &mut String, scope: &Scope, pou: u16, ret: &str) -> R<()> {
        if !has_marker(text) {
            return Ok(());
        }
        let mut result = Ok(());
        for n in 0..self.computed_inits.len() {
            let m = marker(n);
            if !text.contains(&m) {
                continue;
            }
            let (place, value) = self.computed_inits[n].clone();
            let mut sc = scope.clone();
            sc.vars.insert(TARGET.into(), place);
            let at = value.span;
            let target = Expr { kind: ExprKind::Name(Ident { text: TARGET.into(), span: at }), span: at };
            let s = Stmt { kind: StmtKind::Assign { target, value }, span: at };
            let mut code = String::new();
            // Set up, not run: no debug hook stops there.
            let debug = std::mem::replace(&mut self.debug, false);
            let mut cx = Body {
                pou,
                scope: &sc,
                out: &mut code,
                depth: 1,
                loops: Vec::new(),
                ret: ret.to_string(),
                on_return: None,
            };
            let r = self.stmts(std::slice::from_ref(&s), &mut cx);
            self.debug = debug;
            if r.is_err() {
                result = r;
                continue;
            }
            *text = text.replacen(&format!("    {m}\n"), &code, 1);
        }
        result
    }
}

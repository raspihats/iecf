//! CODESYS's `REFERENCE TO` (the reference-to extension), written as the
//! `REF_TO` it stands for (refs.rs); and `THIS.x`, written as the `x` it
//! is.
//!
//! The checker decides at each use what a `REFERENCE TO` does
//! (`Types::ref_use`): where it stands for what it references it is
//! `r^`; a variable REF= or a call binds it to is `REF(x)`; `r REF= 0` is
//! `r := NULL`; and `r REF= x` is the assignment `r := REF(x)`, or a copy
//! of another reference, `r := r2`. Each body is rewritten so before it is
//! generated: in C a `REFERENCE TO` is an `iecf_ref_t` like a `REF_TO`,
//! checked where it is dereferenced, so an unbound one faults
//! (IECF_FAULT_NULL) as a NULL `REF_TO` does.
//!
//! A rewritten node keeps the span of the one it wraps: the checker
//! recorded there the type of the use, which is the wrapper's.

use iecf_sema::{RefUse, Types};
use iecf_syntax::ast::*;

/// The units, each POU's body and chart rewritten; the same units when
/// the project uses no `REFERENCE TO`.
pub(crate) fn rewrite(units: &[&SourceUnit], types: &Types) -> Option<Vec<SourceUnit>> {
    if !types.rewrites() {
        return None;
    }
    let mut out: Vec<SourceUnit> = units.iter().map(|u| (*u).clone()).collect();
    for u in &mut out {
        for item in &mut u.items {
            if let Item::Pou(p) = item {
                pou(p, types);
            }
        }
    }
    Some(out)
}

/// A POU's body, chart and methods.
fn pou(p: &mut Pou, types: &Types) {
    stmts(&mut p.body, types);
    for el in &mut p.sfc {
        match el {
            SfcElement::Action(a) => stmts(&mut a.body, types),
            SfcElement::Transition(t) => expr(&mut t.condition, types),
            SfcElement::Step(s) => {
                for a in &mut s.actions {
                    if let Some(t) = &mut a.time {
                        expr(t, types);
                    }
                }
            }
        }
    }
    for m in &mut p.methods {
        pou(m, types);
    }
}

fn stmts(body: &mut [Stmt], types: &Types) {
    for s in body {
        stmt(s, types);
    }
}

fn stmt(s: &mut Stmt, types: &Types) {
    match &mut s.kind {
        StmtKind::Assign { target, value } => {
            expr(target, types);
            expr(value, types);
        }
        StmtKind::RefAssign { target, value } => {
            expr(target, types);
            expr(value, types);
            let (target, value) = (target.clone(), value.clone());
            s.kind = StmtKind::Assign { target, value };
        }
        StmtKind::Call(e) => expr(e, types),
        StmtKind::If { branches, else_body } => {
            for (c, b) in branches {
                expr(c, types);
                stmts(b, types);
            }
            if let Some(b) = else_body {
                stmts(b, types);
            }
        }
        StmtKind::Case { selector, arms, else_body } => {
            expr(selector, types);
            for a in arms {
                stmts(&mut a.body, types);
            }
            if let Some(b) = else_body {
                stmts(b, types);
            }
        }
        StmtKind::For { from, to, by, body, .. } => {
            expr(from, types);
            expr(to, types);
            if let Some(b) = by {
                expr(b, types);
            }
            stmts(body, types);
        }
        StmtKind::While { cond, body } => {
            expr(cond, types);
            stmts(body, types);
        }
        StmtKind::Repeat { body, until } => {
            stmts(body, types);
            expr(until, types);
        }
        StmtKind::Exit | StmtKind::Continue | StmtKind::Return | StmtKind::Empty => {}
    }
}

/// `e` and what is in it, inside first, so a wrapper is not rewritten
/// again.
fn expr(e: &mut Expr, types: &Types) {
    match &mut e.kind {
        ExprKind::Member(b, _) | ExprKind::BitAccess(b, _) | ExprKind::Deref(b) | ExprKind::Paren(b) => expr(b, types),
        ExprKind::Unary(_, b) => expr(b, types),
        ExprKind::Index(b, idx) => {
            expr(b, types);
            for i in idx {
                expr(i, types);
            }
        }
        ExprKind::Binary(_, a, b) => {
            expr(a, types);
            expr(b, types);
        }
        ExprKind::Call { callee, args } => {
            expr(callee, types);
            for a in args {
                match a {
                    Arg::Positional(x) | Arg::Input(_, x) | Arg::Output { target: x, .. } => expr(x, types),
                }
            }
        }
        ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::DirectAddress(_)
        | ExprKind::EnumValue { .. }
        | ExprKind::Error => {}
    }
    let span = e.span;
    // `THIS.x`, the instance's own `x` (Ed.3 6.6.5.7.2): `x`.
    if types.this_member(span)
        && let ExprKind::Member(_, m) = &e.kind
    {
        e.kind = ExprKind::Name(m.clone());
    }
    match types.ref_use(span) {
        Some(RefUse::Deref) => {
            let inner = std::mem::replace(e, Expr { kind: ExprKind::Error, span });
            e.kind = ExprKind::Deref(Box::new(inner));
        }
        Some(RefUse::Bind) => {
            let inner = std::mem::replace(e, Expr { kind: ExprKind::Error, span });
            let callee = Expr { kind: ExprKind::Name(Ident { text: "REF".into(), span }), span };
            e.kind = ExprKind::Call { callee: Box::new(callee), args: vec![Arg::Positional(inner)] };
        }
        Some(RefUse::Unbind) => e.kind = ExprKind::Name(Ident { text: "NULL".into(), span }),
        None => {}
    }
}

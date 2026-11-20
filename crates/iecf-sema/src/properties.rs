//! CODESYS's PROPERTY (the properties extension), for the generator.
//!
//! The parser reads a property's GET and SET as the methods `PROPERTY_GET_p`
//! and `PROPERTY_SET_p`. The checker takes a read of `x.p` (or of `p` in its own
//! block) as the call `x.PROPERTY_GET_p()`, and `x.p := v` as `x.PROPERTY_SET_p(p :=
//! v)`, checking each as that call and keeping its span. Here, after the
//! check, each is written as that call, with the spans the check gave
//! it, so the generator sees only method calls.

use crate::Types;
use iecf_syntax::ast::*;

/// Rewrite every property read and assignment the check found.
pub fn rewrite(units: &mut [SourceUnit], types: &Types) {
    if types.prop_reads.is_empty() && types.prop_writes.is_empty() {
        return;
    }
    for unit in units.iter_mut() {
        for item in unit.items.iter_mut() {
            if let Item::Pou(p) = item {
                pou(p, types);
            }
        }
    }
}

fn pou(p: &mut Pou, types: &Types) {
    stmts(&mut p.body, types);
    for el in p.sfc.iter_mut() {
        match el {
            SfcElement::Action(a) => stmts(&mut a.body, types),
            SfcElement::Transition(t) => expr(&mut t.condition, types),
            SfcElement::Step(_) => {}
        }
    }
    for m in p.methods.iter_mut() {
        pou(m, types);
    }
}

/// `base.__<which>_m`, the accessor of property `m` of `base` - THIS when
/// there is none, a bare name in its own block.
fn accessor(base: Option<Expr>, m: &Ident, which: &str) -> Expr {
    let base = base.unwrap_or(Expr { kind: ExprKind::Name(Ident { text: "THIS".into(), span: m.span }), span: m.span });
    let id = Ident { text: format!("PROPERTY_{which}_{}", m.text.to_ascii_uppercase()), span: m.span };
    Expr { kind: ExprKind::Member(Box::new(base), id), span: m.span }
}

fn stmts(body: &mut [Stmt], types: &Types) {
    for s in body.iter_mut() {
        let span = s.span;
        match &mut s.kind {
            StmtKind::Assign { target, value } if types.prop_writes.contains(&span) => {
                expr(value, types);
                let (base, m) = match &mut target.kind {
                    ExprKind::Member(b, m) => {
                        expr(b, types);
                        (Some((**b).clone()), m.clone())
                    }
                    ExprKind::Name(id) => (None, id.clone()),
                    _ => continue,
                };
                let call = Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(accessor(base, &m, "SET")),
                        args: vec![Arg::Input(m.clone(), value.clone())],
                    },
                    span,
                };
                s.kind = StmtKind::Call(call);
            }
            StmtKind::Assign { target, value }
            | StmtKind::RefAssign { target, value }
            | StmtKind::AssignAttempt { target, value } => {
                expr(target, types);
                expr(value, types);
            }
            StmtKind::Call(e) => expr(e, types),
            StmtKind::If { branches, else_body } => {
                for (c, b) in branches.iter_mut() {
                    expr(c, types);
                    stmts(b, types);
                }
                if let Some(b) = else_body {
                    stmts(b, types);
                }
            }
            StmtKind::Case { selector, arms, else_body } => {
                expr(selector, types);
                for a in arms.iter_mut() {
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
            StmtKind::Label(_)
            | StmtKind::Jmp(_)
            | StmtKind::Exit
            | StmtKind::Continue
            | StmtKind::Return
            | StmtKind::Empty => {}
        }
    }
}

fn expr(e: &mut Expr, types: &Types) {
    match &mut e.kind {
        ExprKind::Member(x, _)
        | ExprKind::BitAccess(x, _)
        | ExprKind::Deref(x)
        | ExprKind::Unary(_, x)
        | ExprKind::Paren(x) => expr(x, types),
        ExprKind::Index(x, ix) => {
            expr(x, types);
            ix.iter_mut().for_each(|i| expr(i, types));
        }
        ExprKind::Binary(_, x, y) => {
            expr(x, types);
            expr(y, types);
        }
        ExprKind::Call { callee, args } => {
            expr(callee, types);
            for a in args.iter_mut() {
                match a {
                    Arg::Positional(x) | Arg::Input(_, x) | Arg::Output { target: x, .. } => expr(x, types),
                }
            }
        }
        _ => {}
    }
    if !types.prop_reads.contains(&e.span) {
        return;
    }
    let callee = match &e.kind {
        ExprKind::Member(b, m) => accessor(Some((**b).clone()), m, "GET"),
        ExprKind::Name(id) => accessor(None, id, "GET"),
        _ => return,
    };
    e.kind = ExprKind::Call { callee: Box::new(callee), args: Vec::new() };
}

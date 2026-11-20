//! CODESYS's way of writing object orientation (the codesys-oo
//! extension), rewritten as Ed.3 writes it before anything checks it, so
//! the checker and the code generator see only Ed.3's forms:
//!
//! - `THIS^` and `SUPER^` are `THIS` and `SUPER` (CODESYS's THIS and SUPER
//!   are pointers; Ed.3's are the instance): `THIS^.x`, `SUPER^.m()`,
//!   `SUPER^()`, the base's body;
//! - a method with no access specifier is PUBLIC, as CODESYS and TwinCAT
//!   have it ("PUBLIC or not specified: Access is not restricted"), not
//!   PROTECTED (Ed.3 6.6.5.9, OO.md D1);
//! - a method of a base method's name, not PRIVATE there, overrides it,
//!   OVERRIDE written or not (CODESYS has no OVERRIDE; OO.md D6).
//!
//! An interface's methods are left alone: they have no access and no
//! body.

use iecf_syntax::ast::*;
use std::collections::HashMap;

/// Whether `units` have anything this rewrites.
pub fn needed(units: &[&SourceUnit]) -> bool {
    let mut found = false;
    for unit in units {
        for item in &unit.items {
            if let Item::Pou(p) = item {
                found |= pou_needs(p);
            }
        }
    }
    found || !overrides(units.iter().copied()).is_empty()
}

fn pou_needs(p: &Pou) -> bool {
    let mut found = false;
    if p.kind != PouKind::Interface {
        found |= p.methods.iter().any(|m| m.specs.access.is_none());
    }
    let mut look = |e: &mut Expr| found |= is_ptr_self(e);
    // A read-only walk through the mutable walker, on a copy.
    let mut copy = p.clone();
    walk_pou(&mut copy, &mut |e| look(e));
    found
}

/// `THIS^` or `SUPER^`.
fn is_ptr_self(e: &Expr) -> bool {
    matches!(&e.kind, ExprKind::Deref(inner) if matches!(&inner.kind, ExprKind::Name(n) if n.is("THIS") || n.is("SUPER")))
}

/// Rewrite `units` as Ed.3 writes them.
pub fn rewrite(units: &mut [SourceUnit]) {
    let over = overrides(units.iter());
    for unit in units.iter_mut() {
        for item in unit.items.iter_mut() {
            let Item::Pou(p) = item else { continue };
            let owner = p.name.text.to_ascii_uppercase();
            if p.kind != PouKind::Interface {
                for m in p.methods.iter_mut() {
                    if m.specs.access.is_none() {
                        m.specs.access = Some((Access::Public, m.name.span));
                    }
                    let key = (owner.clone(), m.name.text.to_ascii_uppercase());
                    if m.specs.override_.is_none() && over.contains_key(&key) {
                        m.specs.override_ = Some(m.name.span);
                    }
                }
            }
            walk_pou(p, &mut |e| {
                if is_ptr_self(e) {
                    let ExprKind::Deref(inner) = std::mem::replace(&mut e.kind, ExprKind::Error) else {
                        unreachable!()
                    };
                    e.kind = inner.kind;
                }
            });
        }
    }
}

/// Each (block, method) that has a base method of its name, not PRIVATE
/// there, and no OVERRIDE written.
fn overrides<'a>(units: impl Iterator<Item = &'a SourceUnit>) -> HashMap<(String, String), ()> {
    let mut pous: HashMap<String, &Pou> = HashMap::new();
    for unit in units {
        for item in &unit.items {
            if let Item::Pou(p) = item
                && matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)
            {
                pous.insert(p.name.text.to_ascii_uppercase(), p);
            }
        }
    }
    let mut out = HashMap::new();
    for (name, p) in &pous {
        for m in p.methods.iter().filter(|m| m.specs.override_.is_none()) {
            let mname = m.name.text.to_ascii_uppercase();
            let mut base = p.extends.as_ref().map(|b| b.text.to_ascii_uppercase());
            let mut seen = 0;
            while let Some(b) = base {
                seen += 1;
                let Some(bp) = pous.get(&b) else { break };
                if seen > 64 {
                    break;
                }
                if bp.methods.iter().any(|bm| {
                    bm.name.text.eq_ignore_ascii_case(&mname) && !matches!(bm.specs.access, Some((Access::Private, _)))
                }) {
                    out.insert((name.clone(), mname.clone()), ());
                    break;
                }
                base = bp.extends.as_ref().map(|b| b.text.to_ascii_uppercase());
            }
        }
    }
    out
}

/// Every expression of a POU, its methods' and actions' too, in place.
fn walk_pou(p: &mut Pou, f: &mut dyn FnMut(&mut Expr)) {
    stmts(&mut p.body, f);
    for el in p.sfc.iter_mut() {
        match el {
            SfcElement::Action(a) => stmts(&mut a.body, f),
            SfcElement::Transition(t) => expr(&mut t.condition, f),
            SfcElement::Step(_) => {}
        }
    }
    for m in p.methods.iter_mut() {
        walk_pou(m, f);
    }
}

fn stmts(body: &mut [Stmt], f: &mut dyn FnMut(&mut Expr)) {
    for s in body {
        match &mut s.kind {
            StmtKind::Assign { target, value }
            | StmtKind::RefAssign { target, value }
            | StmtKind::AssignAttempt { target, value } => {
                expr(target, f);
                expr(value, f);
            }
            StmtKind::Call(e) => expr(e, f),
            StmtKind::If { branches, else_body } => {
                for (c, b) in branches.iter_mut() {
                    expr(c, f);
                    stmts(b, f);
                }
                if let Some(b) = else_body {
                    stmts(b, f);
                }
            }
            StmtKind::Case { selector, arms, else_body } => {
                expr(selector, f);
                for a in arms.iter_mut() {
                    stmts(&mut a.body, f);
                }
                if let Some(b) = else_body {
                    stmts(b, f);
                }
            }
            StmtKind::For { from, to, by, body, .. } => {
                expr(from, f);
                expr(to, f);
                if let Some(b) = by {
                    expr(b, f);
                }
                stmts(body, f);
            }
            StmtKind::While { cond, body } => {
                expr(cond, f);
                stmts(body, f);
            }
            StmtKind::Repeat { body, until } => {
                stmts(body, f);
                expr(until, f);
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

/// The expression and those inside it, the outer one after its parts.
fn expr(e: &mut Expr, f: &mut dyn FnMut(&mut Expr)) {
    match &mut e.kind {
        ExprKind::Member(x, _)
        | ExprKind::BitAccess(x, _)
        | ExprKind::Deref(x)
        | ExprKind::Unary(_, x)
        | ExprKind::Paren(x) => expr(x, f),
        ExprKind::Index(x, ix) => {
            expr(x, f);
            ix.iter_mut().for_each(|i| expr(i, f));
        }
        ExprKind::Binary(_, x, y) => {
            expr(x, f);
            expr(y, f);
        }
        ExprKind::Call { callee, args } => {
            expr(callee, f);
            for a in args.iter_mut() {
                match a {
                    Arg::Positional(x) | Arg::Input(_, x) | Arg::Output { target: x, .. } => expr(x, f),
                }
            }
        }
        ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::DirectAddress(_)
        | ExprKind::EnumValue { .. }
        | ExprKind::Error => {}
    }
    f(e);
}

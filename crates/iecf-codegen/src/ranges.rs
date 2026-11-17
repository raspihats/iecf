//! What a FOR loop proves about its control variable, so an index the
//! variable makes need not be checked (docs/c-output.md §2).
//!
//! Inside the body of `FOR i := 1 TO 100 DO`, with constant bounds and a
//! body that leaves `i` alone, `i` is one of 1 .. 100: the runtime's FOR
//! (runtime/iecf.h) runs the body only with a value between the start and
//! the end. `a[i]`, `a[i - 1]`, `m[i].x` over an array whose bounds hold
//! that range cannot be out of them, and their offset is taken directly.
//! The proof is kept to what nothing else can break: a plain VAR or
//! VAR_TEMP of a PROGRAM, FUNCTION or FUNCTION_BLOCK with no methods and
//! no derived block, never assigned, given to an output or an in-out, or
//! referenced in its POU. Anything else is checked, as before.

use crate::Gen;
use crate::expr::{const_int, strip_paren};
use crate::names::{Place, Scope};
use iecf_sema::types::Elem;
use iecf_syntax::ast::*;

/// A FOR's control variable while its body is generated: its name, its C
/// lvalue (to know the same variable is meant), the range it is in and
/// its type.
pub(crate) struct ForRange {
    name: String,
    lv: String,
    lo: i128,
    hi: i128,
    ty: Elem,
}

impl Gen<'_> {
    /// The range `var` is proven in over `body`, if the proof holds.
    pub(crate) fn for_range(
        &self,
        var: &Ident,
        (from, to, by): (&Expr, &Expr, Option<&Expr>),
        place: &Place,
        body: &[Stmt],
    ) -> Option<ForRange> {
        let Place::Field(lv, ty) = place else { return None };
        let plain = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !(lv.strip_prefix("self->V_").is_some_and(plain) || lv.strip_prefix("V_").is_some_and(plain)) {
            return None;
        }
        let key = self.const_ctx.last()?;
        let pou = *self.pous.get(key.as_str())?;
        let pou_ok = match pou.kind {
            PouKind::Program | PouKind::Function => true,
            PouKind::FunctionBlock => {
                pou.methods.is_empty()
                    && !self.pous.values().any(|p| p.extends.as_ref().is_some_and(|b| b.is(&pou.name.text)))
            }
            _ => false,
        };
        let declared = pou.var_blocks.iter().any(|b| {
            matches!(b.kind, VarKind::Var | VarKind::Temp)
                && b.access.is_none()
                && b.decls.iter().any(|d| d.location.is_none() && d.names.iter().any(|n| n.is(&var.text)))
        });
        if !pou_ok || !declared {
            return None;
        }
        let (start, end) = (const_int(from)?, const_int(to)?);
        let step = match by {
            Some(b) => const_int(b)?,
            None => 1,
        };
        if step == 0 || writes(body, &var.text) || pou_refs(pou, &var.text) {
            return None;
        }
        let (lo, hi) = (start.min(end), start.max(end));
        let (tlo, thi) = ty.int_range()?;
        (lo >= tlo && hi <= thi).then(|| ForRange {
            name: var.text.to_ascii_uppercase(),
            lv: lv.clone(),
            lo,
            hi,
            ty: *ty,
        })
    }

    /// The offset of index `i` in `lo..hi`, unchecked, when an enclosing
    /// FOR proves it in range: `i` is its variable plus a constant.
    pub(crate) fn proven_index(&self, i: &Expr, lo: i64, hi: i64, scope: &Scope) -> Option<String> {
        let (id, c) = affine(i)?;
        let upper = id.text.to_ascii_uppercase();
        let r = self.for_ranges.iter().rev().find(|r| r.name == upper)?;
        // The same variable: not one of another scope by the same name.
        match scope.vars.get(&upper) {
            Some(Place::Field(lv, _)) if *lv == r.lv => {}
            _ => return None,
        }
        // `i + c` is computed in i's type: it must not leave it either.
        let (tlo, thi) = r.ty.int_range()?;
        let (vlo, vhi) = (r.lo.checked_add(c)?, r.hi.checked_add(c)?);
        if vlo < tlo || vhi > thi || vlo < lo as i128 || vhi > hi as i128 {
            return None;
        }
        let k = c - lo as i128;
        let fits32 = |v: i128| v >= i32::MIN as i128 && v <= i32::MAX as i128;
        Some(if fits32(r.lo) && fits32(r.hi) && fits32(k) {
            format!("((uint32_t)((int32_t)({}) + ({k})))", r.lv)
        } else {
            format!("((uint32_t)((int64_t)({}) + INT64_C({k})))", r.lv)
        })
    }
}

/// `e` as a name plus a constant: `i`, `i + 2`, `3 + i`, `i - 1`.
fn affine(e: &Expr) -> Option<(&Ident, i128)> {
    match &strip_paren(e).kind {
        ExprKind::Name(id) => Some((id, 0)),
        ExprKind::Binary(BinaryOp::Add, a, b) => match (affine(a), const_int(b)) {
            (Some((id, c)), Some(k)) => Some((id, c.checked_add(k)?)),
            _ => {
                let (k, (id, c)) = (const_int(a)?, affine(b)?);
                Some((id, c.checked_add(k)?))
            }
        },
        ExprKind::Binary(BinaryOp::Sub, a, b) => {
            let ((id, c), k) = (affine(a)?, const_int(b)?);
            Some((id, c.checked_sub(k)?))
        }
        _ => None,
    }
}

/// The variable an assignment, an output or an in-out writes: the name
/// at the root of `x`, `x.3`, `x[i]`, `x.f`.
fn root(e: &Expr) -> Option<&Ident> {
    match &strip_paren(e).kind {
        ExprKind::Name(id) => Some(id),
        ExprKind::Member(x, _) | ExprKind::BitAccess(x, _) | ExprKind::Index(x, _) => root(x),
        _ => None,
    }
}

/// Whether `body` may change `name`: an assignment to it or a part of
/// it, an output given it, a nested FOR over it, or the name itself
/// given to anything but a standard function, whose parameters are
/// inputs (a user's FUNCTION or block may take it as an in-out).
fn writes(body: &[Stmt], name: &str) -> bool {
    body.iter().any(|s| stmt_writes(s, name))
}

fn stmt_writes(s: &Stmt, name: &str) -> bool {
    let is = |e: &Expr| root(e).is_some_and(|id| id.is(name));
    match &s.kind {
        StmtKind::Assign { target, value }
        | StmtKind::RefAssign { target, value }
        | StmtKind::AssignAttempt { target, value } => {
            is(target) || expr_writes(target, name) || expr_writes(value, name)
        }
        StmtKind::Call(e) => expr_writes(e, name),
        StmtKind::If { branches, else_body } => {
            branches.iter().any(|(c, b)| expr_writes(c, name) || writes(b, name))
                || else_body.as_ref().is_some_and(|b| writes(b, name))
        }
        StmtKind::Case { selector, arms, else_body } => {
            expr_writes(selector, name)
                || arms.iter().any(|a| writes(&a.body, name))
                || else_body.as_ref().is_some_and(|b| writes(b, name))
        }
        StmtKind::For { var, from, to, by, body } => {
            var.is(name)
                || expr_writes(from, name)
                || expr_writes(to, name)
                || by.as_ref().is_some_and(|b| expr_writes(b, name))
                || writes(body, name)
        }
        StmtKind::While { cond, body } => expr_writes(cond, name) || writes(body, name),
        StmtKind::Repeat { body, until } => writes(body, name) || expr_writes(until, name),
        StmtKind::Exit | StmtKind::Continue | StmtKind::Return | StmtKind::Empty => false,
    }
}

/// Whether evaluating `e` may change `name`: through a call's output,
/// or the bare name given to a call that is not a standard function.
fn expr_writes(e: &Expr, name: &str) -> bool {
    match &e.kind {
        ExprKind::Call { callee, args } => {
            let standard = matches!(&callee.kind, ExprKind::Name(f) if iecf_sema::stdlib::std_sig(&f.text).is_some());
            expr_writes(callee, name)
                || args.iter().any(|a| match a {
                    Arg::Output { target, .. } => {
                        root(target).is_some_and(|id| id.is(name)) || expr_writes(target, name)
                    }
                    Arg::Positional(x) | Arg::Input(_, x) => {
                        (!standard && root(x).is_some_and(|id| id.is(name))) || expr_writes(x, name)
                    }
                })
        }
        ExprKind::Member(x, _) | ExprKind::BitAccess(x, _) | ExprKind::Deref(x) | ExprKind::Paren(x) => {
            expr_writes(x, name)
        }
        ExprKind::Unary(_, x) => expr_writes(x, name),
        ExprKind::Index(x, is) => expr_writes(x, name) || is.iter().any(|i| expr_writes(i, name)),
        ExprKind::Binary(_, a, b) => expr_writes(a, name) || expr_writes(b, name),
        _ => false,
    }
}

/// Whether the POU takes a reference to `name` anywhere - REF(name) or
/// of a part of it - through which a body could change it unseen.
fn pou_refs(pou: &Pou, name: &str) -> bool {
    pou.body.iter().any(|s| stmt_refs(s, name))
        || sfc_elements(&pou.sfc).into_iter().any(|(_, el)| match el {
            SfcElement::Action(a) => a.body.iter().any(|s| stmt_refs(s, name)),
            SfcElement::Transition(t) => expr_refs(&t.condition, name),
            SfcElement::Step(st) => st.actions.iter().any(|a| a.time.as_ref().is_some_and(|t| expr_refs(t, name))),
        })
}

fn stmt_refs(s: &Stmt, name: &str) -> bool {
    let all = |b: &[Stmt]| b.iter().any(|s| stmt_refs(s, name));
    match &s.kind {
        StmtKind::Assign { target, value }
        | StmtKind::RefAssign { target, value }
        | StmtKind::AssignAttempt { target, value } => expr_refs(target, name) || expr_refs(value, name),
        StmtKind::Call(e) => expr_refs(e, name),
        StmtKind::If { branches, else_body } => {
            branches.iter().any(|(c, b)| expr_refs(c, name) || all(b)) || else_body.as_deref().is_some_and(all)
        }
        StmtKind::Case { selector, arms, else_body } => {
            expr_refs(selector, name) || arms.iter().any(|a| all(&a.body)) || else_body.as_deref().is_some_and(all)
        }
        StmtKind::For { from, to, by, body, .. } => {
            expr_refs(from, name) || expr_refs(to, name) || by.as_ref().is_some_and(|b| expr_refs(b, name)) || all(body)
        }
        StmtKind::While { cond, body } => expr_refs(cond, name) || all(body),
        StmtKind::Repeat { body, until } => all(body) || expr_refs(until, name),
        StmtKind::Exit | StmtKind::Continue | StmtKind::Return | StmtKind::Empty => false,
    }
}

fn expr_refs(e: &Expr, name: &str) -> bool {
    match &e.kind {
        ExprKind::Call { callee, args } => {
            let reference = crate::refs::is_ref_call(callee);
            expr_refs(callee, name)
                || args.iter().any(|a| match a {
                    Arg::Positional(x) | Arg::Input(_, x) | Arg::Output { target: x, .. } => {
                        (reference && root(x).is_some_and(|id| id.is(name))) || expr_refs(x, name)
                    }
                })
        }
        ExprKind::Member(x, _) | ExprKind::BitAccess(x, _) | ExprKind::Deref(x) | ExprKind::Paren(x) => {
            expr_refs(x, name)
        }
        ExprKind::Unary(_, x) => expr_refs(x, name),
        ExprKind::Index(x, is) => expr_refs(x, name) || is.iter().any(|i| expr_refs(i, name)),
        ExprKind::Binary(_, a, b) => expr_refs(a, name) || expr_refs(b, name),
        _ => false,
    }
}

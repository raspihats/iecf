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
//! referenced in its POU. Anything else is checked, as before. A
//! subrange variable's limits prove its value wherever it is read, as
//! every store into it is clamped, the platform's from outside too
//! (docs/c-output.md §6) - not one at an address, nor an in-out.

use crate::derived::{DTy, Kind};
use crate::expr::{const_int, strip_paren};
use crate::names::{Lv, Place, Scope};
use crate::{Gen, R};
use iecf_sema::types::Elem;
use iecf_syntax::ast::*;

/// A FOR's control variable while its body is generated: its name, its C
/// lvalue (to know the same variable is meant) and the range it is in.
pub(crate) struct ForRange {
    name: String,
    lv: String,
    lo: i128,
    hi: i128,
}

/// What an assignment proved: after `k := e`, with `e`'s range known,
/// `k` is in it until something may write `k` again (`forget`). `level`
/// is the statement list it was learned in: leaving that list forgets it
/// (a branch, a loop's body).
pub(crate) struct Fact {
    name: String,
    lv: String,
    lo: i128,
    hi: i128,
    level: u32,
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
        if !self.plain_local(&var.text, place) {
            return None;
        }
        let (start, end) = (self.known_int(from, 0)?, self.known_int(to, 0)?);
        let step = match by {
            Some(b) => self.known_int(b, 0)?,
            None => 1,
        };
        if step == 0 || writes(body, &var.text) {
            return None;
        }
        let (lo, hi) = (start.min(end), start.max(end));
        let (tlo, thi) = ty.int_range()?;
        (lo >= tlo && hi <= thi).then(|| ForRange { name: var.text.to_ascii_uppercase(), lv: lv.clone(), lo, hi })
    }

    /// Whether `name`, at `place`, is a variable nothing but its own POU's
    /// statements can change: a plain VAR or VAR_TEMP (not PUBLIC, not at
    /// an address) of a PROGRAM, FUNCTION or FUNCTION_BLOCK with no methods
    /// and no derived block, of which the POU takes no REF().
    fn plain_local(&self, name: &str, place: &Place) -> bool {
        let Place::Field(lv, _) = place else { return false };
        let plain = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !(lv.strip_prefix("self->V_").is_some_and(plain) || lv.strip_prefix("V_").is_some_and(plain)) {
            return false;
        }
        let Some(pou) = self.const_ctx.last().and_then(|k| self.pous.get(k.as_str())).copied() else {
            return false;
        };
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
                && b.decls.iter().any(|d| d.location.is_none() && d.names.iter().any(|n| n.is(name)))
        });
        pou_ok && declared && !pou_refs(pou, name)
    }

    /// What `target := value` will prove of the target, if anything: the
    /// value's range, worked out with what holds before the statement (the
    /// value is computed before the store), when it fits the target.
    pub(crate) fn learned(&mut self, target: &Expr, value: &Expr, scope: &Scope) -> R<Option<Fact>> {
        let ExprKind::Name(id) = &strip_paren(target).kind else { return Ok(None) };
        let upper = id.text.to_ascii_uppercase();
        let Some(place) = scope.vars.get(&upper) else { return Ok(None) };
        let Place::Field(lv, t) = place else { return Ok(None) };
        if !t.is_int() || !self.plain_local(&id.text, place) {
            return Ok(None);
        }
        let vt = self.elem_of(value)?;
        let (Some((tlo, thi)), true) = (t.int_range(), vt.is_int()) else { return Ok(None) };
        let lv = lv.clone();
        Ok(match self.int_range(value, vt, scope)? {
            Some((lo, hi)) if lo >= tlo && hi <= thi => Some(Fact { name: upper, lv, lo, hi, level: self.fact_level }),
            _ => None,
        })
    }

    /// The fact `learned` gave, once the statement is written.
    pub(crate) fn learn(&mut self, fact: Fact) {
        self.facts.push(fact);
    }

    /// Every fact of a variable `s` may write: an assignment, an output, a
    /// user's call it is given to, a loop over it (`writes`).
    pub(crate) fn forget(&mut self, s: &Stmt) {
        self.facts.retain(|f| !stmt_writes(s, &f.name));
    }

    /// Leaving a statement list: what it learned no longer holds.
    pub(crate) fn forget_level(&mut self, level: u32) {
        self.facts.retain(|f| f.level < level);
    }

    /// An integer known as the C is written: a literal, a CONSTANT whose
    /// initial value is one (followed through other constants, eight
    /// deep), and + - * of those.
    pub(crate) fn known_int(&self, e: &Expr, depth: u32) -> Option<i128> {
        if let Some(v) = const_int(e) {
            return Some(v);
        }
        if depth > 8 {
            return None;
        }
        match &strip_paren(e).kind {
            ExprKind::Name(id) => match &self.const_decl(&id.text, e.span)?.init {
                Some(Init::Expr(x)) => self.known_int(x, depth + 1),
                _ => None,
            },
            ExprKind::Unary(UnaryOp::Neg, x) => self.known_int(x, depth + 1)?.checked_neg(),
            ExprKind::Binary(op, a, b) => {
                let (a, b) = (self.known_int(a, depth + 1)?, self.known_int(b, depth + 1)?);
                match op {
                    BinaryOp::Add => a.checked_add(b),
                    BinaryOp::Sub => a.checked_sub(b),
                    BinaryOp::Mul => a.checked_mul(b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The offset of index `i` in `lo..hi`, unchecked, when `i` is proven
    /// in that range (int_range): its C is then the operators' own.
    pub(crate) fn proven_index(&mut self, i: &Expr, lo: i64, hi: i64, scope: &Scope) -> R<Option<String>> {
        let t = self.elem_of(i)?;
        let Some((a, b)) = self.int_range(i, t, scope)? else { return Ok(None) };
        if a < lo as i128 || b > hi as i128 {
            return Ok(None);
        }
        let v = self.value(i, scope)?;
        let fits32 = |x: i128| x >= i32::MIN as i128 && x <= i32::MAX as i128;
        Ok(Some(if fits32(a) && fits32(b) && fits32(lo as i128) {
            format!("((uint32_t)((int32_t)({v}) - ({lo})))")
        } else {
            format!("((uint32_t)((int64_t)({v}) - INT64_C({lo})))")
        }))
    }

    /// The values an integer expression of type `t` can take, when every
    /// value it is made of is known - literals, and the variable of an
    /// enclosing FOR proven in its range - and each `+ - * / MOD` of them
    /// stays inside `t`, a divisor never 0 nor -1. Such an operation can
    /// neither overflow nor fault: it is written as C's own operator
    /// (expr.rs), whose / and % truncate toward zero as IEC's do.
    pub(crate) fn int_range(&mut self, e: &Expr, t: Elem, scope: &Scope) -> R<Option<(i128, i128)>> {
        let Some((tlo, thi)) = t.int_range() else { return Ok(None) };
        let e = strip_paren(e);
        let r = match &e.kind {
            ExprKind::Literal(_) => const_int(e).map(|v| (v, v)),
            ExprKind::Name(id) => {
                let upper = id.text.to_ascii_uppercase();
                match (self.for_ranges.iter().rev().find(|r| r.name == upper), scope.vars.get(&upper)) {
                    // A FOR's proven variable.
                    (Some(r), Some(Place::Field(lv, _))) if *lv == r.lv => Some((r.lo, r.hi)),
                    // One an assignment proved (learn).
                    (_, Some(Place::Field(lv, _)))
                        if let Some(f) = self.facts.iter().rev().find(|f| f.name == upper && f.lv == *lv) =>
                    {
                        Some((f.lo, f.hi))
                    }
                    // A subrange variable's limits: every store into it is
                    // clamped - the program's own, a call's input, an
                    // in-out's copy stored back, and one from outside the
                    // program, which the platform keeps within the limits
                    // the symbol table gives (docs/c-output.md §6). Not one
                    // at an address, in the image the platform writes, nor
                    // an in-out, the caller's variable of any integer type.
                    (_, Some(Place::Data(Lv::C(lv), DTy { kind: Kind::Elem(_), range: Some((lo, hi)), .. })))
                        if !lv.starts_with("(*") =>
                    {
                        Some((*lo, *hi))
                    }
                    // A CONSTANT, its value.
                    _ => self.known_int(e, 0).map(|v| (v, v)),
                }
            }
            ExprKind::Unary(UnaryOp::Neg, x) if self.elem_of(e)? == t => {
                self.int_range(x, t, scope)?.map(|(lo, hi)| (-hi, -lo))
            }
            ExprKind::Binary(op, a, b)
                if matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod)
                    && self.elem_of(e)? == t =>
            {
                let (Some(x), Some(y)) = (self.int_range(a, t, scope)?, self.int_range(b, t, scope)?) else {
                    return Ok(None);
                };
                let corners = |f: fn(i128, i128) -> i128| {
                    let c = [f(x.0, y.0), f(x.0, y.1), f(x.1, y.0), f(x.1, y.1)];
                    (c.iter().copied().min().unwrap_or(0), c.iter().copied().max().unwrap_or(0))
                };
                // A divisor that cannot be 0 or -1: wholly above 0, or below -1.
                let divides = y.0 >= 1 || y.1 <= -2;
                match op {
                    BinaryOp::Add => Some((x.0 + y.0, x.1 + y.1)),
                    BinaryOp::Sub => Some((x.0 - y.1, x.1 - y.0)),
                    BinaryOp::Mul => Some(corners(|p, q| p * q)),
                    BinaryOp::Div if divides => Some(corners(|p, q| p / q)),
                    BinaryOp::Mod if divides => {
                        let m = y.0.abs().max(y.1.abs()) - 1;
                        Some((if x.0 < 0 { -m } else { 0 }, if x.1 > 0 { m } else { 0 }))
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        Ok(r.filter(|&(lo, hi)| lo >= tlo && hi <= thi))
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

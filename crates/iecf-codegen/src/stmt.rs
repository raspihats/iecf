//! Statements (Ed.3 7.3.3). An expression that can fault is computed into
//! a temporary and checked (IECF_STOP) before anything uses it, so a
//! faulted statement changes nothing and the task stops there.
//!
//! Loops are C `for (;;)` with `break` for EXIT; CASE is an `if` chain,
//! not a `switch`, so `break` always leaves the loop. CONTINUE in REPEAT
//! jumps to the UNTIL test (a `goto`), which C's `continue` would skip.

use crate::expr::{const_int, lit};
use crate::names::Scope;
use crate::{Gen, R, c_type};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;

pub(crate) enum Loop {
    /// FOR and WHILE: C's `continue` is right.
    Plain,
    /// REPEAT: `continue` jumps to this label, written only if used.
    Repeat(String, bool),
}

pub(crate) struct Body<'s> {
    pub(crate) pou: u16,
    pub(crate) scope: &'s Scope,
    pub(crate) out: &'s mut String,
    pub(crate) depth: usize,
    pub(crate) loops: Vec<Loop>,
}

impl Body<'_> {
    fn line(&mut self, text: &str) {
        for _ in 0..self.depth {
            self.out.push_str("    ");
        }
        self.out.push_str(text);
        self.out.push('\n');
    }
}

impl Gen<'_> {
    pub(crate) fn stmts(&mut self, body: &[Stmt], cx: &mut Body) -> R<()> {
        let mut result = Ok(());
        for s in body {
            // Every statement is tried, so one run reports every gap.
            if self.stmt(s, cx).is_err() {
                result = Err(crate::Skip);
            }
        }
        result
    }

    /// `e` as `t` into a fresh temporary, checked; the temporary's name.
    fn checked(&mut self, e: &Expr, t: Elem, at: &Stmt, cx: &mut Body) -> R<String> {
        self.may_fault = false;
        let v = self.value_as(e, t, cx.scope)?;
        let name = self.fresh("t");
        cx.line(&format!("{} {name} = {v};", c_type(t)));
        if self.may_fault {
            let l = self.line(at.span);
            cx.line(&format!("IECF_STOP(ctx, {}u, {l}u);", cx.pou));
        }
        Ok(name)
    }

    fn condition(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<String> {
        self.checked(e, Elem::Bool, at, cx)
    }

    fn stmt(&mut self, s: &Stmt, cx: &mut Body) -> R<()> {
        let line = self.line(s.span);
        cx.line(&format!("/* line {line} */"));
        match &s.kind {
            StmtKind::Assign { target, value } => {
                let ExprKind::Name(id) = &target.kind else {
                    return self.not_yet(target.span, "assigning to this");
                };
                let Some(place) = cx.scope.vars.get(&id.text.to_ascii_uppercase()).cloned() else {
                    return self.not_yet(id.span, format!("'{}' here", id.text));
                };
                cx.line("{");
                cx.depth += 1;
                let t = self.checked(value, place.ty(), s, cx)?;
                cx.line(&place.store(&t));
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::If { branches, else_body } => {
                let mut closes = 0;
                for (i, (cond, body)) in branches.iter().enumerate() {
                    if i > 0 {
                        cx.line("} else {");
                        cx.depth += 1;
                        closes += 1;
                    }
                    let c = self.condition(cond, s, cx)?;
                    cx.line(&format!("if ({c}) {{"));
                    cx.depth += 1;
                    self.stmts(body, cx)?;
                    cx.depth -= 1;
                }
                if let Some(body) = else_body {
                    cx.line("} else {");
                    cx.depth += 1;
                    self.stmts(body, cx)?;
                    cx.depth -= 1;
                }
                cx.line("}");
                for _ in 0..closes {
                    cx.depth -= 1;
                    cx.line("}");
                }
            }
            StmtKind::Case { selector, arms, else_body } => {
                let t = match self.types.expr(selector.span) {
                    Some(Ty::Elem(t)) if t.is_int() || t.is_bit() => *t,
                    _ => return self.not_yet(selector.span, "a CASE on this selector"),
                };
                cx.line("{");
                cx.depth += 1;
                let sel = self.checked(selector, t, s, cx)?;
                let mut first = true;
                for arm in arms {
                    let mut tests = Vec::new();
                    for label in &arm.labels {
                        let value = |g: &mut Self, e: &Expr| -> R<Option<String>> {
                            match const_int(e) {
                                Some(v) => Ok(lit(v, t)),
                                None => g.not_yet(e.span, "a CASE label that is not an integer literal"),
                            }
                        };
                        match label {
                            CaseLabel::Value(e) => {
                                if let Some(v) = value(self, e)? {
                                    tests.push(format!("{sel} == {v}"));
                                }
                            }
                            CaseLabel::Range(a, b) => {
                                if let (Some(a), Some(b)) = (value(self, a)?, value(self, b)?) {
                                    tests.push(format!("({sel} >= {a} && {sel} <= {b})"));
                                }
                            }
                        }
                    }
                    if tests.is_empty() {
                        // No label fits the selector's type: the arm never runs.
                        continue;
                    }
                    let kw = if first { "if" } else { "} else if" };
                    first = false;
                    cx.line(&format!("{kw} ({}) {{", tests.join(" || ")));
                    cx.depth += 1;
                    self.stmts(&arm.body, cx)?;
                    cx.depth -= 1;
                }
                if let Some(body) = else_body {
                    if first {
                        cx.line("{");
                    } else {
                        cx.line("} else {");
                    }
                    cx.depth += 1;
                    self.stmts(body, cx)?;
                    cx.depth -= 1;
                    cx.line("}");
                } else if !first {
                    cx.line("}");
                }
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::For { var, from, to, by, body } => {
                let Some(place) = cx.scope.vars.get(&var.text.to_ascii_uppercase()).cloned() else {
                    return self.not_yet(var.span, format!("'{}' here", var.text));
                };
                let t = place.ty();
                let Some(lv) = place.lvalue().map(str::to_owned) else {
                    return self.not_yet(var.span, "a FOR variable at an address");
                };
                if !t.is_int() {
                    return self.not_yet(var.span, format!("a FOR variable of type {}", t.name()));
                }
                cx.line("{");
                cx.depth += 1;
                let start = self.checked(from, t, s, cx)?;
                let end = self.checked(to, t, s, cx)?;
                let step = match by {
                    Some(b) => self.checked(b, t, s, cx)?,
                    None => lit(1, t).unwrap_or_default(),
                };
                let n = t.name();
                let done = self.fresh("d");
                cx.line(&format!("int {done};"));
                cx.line(&format!(
                    "for ({lv} = {start}, {done} = iecf_for_done_{n}({lv}, {end}, {step}); !{done}; {done} = iecf_for_next_{n}(&{lv}, {end}, {step})) {{"
                ));
                cx.depth += 1;
                cx.loops.push(Loop::Plain);
                let r = self.stmts(body, cx);
                cx.loops.pop();
                r?;
                cx.depth -= 1;
                cx.line("}");
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::While { cond, body } => {
                cx.line("for (;;) {");
                cx.depth += 1;
                let c = self.condition(cond, s, cx)?;
                cx.line(&format!("if (!{c}) break;"));
                cx.loops.push(Loop::Plain);
                let r = self.stmts(body, cx);
                cx.loops.pop();
                r?;
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::Repeat { body, until } => {
                let label = self.fresh("next");
                cx.line("for (;;) {");
                cx.depth += 1;
                cx.loops.push(Loop::Repeat(label.clone(), false));
                let r = self.stmts(body, cx);
                let used = matches!(cx.loops.pop(), Some(Loop::Repeat(_, true)));
                r?;
                if used {
                    cx.line(&format!("{label}:;"));
                }
                let c = self.condition(until, s, cx)?;
                cx.line(&format!("if ({c}) break;"));
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::Exit => cx.line("break;"),
            StmtKind::Continue => match cx.loops.last_mut() {
                Some(Loop::Repeat(label, used)) => {
                    *used = true;
                    let l = format!("goto {label};");
                    cx.line(&l);
                }
                _ => cx.line("continue;"),
            },
            StmtKind::Return => cx.line("return;"),
            StmtKind::Empty => {}
            StmtKind::Call(e) => {
                cx.line("{");
                cx.depth += 1;
                self.may_fault = false;
                let v = self.value(e, cx.scope)?;
                cx.line(&format!("(void)({v});"));
                if self.may_fault {
                    cx.line(&format!("IECF_STOP(ctx, {}u, {line}u);", cx.pou));
                }
                cx.depth -= 1;
                cx.line("}");
            }
        }
        Ok(())
    }
}

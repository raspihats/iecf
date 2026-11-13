//! Statements (Ed.3 7.3.3). An expression that can fault is computed into
//! a temporary and checked (iecf_stopped) before anything uses it, so a
//! faulted statement changes nothing and the task stops there.
//!
//! A real converted to an integer it does not fit sets the statement's
//! `keep` (runtime/iecf.h): the statement is left undone and the task
//! goes on. An assignment keeps its target's value; IF and CASE run no
//! branch; FOR does not run; WHILE and REPEAT end; a function block's
//! input keeps its value and the block is called (docs/c-output.md §5).
//!
//! Loops are C `for (;;)` with `break` for EXIT; CASE is an `if` chain,
//! not a `switch`, so `break` always leaves the loop. CONTINUE in REPEAT
//! jumps to the UNTIL test (a `goto`), which C's `continue` would skip.

use crate::derived::{DTy, Kind, RefTo};
use crate::expr::{const_int, lit};
use crate::names::{Lv, Place, Scope};
use crate::{Gen, R, c_type};
use iecf_sema::stdlib::Dir;
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
    /// What leaves the body: `return;`, or `return V_F;` in a FUNCTION.
    pub(crate) ret: String,
    /// What RETURN does where it is not `ret`: in an SFC action, end the
    /// action (sfc.rs). A fault still takes `ret`.
    pub(crate) on_return: Option<String>,
}

impl Body<'_> {
    pub(crate) fn line(&mut self, text: &str) {
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

    /// `e` as `t` into a fresh temporary, checked.
    fn checked(&mut self, e: &Expr, t: Elem, at: &Stmt, cx: &mut Body) -> R<Val> {
        self.may_fault = false;
        self.may_flag = false;
        self.keep_ok = true;
        self.pre_ok = true;
        self.keep = None;
        self.seq = self.calls_effects(e, cx.scope)?;
        let v = self.value_as(e, t, cx.scope);
        self.seq = false;
        self.keep_ok = false;
        self.pre_ok = false;
        let keep = self.keep.take();
        let v = v?;
        let l = self.line(at.span);
        if let Some(k) = &keep {
            cx.line(&format!("int {k} = 0;"));
            self.undone.push(k.clone());
        }
        self.declare_buffers(cx, l);
        let name = self.fresh("t");
        cx.line(&format!("{} {name} = {v};", c_type(t)));
        if self.may_fault {
            self.stop(cx, l);
        }
        if self.may_flag {
            noted(cx, l);
        }
        Ok(Val { v: name, keep })
    }

    /// A reference or NULL into a fresh `iecf_ref_t` temporary, checked:
    /// a `^` in it that meets NULL stops the statement here.
    fn checked_reference(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<String> {
        self.may_fault = false;
        self.may_flag = false;
        self.pre_ok = true;
        // A call with effects, its own line in operand order (expr.rs).
        let v = match self.calls_effects(e, cx.scope) {
            Ok(seq) => {
                self.seq = seq;
                self.ref_value(e, cx.scope)
            }
            Err(x) => Err(x),
        };
        self.seq = false;
        self.pre_ok = false;
        let v = v?;
        let l = self.line(at.span);
        self.declare_buffers(cx, l);
        let name = self.fresh("t");
        cx.line(&format!("iecf_ref_t {name} = {v};"));
        if self.may_fault {
            self.stop(cx, l);
        }
        if self.may_flag {
            noted(cx, l);
        }
        Ok(name)
    }

    /// `base^` as a place a statement writes or copies from: the reference
    /// checked into a temporary first, so NULL stops the statement before
    /// anything is stored.
    fn deref_target(&mut self, base: &Expr, at: &Stmt, cx: &mut Body) -> R<(String, RefTo)> {
        self.may_fault = false;
        self.may_flag = false;
        // The reference may be computed: SEL(g, r1, r2)^ := ..., a
        // FUNCTION's result, with effects on lines of their own.
        self.pre_ok = true;
        let found = match self.calls_effects(base, cx.scope) {
            Ok(seq) => {
                self.seq = seq;
                self.ref_of(base, cx.scope)
            }
            Err(x) => Err(x),
        };
        self.seq = false;
        self.pre_ok = false;
        let Some((r, to)) = found? else {
            return self.not_yet(base.span, "dereferencing this");
        };
        let l = self.line(at.span);
        self.declare_buffers(cx, l);
        let chk = self.checked_ref(&r, &to);
        let t = self.fresh("r");
        cx.line(&format!("iecf_ref_t {t} = {chk};"));
        self.stop(cx, l);
        if self.may_flag {
            noted(cx, l);
        }
        Ok((t, to))
    }

    fn condition(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<Val> {
        self.checked(e, Elem::Bool, at, cx)
    }

    /// A condition as C that is TRUE only when it was computed and holds:
    /// one left undone is FALSE (an SFC transition, sfc.rs).
    pub(crate) fn cond(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<String> {
        let v = self.condition(e, at, cx)?;
        Ok(match &v.keep {
            Some(k) => format!("(!{k} && {})", v.v),
            None => v.v,
        })
    }

    /// The string buffers the expression just written uses (strings.rs),
    /// and its calls with EN or ENO (enable.rs), which stop the statement
    /// at `line` if they fault.
    fn declare_buffers(&mut self, cx: &mut Body, line: u32) {
        self.call_bufs.clear();
        self.sel_bufs.clear();
        for d in std::mem::take(&mut self.str_temps) {
            let d = d.replacen(crate::enable::DECL, "", 1);
            match d.trim_start().strip_prefix(crate::enable::STOP) {
                Some(_) => {
                    let indent = &d[..d.len() - d.trim_start().len()];
                    for k in &self.undone {
                        cx.line(&format!("{indent}iecf_undone(&ctx->fault, {k});"));
                    }
                    let r = cx.ret.clone();
                    cx.line(&format!("{indent}if (iecf_stopped(&ctx->fault, {}u, {line}u)) {r}", cx.pou));
                }
                None => cx.line(&d),
            }
        }
    }

    /// After a statement that can fault: leave if it did. Where one of
    /// its conversions failed, the statement is left undone (`undone`):
    /// what its own checks then fault on, on the conversion's
    /// placeholder, does not stop the task (iecf.h `iecf_undone`).
    pub(crate) fn stop(&self, cx: &mut Body, line: u32) {
        for k in &self.undone {
            cx.line(&format!("iecf_undone(&ctx->fault, {k});"));
        }
        let r = cx.ret.clone();
        cx.line(&format!("if (iecf_stopped(&ctx->fault, {}u, {line}u)) {r}", cx.pou));
    }

    fn stmt(&mut self, s: &Stmt, cx: &mut Body) -> R<()> {
        let outer = (std::mem::take(&mut self.undone), std::mem::take(&mut self.target_keeps));
        let r = self.stmt_in(s, cx);
        (self.undone, self.target_keeps) = outer;
        r
    }

    fn stmt_in(&mut self, s: &Stmt, cx: &mut Body) -> R<()> {
        self.str_temps.clear();
        self.call_bufs.clear();
        self.sel_bufs.clear();
        let line = self.line(s.span);
        cx.line(&format!("/* line {line} */"));
        match &s.kind {
            StmtKind::Assign { target, value } if !matches!(target.kind, ExprKind::Name(_)) => {
                cx.line("{");
                cx.depth += 1;
                match &target.kind {
                    ExprKind::Index(base, idx) if self.is_string(base)? => {
                        self.assign_char(base, &idx[0], value, s, cx)?;
                    }
                    _ => {
                        self.target_keeps.clear();
                        self.target_keep_ok = true;
                        let found = self.target(target, s, cx);
                        self.target_keep_ok = false;
                        let (lv, t) = found?;
                        let r = self.assign_data(&lv, &t, value, s, cx);
                        self.target_keeps.clear();
                        r?;
                    }
                }
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::Assign { target, value } => {
                let ExprKind::Name(id) = &target.kind else {
                    return self.not_yet(target.span, "assigning to this");
                };
                let Some(place) = cx.scope.vars.get(&id.text.to_ascii_uppercase()).cloned() else {
                    return self.not_yet(id.span, format!("'{}' here", id.text));
                };
                cx.line("{");
                cx.depth += 1;
                if let Place::Data(lv, t) = &place {
                    self.assign_data(lv, t, value, s, cx)?;
                    cx.depth -= 1;
                    cx.line("}");
                    return Ok(());
                }
                if let Place::Itf(lv, i) = &place {
                    let v = self.checked_itf(value, i, s, cx)?;
                    cx.line(&format!("{lv} = {v};"));
                    cx.depth -= 1;
                    cx.line("}");
                    return Ok(());
                }
                // A whole instance: every variable of it copied (inst.rs).
                if let Some((lv, t)) = crate::inst::whole(&place) {
                    self.assign_data(&lv, &t, value, s, cx)?;
                    cx.depth -= 1;
                    cx.line("}");
                    return Ok(());
                }
                let Some(ty) = place.elem() else {
                    return self.not_yet(target.span, "assigning to this");
                };
                let t = self.checked(value, ty, s, cx)?;
                match &t.keep {
                    Some(k) => cx.line(&format!("if (!{k}) {}", place.store(&t.v))),
                    None => cx.line(&place.store(&t.v)),
                }
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::If { branches, else_body } => {
                let mut closes = 0;
                // A condition left undone ends the chain: `} else if (!k) {`.
                let mut kept: Option<String> = None;
                for (i, (cond, body)) in branches.iter().enumerate() {
                    if i > 0 {
                        cx.line(&else_unless(&kept));
                        cx.depth += 1;
                        closes += 1;
                    }
                    let c = self.condition(cond, s, cx)?;
                    match &c.keep {
                        Some(k) => cx.line(&format!("if (!{k} && {}) {{", c.v)),
                        None => cx.line(&format!("if ({}) {{", c.v)),
                    }
                    kept = c.keep;
                    cx.depth += 1;
                    self.stmts(body, cx)?;
                    cx.depth -= 1;
                }
                if let Some(body) = else_body {
                    cx.line(&else_unless(&kept));
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
                // Ed.3 7.3.3.3.3: a selector of any elementary type, its
                // labels constants compared with it - by `==` and, a
                // subrange, `>=` and `<=`, as the type compares; a STRING
                // by `iecf_scmp`.
                let sel_ty = self.ty_of(selector)?;
                let t = match &sel_ty {
                    Ty::Elem(t) => Some(*t),
                    Ty::Str { .. } => None,
                    _ => return self.not_yet(selector.span, "a CASE on this selector"),
                };
                cx.line("{");
                cx.depth += 1;
                let (sel, keep) = match t {
                    Some(t) => {
                        let Val { v, keep } = self.checked(selector, t, s, cx)?;
                        (v, keep)
                    }
                    None => {
                        // The selector once, the statement's buffers
                        // declared before it.
                        let wide = self.is_wide(selector)?;
                        self.may_fault = false;
                        self.may_flag = false;
                        self.keep_ok = true;
                        self.pre_ok = true;
                        self.keep = None;
                        self.seq = self.calls_effects(selector, cx.scope)?;
                        let v = self.string(selector, cx.scope);
                        self.seq = false;
                        self.keep_ok = false;
                        self.pre_ok = false;
                        let keep = self.keep.take();
                        let (x, _) = v?;
                        let l = self.line(s.span);
                        if let Some(k) = &keep {
                            cx.line(&format!("int {k} = 0;"));
                            self.undone.push(k.clone());
                        }
                        self.declare_buffers(cx, l);
                        let name = self.fresh("t");
                        cx.line(&format!("const {} *{name} = {x};", crate::strings::unit(wide)));
                        if self.may_fault {
                            self.stop(cx, l);
                        }
                        if self.may_flag {
                            noted(cx, l);
                        }
                        (name, keep)
                    }
                };
                if let Some(k) = &keep {
                    cx.line(&format!("if (!{k}) {{"));
                    cx.depth += 1;
                }
                // Each arm's tests first: a label is a constant, and a
                // string one may need a buffer declared before the `if`.
                let mut arm_tests: Vec<Vec<String>> = Vec::new();
                for arm in arms {
                    let mut tests = Vec::new();
                    for label in &arm.labels {
                        if t.is_none() {
                            let w = crate::strings::infix(self.is_wide(selector)?);
                            let text = |g: &mut Self, e: &Expr| -> R<String> { Ok(g.string(e, cx.scope)?.0) };
                            match label {
                                CaseLabel::Value(e) => {
                                    let v = text(self, e)?;
                                    tests.push(format!("iecf_{w}scmp({sel}, {v}) == 0"));
                                }
                                CaseLabel::Range(a, b) => {
                                    let (a, b) = (text(self, a)?, text(self, b)?);
                                    tests.push(format!(
                                        "(iecf_{w}scmp({sel}, {a}) >= 0 && iecf_{w}scmp({sel}, {b}) <= 0)"
                                    ));
                                }
                            }
                            continue;
                        }
                        let t = t.unwrap_or(Elem::Int);
                        let value = |g: &mut Self, e: &Expr| -> R<Option<String>> {
                            if !(t.is_int() || t.is_bit()) {
                                return g.init_scalar(e, t).map(Some);
                            }
                            match const_int(e).or_else(|| g.types.const_value(e.span)) {
                                Some(v) => Ok(lit(v, t)),
                                None => match g.enum_const(e)? {
                                    Some(v) => Ok(Some(v)),
                                    None => g.init_scalar(e, t).map(Some),
                                },
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
                    arm_tests.push(tests);
                }
                let l = self.line(s.span);
                self.declare_buffers(cx, l);
                let mut first = true;
                for (arm, tests) in arms.iter().zip(arm_tests) {
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
                if keep.is_some() {
                    cx.depth -= 1;
                    cx.line("}");
                }
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::For { var, from, to, by, body } => {
                let Some(place) = cx.scope.vars.get(&var.text.to_ascii_uppercase()).cloned() else {
                    return self.not_yet(var.span, format!("'{}' here", var.text));
                };
                let Some(t) = place.elem() else { return self.not_yet(var.span, "this FOR variable") };
                // At an address: a C copy counts, written to the address
                // as each pass starts and read back after it, so the body
                // sees and may change it there (`at`).
                let at = place.lvalue().is_none().then(|| place.clone());
                let lv = match place.lvalue() {
                    Some(lv) => lv.to_owned(),
                    None => self.fresh("fv"),
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
                    None => Val { v: lit(1, t).unwrap_or_default(), keep: None },
                };
                // A FOR over a subrange runs inside it: its start clamped,
                // its end the bound it would pass, and it ends at its last
                // value (runtime/iecf.h).
                let inside = matches!(&place, Place::Data(_, dt) if dt.range.is_some());
                if let Place::Data(_, dt) = &place
                    && let Some((lo, hi)) = dt.range
                {
                    self.clamped(&start.v, t, dt.range, s, cx);
                    let ct = c_type(t);
                    let (e, st) = (&end.v, &step.v);
                    cx.line(&if t.is_signed_int() {
                        format!("{e} = (({ct})iecf_for_end_s(&ctx->flag, (int64_t)({e}), (int64_t)({st}), INT64_C({lo}), INT64_C({hi})));")
                    } else {
                        format!("{e} = (({ct})iecf_for_end_u(&ctx->flag, (uint64_t)({e}), UINT64_C({hi})));")
                    });
                    let l = self.line(s.span);
                    noted(cx, l);
                }
                let n = t.name();
                let done = self.fresh("d");
                cx.line(&format!("int {done};"));
                if at.is_some() {
                    cx.line(&format!("{} {lv};", c_type(t)));
                }
                // A bound left undone: the loop does not run, and the
                // variable keeps its value.
                let keeps: Vec<&String> = [&start, &end, &step].iter().filter_map(|v| v.keep.as_ref()).collect();
                let guarded = !keeps.is_empty();
                if guarded {
                    let any = keeps.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(" || ");
                    cx.line(&format!("if (!({any})) {{"));
                    cx.depth += 1;
                }
                let (start, end, step) = (start.v, end.v, step.v);
                // After the loop the variable is one step past its end; a
                // step past the type's limit wraps, flagged (runtime/iecf.h).
                let next = if inside {
                    format!("iecf_for_next_in_{n}(&{lv}, {end}, {step})")
                } else {
                    format!("iecf_for_next_{n}(&ctx->flag, &{lv}, {end}, {step})")
                };
                let next = match &at {
                    Some(p) => format!("{lv} = {}, {done} = {next}", p.load()),
                    None => format!("{done} = {next}"),
                };
                cx.line(&format!(
                    "for ({lv} = {start}, {done} = iecf_for_done_{n}({lv}, {end}, {step}); !{done}; {next}) {{"
                ));
                cx.depth += 1;
                if let Some(p) = &at {
                    cx.line(&p.store(&lv));
                }
                cx.loops.push(Loop::Plain);
                let r = self.stmts(body, cx);
                cx.loops.pop();
                r?;
                cx.depth -= 1;
                cx.line("}");
                // Ended, not left by EXIT (which leaves it as the body
                // has it there): one step past the end, at its address.
                if let Some(p) = &at {
                    cx.line(&format!("if ({done}) {}", p.store(&lv)));
                }
                if !inside {
                    let l = self.line(s.span);
                    noted(cx, l);
                }
                if guarded {
                    cx.depth -= 1;
                    cx.line("}");
                }
                cx.depth -= 1;
                cx.line("}");
            }
            StmtKind::While { cond, body } => {
                cx.line("for (;;) {");
                cx.depth += 1;
                let c = self.condition(cond, s, cx)?;
                match &c.keep {
                    Some(k) => cx.line(&format!("if ({k} || !{}) break;", c.v)),
                    None => cx.line(&format!("if (!{}) break;", c.v)),
                }
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
                match &c.keep {
                    Some(k) => cx.line(&format!("if ({k} || {}) break;", c.v)),
                    None => cx.line(&format!("if ({}) break;", c.v)),
                }
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
            StmtKind::Return => {
                let r = cx.on_return.clone().unwrap_or_else(|| cx.ret.clone());
                cx.line(&r);
            }
            StmtKind::Empty => {}
            // reference.rs wrote each as an assignment.
            StmtKind::RefAssign { target, .. } => return self.not_yet(target.span, "this REF="),
            StmtKind::AssignAttempt { target, value } => self.assign_attempt(target, value, s, cx)?,
            StmtKind::Call(e) if self.fb_call(e, s, cx)? => {}
            StmtKind::Call(e) => {
                cx.line("{");
                cx.depth += 1;
                self.may_fault = false;
                self.may_flag = false;
                self.pre_ok = true;
                self.seq = self.calls_effects(e, cx.scope)?;
                let v = self.value(e, cx.scope);
                self.seq = false;
                self.pre_ok = false;
                let v = v?;
                self.declare_buffers(cx, line);
                cx.line(&format!("(void)({v});"));
                if self.may_fault {
                    self.stop(cx, line);
                }
                if self.may_flag {
                    noted(cx, line);
                }
                cx.depth -= 1;
                cx.line("}");
            }
        }
        Ok(())
    }
}

impl Gen<'_> {
    /// An element or field assigned to: where it is, each index that is
    /// not a constant computed and checked first, so a statement that
    /// faults stores nothing.
    /// The instance `base` names, as a target's base: through a reference
    /// a FUNCTION gives, the reference computed and checked first.
    fn fb_target(&mut self, base: &Expr, at: &Stmt, cx: &mut Body) -> R<Option<Place>> {
        match &crate::expr::strip_paren(base).kind {
            ExprKind::Deref(b) if crate::expr::has_call(b) => {
                let (t, to) = self.deref_target(b, at, cx)?;
                Ok(Some(self.fb_through(&t, &to)))
            }
            _ => self.fb_place(base, cx.scope),
        }
    }

    fn target(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<(Lv, DTy)> {
        match &e.kind {
            ExprKind::Paren(x) => self.target(x, at, cx),
            ExprKind::Name(_) => match self.data_path(e, cx.scope)? {
                Some(p) => Ok(p),
                None => self.not_yet(e.span, "assigning to this"),
            },
            // An input of an instance set from outside (Ed.3 6.6.7.2.2,
            // Figure 20 a), or a PUBLIC variable (6.6.5.2 rule 7): its
            // field, which the next call reads. The instance is found
            // once: through a FUNCTION's reference, that call made once.
            ExprKind::Member(base, m) if matches!(self.types.expr(base.span), Some(Ty::Fb(_))) => {
                let k = m.text.to_ascii_uppercase();
                match self.fb_target(base, at, cx)? {
                    // A standard block's input: its field.
                    Some(Place::Fb(lv, fb)) => match fb.param(&m.text) {
                        Some((n, t, Dir::Input)) => {
                            Ok((Lv::C(format!("{lv}.{n}")), DTy { kind: Kind::Elem(*t), init: None, range: None }))
                        }
                        _ => self.not_yet(e.span, "assigning to this"),
                    },
                    Some(Place::Block(lv, key)) => {
                        let Some(sig) = self.blocks.get(&key) else {
                            return self.not_yet(e.span, "assigning to this");
                        };
                        let field = Lv::C(format!("{lv}.{}", sig.field(&k)));
                        if let Some((_, t, Dir::Input)) = sig.params.iter().find(|(n, _, _)| *n == k) {
                            let range = sig.ranges.get(&k).copied();
                            return Ok((field, DTy { kind: Kind::Elem(*t), init: None, range }));
                        }
                        if let Some((_, t, Dir::Input)) = sig.data.iter().find(|(n, _, _)| *n == k) {
                            return Ok((field, t.clone()));
                        }
                        self.not_yet(e.span, "assigning to this")
                    }
                    // An element or field that is an instance (inst.rs).
                    _ => {
                        let (lv, t) = self.target(base, at, cx)?;
                        self.lv_member(&lv, &t, &k, m.span)
                    }
                }
            }
            ExprKind::Member(base, m) => {
                let (lv, t) = self.target(base, at, cx)?;
                self.lv_member(&lv, &t, &m.text.to_ascii_uppercase(), m.span)
            }
            ExprKind::Index(base, idx) => {
                let (lv, t) = self.target(base, at, cx)?;
                // Each dimension's bounds; an ARRAY[*]'s read from it.
                let dims: Vec<Option<(i64, i64)>> = match (&t.kind, &lv) {
                    (Kind::Array(_, bounds), _) => bounds.iter().copied().map(Some).collect(),
                    (Kind::Flex(_, n), Lv::C(_)) => vec![None; *n],
                    _ => return self.not_yet(e.span, "this index"),
                };
                if idx.len() > dims.len() {
                    return self.not_yet(e.span, "an index of another count of dimensions");
                }
                let mut offs = Vec::new();
                for (k, (i, b)) in idx.iter().zip(dims).enumerate() {
                    // Computed as a statement's own expression is, before
                    // the value (Ed.3 7.3.2): a FUNCTION with effects on a
                    // line of its own; in an assignment's target, a real
                    // converted out of range leaves the store undone.
                    self.may_fault = false;
                    self.may_flag = false;
                    self.pre_ok = true;
                    self.keep_ok = self.target_keep_ok;
                    self.keep = None;
                    let off = match self.calls_effects(i, cx.scope) {
                        Ok(seq) => {
                            self.seq = seq;
                            match (b, &lv) {
                                (Some((lo, hi)), _) => self.index(i, lo, hi, cx.scope),
                                (None, Lv::C(d)) => self.flex_index(i, d, k, cx.scope),
                                (None, _) => self.not_yet(e.span, "this index"),
                            }
                        }
                        Err(x) => Err(x),
                    };
                    self.seq = false;
                    self.pre_ok = false;
                    self.keep_ok = false;
                    let keep = self.keep.take();
                    let off = off?;
                    let l = self.line(at.span);
                    if let Some(k) = &keep {
                        cx.line(&format!("int {k} = 0;"));
                        self.undone.push(k.clone());
                        self.target_keeps.push(k.clone());
                    }
                    let hoisted = !self.str_temps.is_empty();
                    self.declare_buffers(cx, l);
                    if self.may_fault || self.may_flag || hoisted || keep.is_some() {
                        let o = self.fresh("o");
                        cx.line(&format!("uint32_t {o} = {off};"));
                        if self.may_fault {
                            self.stop(cx, l);
                        }
                        if self.may_flag {
                            noted(cx, l);
                        }
                        offs.push(o);
                    } else {
                        offs.push(off);
                    }
                }
                self.lv_index(&lv, &t, &offs, e.span)
            }
            // A part of a bit string: its variable found as a target is,
            // an index in it checked first.
            ExprKind::BitAccess(base, lit) => {
                let whole = match &crate::expr::strip_paren(base).kind {
                    ExprKind::Name(id) => {
                        cx.scope.vars.get(&id.text.to_ascii_uppercase()).and_then(crate::expr::elem_lv)
                    }
                    _ => {
                        let (lv, t) = self.target(base, at, cx)?;
                        t.scalar().map(|e| (lv, e))
                    }
                };
                let Some((lv, of)) = whole else { return self.not_yet(e.span, "assigning to this") };
                Ok(crate::expr::part_of(lv, of, lit))
            }
            ExprKind::Deref(base) => {
                let (t, to) = self.deref_target(base, at, cx)?;
                // An instance: the whole of it, or its input set from
                // outside, as one named directly is.
                match self.fb_through(&t, &to) {
                    Place::Data(lv, d) => Ok((lv, d)),
                    Place::Block(lv, k) => Ok((Lv::C(lv), DTy { kind: Kind::Block(k), init: None, range: None })),
                    Place::Fb(lv, fb) => Ok((Lv::C(lv), DTy { kind: Kind::Std(fb), init: None, range: None })),
                    _ => self.not_yet(e.span, "assigning through this reference"),
                }
            }
            _ => self.not_yet(e.span, "assigning to this"),
        }
    }

    /// A whole array or structure read: where it is, its indices computed
    /// and checked first, as a target's; or a function block's output.
    fn source(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<(Lv, DTy)> {
        if let ExprKind::Member(base, m) = &e.kind {
            if let ExprKind::Name(id) = &base.kind
                && matches!(cx.scope.vars.get(&id.text.to_ascii_uppercase()), Some(Place::Block(..)))
            {
                return match self.data_path(e, cx.scope)? {
                    Some(p) => Ok(p),
                    None => self.not_yet(e.span, "this value of an array or structure"),
                };
            }
            // An output of an instance reached through a reference.
            if let ExprKind::Deref(b) = &base.kind
                && matches!(self.ref_of(b, cx.scope)?, Some((_, RefTo::Block(_))))
            {
                let (t, to) = self.deref_target(b, at, cx)?;
                if let Place::Block(lv, key) = self.fb_through(&t, &to) {
                    let name = m.text.to_ascii_uppercase();
                    if let Some(b) = self.blocks.get(&key)
                        && let Some((n, dt, _)) = b.data.iter().find(|(n, _, _)| *n == name)
                    {
                        return Ok((Lv::C(format!("{lv}.{}", b.field(n))), dt.clone()));
                    }
                }
                return self.not_yet(e.span, "this value of an array or structure");
            }
        }
        match &e.kind {
            ExprKind::Name(_)
            | ExprKind::Member(..)
            | ExprKind::Index(..)
            | ExprKind::Paren(_)
            | ExprKind::Deref(_) => self.target(e, at, cx),
            // The project's FUNCTION or method giving an array or a
            // structure: into a buffer of the statement's (params.rs).
            // SEL, MUX or MOVE of whole values: the input selected.
            ExprKind::Call { .. } if self.types.std_call(e.span).is_some() => {
                self.may_fault = false;
                self.may_flag = false;
                self.pre_ok = true;
                let seq = self.calls_effects(e, cx.scope);
                self.seq = seq.unwrap_or(false);
                let r = self.data_path(e, cx.scope);
                self.seq = false;
                self.pre_ok = false;
                let Some(p) = r? else { return self.not_yet(e.span, "this value of an array or structure") };
                let l = self.line(at.span);
                self.declare_buffers(cx, l);
                if self.may_fault {
                    self.stop(cx, l);
                }
                if self.may_flag {
                    noted(cx, l);
                }
                Ok(p)
            }
            ExprKind::Call { callee, args } => {
                self.may_fault = false;
                self.may_flag = false;
                self.pre_ok = true;
                let seq = self.calls_effects(e, cx.scope);
                self.seq = seq.unwrap_or(false);
                // With EN or ENO: the call in its block (enable.rs); EN
                // FALSE leaves the store undone.
                self.keep_ok = true;
                self.keep = None;
                let r = match self.gated(e, args, cx.scope, false) {
                    Ok(Some((r, _))) => Ok(r),
                    Ok(None) => self.user_call(e, callee, args, cx.scope),
                    Err(x) => Err(x),
                };
                self.keep_ok = false;
                self.seq = false;
                self.pre_ok = false;
                let r = r?;
                if let Some(k) = self.keep.take() {
                    cx.line(&format!("int {k} = 0;"));
                    self.undone.push(k.clone());
                    self.target_keeps.push(k);
                }
                let Some(t) = self.called_sig(e).and_then(|s| s.ret_data) else {
                    return self.not_yet(e.span, "this value of an array or structure");
                };
                let l = self.line(at.span);
                self.declare_buffers(cx, l);
                if self.may_fault {
                    self.stop(cx, l);
                }
                if self.may_flag {
                    noted(cx, l);
                }
                Ok((Lv::C(r), t))
            }
            _ => self.not_yet(e.span, "this value of an array or structure"),
        }
    }

    /// A subrange's value in the temporary `v`, clamped and flagged.
    fn clamped(&mut self, v: &str, e: Elem, range: Option<(i128, i128)>, at: &Stmt, cx: &mut Body) {
        if range.is_none() {
            return;
        }
        let c = self.clamp(v, e, range);
        cx.line(&format!("{v} = {c};"));
        let l = self.line(at.span);
        noted(cx, l);
    }

    /// `lv := value` for `lv` of type `t`: a value, or a whole array or
    /// structure copied from another of its type.
    fn assign_data(&mut self, lv: &Lv, t: &DTy, value: &Expr, at: &Stmt, cx: &mut Body) -> R<()> {
        if let Kind::Ref(to, _) = &t.kind {
            self.check_ref_target(to, value, cx.scope)?;
            let v = self.checked_reference(value, at, cx)?;
            let Some(c) = lv.c() else { return self.not_yet(value.span, "storing a reference here") };
            self.store_line(cx, &format!("{c} = {v};"), None);
            return Ok(());
        }
        // An interface: NULL, an instance or an interface (itf.rs).
        if let Kind::Itf(i) = &t.kind {
            let v = self.checked_itf(value, i, at, cx)?;
            let Some(c) = lv.c() else { return self.not_yet(value.span, "storing an interface here") };
            self.store_line(cx, &format!("{c} = {v};"), None);
            return Ok(());
        }
        if let Some(e) = t.scalar() {
            let v = self.checked(value, e, at, cx)?;
            self.clamped(&v.v, e, t.range, at, cx);
            let store = lv.store(e, &v.v);
            self.store_line(cx, &store, v.keep.as_deref());
            return Ok(());
        }
        if let Kind::Str(n, w) = t.kind {
            return self.assign_string(lv, n, w, value, at, cx);
        }
        // Into an ARRAY[*], whose size only the run knows: from an array
        // of as many elements in each dimension, else a fault (INDEX) and
        // nothing copied (flex.rs).
        if let (Kind::Flex(elem, n), Lv::C(d)) = (&t.kind, lv) {
            let (src, st) = self.source(value, at, cx)?;
            let l = self.line(at.span);
            let differ = |of: &dyn Fn(usize) -> String| {
                (0..*n).map(|k| format!("iecf_flex_n({d}, {k}u) != {}", of(k))).collect::<Vec<_>>().join(" || ")
            };
            let (cond, copy) = match (&st.kind, &src) {
                (Kind::Flex(..), Lv::C(s)) => {
                    let total: Vec<String> = (0..*n).map(|k| format!("(size_t)iecf_flex_n({d}, {k}u)")).collect();
                    let size = format!("{} * sizeof({})", total.join(" * "), self.declare(elem, ""));
                    (differ(&|k| format!("iecf_flex_n({s}, {k}u)")), format!("iecf_move({d}->p, {s}->p, {size});"))
                }
                (Kind::Array(_, bounds), _) if bounds.len() == *n => {
                    let bounds = bounds.clone();
                    // The ARRAY[*], seen as an array of the source's type.
                    let dst = Lv::C(format!("(*({}){d}->p)", self.declare(&st, "(*)")));
                    let copy = self.copy_data(&dst, &src, &st, value.span)?;
                    (differ(&|k| format!("{}u", i128::from(bounds[k].1) - i128::from(bounds[k].0) + 1)), copy)
                }
                _ => return self.flex_whole(value.span),
            };
            cx.line(&format!("if ({cond}) iecf_fail(&ctx->fault, IECF_FAULT_INDEX);"));
            self.stop(cx, l);
            self.store_line(cx, &copy, None);
            return Ok(());
        }
        let (src, _) = self.source(value, at, cx)?;
        let copy = self.copy_data(lv, &src, t, value.span)?;
        self.store_line(cx, &copy, None);
        Ok(())
    }

    /// The store of an assignment: left undone when `keep`, its value's
    /// marker, or one its target's indices set is set.
    fn store_line(&mut self, cx: &mut Body, store: &str, keep: Option<&str>) {
        let keeps: Vec<String> =
            keep.into_iter().map(str::to_string).chain(self.target_keeps.iter().cloned()).collect();
        if keeps.is_empty() {
            cx.line(store);
        } else {
            let cond: Vec<String> = keeps.iter().map(|k| format!("!{k}")).collect();
            // One C statement as it is; more in a block.
            if !store.contains('\n') && store.matches(';').count() == 1 {
                cx.line(&format!("if ({}) {store}", cond.join(" && ")));
            } else {
                cx.line(&format!("if ({}) {{", cond.join(" && ")));
                for l in store.lines() {
                    cx.line(&format!("    {l}"));
                }
                cx.line("}");
            }
        }
    }
}

impl Gen<'_> {
    /// `s[i] := value`: the string's place, then the position and the
    /// value, each computed and checked, then the character replaced
    /// (strings.rs). A statement that faults stores nothing.
    fn assign_char(&mut self, base: &Expr, i: &Expr, value: &Expr, at: &Stmt, cx: &mut Body) -> R<()> {
        let (lv, t) = self.target(base, at, cx)?;
        let Kind::Str(n, w) = t.kind else { return self.not_yet(base.span, "this string") };
        let p = self.checked(i, Elem::Lint, at, cx)?;
        let c = self.checked(value, if w { Elem::Wchar } else { Elem::Char }, at, cx)?;
        let put = self.put_char("@", w, &p.v, &c.v);
        let put = crate::strings::store_string(&lv, n, w, &put);
        let l = self.line(at.span);
        self.declare_buffers(cx, l);
        let keep: Vec<String> = [p.keep, c.keep].into_iter().flatten().map(|k| format!("!{k}")).collect();
        if keep.is_empty() {
            cx.line(&put);
        } else {
            cx.line(&format!("if ({}) {put}", keep.join(" && ")));
        }
        noted(cx, l);
        Ok(())
    }

    /// `lv := value` for a `STRING(n)`, `w` a `WSTRING(n)`: the value
    /// computed and checked, then copied, cut to `n` characters if longer
    /// (runtime/iecf.h).
    fn assign_string(&mut self, lv: &Lv, n: u32, w: bool, value: &Expr, at: &Stmt, cx: &mut Body) -> R<()> {
        if !self.is_string(value)? && !matches!(self.ty_of(value)?, Ty::Elem(Elem::Char | Elem::Wchar)) {
            return self.not_yet(value.span, "this value of a STRING");
        }
        self.may_fault = false;
        self.may_flag = false;
        self.keep_ok = true;
        self.pre_ok = true;
        self.keep = None;
        self.seq = self.calls_effects(value, cx.scope)?;
        let v = self.string(value, cx.scope);
        self.seq = false;
        self.keep_ok = false;
        self.pre_ok = false;
        let keep = self.keep.take();
        let (src, _) = v?;
        let l = self.line(at.span);
        if let Some(k) = &keep {
            cx.line(&format!("int {k} = 0;"));
            self.undone.push(k.clone());
        }
        self.declare_buffers(cx, l);
        let s = self.fresh("s");
        cx.line(&format!("const {} *{s} = {src};", crate::strings::unit(w)));
        if self.may_fault {
            self.stop(cx, l);
        }
        let copy = format!("iecf_{}scopy(&ctx->flag, @, {n}u, {s});", crate::strings::infix(w));
        let copy = crate::strings::store_string(lv, n, w, &copy);
        self.store_line(cx, &copy, keep.as_deref());
        noted(cx, l);
        Ok(())
    }
}

/// A checked value: its temporary, and the `keep` its conversions set.
struct Val {
    v: String,
    keep: Option<String>,
}

/// The `else` of an IF chain, skipped when the condition was left undone.
fn else_unless(keep: &Option<String>) -> String {
    match keep {
        Some(k) => format!("}} else if (!{k}) {{"),
        None => "} else {".to_string(),
    }
}

/// After a statement that can flag: say where, the first time.
fn noted(cx: &mut Body, line: u32) {
    cx.line(&format!("iecf_noted(&ctx->flag, {}u, {line}u);", cx.pou));
}

/// A function block instance as a call sees it.
pub(crate) struct Iface {
    pub(crate) lv: String,
    pub(crate) label: String,
    /// Upper-cased IEC name, C field, type, direction.
    pub(crate) params: Vec<(String, String, Elem, Dir)>,
    /// The call, `&lv` filled in.
    pub(crate) call: String,
    /// Whether the call can fault: a user's block can.
    pub(crate) faults: bool,
    /// Its inputs of a subrange type, by upper-cased name: their bounds.
    pub(crate) ranges: std::collections::HashMap<String, (i128, i128)>,
    std: Option<crate::names::StdFb>,
}

impl Iface {
    pub(crate) fn param(&self, name: &str) -> Option<&(String, String, Elem, Dir)> {
        let canon = match &self.std {
            Some(fb) => fb.param(name)?.0.clone(),
            None => name.to_ascii_uppercase(),
        };
        self.params.iter().find(|p| p.0 == canon)
    }
}

impl Gen<'_> {
    pub(crate) fn iface(&self, place: &Place) -> Option<Iface> {
        match place {
            Place::Fb(lv, fb) => Some(Iface {
                lv: lv.clone(),
                label: fb.stem.clone(),
                params: fb.params.iter().map(|(n, t, d)| (n.clone(), n.clone(), *t, *d)).collect(),
                call: format!("iecf_{}(&{lv}{});", fb.stem, if fb.timer { ", ctx->now_ns" } else { "" }),
                faults: false,
                ranges: Default::default(),
                std: Some(fb.clone()),
            }),
            Place::Block(lv, key) => Some(Iface {
                lv: lv.clone(),
                label: key.clone(),
                params: self
                    .blocks
                    .get(key)?
                    .params
                    .iter()
                    .map(|(n, t, d)| {
                        (n.clone(), self.blocks.get(key).map_or_else(|| format!("V_{n}"), |b| b.field(n)), *t, *d)
                    })
                    .collect(),
                call: format!("B_{key}(ctx, &{lv});"),
                faults: true,
                ranges: self.blocks.get(key)?.ranges.clone(),
                std: None,
            }),
            _ => None,
        }
    }

    /// An input of an array, structure or STRING type: its source found or
    /// computed now, copied in with the other inputs (`sets`); a STRING
    /// too long for it cut and flagged, as any store.
    fn data_in(&mut self, field: &str, t: &DTy, x: &Expr, at: &Stmt, cx: &mut Body, sets: &mut Vec<String>) -> R<()> {
        if let Kind::Ref(to, _) = &t.kind {
            self.check_ref_target(to, x, cx.scope)?;
            let v = self.checked_reference(x, at, cx)?;
            sets.push(format!("{field} = {v};"));
            return Ok(());
        }
        let l = self.line(at.span);
        if let Kind::Str(n, w) = t.kind {
            self.may_fault = false;
            self.may_flag = false;
            self.keep_ok = true;
            self.pre_ok = true;
            self.keep = None;
            let v = self.string(x, cx.scope);
            self.keep_ok = false;
            self.pre_ok = false;
            let keep = self.keep.take();
            let (src, _) = v?;
            if let Some(k) = &keep {
                cx.line(&format!("int {k} = 0;"));
                self.undone.push(k.clone());
            }
            self.declare_buffers(cx, l);
            let s = self.fresh("s");
            cx.line(&format!("const {} *{s} = {src};", crate::strings::unit(w)));
            if self.may_fault {
                self.stop(cx, l);
            }
            let copy = format!("iecf_{}scopy(&ctx->flag, {field}, {n}u, {s});", crate::strings::infix(w));
            sets.push(match &keep {
                Some(k) => format!("if (!{k}) {copy}"),
                None => copy,
            });
            return Ok(());
        }
        let _ = l;
        let (src, _) = self.source(x, at, cx)?;
        let copy = self.copy_data(&Lv::C(field.to_string()), &src, t, x.span)?;
        sets.push(copy);
        Ok(())
    }

    /// An output of an array, structure or STRING type taken into
    /// `target` after the call (`gets`).
    fn data_out(&mut self, field: &str, t: &DTy, target: &Expr, cx: &mut Body, gets: &mut Vec<String>) -> R<()> {
        self.may_fault = false;
        let Some((lv, tt)) = self.data_path(target, cx.scope)? else {
            return self.not_yet(target.span, "an output taken into this");
        };
        if self.may_fault {
            return self.not_yet(target.span, "an output taken into an element by a computed index");
        }
        gets.push(match (&t.kind, &tt.kind) {
            (Kind::Str(..), Kind::Str(n, w)) => {
                let copy = format!("iecf_{}scopy(&ctx->flag, @, {n}u, {field});", crate::strings::infix(*w));
                crate::strings::store_string(&lv, *n, *w, &copy)
            }
            (Kind::Itf(a), Kind::Itf(b)) => self.itf_store(&lv, b, field, a, target.span)?,
            _ => self.copy_data(&lv, &Lv::C(field.to_string()), t, target.span)?,
        });
        Ok(())
    }

    /// `t1(IN := x, PT := T#1s, Q => y);` for a function block instance:
    /// its inputs set and in-outs bound, the block called, its outputs
    /// taken. `false` if `e` is not such a call.
    fn fb_call(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<bool> {
        let ExprKind::Call { callee, args } = &e.kind else { return Ok(false) };
        // SUPER(): the base's body, on the instance's base part (Ed.3
        // 6.6.7.2.9), bound statically.
        if let ExprKind::Name(id) = &callee.kind
            && id.is("SUPER")
        {
            let base = match cx.scope.vars.get("THIS") {
                Some(Place::Block(_, d)) => self.types.base(d).map(str::to_string),
                _ => None,
            };
            let Some(b) = base else { return self.not_yet(e.span, "SUPER() here") };
            cx.line(&format!("B_{b}(ctx, &self->base);"));
            let l = self.line(at.span);
            self.stop(cx, l);
            return Ok(true);
        }
        // Through a reference: the reference checked first, so NULL stops
        // the statement before the block runs on nothing.
        if let ExprKind::Deref(base) = &callee.kind
            && matches!(self.types.expr(callee.span), Some(Ty::Fb(_)))
        {
            cx.line("{");
            cx.depth += 1;
            let (t, to) = self.deref_target(base, at, cx)?;
            let place = self.fb_through(&t, &to);
            let r = self.fb_call_on(place, true, args, e, at, cx);
            cx.depth -= 1;
            cx.line("}");
            return r;
        }
        // An element or a field, `ts[i](...)`, `c.t(...)` (Ed.3 Figure 8
        // d, e): its address taken first, the index checked, so one out
        // of bounds stops the statement before the block runs (inst.rs).
        if matches!(callee.kind, ExprKind::Index(..) | ExprKind::Member(..)) && self.types.method_call(e.span).is_none()
        {
            self.may_fault = false;
            self.may_flag = false;
            self.pre_ok = true;
            let p = self.fb_place(callee, cx.scope);
            self.pre_ok = false;
            let Some(place) = p? else { return Ok(false) };
            let (ty, lv) = match &place {
                Place::Block(lv, k) => (format!("B_{k}_t"), lv.clone()),
                Place::Fb(lv, fb) => (format!("iecf_{}_t", fb.stem), lv.clone()),
                _ => return Ok(false),
            };
            cx.line("{");
            cx.depth += 1;
            let l = self.line(at.span);
            self.declare_buffers(cx, l);
            let o = self.fresh("o");
            cx.line(&format!("{ty} *{o} = &({lv});"));
            if self.may_fault {
                self.stop(cx, l);
            }
            let place = match place {
                Place::Block(_, k) => Place::Block(format!("(*{o})"), k),
                Place::Fb(_, fb) => Place::Fb(format!("(*{o})"), fb),
                p => p,
            };
            let r = self.fb_call_on(place, false, args, e, at, cx);
            cx.depth -= 1;
            cx.line("}");
            return r;
        }
        let ExprKind::Name(id) = &callee.kind else { return Ok(false) };
        let Some(place) = cx.scope.vars.get(&id.text.to_ascii_uppercase()).cloned() else { return Ok(false) };
        let poly = cx.scope.poly.contains(&id.text.to_ascii_uppercase());
        self.fb_call_on(place, poly, args, e, at, cx)
    }

    /// `poly`: the instance may be of a type derived from its place's,
    /// so its body is found in its table (oo.rs).
    fn fb_call_on(&mut self, place: Place, poly: bool, args: &[Arg], e: &Expr, at: &Stmt, cx: &mut Body) -> R<bool> {
        let Some(mut fb) = self.iface(&place) else { return Ok(false) };
        if let (true, Place::Block(lv, key)) = (poly, &place)
            && let Some(call) = self.dispatch_body(lv, key)
        {
            fb.call = call;
        }
        let lv = fb.lv.clone();
        cx.line("{");
        cx.depth += 1;
        // EN and ENO (Ed.3 6.6.1.5), unless the block declares them as its
        // own parameters: EN FALSE leaves the block uncalled - its inputs
        // not evaluated nor set, its outputs not taken - and ENO FALSE;
        // otherwise ENO is FALSE when a flag is raised while the block runs
        // (docs/dependencies.md).
        let gate = |n: &str| fb.param(n).is_none();
        let en = args.iter().find_map(|a| match a {
            Arg::Input(n, x) if n.is("EN") && gate("EN") => Some(x),
            _ => None,
        });
        let eno = args.iter().find_map(|a| match a {
            Arg::Output { name, not, target } if name.is("ENO") && gate("ENO") => Some((*not, target)),
            _ => None,
        });
        let args: Vec<&Arg> = args
            .iter()
            .filter(|a| match a {
                Arg::Input(n, _) => !(n.is("EN") && gate("EN")),
                Arg::Output { name, .. } => !(name.is("ENO") && gate("ENO")),
                Arg::Positional(_) => true,
            })
            .collect();
        let eno_store = match eno {
            Some((not, target)) => Some(self.eno_target(not, target, at, cx)?),
            None => None,
        };
        let o = self.fresh("eno");
        if eno.is_some() {
            cx.line(&format!("IEC_BOOL {o} = 0;"));
        }
        match en {
            Some(x) => {
                self.may_fault = false;
                let c = self.value_as(x, Elem::Bool, cx.scope)?;
                if self.may_fault {
                    let n = self.fresh("en");
                    cx.line(&format!("IEC_BOOL {n} = {c};"));
                    let l = self.line(at.span);
                    self.stop(cx, l);
                    cx.line(&format!("if ({n}) {{"));
                } else {
                    cx.line(&format!("if ({c}) {{"));
                }
            }
            None => cx.line("{"),
        }
        cx.depth += 1;
        let mut sets = Vec::new();
        let mut gets = Vec::new();
        // In-outs given a copy, stored back before the outputs are taken.
        let mut backs = Vec::new();
        let mut bound: Vec<String> = Vec::new();
        let mut flag_outputs = false;
        // Inputs and outputs of an array, structure or STRING type.
        let (data, in_blocks, sig_paths, itfs, in_data) = match &place {
            Place::Block(_, key) => self
                .blocks
                .get(key)
                .map(|b| (b.data.clone(), b.blocks.clone(), b.paths.clone(), b.itfs.clone(), b.in_data.clone()))
                .unwrap_or_default(),
            _ => Default::default(),
        };
        for &a in &args {
            let pname = match a {
                Arg::Input(n, _) | Arg::Output { name: n, .. } => Some(n),
                Arg::Positional(_) => None,
            };
            // An instance for an in-out of a block type: its part of that
            // type (the checker allowed a derived one, OO.md D7).
            if let (Some(n), Arg::Input(_, x)) = (pname, a)
                && let Some((key, ty)) = in_blocks.iter().find(|(k, _)| n.is(k)).cloned()
            {
                self.pre_ok = true;
                let got = self.call_instance(x, cx.scope);
                self.pre_ok = false;
                let Some(Place::Block(xlv, xs)) = got? else {
                    return self.not_yet(x.span, "this instance given to an in-out");
                };
                let l = self.line(at.span);
                self.declare_buffers(cx, l);
                let field = sig_paths.get(&key).cloned().unwrap_or_else(|| format!("V_{key}"));
                sets.push(format!("{lv}.{field} = &({xlv}){};", self.up_path(&xs, &ty)));
                bound.push(key);
                continue;
            }
            // An in-out of an array, structure or STRING type: the
            // variable's address, or an ARRAY[*]'s iecf_flex_t (flex.rs).
            if let (Some(n), Arg::Input(_, x)) = (pname, a)
                && let Some((key, t)) = in_data.iter().find(|(k, _)| n.is(k)).cloned()
            {
                let was = std::mem::replace(&mut self.may_fault, false);
                self.pre_ok = true;
                let p = self.data_inout(x, &t, cx.scope);
                self.pre_ok = false;
                let p = p?;
                // One at an address or through a reference is given a
                // copy, stored back as the call returns (params.rs).
                let l = self.line(at.span);
                self.declare_buffers(cx, l);
                backs.append(&mut self.in_out_backs);
                // An element by a computed index: found now, so an index
                // out of bounds stops the statement before the block runs.
                let p = if std::mem::replace(&mut self.may_fault, was) {
                    let q = self.fresh("q");
                    cx.line(&format!("void *{q} = {p};"));
                    self.stop(cx, l);
                    q
                } else {
                    p
                };
                let field = sig_paths.get(&key).cloned().unwrap_or_else(|| format!("V_{key}"));
                sets.push(format!("{lv}.{field} = {p};"));
                bound.push(key);
                continue;
            }
            // An input of an interface: the instance, checked as an input
            // is, set with the others (itf.rs).
            if let (Some(n), Arg::Input(_, x)) = (pname, a)
                && let Some((key, i)) = itfs.iter().find(|(k, _)| n.is(k)).cloned()
            {
                let v = self.checked_itf(x, &i, at, cx)?;
                let field = sig_paths.get(&key).cloned().unwrap_or_else(|| format!("V_{key}"));
                sets.push(format!("{lv}.{field} = {v};"));
                continue;
            }
            if let Some(n) = pname
                && let Some((key, t, dir)) = data.iter().find(|(k, _, _)| n.is(k)).cloned()
            {
                let field = format!("{lv}.{}", sig_paths.get(&key).cloned().unwrap_or_else(|| format!("V_{key}")));
                match (a, dir) {
                    (Arg::Input(_, x), Dir::Input) => self.data_in(&field, &t, x, at, cx, &mut sets)?,
                    (Arg::Output { not: false, target, .. }, Dir::Output) => {
                        self.data_out(&field, &t, target, cx, &mut gets)?
                    }
                    _ => return self.not_yet(n.span, format!("{} of {} here", n.text, fb.label)),
                }
                flag_outputs |= matches!(t.kind, Kind::Str(..));
                continue;
            }
            match a {
                Arg::Input(n, x) => match fb.param(&n.text).cloned() {
                    Some((name, field, t, Dir::Input)) => {
                        let v = self.checked(x, t, at, cx)?;
                        let range = fb.ranges.get(&name).copied();
                        self.clamped(&v.v, t, range, at, cx);
                        sets.push(match &v.keep {
                            Some(k) => format!("if (!{k}) {lv}.{field} = {};", v.v),
                            None => format!("{lv}.{field} = {};", v.v),
                        });
                    }
                    // Any variable of the in-out's type: its address; an
                    // element by a computed index found first, so one out
                    // of bounds stops the statement before the block
                    // runs; one at an address or of a subrange given a
                    // copy, stored back with the outputs (expr.rs
                    // `in_out_arg`, as a FUNCTION's).
                    Some((name, field, t, Dir::InOut)) => {
                        let was = std::mem::replace(&mut self.may_fault, false);
                        self.pre_ok = true;
                        let p = self.in_out_arg(x, t, cx.scope);
                        self.pre_ok = false;
                        let p = p?;
                        self.may_fault = was;
                        let l = self.line(at.span);
                        self.declare_buffers(cx, l);
                        flag_outputs |= !self.in_out_backs.is_empty();
                        backs.append(&mut self.in_out_backs);
                        sets.push(format!("{lv}.{field} = {p};"));
                        bound.push(name);
                    }
                    _ => return self.not_yet(n.span, format!("{} of {}", n.text, fb.label)),
                },
                Arg::Output { not, name, target } => {
                    let Some((_, field, fty, Dir::Output)) = fb.param(&name.text).cloned() else {
                        return self.not_yet(name.span, format!("{} of {}", name.text, fb.label));
                    };
                    let v = if *not { format!("(IEC_BOOL)!{lv}.{field}") } else { format!("{lv}.{field}") };
                    // Into any variable: where it goes found before the
                    // block runs, an index checked there; widened as the
                    // checker allowed (Ed.3 6.6.1.6, rule 5), clamped
                    // into a subrange and flagged as any store.
                    let want = self.elem_of(target)?;
                    let v = self.widen(target.span, v, fty, want)?;
                    let was = std::mem::replace(&mut self.may_fault, false);
                    self.pre_ok = true;
                    let store = self.store_to(target, v, want, cx.scope, "an output taken into this");
                    self.pre_ok = false;
                    let store = store?;
                    if std::mem::replace(&mut self.may_fault, was) {
                        self.str_temps.push(crate::enable::STOP.to_string());
                    }
                    let l = self.line(at.span);
                    self.declare_buffers(cx, l);
                    flag_outputs = true;
                    gets.push(store);
                }
                Arg::Positional(x) => return self.not_yet(x.span, "a function block's inputs given in order"),
            }
        }
        if let Some(name) = fb
            .params
            .iter()
            .filter(|p| p.3 == Dir::InOut)
            .map(|p| &p.0)
            .chain(in_blocks.iter().map(|b| &b.0))
            .chain(in_data.iter().map(|b| &b.0))
            .find(|n| !bound.contains(n))
        {
            return self.not_yet(e.span, format!("a call that leaves the VAR_IN_OUT {name} unbound"));
        }
        for line in sets {
            cx.line(&line);
        }
        let saved = self.fresh("f");
        if eno.is_some() {
            cx.line(&format!("iecf_fault_t {saved} = ctx->flag;"));
            cx.line("ctx->flag.code = IECF_FAULT_NONE;");
            cx.line("ctx->flag.pou = 0u;");
            cx.line("ctx->flag.line = 0u;");
        }
        cx.line(&fb.call);
        if fb.faults {
            let l = self.line(at.span);
            self.stop(cx, l);
        }
        if eno.is_some() {
            // The block's own ENO too, when its body sets it.
            let own = match &place {
                Place::Block(_, key) if self.types.implicit_eno(key) => format!(" && {lv}.eno"),
                _ => String::new(),
            };
            cx.line(&format!("{o} = (IEC_BOOL)(ctx->flag.code == IECF_FAULT_NONE{own});"));
            cx.line(&format!("if ({saved}.code != IECF_FAULT_NONE) ctx->flag = {saved};"));
        }
        for line in backs.into_iter().chain(gets) {
            cx.line(&line);
        }
        if flag_outputs {
            let l = self.line(at.span);
            noted(cx, l);
        }
        cx.depth -= 1;
        cx.line("}");
        if let Some(store) = eno_store {
            cx.line(&store.replace("@ENO@", &o));
        }
        cx.depth -= 1;
        cx.line("}");
        Ok(true)
    }

    /// `ENO => target` or `NOT ENO => target` of a block call: the store,
    /// with `@ENO@` for the value. A BOOL variable, or an element, a field
    /// or a bit, found - its index checked - before the block runs, as an
    /// output's target is.
    fn eno_target(&mut self, not: bool, target: &Expr, at: &Stmt, cx: &mut Body) -> R<String> {
        let v = if not { "((IEC_BOOL)!@ENO@)" } else { "@ENO@" };
        if let ExprKind::Name(tid) = &target.kind
            && let Some(p) = cx.scope.vars.get(&tid.text.to_ascii_uppercase())
            && p.elem() == Some(Elem::Bool)
        {
            return Ok(p.store(v));
        }
        self.may_fault = false;
        self.pre_ok = true;
        let store = self.store_to(target, v.to_string(), Elem::Bool, cx.scope, "ENO taken into this");
        self.pre_ok = false;
        let store = store?;
        let l = self.line(at.span);
        self.declare_buffers(cx, l);
        if self.may_fault {
            self.stop(cx, l);
        }
        Ok(store)
    }
}

impl Gen<'_> {
    /// An interface's value into a fresh `iecf_itf_t` temporary, checked:
    /// a reference in it that meets NULL stops the statement here.
    pub(crate) fn checked_itf(&mut self, e: &Expr, want: &str, at: &Stmt, cx: &mut Body) -> R<String> {
        self.may_fault = false;
        self.may_flag = false;
        self.pre_ok = true;
        // A call with effects, its own line in operand order (expr.rs).
        let v = match self.calls_effects(e, cx.scope) {
            Ok(seq) => {
                self.seq = seq;
                self.itf_value(e, want, cx.scope)
            }
            Err(x) => Err(x),
        };
        self.seq = false;
        self.pre_ok = false;
        let v = v?;
        let l = self.line(at.span);
        self.declare_buffers(cx, l);
        let name = self.fresh("t");
        cx.line(&format!("iecf_itf_t {name} = {v};"));
        if self.may_fault {
            self.stop(cx, l);
        }
        if self.may_flag {
            noted(cx, l);
        }
        Ok(name)
    }

    /// `a ?= b` (Ed.3 6.6.6.7, itf.rs): an interface from an interface,
    /// a reference from an interface, a reference from a reference; `a`
    /// NULL when what `b` references is not of a type `a` can hold.
    fn assign_attempt(&mut self, target: &Expr, value: &Expr, at: &Stmt, cx: &mut Body) -> R<()> {
        let want = self.types.expr(target.span).cloned();
        let got = self.types.expr(value.span).cloned();
        let block_of = |t: &Option<Ty>| match t {
            Some(Ty::Ref(to)) => match &**to {
                Ty::Fb(k) => Some(k.clone()),
                _ => None,
            },
            _ => None,
        };
        cx.line("{");
        cx.depth += 1;
        match (&want, &got) {
            (Some(Ty::Itf(i)), Some(Ty::Itf(_) | Ty::Null)) => {
                let lv = self.itf_target(target, at, cx)?;
                let v = self.attempt_source(value, &got, at, cx)?;
                self.itfs_used.insert(i.clone());
                cx.line(&format!("{lv} = iecf_itf_as({v}, &ITY_{i});"));
            }
            (Some(Ty::Ref(_)), Some(Ty::Itf(_) | Ty::Null)) => {
                let Some(t) = block_of(&want) else { return self.not_yet(target.span, "?= into this") };
                let lv = self.ref_target(target, cx)?;
                let v = self.attempt_source(value, &got, at, cx)?;
                // No instance of a type without its own is in an interface.
                if self.carries_type(&t) {
                    cx.line(&format!("{lv} = iecf_itf_ref({v}, &TY_{t});"));
                } else {
                    cx.line(&format!("(void){v};"));
                    cx.line(&format!("{lv} = iecf_ref_null();"));
                }
            }
            (Some(Ty::Ref(_)), Some(Ty::Ref(_))) => {
                let (Some(d), Some(b)) = (block_of(&want), block_of(&got)) else {
                    return self.not_yet(target.span, "?= into this");
                };
                let lv = self.ref_target(target, cx)?;
                let v = self.checked_reference(value, at, cx)?;
                // Up its line it always holds; down, if the instance is.
                if self.types.derives(&b, &d) {
                    cx.line(&format!("{lv} = {v};"));
                } else {
                    cx.line(&format!("{lv} = iecf_ref_as({v}, &TY_{d});"));
                }
            }
            _ => return self.not_yet(at.span, "this ?="),
        }
        cx.depth -= 1;
        cx.line("}");
        Ok(())
    }

    /// The interface variable, element or field `?=` writes, its C
    /// lvalue, an index checked first.
    fn itf_target(&mut self, target: &Expr, at: &Stmt, cx: &mut Body) -> R<String> {
        match self.target(target, at, cx)? {
            (Lv::C(c), DTy { kind: Kind::Itf(_), .. }) => Ok(c),
            _ => self.not_yet(target.span, "?= into this"),
        }
    }

    /// The reference variable `?=` writes, its C lvalue.
    fn ref_target(&mut self, target: &Expr, cx: &mut Body) -> R<String> {
        self.may_fault = false;
        let found = self.data_path(target, cx.scope)?;
        match found {
            Some((Lv::C(c), DTy { kind: Kind::Ref(..), .. })) if !self.may_fault => Ok(c),
            _ => self.not_yet(target.span, "?= into this"),
        }
    }

    /// What `?=` tries, an interface or NULL, into a temporary.
    fn attempt_source(&mut self, value: &Expr, got: &Option<Ty>, at: &Stmt, cx: &mut Body) -> R<String> {
        match got {
            Some(Ty::Itf(j)) => {
                let j = j.clone();
                self.checked_itf(value, &j, at, cx)
            }
            _ => Ok("iecf_itf_null()".into()),
        }
    }
}

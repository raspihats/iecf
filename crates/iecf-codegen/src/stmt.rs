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
        }
        self.declare_buffers(cx, l);
        let name = self.fresh("t");
        cx.line(&format!("{} {name} = {v};", c_type(t)));
        if self.may_fault {
            stop(cx, l);
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
        let v = self.ref_value(e, cx.scope);
        self.pre_ok = false;
        let v = v?;
        let l = self.line(at.span);
        self.declare_buffers(cx, l);
        let name = self.fresh("t");
        cx.line(&format!("iecf_ref_t {name} = {v};"));
        if self.may_fault {
            stop(cx, l);
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
        let Some((r, to)) = self.ref_of(base, cx.scope)? else {
            return self.not_yet(base.span, "dereferencing this");
        };
        let chk = self.checked_ref(&r, &to);
        let t = self.fresh("r");
        cx.line(&format!("iecf_ref_t {t} = {chk};"));
        let l = self.line(at.span);
        stop(cx, l);
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
        for d in std::mem::take(&mut self.str_temps) {
            match d.trim_start().strip_prefix(crate::enable::STOP) {
                Some(_) => {
                    let indent = &d[..d.len() - d.trim_start().len()];
                    let r = cx.ret.clone();
                    cx.line(&format!("{indent}if (iecf_stopped(&ctx->fault, {}u, {line}u)) {r}", cx.pou));
                }
                None => cx.line(&d),
            }
        }
    }

    fn stmt(&mut self, s: &Stmt, cx: &mut Body) -> R<()> {
        self.str_temps.clear();
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
                        let (lv, t) = self.target(target, s, cx)?;
                        self.assign_data(&lv, &t, value, s, cx)?;
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
                let t = match self.ty_of(selector)? {
                    Ty::Elem(t) if t.is_int() || t.is_bit() => t,
                    _ => return self.not_yet(selector.span, "a CASE on this selector"),
                };
                cx.line("{");
                cx.depth += 1;
                let Val { v: sel, keep } = self.checked(selector, t, s, cx)?;
                if let Some(k) = &keep {
                    cx.line(&format!("if (!{k}) {{"));
                    cx.depth += 1;
                }
                let mut first = true;
                for arm in arms {
                    let mut tests = Vec::new();
                    for label in &arm.labels {
                        let value = |g: &mut Self, e: &Expr| -> R<Option<String>> {
                            match const_int(e).or_else(|| g.types.const_value(e.span)) {
                                Some(v) => Ok(lit(v, t)),
                                None => match g.enum_const(e)? {
                                    Some(v) => Ok(Some(v)),
                                    None => g.not_yet(e.span, "a CASE label that is not a literal"),
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
                cx.line(&format!(
                    "for ({lv} = {start}, {done} = iecf_for_done_{n}({lv}, {end}, {step}); !{done}; {done} = {next}) {{"
                ));
                cx.depth += 1;
                cx.loops.push(Loop::Plain);
                let r = self.stmts(body, cx);
                cx.loops.pop();
                r?;
                cx.depth -= 1;
                cx.line("}");
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
                    stop(cx, line);
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
    fn target(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<(Lv, DTy)> {
        match &e.kind {
            ExprKind::Paren(x) => self.target(x, at, cx),
            ExprKind::Name(_) => match self.data_path(e, cx.scope)? {
                Some(p) => Ok(p),
                None => self.not_yet(e.span, "assigning to this"),
            },
            // An input of an instance set from outside (Ed.3 6.6.7.2.2,
            // Figure 20 a), or a PUBLIC variable (6.6.5.2 rule 7): its
            // field, which the next call reads.
            ExprKind::Member(base, m) if let Some(Place::Block(lv, key)) = self.fb_place(base, cx.scope)? => {
                let k = m.text.to_ascii_uppercase();
                let Some(sig) = self.blocks.get(&key) else { return self.not_yet(e.span, "assigning to this") };
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
                    self.may_fault = false;
                    self.may_flag = false;
                    let off = match (b, &lv) {
                        (Some((lo, hi)), _) => self.index(i, lo, hi, cx.scope)?,
                        (None, Lv::C(d)) => self.flex_index(i, d, k, cx.scope)?,
                        (None, _) => return self.not_yet(e.span, "this index"),
                    };
                    if self.may_fault || self.may_flag {
                        let o = self.fresh("o");
                        cx.line(&format!("uint32_t {o} = {off};"));
                        let l = self.line(at.span);
                        if self.may_fault {
                            stop(cx, l);
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
            ExprKind::Deref(base) => {
                let (t, to) = self.deref_target(base, at, cx)?;
                match &to {
                    RefTo::Data(d) => Ok((self.data_at(&t, d), d.clone())),
                    _ => self
                        .not_yet(e.span, "assigning to a function block instance, or its input, through a reference"),
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
            ExprKind::Call { callee, args } => {
                self.may_fault = false;
                self.may_flag = false;
                self.pre_ok = true;
                let seq = self.calls_effects(e, cx.scope);
                self.seq = seq.unwrap_or(false);
                let r = self.user_call(e, callee, args, cx.scope);
                self.seq = false;
                self.pre_ok = false;
                let r = r?;
                let Some(t) = self.called_sig(e).and_then(|s| s.ret_data) else {
                    return self.not_yet(e.span, "this value of an array or structure");
                };
                let l = self.line(at.span);
                self.declare_buffers(cx, l);
                if self.may_fault {
                    stop(cx, l);
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
            cx.line(&format!("{c} = {v};"));
            return Ok(());
        }
        // An interface: NULL, an instance or an interface (itf.rs).
        if let Kind::Itf(i) = &t.kind {
            let v = self.checked_itf(value, i, at, cx)?;
            let Some(c) = lv.c() else { return self.not_yet(value.span, "storing an interface here") };
            cx.line(&format!("{c} = {v};"));
            return Ok(());
        }
        if let Some(e) = t.scalar() {
            let v = self.checked(value, e, at, cx)?;
            self.clamped(&v.v, e, t.range, at, cx);
            let store = lv.store(e, &v.v);
            match &v.keep {
                Some(k) => cx.line(&format!("if (!{k}) {store}")),
                None => cx.line(&store),
            }
            return Ok(());
        }
        if let Kind::Str(n, w) = t.kind {
            return self.assign_string(lv, n, w, value, at, cx);
        }
        let (src, _) = self.source(value, at, cx)?;
        let copy = self.copy_data(lv, &src, t, value.span)?;
        cx.line(&copy);
        Ok(())
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
        }
        self.declare_buffers(cx, l);
        let s = self.fresh("s");
        cx.line(&format!("const {} *{s} = {src};", crate::strings::unit(w)));
        if self.may_fault {
            stop(cx, l);
        }
        let copy = format!("iecf_{}scopy(&ctx->flag, @, {n}u, {s});", crate::strings::infix(w));
        let copy = crate::strings::store_string(lv, n, w, &copy);
        match &keep {
            Some(k) => cx.line(&format!("if (!{k}) {copy}")),
            None => cx.line(&copy),
        }
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

/// After a statement that can fault: leave if it did.
fn stop(cx: &mut Body, line: u32) {
    let r = cx.ret.clone();
    cx.line(&format!("if (iecf_stopped(&ctx->fault, {}u, {line}u)) {r}", cx.pou));
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
            }
            self.declare_buffers(cx, l);
            let s = self.fresh("s");
            cx.line(&format!("const {} *{s} = {src};", crate::strings::unit(w)));
            if self.may_fault {
                stop(cx, l);
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
            stop(cx, l);
            return Ok(true);
        }
        // Through a reference: the reference checked first, so NULL stops
        // the statement before the block runs on nothing.
        if let ExprKind::Deref(base) = &callee.kind
            && let Some((_, RefTo::Block(_) | RefTo::Std(_))) = self.ref_of(base, cx.scope)?
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
                stop(cx, l);
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
            Some((not, target)) => Some(self.eno_target(not, target, &fb.label, cx)?),
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
                    stop(cx, l);
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
                let p = self.data_inout(x, &t, cx.scope)?;
                if std::mem::replace(&mut self.may_fault, was) {
                    return self.not_yet(x.span, "an in-out given an element by a computed index");
                }
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
                    Some((name, field, t, Dir::InOut)) => {
                        let target = match &x.kind {
                            ExprKind::Name(tid) => cx.scope.vars.get(&tid.text.to_ascii_uppercase()).cloned(),
                            _ => None,
                        };
                        let copied = |p: &Place| {
                            matches!(p, Place::Located { .. }) || matches!(p, Place::Data(_, dt) if dt.range.is_some())
                        };
                        match target {
                            // At an address, or of a subrange: given a copy,
                            // stored back with the outputs (expr.rs
                            // `in_out_copy`).
                            Some(p) if p.elem() == Some(t) && copied(&p) => {
                                let q = self.fresh("q");
                                cx.line(&format!("{} {q} = {};", c_type(t), p.load()));
                                sets.push(format!("{lv}.{field} = &{q};"));
                                let back = match &p {
                                    Place::Data(_, dt) => {
                                        flag_outputs = true;
                                        let c = self.clamp(&q, t, dt.range);
                                        p.store(&c)
                                    }
                                    _ => p.store(&q),
                                };
                                backs.push(back);
                                bound.push(name);
                            }
                            Some(p) if p.elem() == Some(t) && p.lvalue().is_some() => {
                                sets.push(format!("{lv}.{field} = &({});", p.lvalue().unwrap_or_default()));
                                bound.push(name);
                            }
                            _ => {
                                return self.not_yet(x.span, "this VAR_IN_OUT argument (a variable not at an address)");
                            }
                        }
                    }
                    _ => return self.not_yet(n.span, format!("{} of {}", n.text, fb.label)),
                },
                Arg::Output { not, name, target } => {
                    let Some((_, field, fty, Dir::Output)) = fb.param(&name.text).cloned() else {
                        return self.not_yet(name.span, format!("{} of {}", name.text, fb.label));
                    };
                    let ExprKind::Name(tid) = &target.kind else {
                        return self.not_yet(target.span, "an output taken into this");
                    };
                    let Some(p) = cx.scope.vars.get(&tid.text.to_ascii_uppercase()).cloned() else {
                        return self.not_yet(tid.span, format!("'{}' here", tid.text));
                    };
                    let Some(want) = p.elem() else {
                        return self.not_yet(target.span, "an output taken into this");
                    };
                    let v = if *not { format!("(IEC_BOOL)!{lv}.{field}") } else { format!("{lv}.{field}") };
                    // Widened as the checker allowed (Ed.3 6.6.1.6, rule 5).
                    let v = self.widen(target.span, v, fty, want)?;
                    let v = match &p {
                        Place::Data(_, dt) if dt.range.is_some() => {
                            let c = self.clamp(&v, want, dt.range);
                            flag_outputs = true;
                            c
                        }
                        _ => v,
                    };
                    gets.push(p.store(&v));
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
            stop(cx, l);
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
    /// with `@ENO@` for the value. The target is a BOOL variable.
    fn eno_target(&mut self, not: bool, target: &Expr, label: &str, cx: &Body) -> R<String> {
        let v = if not { "((IEC_BOOL)!@ENO@)" } else { "@ENO@" };
        if let ExprKind::Name(tid) = &target.kind
            && let Some(p) = cx.scope.vars.get(&tid.text.to_ascii_uppercase())
            && p.elem() == Some(Elem::Bool)
        {
            return Ok(p.store(v));
        }
        self.not_yet(target.span, format!("ENO of {label} taken into this"))
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
            stop(cx, l);
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

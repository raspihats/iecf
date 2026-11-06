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

use crate::derived::{DTy, Kind};
use crate::expr::{const_int, lit};
use crate::names::{Place, Scope};
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

    /// `e` as `t` into a fresh temporary, checked.
    fn checked(&mut self, e: &Expr, t: Elem, at: &Stmt, cx: &mut Body) -> R<Val> {
        self.may_fault = false;
        self.may_flag = false;
        self.keep_ok = true;
        self.keep = None;
        let v = self.value_as(e, t, cx.scope);
        self.keep_ok = false;
        let keep = self.keep.take();
        let v = v?;
        self.declare_buffers(cx);
        if let Some(k) = &keep {
            cx.line(&format!("int {k} = 0;"));
        }
        let name = self.fresh("t");
        cx.line(&format!("{} {name} = {v};", c_type(t)));
        let l = self.line(at.span);
        if self.may_fault {
            stop(cx, l);
        }
        if self.may_flag {
            noted(cx, l);
        }
        Ok(Val { v: name, keep })
    }

    fn condition(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<Val> {
        self.checked(e, Elem::Bool, at, cx)
    }

    /// The string buffers the expression just written uses (strings.rs).
    fn declare_buffers(&mut self, cx: &mut Body) {
        for d in std::mem::take(&mut self.str_temps) {
            cx.line(&d);
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
                let (lv, t) = self.target(target, s, cx)?;
                self.assign_data(&lv, &t, value, s, cx)?;
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
                let Some(ty) = place.elem() else {
                    return self.not_yet(target.span, "assigning a whole function block instance");
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
                            match const_int(e) {
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
                // its end the bound it would pass (runtime/iecf.h).
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
                let r = cx.ret.clone();
                cx.line(&r);
            }
            StmtKind::Empty => {}
            StmtKind::Call(e) if self.fb_call(e, s, cx)? => {}
            StmtKind::Call(e) => {
                cx.line("{");
                cx.depth += 1;
                self.may_fault = false;
                self.may_flag = false;
                let v = self.value(e, cx.scope)?;
                self.declare_buffers(cx);
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
    /// An element or field assigned to: its C lvalue, each index that is
    /// not a constant computed and checked first, so a statement that
    /// faults stores nothing.
    fn target(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<(String, DTy)> {
        match &e.kind {
            ExprKind::Paren(x) => self.target(x, at, cx),
            ExprKind::Name(_) => match self.data_path(e, cx.scope)? {
                Some(p) => Ok(p),
                None => self.not_yet(e.span, "assigning to this"),
            },
            ExprKind::Member(base, m) => {
                let (lv, t) = self.target(base, at, cx)?;
                let Kind::Struct(k) = &t.kind else { return self.not_yet(m.span, "this member") };
                let key = m.text.to_ascii_uppercase();
                match self.structs.get(k).and_then(|fs| fs.iter().find(|(n, _)| *n == key)) {
                    Some((_, ft)) => Ok((format!("{lv}.V_{key}"), ft.clone())),
                    None => self.not_yet(m.span, format!("the field {}", m.text)),
                }
            }
            ExprKind::Index(base, idx) => {
                let (mut lv, t) = self.target(base, at, cx)?;
                let Kind::Array(elem, bounds) = &t.kind else { return self.not_yet(e.span, "this index") };
                if idx.len() != bounds.len() {
                    return self.not_yet(e.span, "an index of another count of dimensions");
                }
                for (i, (lo, hi)) in idx.iter().zip(bounds) {
                    self.may_fault = false;
                    self.may_flag = false;
                    let off = self.index(i, *lo, *hi, cx.scope)?;
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
                        lv = format!("{lv}[{o}]");
                    } else {
                        lv = format!("{lv}[{off}]");
                    }
                }
                Ok((lv, (**elem).clone()))
            }
            _ => self.not_yet(e.span, "assigning to this"),
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
    fn assign_data(&mut self, lv: &str, t: &DTy, value: &Expr, at: &Stmt, cx: &mut Body) -> R<()> {
        if let Some(e) = t.scalar() {
            let v = self.checked(value, e, at, cx)?;
            self.clamped(&v.v, e, t.range, at, cx);
            match &v.keep {
                Some(k) => cx.line(&format!("if (!{k}) {lv} = {};", v.v)),
                None => cx.line(&format!("{lv} = {};", v.v)),
            }
            return Ok(());
        }
        if let Kind::Str(n) = t.kind {
            return self.assign_string(lv, n, value, at, cx);
        }
        self.may_fault = false;
        let Some((src, _)) = self.data_path(value, cx.scope)? else {
            return self.not_yet(value.span, "this value of an array or structure");
        };
        let s = self.fresh("s");
        cx.line(&format!("const void *{s} = &({src});"));
        if self.may_fault {
            let l = self.line(at.span);
            stop(cx, l);
        }
        cx.line(&format!("iecf_bytes(&({lv}), {s}, sizeof ({lv}));"));
        Ok(())
    }
}

impl Gen<'_> {
    /// `lv := value` for a `STRING(n)`: the value computed and checked,
    /// then copied, cut to `n` characters if longer (runtime/iecf.h).
    fn assign_string(&mut self, lv: &str, n: u32, value: &Expr, at: &Stmt, cx: &mut Body) -> R<()> {
        if !self.is_string(value)? {
            return self.not_yet(value.span, "this value of a STRING");
        }
        self.may_fault = false;
        self.may_flag = false;
        self.keep_ok = true;
        self.keep = None;
        let v = self.string(value, cx.scope);
        self.keep_ok = false;
        let keep = self.keep.take();
        let (src, _) = v?;
        self.declare_buffers(cx);
        if let Some(k) = &keep {
            cx.line(&format!("int {k} = 0;"));
        }
        let s = self.fresh("s");
        cx.line(&format!("const char *{s} = {src};"));
        let l = self.line(at.span);
        if self.may_fault {
            stop(cx, l);
        }
        let copy = format!("iecf_scopy(&ctx->flag, {lv}, {n}u, {s});");
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
                    .map(|(n, t, d)| (n.clone(), format!("V_{n}"), *t, *d))
                    .collect(),
                call: format!("B_{key}(ctx, &{lv});"),
                faults: true,
                ranges: self.blocks.get(key)?.ranges.clone(),
                std: None,
            }),
            _ => None,
        }
    }

    /// `t1(IN := x, PT := T#1s, Q => y);` for a function block instance:
    /// its inputs set and in-outs bound, the block called, its outputs
    /// taken. `false` if `e` is not such a call.
    fn fb_call(&mut self, e: &Expr, at: &Stmt, cx: &mut Body) -> R<bool> {
        let ExprKind::Call { callee, args } = &e.kind else { return Ok(false) };
        let ExprKind::Name(id) = &callee.kind else { return Ok(false) };
        let Some(place) = cx.scope.vars.get(&id.text.to_ascii_uppercase()).cloned() else { return Ok(false) };
        let Some(fb) = self.iface(&place) else { return Ok(false) };
        let lv = fb.lv.clone();
        cx.line("{");
        cx.depth += 1;
        let mut sets = Vec::new();
        let mut gets = Vec::new();
        let mut bound: Vec<String> = Vec::new();
        let mut flag_outputs = false;
        for a in args {
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
                        match target {
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
                    if p.elem() != Some(fty) {
                        return self.not_yet(target.span, "an output taken into another type");
                    }
                    let v = if *not { format!("(IEC_BOOL)!{lv}.{field}") } else { format!("{lv}.{field}") };
                    let v = match &p {
                        Place::Data(_, dt) if dt.range.is_some() => {
                            let c = self.clamp(&v, fty, dt.range);
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
        if let Some((name, ..)) = fb.params.iter().find(|p| p.3 == Dir::InOut && !bound.contains(&p.0)) {
            return self.not_yet(e.span, format!("a call that leaves the VAR_IN_OUT {name} unbound"));
        }
        for line in sets {
            cx.line(&line);
        }
        cx.line(&fb.call);
        if fb.faults {
            let l = self.line(at.span);
            stop(cx, l);
        }
        for line in gets {
            cx.line(&line);
        }
        if flag_outputs {
            let l = self.line(at.span);
            noted(cx, l);
        }
        cx.depth -= 1;
        cx.line("}");
        Ok(true)
    }
}

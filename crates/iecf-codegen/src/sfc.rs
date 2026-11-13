//! SFC (Ed.3 6.7) in a PROGRAM or FUNCTION_BLOCK, decided 2026-11-07
//! (florin; docs/dependencies.md):
//!
//! - each scan runs the actions of the active steps first, then tests
//!   their transitions, as CODESYS and TwinCAT do: a step entered in this
//!   scan runs its actions from the next one;
//! - with "final scan" logic (Table 60 feature 1): an action runs once
//!   more, with its Q FALSE, after Q falls; those ending run before those
//!   whose Q is TRUE, as CODESYS's two passes;
//! - of the transitions out of the same active step, the first true one
//!   in priority order is cleared - a PRIORITY's lowest number first,
//!   then source order (Table 61 2a, 2b); transitions out of different
//!   steps clear together;
//! - a step entered while still active ("unsafe", Figure 26 a) stays
//!   active once and is flagged IECF_FAULT_SFC.
//!
//! The instance holds an `iecf_step_t` per step (`S_<NAME>`) and an
//! `iecf_action_t` per action (`A_<NAME>`), runtime/iecf.h.

use crate::names::{Place, Scope};
use crate::stmt::Body;
use crate::{Gen, R};
use iecf_diagnostics::Span;
use iecf_sema::types::Elem;
use iecf_syntax::ast::*;
use std::fmt::Write as _;

/// An action and the step associations that drive it.
pub(crate) struct Act<'a> {
    key: String,
    /// The statements of an ACTION ... END_ACTION.
    body: Option<&'a [Stmt]>,
    /// A BOOL variable used as an action: it holds Q.
    var: Option<Place>,
    /// (step, qualifier bit, duration, where).
    assocs: Vec<(String, &'static str, Option<&'a Expr>, Span)>,
}

/// A POU's chart, as the body is written from it.
pub(crate) struct Chart<'a> {
    owner: String,
    steps: Vec<String>,
    actions: Vec<Act<'a>>,
    transitions: Vec<&'a Transition>,
}

fn qualifier(q: Option<&Ident>) -> Option<(&'static str, bool)> {
    let name = q.map_or("N".to_string(), |q| q.text.to_ascii_uppercase());
    // (the IECF_Q_ bit, whether it takes a duration)
    Some(match name.as_str() {
        "N" => ("IECF_Q_N", false),
        "R" => ("IECF_Q_R", false),
        "S" => ("IECF_Q_S", false),
        "L" => ("IECF_Q_L", true),
        "D" => ("IECF_Q_D", true),
        "P" => ("IECF_Q_P", false),
        "P1" => ("IECF_Q_P1", false),
        "P0" => ("IECF_Q_P0", false),
        "SD" => ("IECF_Q_SD", true),
        "DS" => ("IECF_Q_DS", true),
        "SL" => ("IECF_Q_SL", true),
        _ => return None,
    })
}

impl<'a> Gen<'a> {
    /// The chart's fields and initialisation, and `S.X`, `S.T`, `A.Q` in
    /// the scope; `guard` wraps the initialisation (a FUNCTION_BLOCK's
    /// `keep != 2`: an instance kept whole keeps its steps, Ed.3 6.7.2).
    pub(crate) fn chart(
        &mut self,
        pou: &'a Pou,
        owner: &str,
        guard: Option<&str>,
        fields: &mut String,
        init: &mut String,
        scope: &mut Scope,
    ) -> R<Option<Chart<'a>>> {
        if pou.sfc.is_empty() {
            return Ok(None);
        }
        if !pou.body.is_empty() {
            return self.not_yet(pou.name.span, "statements beside an SFC");
        }
        let mut chart =
            Chart { owner: owner.to_string(), steps: Vec::new(), actions: Vec::new(), transitions: Vec::new() };
        let mut inits = String::new();
        for el in &pou.sfc {
            match el {
                SfcElement::Step(s) => {
                    let k = s.name.text.to_ascii_uppercase();
                    let _ = writeln!(fields, "    iecf_step_t S_{k};");
                    let _ = writeln!(inits, "    iecf_step_init(&{owner}S_{k}, {});", if s.initial { 1 } else { 0 });
                    scope.vars.insert(format!("{k}.X"), Place::Field(format!("{owner}S_{k}.X"), Elem::Bool));
                    scope.vars.insert(format!("{k}.T"), Place::Field(format!("{owner}S_{k}.T"), Elem::Time));
                    chart.steps.push(k);
                }
                SfcElement::Action(a) => chart.actions.push(Act {
                    key: a.name.text.to_ascii_uppercase(),
                    body: Some(&a.body),
                    var: None,
                    assocs: Vec::new(),
                }),
                SfcElement::Transition(t) => chart.transitions.push(t),
            }
        }
        // Each association, in source order; an action no ACTION declares
        // is a BOOL variable (6.7.4.1).
        for el in &pou.sfc {
            let SfcElement::Step(s) = el else { continue };
            let step = s.name.text.to_ascii_uppercase();
            for a in &s.actions {
                // An indicator variable does nothing of its own (Ed.3
                // 6.7.4.2): the action sets it, as any variable.
                let Some((bit, timed)) = qualifier(a.qualifier.as_ref()) else {
                    return self.not_yet(a.span, "this action qualifier");
                };
                if timed != a.time.is_some() {
                    return self.not_yet(a.span, "this qualifier and duration");
                }
                let key = a.action.text.to_ascii_uppercase();
                if !chart.actions.iter().any(|x| x.key == key) {
                    match scope.vars.get(&key) {
                        Some(p) if p.elem() == Some(Elem::Bool) && !matches!(p, Place::Data(..)) => chart
                            .actions
                            .push(Act { key: key.clone(), body: None, var: Some(p.clone()), assocs: Vec::new() }),
                        _ => return self.not_yet(a.action.span, "this action"),
                    }
                }
                if let Some(x) = chart.actions.iter_mut().find(|x| x.key == key) {
                    x.assocs.push((step.clone(), bit, a.time.as_ref(), a.span));
                }
            }
        }
        for a in &chart.actions {
            let k = &a.key;
            let _ = writeln!(fields, "    iecf_action_t A_{k};");
            let _ = writeln!(inits, "    iecf_action_init(&{owner}A_{k});");
            if a.body.is_some() {
                scope.vars.insert(format!("{k}.Q"), Place::Field(format!("{owner}A_{k}.Q"), Elem::Bool));
            }
        }
        // Priority: the lowest PRIORITY first, then source order.
        let mut keyed = Vec::new();
        for (i, t) in chart.transitions.iter().enumerate() {
            let p = match &t.priority {
                Some(e) => match crate::expr::const_int(e) {
                    Some(v) => v,
                    None => return self.not_yet(e.span, "this PRIORITY"),
                },
                None => i128::MAX,
            };
            keyed.push((p, i, *t));
        }
        keyed.sort_by_key(|(p, i, _)| (*p, *i));
        chart.transitions = keyed.into_iter().map(|(_, _, t)| t).collect();
        match guard {
            Some(g) => {
                let _ = writeln!(init, "    if ({g}) {{");
                for l in inits.lines() {
                    let _ = writeln!(init, "    {l}");
                }
                let _ = writeln!(init, "    }}");
            }
            None => init.push_str(&inits),
        }
        Ok(Some(chart))
    }

    /// One scan of the chart: step times, action control, the actions,
    /// then the transitions.
    pub(crate) fn chart_scan(&mut self, chart: &Chart<'a>, cx: &mut Body) -> R<()> {
        let o = chart.owner.clone();
        cx.line("/* SFC: step times */");
        for s in &chart.steps {
            cx.line(&format!("iecf_step_time(&{o}S_{s}, ctx->now_ns);"));
        }
        cx.line("/* SFC: action control (ACTION_CONTROL, final scan) */");
        for a in &chart.actions {
            cx.line("{");
            cx.depth += 1;
            cx.line("unsigned q = 0u;");
            cx.line("IEC_TIME t = 0;");
            let timed = a.assocs.iter().filter(|x| x.2.is_some()).count();
            if timed > 1 {
                cx.line("int timed = 0;");
            }
            for (step, bit, time, _) in &a.assocs {
                match time {
                    Some(e) => {
                        self.may_fault = false;
                        self.may_flag = false;
                        let v = self.value_as(e, Elem::Time, cx.scope)?;
                        if self.may_fault || self.may_flag {
                            return self.not_yet(e.span, "a duration that can fault or be flagged");
                        }
                        // Two time-related associations active: Ed.3
                        // 6.7.4.6 d, flagged; the last one's duration.
                        let twice = if timed > 1 { " if (timed++) iecf_flag(&ctx->flag, IECF_FAULT_SFC);" } else { "" };
                        cx.line(&format!("if ({o}S_{step}.X) {{ q |= {bit}; t = {v};{twice} }}"));
                    }
                    None => cx.line(&format!("if ({o}S_{step}.X) q |= {bit};")),
                }
            }
            let k = &a.key;
            cx.line(&format!("iecf_action(&ctx->flag, &{o}A_{k}, q, t, ctx->now_ns);"));
            let line = a.assocs.first().map_or(0, |x| self.line(x.3));
            cx.line(&format!("iecf_noted(&ctx->flag, {}u, {line}u);", cx.pou));
            cx.depth -= 1;
            cx.line("}");
        }
        cx.line("/* SFC: actions; BOOL variables take Q */");
        for a in &chart.actions {
            if let Some(p) = &a.var {
                cx.line(&p.store(&format!("{o}A_{}.Q", a.key)));
            }
        }
        // Two passes, as CODESYS and TwinCAT: first the actions in their
        // final scan or a P1/P0 pulse (Q FALSE), then those whose Q is
        // TRUE, each in source order - an active step's action has the
        // last word over one that is ending.
        if chart.actions.iter().any(|a| a.body.is_some()) {
            let pass = self.fresh("pass");
            cx.line(&format!("for (int {pass} = 0; {pass} < 2; {pass}++) {{"));
            cx.depth += 1;
            for a in &chart.actions {
                let k = &a.key;
                let Some(body) = a.body else { continue };
                // RETURN ends the action; a fault still ends the POU.
                let label = self.fresh("end");
                cx.line(&format!("if ({o}A_{k}.A && {o}A_{k}.Q == {pass}) {{ /* ACTION {k} */"));
                cx.depth += 1;
                let saved = cx.on_return.replace(format!("goto {label};"));
                let r = self.stmts(body, cx);
                cx.on_return = saved;
                r?;
                cx.depth -= 1;
                cx.line("}");
                // Only where a RETURN jumps to it: an unused label warns.
                if cx.out.contains(&format!("goto {label};")) {
                    cx.line(&format!("{label}:;"));
                }
            }
            cx.depth -= 1;
            cx.line("}");
        }
        cx.line("/* SFC: transitions, against the steps active when the scan began */");
        cx.line("{");
        cx.depth += 1;
        // Each step a transition leaves.
        for s in &chart.steps {
            if chart.transitions.iter().any(|t| t.from.iter().any(|f| f.is(s))) {
                cx.line(&format!("IEC_BOOL e_{s} = {o}S_{s}.X;"));
            }
        }
        let mut cleared = Vec::new();
        for t in &chart.transitions {
            let c = self.fresh("c");
            cx.line(&format!("IEC_BOOL {c} = 0;"));
            let from: Vec<String> = t.from.iter().map(|s| s.text.to_ascii_uppercase()).collect();
            let to: Vec<String> = t.to.iter().map(|s| s.text.to_ascii_uppercase()).collect();
            let enabled: Vec<String> = from.iter().map(|s| format!("e_{s}")).collect();
            cx.line(&format!("if ({}) {{", enabled.join(" && ")));
            cx.depth += 1;
            let at = Stmt { kind: StmtKind::Empty, span: t.condition.span };
            let v = self.cond(&t.condition, &at, cx)?;
            cx.line(&format!("if ({v}) {{"));
            cx.depth += 1;
            cx.line(&format!("{c} = 1;"));
            // A step's first true transition takes it; the others wait.
            for s in &from {
                cx.line(&format!("e_{s} = 0;"));
            }
            cx.depth -= 1;
            cx.line("}");
            cx.depth -= 1;
            cx.line("}");
            cleared.push((c, from, to, self.line(t.span)));
        }
        for (c, from, _, _) in &cleared {
            for s in from {
                cx.line(&format!("if ({c}) iecf_step_leave(&{o}S_{s});"));
            }
        }
        for (c, _, to, line) in &cleared {
            for s in to {
                cx.line(&format!("if ({c}) iecf_step_enter(&ctx->flag, &{o}S_{s}, ctx->now_ns);"));
            }
            cx.line(&format!("iecf_noted(&ctx->flag, {}u, {line}u);", cx.pou));
        }
        cx.depth -= 1;
        cx.line("}");
        Ok(())
    }
}

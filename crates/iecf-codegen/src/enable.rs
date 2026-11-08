//! EN and ENO on a function call (Ed.3 6.6.1.5), decided 2026-11-07
//! (florin; docs/dependencies.md):
//!
//! - EN = FALSE: Ed.3's "external implementation" (its Example 2): the
//!   inputs are not evaluated, the function is not called, ENO is FALSE,
//!   and the statement the call is in is left undone, unflagged - an
//!   assignment keeps its target's value, as a real that does not fit an
//!   integer leaves it (stmt.rs).
//! - ENO is FALSE when the function's own operation raised a flag; a
//!   flag raised by an argument is not the function's.
//!
//! The call is written before its statement, through the statement's
//! buffers (stmt.rs, `declare_buffers`): EN computed, then - if it is
//! TRUE - each argument into a temporary, then the call alone with
//! `ctx->flag` cleared, so what it holds afterwards is the call's own;
//! the flag the scan had before is put back. The statement then uses the
//! call's temporary.

use crate::names::{Place, Scope};
use crate::{Gen, R, c_type};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;

/// What stands in for an argument already computed (`Gen::subst`).
#[derive(Clone)]
pub(crate) enum Sub {
    Val(String),
    Str(String, u32),
}

/// A line written before the statement that stops it there when a fault
/// was recorded (stmt.rs fills in the POU and line).
pub(crate) const STOP: &str = "@STOP@";

impl Gen<'_> {
    /// `e`, a call with EN or ENO among `args`: the temporary holding its
    /// result (and, for a STRING, its most characters), the call written
    /// before the statement. `None` when it has neither.
    pub(crate) fn gated(&mut self, e: &Expr, args: &[Arg], scope: &Scope, string: bool) -> R<Option<(String, u32)>> {
        let named = |a: &Arg, n: &str| match a {
            Arg::Input(id, _) | Arg::Output { name: id, .. } => id.is(n),
            _ => false,
        };
        let en = args.iter().find_map(|a| match a {
            Arg::Input(id, x) if id.is("EN") => Some(x),
            _ => None,
        });
        let eno = args.iter().find_map(|a| match a {
            Arg::Output { name, not, target } if name.is("ENO") => Some((*not, target)),
            _ => None,
        });
        if en.is_none() && eno.is_none() {
            return Ok(None);
        }
        if !self.pre_ok {
            return self.not_yet(e.span, "EN or ENO here");
        }
        // One such call in another's arguments would be written before the
        // outer `if (EN)`, and run whatever EN is: refused, not yet.
        self.pre_ok = false;
        let rest: Vec<Arg> = args.iter().filter(|a| !named(a, "EN") && !named(a, "ENO")).cloned().collect();
        let (outer_fault, outer_flag) = (self.may_fault, self.may_flag);
        let mut pre = Vec::new();

        // EN, evaluated first, whatever it is.
        let en_code = match en {
            Some(x) => {
                self.may_fault = false;
                let c = self.value_as(x, Elem::Bool, scope)?;
                let n = self.fresh("en");
                pre.push(format!("IEC_BOOL {n} = {c};"));
                if self.may_fault {
                    pre.push(STOP.to_string());
                }
                Some(n)
            }
            None => None,
        };
        // EN = FALSE leaves the statement undone, where one can be.
        let keep = match (&en_code, self.keep_ok) {
            (Some(_), true) => Some(match &self.keep {
                Some(k) => k.clone(),
                None => {
                    let k = self.fresh("k");
                    self.keep = Some(k.clone());
                    k
                }
            }),
            _ => None,
        };

        // Inside `if (EN)`: the arguments, then the call alone.
        let mut body = Vec::new();
        let mut added = Vec::new();
        if eno.is_some() {
            self.may_fault = false;
            for a in &rest {
                let x = match a {
                    Arg::Positional(x) | Arg::Input(_, x) => x,
                    Arg::Output { .. } => continue,
                };
                match self.ty_of(x)? {
                    Ty::Elem(t) if crate::expr::held(t) => {
                        let c = self.value(x, scope)?;
                        let n = self.fresh("a");
                        body.push(format!("{} {n} = {c};", c_type(t)));
                        self.subst.insert(x.span, Sub::Val(n));
                        added.push(x.span);
                    }
                    Ty::Str { wide: false, .. } => {
                        let (c, len) = self.string(x, scope)?;
                        let n = self.fresh("a");
                        body.push(format!("const char *{n} = {c};"));
                        self.subst.insert(x.span, Sub::Str(n, len));
                        added.push(x.span);
                    }
                    // Literals: nothing to flag or fault.
                    _ => {}
                }
            }
            if self.may_fault {
                body.push(STOP.to_string());
            }
        }
        self.may_fault = false;
        // A user FUNCTION that sets ENO in its body writes it here.
        let body_eno = self.fresh("e");
        self.eno_used = false;
        self.want_eno = if eno.is_some() { Some(body_eno.clone()) } else { None };
        self.split_ok = !string;
        let call = if string {
            self.string_of_call(e, &rest, scope)
        } else {
            match &e.kind {
                ExprKind::Call { callee, .. } => self.call(e, callee, &rest, scope).map(|c| (c, 0)),
                _ => self.not_yet(e.span, "this call"),
            }
        };
        for s in added {
            self.subst.remove(&s);
        }
        self.want_eno = None;
        self.split_ok = false;
        let split = self.split_lines.take();
        let (code, len) = call?;
        let call_fault = self.may_fault;
        let body_set = std::mem::take(&mut self.eno_used);
        if body_set {
            body.push(format!("IEC_BOOL {body_eno} = 1;"));
        }

        // A SPLIT has no result; its lines stand where the call would.
        let r = if split.is_some() { code.clone() } else { self.fresh("r") };
        if split.is_none() {
            let (ty, zero) = if string {
                ("const char *".to_string(), "\"\"".to_string())
            } else {
                let t = self.elem_of(e)?;
                (c_type(t), crate::expr::zero(t))
            };
            pre.push(format!("{ty} {r} = {zero};"));
        }
        let o = self.fresh("eno");
        if eno.is_some() {
            pre.push(format!("IEC_BOOL {o} = 0;"));
        }
        pre.push(match &en_code {
            Some(n) => format!("if ({n}) {{"),
            None => "{".to_string(),
        });
        for l in body {
            pre.push(format!("    {l}"));
        }
        let s = self.fresh("f");
        if eno.is_some() {
            pre.push(format!("    iecf_fault_t {s} = ctx->flag;"));
            pre.push("    ctx->flag.code = IECF_FAULT_NONE;".into());
            pre.push("    ctx->flag.pou = 0u;".into());
            pre.push("    ctx->flag.line = 0u;".into());
        }
        match &split {
            Some(lines) => pre.extend(lines.iter().map(|l| format!("    {l}"))),
            None => pre.push(format!("    {r} = {code};")),
        }
        if call_fault {
            pre.push(format!("    {STOP}"));
        }
        if eno.is_some() {
            let own = if body_set { format!(" && {body_eno}") } else { String::new() };
            pre.push(format!("    {o} = (IEC_BOOL)(ctx->flag.code == IECF_FAULT_NONE{own});"));
            pre.push(format!("    if ({s}.code != IECF_FAULT_NONE) ctx->flag = {s};"));
        }
        match &keep {
            Some(k) => pre.push(format!("}} else {{\n    {k} = 1;\n}}")),
            None => pre.push("}".into()),
        }
        if let Some((not, target)) = eno {
            let v = if not { format!("((IEC_BOOL)!{o})") } else { o.clone() };
            pre.push(self.store_bool(target, &v, scope)?);
        }
        for l in pre {
            for line in l.lines() {
                self.str_temps.push(line.to_string());
            }
        }
        self.pre_ok = true;
        self.may_fault = outer_fault;
        // The statement notes where a flag of the call was raised.
        self.may_flag = outer_flag || eno.is_some() || self.may_flag;
        Ok(Some((r, len)))
    }

    /// `target := v`, `target` a BOOL variable, element or field.
    fn store_bool(&mut self, target: &Expr, v: &str, scope: &Scope) -> R<String> {
        if let ExprKind::Name(id) = &target.kind
            && let Some(p) = scope.vars.get(&id.text.to_ascii_uppercase())
            && !matches!(p, Place::Data(..))
        {
            return Ok(p.store(v));
        }
        match self.data_path(target, scope)? {
            Some((lv, t)) if t.scalar() == Some(Elem::Bool) => Ok(format!("{lv} = {v};")),
            _ => self.not_yet(target.span, "ENO given to this"),
        }
    }
}

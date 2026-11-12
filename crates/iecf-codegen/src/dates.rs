//! DATE, TOD and DT (docs/dependencies.md): each a `uint32_t`, DATE and
//! DT in seconds since 1970, TOD in milliseconds since midnight; Ed.3
//! Table 30's arithmetic and the conversions between them through
//! runtime/iecf.h, which wraps a TOD at midnight and wraps and flags a
//! DATE, DT or difference out of range; Table 36's CONCAT_DATE,
//! CONCAT_TOD, CONCAT_DT (a part out of range: the default value,
//! flagged) and SPLIT_DATE, SPLIT_TOD, SPLIT_DT. LDATE, LTOD and LDT
//! the same way, each an `int64_t` of nanoseconds: an LDT's arithmetic a
//! LINT's (wrapped and flagged), an LTOD's wrapped at midnight.

use crate::expr::arg_value;
use crate::names::{Lv, Place, Scope};
use crate::{Gen, R};
use iecf_diagnostics::Span;
use iecf_sema::StdCall;
use iecf_sema::types::Elem;
use iecf_syntax::ast::{Arg, BinaryOp, Expr, ExprKind};

const FLAG: &str = "&ctx->flag";
const FAULT: &str = "&ctx->fault";

pub(crate) fn is_date(e: Elem) -> bool {
    matches!(e, Elem::Date | Elem::Tod | Elem::Dt | Elem::Ldate | Elem::Ltod | Elem::Ldt)
}

impl Gen<'_> {
    /// `a op b` where either is a DATE, TOD or DT (Ed.3 Table 30), as the
    /// operator or as ADD_TOD_TIME and the other functions by name.
    pub(crate) fn date_arith(&mut self, at: Span, op: BinaryOp, a: &Expr, b: &Expr, scope: &Scope) -> R<String> {
        use BinaryOp::{Add, Sub};
        use Elem::{Date, Dt, Ldate, Ldt, Ltime, Ltod, Time, Tod};
        let (ta, tb) = (self.elem_of(a)?, self.elem_of(b)?);
        // With a long operand, both made long first (Ed.3 6.6.1.6, rule
        // 7): a TIME widened to LTIME, a DT to LDT, as Figure 11 allows.
        let long = |t: Elem| match t {
            Time => Ltime,
            Dt => Ldt,
            Date => Ldate,
            Tod => Ltod,
            t => t,
        };
        let (ta, tb) = if [ta, tb].iter().any(|t| matches!(t, Ltime | Ldt | Ldate | Ltod)) {
            (long(ta), long(tb))
        } else {
            (ta, tb)
        };
        let x = self.value_as(a, ta, scope)?;
        let y = self.value_as(b, tb, scope)?;
        Ok(match (op, ta, tb) {
            (Add, Ltod, Ltime) => format!("iecf_add_LTOD({x}, {y})"),
            (Sub, Ltod, Ltime) => format!("iecf_sub_LTOD({x}, {y})"),
            // Past the range, or LTIME's: wrapped and flagged, as a LINT.
            (Add, Ldt, Ltime) => {
                self.may_flag = true;
                format!("iecf_add_LINT({FLAG}, {x}, {y})")
            }
            (Sub, Ldt, Ltime) | (Sub, Ldt, Ldt) | (Sub, Ldate, Ldate) | (Sub, Ltod, Ltod) => {
                self.may_flag = true;
                format!("iecf_sub_LINT({FLAG}, {x}, {y})")
            }
            (Add, Tod, Time) => format!("iecf_add_TOD({x}, {y})"),
            (Sub, Tod, Time) => format!("iecf_sub_TOD({x}, {y})"),
            (Sub, Tod, Tod) => {
                self.may_flag = true;
                format!("iecf_sub_TOD_TOD({FLAG}, {x}, {y})")
            }
            (Add, Dt, Time) => {
                self.may_flag = true;
                format!("iecf_add_DT({FLAG}, {x}, {y})")
            }
            (Sub, Dt, Time) => {
                self.may_flag = true;
                format!("iecf_sub_DT({FLAG}, {x}, {y})")
            }
            (Sub, Dt, Dt) | (Sub, Date, Date) => {
                self.may_flag = true;
                format!("iecf_sub_DT_DT({FLAG}, {x}, {y})")
            }
            _ => return self.not_yet(at, format!("{} and {} here", ta.name(), tb.name())),
        })
    }

    /// A standard function on DATE, TOD or DT, if `name` is one.
    pub(crate) fn date_call(&mut self, at: Span, name: &str, all: &[&Expr], scope: &Scope) -> R<Option<String>> {
        let op = match name {
            "ADD_TOD_TIME" | "ADD_DT_TIME" | "ADD_LTOD_LTIME" | "ADD_LDT_LTIME" => Some(BinaryOp::Add),
            "SUB_DATE_DATE" | "SUB_TOD_TIME" | "SUB_TOD_TOD" | "SUB_DT_TIME" | "SUB_DT_DT" | "SUB_LDATE_LDATE"
            | "SUB_LTOD_LTIME" | "SUB_LTOD_LTOD" | "SUB_LDT_LTIME" | "SUB_LDT_LDT" => Some(BinaryOp::Sub),
            "ADD" | "SUB" if all.len() == 2 => {
                let (ta, tb) = (self.elem_of(all[0])?, self.elem_of(all[1])?);
                if !(is_date(ta) || is_date(tb)) {
                    return Ok(None);
                }
                Some(if name == "ADD" { BinaryOp::Add } else { BinaryOp::Sub })
            }
            _ => None,
        };
        if let Some(op) = op {
            return self.date_arith(at, op, all[0], all[1], scope).map(Some);
        }
        Ok(Some(match name {
            "CONCAT_DATE_TOD" => {
                let d = self.value_as(all[0], Elem::Date, scope)?;
                let t = self.value_as(all[1], Elem::Tod, scope)?;
                self.may_flag = true;
                format!("iecf_concat_date_tod({FLAG}, {d}, {t})")
            }
            "CONCAT_DATE_LTOD" => {
                let d = self.value_as(all[0], Elem::Date, scope)?;
                let t = self.value_as(all[1], Elem::Ltod, scope)?;
                format!("iecf_concat_date_ltod({d}, {t})")
            }
            "CONCAT_DATE" | "CONCAT_TOD" | "CONCAT_DT" | "CONCAT_LTOD" | "CONCAT_LDT" => {
                let mut parts = Vec::new();
                for x in all {
                    parts.push(self.part(x, scope)?);
                }
                self.may_flag = true;
                let t = &name["CONCAT_".len()..];
                format!("iecf_concat_{t}({FLAG}, {})", parts.join(", "))
            }
            "DAY_OF_WEEK" => {
                let d = self.value_as(all[0], Elem::Date, scope)?;
                format!("iecf_day_of_week({d})")
            }
            _ => return Ok(None),
        }))
    }
}

impl Gen<'_> {
    /// An integer part of CONCAT_DATE, CONCAT_TOD or CONCAT_DT as an
    /// `int64_t`: a ULINT above INT64_MAX as -1, out of every range.
    fn part(&mut self, x: &Expr, scope: &Scope) -> R<String> {
        let t = self.elem_of(x)?;
        let v = self.value(x, scope)?;
        Ok(if t == Elem::Ulint { format!("iecf_part({v})") } else { format!("((int64_t)({v}))") })
    }

    /// SPLIT_DATE, SPLIT_TOD, SPLIT_DT (Ed.3 Table 36), a statement of its
    /// own: the parts written before it, each converted to its output's
    /// type as any integer is (wrapped and flagged if it does not fit).
    pub(crate) fn date_split(&mut self, e: &Expr, sc: &StdCall, args: &[Arg], scope: &Scope) -> R<Option<String>> {
        let from = match sc.name.as_str() {
            "SPLIT_DATE" => Elem::Date,
            "SPLIT_TOD" => Elem::Tod,
            "SPLIT_DT" => Elem::Dt,
            "SPLIT_LTOD" => Elem::Ltod,
            "SPLIT_LDT" => Elem::Ldt,
            _ => return Ok(None),
        };
        // Gated by EN/ENO, its lines go inside the call's `if (EN)`
        // (enable.rs): EN FALSE leaves every output as it was, and a part
        // that does not fit its output is the call's own flag.
        let gated = self.gating && std::mem::take(&mut self.split_ok);
        if (!self.pre_ok || self.gating) && !gated {
            return self.not_yet(e.span, format!("{} in another call's arguments", sc.name));
        }
        let start = self.str_temps.len();
        let Some(x) = sc.inputs.first().and_then(|(_, s)| arg_value(args, *s)) else {
            return self.not_yet(e.span, format!("this {}", sc.name));
        };
        let v = self.value_as(x, from, scope)?;
        let parts: Vec<String> = sc.outputs.iter().map(|_| self.fresh("p")).collect();
        let decl: Vec<String> = parts.iter().map(|p| format!("{p} = 0u")).collect();
        self.str_temps.push(format!("uint32_t {};", decl.join(", ")));
        let refs: Vec<String> = parts.iter().map(|p| format!("&{p}")).collect();
        self.str_temps.push(format!("iecf_split_{}({v}, {});", from.name(), refs.join(", ")));
        if self.may_fault {
            self.str_temps.push(crate::enable::STOP.to_string());
        }
        // Each output's element found first, so an index out of bounds
        // stops the task before any part is stored.
        let mut stores = Vec::new();
        self.may_fault = false;
        for ((_, at), p) in sc.outputs.iter().zip(&parts) {
            if let Some(target) = out_target(args, *at) {
                stores.push(self.store_part(target, p, scope)?);
            }
        }
        if self.may_fault {
            self.str_temps.push(crate::enable::STOP.to_string());
        }
        self.str_temps.extend(stores);
        if gated {
            self.split_lines = Some(self.str_temps.split_off(start));
        }
        Ok(Some("0".into()))
    }

    /// `target := p`, `p` a UDINT part, `target` an integer variable,
    /// element or field: the store, after a pointer to an element or field
    /// declared in the statement's lines.
    fn store_part(&mut self, target: &Expr, p: &str, scope: &Scope) -> R<String> {
        let to = self.elem_of(target)?;
        let v = self.convert(target.span, p.to_string(), Elem::Udint, to)?;
        self.store_to(target, v, to, scope, "an output of SPLIT given to this")
    }

    /// `target := v`, `v` a C value of `to`, `target`'s type: the store,
    /// after a pointer to an element or field declared in the
    /// statement's lines (a SPLIT's or a FUNCTION's output).
    pub(crate) fn store_to(&mut self, target: &Expr, v: String, to: Elem, scope: &Scope, what: &str) -> R<String> {
        if let ExprKind::Name(id) = &target.kind
            && let Some(place) = scope.vars.get(&id.text.to_ascii_uppercase())
            && !matches!(place, Place::Data(..))
        {
            return Ok(place.store(&v));
        }
        match self.data_path(target, scope)? {
            Some((lv, t)) => {
                let v = self.clamp(&v, to, t.range);
                let lv = self.hoisted(lv, to);
                Ok(lv.store(to, &v))
            }
            None => self.not_yet(target.span, what.to_string()),
        }
    }

    /// `lv`, a place of `ty`, found now, in the statement's lines: a
    /// pointer to it in C memory, its offset in the image, the reference
    /// checked; a part's whole the same way. Its indices are checked
    /// here, before anything is stored.
    pub(crate) fn hoisted(&mut self, lv: Lv, ty: Elem) -> Lv {
        let q = self.fresh("q");
        match lv {
            Lv::C(c) => {
                self.str_temps.push(format!("{} *{q} = &{c};", crate::c_type(ty)));
                Lv::C(format!("(*{q})"))
            }
            Lv::Image { area, off } => {
                self.str_temps.push(format!("uint32_t {q} = {off};"));
                Lv::Image { area, off: q }
            }
            Lv::Ref(r) => {
                self.str_temps.push(format!("iecf_ref_t {q} = {r};"));
                Lv::Ref(q)
            }
            Lv::Part { base, of, shift } => Lv::Part { base: Box::new(self.hoisted(*base, of)), of, shift },
        }
    }
}

/// The expression an output was given to, by its span: `YEAR => y` or
/// positional.
fn out_target(args: &[Arg], span: Option<Span>) -> Option<&Expr> {
    let span = span?;
    args.iter().find_map(|a| match a {
        Arg::Positional(e) if e.span == span => Some(e),
        Arg::Output { target, .. } if target.span == span => Some(target),
        _ => None,
    })
}

impl Gen<'_> {
    /// A TIME or LTIME `t` times or divided by a number (Ed.3 Table 30,
    /// MUL_TIME, DIV_TIME): `d * n`, `n * d`, `d / n`. By an integer,
    /// exact, an overflow wrapped and flagged; by a real, computed in
    /// LREAL and rounded half to even as REAL_TO_DINT is, out of range
    /// flagged and the statement left undone (docs/dependencies.md).
    /// Dividing by 0 or 0.0 faults.
    pub(crate) fn duration_scale(
        &mut self,
        at: Span,
        op: BinaryOp,
        t: Elem,
        a: &Expr,
        b: &Expr,
        scope: &Scope,
    ) -> R<String> {
        let (d, n) = if self.elem_of(a)?.is_duration() { (a, b) } else { (b, a) };
        if op == BinaryOp::Div && !std::ptr::eq(d, a) {
            return self.not_yet(at, "a number divided by a duration");
        }
        let held = if t == Elem::Time { Elem::Dint } else { Elem::Lint };
        let x = self.value_as(d, t, scope)?;
        let nt = self.elem_of(n)?;
        let div = op == BinaryOp::Div;
        self.may_flag = true;
        if nt.is_real() {
            let f = self.value_as(n, Elem::Lreal, scope)?;
            let v = if div {
                self.may_fault = true;
                format!("iecf_div_LREAL({FAULT}, {FLAG}, (double)({x}), {f})")
            } else {
                format!("((double)({x}) * ({f}))")
            };
            let r = self.real_to_int(at, v, held, "round")?;
            return Ok(format!("(({})({r}))", crate::c_type(t)));
        }
        if !nt.is_int() {
            return self.not_yet(n.span, format!("a duration and {}", nt.name()));
        }
        let f = if nt == Elem::Ulint {
            let c = self.value(n, scope)?;
            self.convert(n.span, c, Elem::Ulint, Elem::Lint)?
        } else {
            self.value_as(n, Elem::Lint, scope)?
        };
        let v = if div {
            self.may_fault = true;
            format!("iecf_div_LINT({FAULT}, {FLAG}, (int64_t)({x}), {f})")
        } else {
            format!("iecf_mul_LINT({FLAG}, (int64_t)({x}), {f})")
        };
        Ok(if t == Elem::Time { format!("((IEC_TIME)iecf_fit_DINT({FLAG}, {v}))") } else { v })
    }
}

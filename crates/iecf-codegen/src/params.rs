//! A FUNCTION's or method's inputs, outputs, in-outs and result of a
//! STRING, array or structure type (Ed.3 6.6.1.2, 6.6.2), which C does
//! not pass by value.
//!
//! Each is a pointer, reached in the function through a cast to its own
//! C type (`at_ptr`), so a copy of it, and `sizeof` it, is of the whole:
//! - an input, `const void *P_<NAME>`: the caller's value, copied into
//!   the local `V_<NAME>` as the function starts (a STRING cut to its
//!   length and flagged, as any store), or NULL when the call does not
//!   give it, the local then its initial value (Ed.3 6.6.1.2.2);
//! - an in-out, `void *V_<NAME>`: the variable itself, of its type
//!   exactly (a STRING of its length);
//! - an output, `void *O_<NAME>`: NULL when the call does not take it;
//!   a local copied there whole as the function returns, not when it
//!   faults, and from there stored into the call's target;
//! - the result, `void *R_`: the caller's buffer of its type, which the
//!   function's name writes into, set to the type's initial value first.
//!   A STRING's is returned too, so a call is a string expression; an
//!   array's or a structure's is the buffer, which a statement copies.

use crate::derived::{DTy, Kind};
use crate::names::{Lv, Scope};
use crate::strings::{infix, unit};
use crate::{Gen, R};
use iecf_syntax::ast::Expr;
use std::fmt::Write as _;

impl Gen<'_> {
    /// The signature of the project's FUNCTION or method the call `e`
    /// calls, once written.
    pub(crate) fn called_sig(&self, e: &Expr) -> Option<crate::FnSig> {
        let key = match (self.types.method_call(e.span), &e.kind) {
            (Some(k), _) => k.to_string(),
            (None, iecf_syntax::ast::ExprKind::Call { callee, .. }) => match &callee.kind {
                iecf_syntax::ast::ExprKind::Name(id) => id.text.to_ascii_uppercase(),
                _ => return None,
            },
            _ => return None,
        };
        self.functions.get(&key).cloned()
    }

    /// As `called_sig`, the interface worked out now if this is the
    /// first call of it.
    pub(crate) fn called_sig_now(&mut self, e: &Expr) -> R<Option<crate::FnSig>> {
        let key = match (self.types.method_call(e.span), &e.kind) {
            (Some(k), _) => k.to_string(),
            (None, iecf_syntax::ast::ExprKind::Call { callee, .. }) => match &callee.kind {
                iecf_syntax::ast::ExprKind::Name(id) => id.text.to_ascii_uppercase(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let known = self
            .pous
            .get(key.as_str())
            .is_some_and(|p| matches!(p.kind, iecf_syntax::ast::PouKind::Function | iecf_syntax::ast::PouKind::Method));
        if !known {
            return Ok(None);
        }
        self.fn_sig(&key, e.span).map(Some)
    }

    /// Whether a parameter or result of `t` passes as params.rs says: a
    /// STRING, an array, a structure or an interface.
    pub(crate) fn passes(&self, t: &DTy) -> bool {
        matches!(
            t.kind,
            Kind::Str(..) | Kind::Array(..) | Kind::Flex(..) | Kind::Struct(_) | Kind::Itf(_) | Kind::Ref(..)
        )
    }

    /// The object of `t` that the pointer `p` points to, as a C lvalue.
    pub(crate) fn at_ptr(&self, t: &DTy, p: &str) -> String {
        format!("(*({}){p})", self.declare(t, "(*)"))
    }

    /// The input `n`'s local, set from `P_<n>` or to its initial value,
    /// into `decls`: its lvalue.
    /// `at`: the function's POU and the input's line, where a STRING
    /// cut is flagged.
    pub(crate) fn data_input(&mut self, n: &str, t: &DTy, at: (u16, u32), decls: &mut String) -> R<String> {
        let v = format!("V_{n}");
        let _ = writeln!(decls, "    {};", self.declare(t, &v));
        let copy = match t.kind {
            Kind::Str(len, w) => format!(
                "iecf_{}scopy(&ctx->flag, {v}, {len}u, (const {} *)P_{n});\n        iecf_noted(&ctx->flag, {}u, {}u);",
                infix(w),
                unit(w),
                at.0,
                at.1
            ),
            _ => format!("iecf_bytes(&{v}, P_{n}, sizeof {v});"),
        };
        let mut init = String::new();
        self.init_data(&v, t, None, &mut init)?;
        let _ = writeln!(decls, "    if (P_{n} != NULL) {{\n        {copy}\n    }} else {{");
        for l in init.lines() {
            let _ = writeln!(decls, "    {l}");
        }
        let _ = writeln!(decls, "    }}");
        Ok(v)
    }

    /// `x` given to an input of `t`: a pointer to its value. A STRING
    /// any string expression, cut in the function; an array or a
    /// structure where it is in C memory, or a copy made first (one at an
    /// address, through a reference, or a call's result).
    pub(crate) fn data_arg(&mut self, x: &Expr, t: &DTy, scope: &Scope) -> R<String> {
        // An ARRAY[*]: the array itself, not a copy (flex.rs).
        if let Kind::Flex(..) = t.kind {
            return self.flex_arg_as(x, scope, true, false);
        }
        if let Kind::Str(..) = t.kind {
            // Cut to the input's length as the function starts, flagged
            // there (`data_input`).
            let (s, _) = self.string(x, scope)?;
            return Ok(s);
        }
        // An interface: its value in a C99 compound literal (itf.rs).
        if let Kind::Itf(i) = &t.kind {
            let v = self.itf_value(x, i, scope)?;
            return Ok(format!("(iecf_itf_t[]){{{v}}}"));
        }
        // A reference - REF(x), NULL, a variable of one, a call giving
        // one - the same way (refs.rs).
        if let Kind::Ref(..) = &t.kind {
            let v = self.ref_value(x, scope)?;
            return Ok(format!("(iecf_ref_t[]){{{v}}}"));
        }
        // The project's FUNCTION giving one: its buffer. SEL, MUX or MOVE
        // of whole values is a place, as a variable is (expr.rs).
        if let iecf_syntax::ast::ExprKind::Call { callee, args } = &crate::expr::strip_paren(x).kind
            && self.types.std_call(crate::expr::strip_paren(x).span).is_none()
        {
            let r = match self.gated(x, args, scope, false)? {
                Some((r, _)) => r,
                None => self.user_call(x, callee, args, scope)?,
            };
            return Ok(format!("&{r}"));
        }
        match self.data_path(x, scope)? {
            Some((Lv::C(c), _)) => Ok(format!("&({c})")),
            Some((lv, _)) => {
                if !self.pre_ok {
                    return self.not_yet(x.span, "this value given to an input here");
                }
                let tmp = self.fresh("a");
                self.str_temps.push(format!("{};", self.declare(t, &tmp)));
                let copy = self.copy_data(&Lv::C(tmp.clone()), &lv, t, x.span)?;
                self.str_temps.push(copy);
                Ok(format!("&{tmp}"))
            }
            None => self.not_yet(x.span, "this value given to an input"),
        }
    }

    /// `x` given to an in-out of `t`: a pointer to the variable, which
    /// is of `t` exactly, in C memory.
    pub(crate) fn data_inout(&mut self, x: &Expr, t: &DTy, scope: &Scope) -> R<String> {
        if let Kind::Flex(..) = t.kind {
            return self.flex_arg(x, scope);
        }
        // A place: its index read where it is used, unless it calls a
        // FUNCTION with effects, made once, before the call (`seq`).
        let effects = self.seq && self.calls_effects(x, scope)?;
        let seq = std::mem::replace(&mut self.seq, effects);
        let saved = std::mem::replace(&mut self.may_fault, false);
        let found = self.data_path(x, scope);
        self.seq = seq;
        let faults = std::mem::replace(&mut self.may_fault, saved);
        match found? {
            Some((Lv::C(c), xt)) => {
                if let (Kind::Str(a, _), Kind::Str(b, _)) = (&t.kind, &xt.kind)
                    && a != b
                {
                    return self.not_yet(x.span, "a STRING of another length given to an in-out");
                }
                // An element by a computed index: found on a line of its
                // own, so one out of bounds stops the statement before the
                // call runs on another element.
                if faults && self.pre_ok {
                    let q = self.fresh("q");
                    self.str_temps.push(format!("void *{q} = &({c});"));
                    self.str_temps.push(crate::enable::STOP.to_string());
                    return Ok(q);
                }
                self.may_fault |= faults;
                Ok(format!("&({c})"))
            }
            // At an address or through a reference: no C object of its
            // type is there, so it is given a copy, stored back after the
            // call (`data_copy`).
            Some((lv, xt)) => {
                if let (Kind::Str(a, _), Kind::Str(b, _)) = (&t.kind, &xt.kind)
                    && a != b
                {
                    return self.not_yet(x.span, "a STRING of another length given to an in-out");
                }
                let tmp = self.data_copy(x, &lv, &xt, true, true)?;
                // An index out of bounds stops it before the call.
                if faults {
                    self.str_temps.push(crate::enable::STOP.to_string());
                }
                Ok(format!("&{tmp}"))
            }
            // A standard block's instance reached otherwise: through a
            // reference, which points at the instance itself.
            None if matches!(t.kind, Kind::Std(_)) => match self.fb_place(x, scope)? {
                Some(crate::Place::Fb(lv, _)) => Ok(format!("&({lv})")),
                _ => self.not_yet(x.span, "this variable given to an in-out"),
            },
            None => self.not_yet(x.span, "this variable given to an in-out"),
        }
    }

    /// A copy of the variable at `lv`, of `t`, which is in the process
    /// image or behind a reference, where no C pointer of its type may
    /// point (docs/c-output.md §4): a local of the statement's, read
    /// from the variable before the call when `fill`, stored back to it
    /// after the call when `back` - with the call's outputs, and not at
    /// all if the call faults. As an elementary in-out at an address is
    /// passed (expr.rs `in_out_copy`), and as Siemens passes a structured
    /// parameter whose memory is not the block's kind.
    pub(crate) fn data_copy(&mut self, x: &Expr, lv: &Lv, t: &DTy, fill: bool, back: bool) -> R<String> {
        if !self.pre_ok {
            return self.not_yet(x.span, "an in-out given a variable at an address or through a reference here");
        }
        let tmp = self.fresh("a");
        self.str_temps.push(format!("{};", self.declare(t, &tmp)));
        let mine = Lv::C(tmp.clone());
        if fill {
            let copy = self.copy_data(&mine, lv, t, x.span)?;
            self.str_temps.push(copy);
        }
        if back {
            let store = self.copy_data(lv, &mine, t, x.span)?;
            self.in_out_backs.push(store);
        }
        Ok(tmp)
    }

    /// The statement storing an output of `t`, taken in the temporary
    /// `o`, into `target`: a STRING cut and flagged, as any store. An
    /// element by a computed index, or a place through a reference, is
    /// found on the statement's lines first, its index or reference
    /// checked there, so the caller's check stops the task before any
    /// output is stored (`may_fault`), as an elementary output's is.
    pub(crate) fn data_output(&mut self, o: &str, t: &DTy, target: &Expr, scope: &Scope) -> R<String> {
        let was = std::mem::replace(&mut self.may_fault, false);
        let Some((lv, tt)) = self.data_path(target, scope)? else {
            return self.not_yet(target.span, "an output taken into this");
        };
        let lv = if self.may_fault { self.hoisted_data(lv, &tt) } else { lv };
        self.may_fault |= was;
        Ok(match (&t.kind, &tt.kind) {
            (Kind::Str(..), Kind::Str(n, w)) => {
                self.may_flag = true;
                let copy = format!("iecf_{}scopy(&ctx->flag, @, {n}u, {o});", infix(*w));
                crate::strings::store_string(&lv, *n, *w, &copy)
            }
            (Kind::Itf(a), Kind::Itf(b)) => self.itf_store(&lv, b, o, a, target.span)?,
            _ => self.copy_data(&lv, &Lv::C(o.to_string()), t, target.span)?,
        })
    }
}

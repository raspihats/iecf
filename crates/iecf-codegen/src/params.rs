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
    /// STRING, an array, a structure or an interface, holding no instance.
    pub(crate) fn passes(&self, t: &DTy) -> bool {
        matches!(t.kind, Kind::Str(..) | Kind::Array(..) | Kind::Flex(..) | Kind::Struct(_) | Kind::Itf(_))
            && !self.has_instance(t)
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
            return self.flex_arg(x, scope);
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
        if let iecf_syntax::ast::ExprKind::Call { callee, args } = &crate::expr::strip_paren(x).kind {
            let r = self.user_call(x, callee, args, scope)?;
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
        let seq = std::mem::replace(&mut self.seq, false);
        let found = self.data_path(x, scope);
        self.seq = seq;
        match found? {
            Some((Lv::C(c), xt)) => {
                if let (Kind::Str(a, _), Kind::Str(b, _)) = (&t.kind, &xt.kind)
                    && a != b
                {
                    return self.not_yet(x.span, "a STRING of another length given to an in-out");
                }
                Ok(format!("&({c})"))
            }
            Some(_) => self.not_yet(x.span, "an in-out given a variable at an address or through a reference"),
            None => self.not_yet(x.span, "this variable given to an in-out"),
        }
    }

    /// The statement storing an output of `t`, taken in the temporary
    /// `o`, into `target`: a STRING cut and flagged, as any store.
    pub(crate) fn data_output(&mut self, o: &str, t: &DTy, target: &Expr, scope: &Scope) -> R<String> {
        let was = std::mem::replace(&mut self.may_fault, false);
        let Some((lv, tt)) = self.data_path(target, scope)? else {
            return self.not_yet(target.span, "an output taken into this");
        };
        if self.may_fault {
            return self.not_yet(target.span, "an output taken into an element by a computed index");
        }
        self.may_fault = was;
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

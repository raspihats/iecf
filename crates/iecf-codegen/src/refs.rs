//! References (Ed.3 6.4.4.10): `REF_TO`, `REF()`, `^` and `NULL`.
//!
//! A reference is runtime/iecf.h's `iecf_ref_t`: a C pointer to a variable
//! in the context, or the area, byte and bit of a variable in the process
//! image (docs/c-output.md §4). `REF(x)` builds one from where `x` is.
//! `r^` checks it (`iecf_deref`): NULL is a fault, IECF_FAULT_NULL, and
//! the statement stops before it stores anything, as an index out of
//! range does (docs/dependencies.md). A dereferenced reference to a type
//! that can lie in the image is read and written through `iecf_rget_T` and
//! `iecf_rset_T`, which look at where it is; to any other type it is a C
//! lvalue, `(*(T (*))r.p)`.
//!
//! An initial value `REF(x)` names a variable of the same POU, which may
//! be declared after it: it is written as a marker and filled in when the
//! POU's variables are all known (`fill_ref_inits`).

use crate::derived::{DTy, Kind, RefTo};
use crate::names::{Lv, Place, Scope};
use crate::{Gen, R};
use iecf_diagnostics::{Code, Diagnostic};
use iecf_sema::stdlib::Dir;
use iecf_syntax::ast::*;

const FAULT: &str = "&ctx->fault";

/// The line an initial value `REF(x)` leaves, until `fill_ref_inits`.
pub(crate) fn marker(n: usize) -> String {
    format!("/*@REF{n}@*/")
}

/// Whether `callee` names `REF`.
pub(crate) fn is_ref_call(callee: &Expr) -> bool {
    matches!(&callee.kind, ExprKind::Name(id) if id.is("REF"))
}

/// `iecf_ref_t` of a part at `lv`.
fn lv_ref(lv: &Lv) -> String {
    match lv {
        Lv::C(c) => format!("iecf_ref_c(&({c}))"),
        Lv::Image { area, off } => format!("iecf_ref_image((uint8_t)'{area}', {off}, IECF_NO_BIT)"),
        Lv::Ref(r) => r.clone(),
    }
}

impl Gen<'_> {
    /// `e`, of a reference type or NULL, as a C `iecf_ref_t`.
    pub(crate) fn ref_value(&mut self, e: &Expr, scope: &Scope) -> R<String> {
        match &e.kind {
            ExprKind::Paren(x) => self.ref_value(x, scope),
            ExprKind::Name(id) if id.is("NULL") && !scope.vars.contains_key("NULL") => Ok("iecf_ref_null()".into()),
            ExprKind::Call { callee, args } if is_ref_call(callee) => match args.as_slice() {
                [Arg::Positional(x)] => self.make_ref(x, scope),
                _ => self.not_yet(e.span, "this REF()"),
            },
            _ => match self.ref_of(e, scope)? {
                Some((r, _)) => Ok(r),
                None => self.not_yet(e.span, "this reference"),
            },
        }
    }

    /// A variable, element or field of a reference type: its C value and
    /// what it references.
    pub(crate) fn ref_of(&mut self, e: &Expr, scope: &Scope) -> R<Option<(String, RefTo)>> {
        match self.data_path(e, scope)? {
            Some((lv, DTy { kind: Kind::Ref(to, _), .. })) => match lv.c() {
                Some(c) => Ok(Some((c.to_string(), *to))),
                None => self.not_yet(e.span, "this reference"),
            },
            _ => Ok(None),
        }
    }

    /// `REF(x)`: where `x` is, as an `iecf_ref_t`. The checker allowed it
    /// (E0026): a variable that lives as long as the program.
    pub(crate) fn make_ref(&mut self, x: &Expr, scope: &Scope) -> R<String> {
        match &x.kind {
            ExprKind::Paren(i) => return self.make_ref(i, scope),
            ExprKind::Name(id) => {
                return match scope.vars.get(&id.text.to_ascii_uppercase()) {
                    Some(Place::Field(f, _) | Place::Fb(f, _) | Place::Block(f, _)) => {
                        Ok(format!("iecf_ref_c(&({f}))"))
                    }
                    Some(Place::Located { area, size, byte, bit, .. }) => {
                        let bit = if *size == 'X' { format!("{bit}u") } else { "IECF_NO_BIT".into() };
                        Ok(format!("iecf_ref_image((uint8_t)'{area}', {byte}u, {bit})"))
                    }
                    Some(Place::Data(lv, _)) => Ok(lv_ref(lv)),
                    Some(Place::Itf(..)) | None => self.not_yet(id.span, format!("REF({})", id.text)),
                };
            }
            // A function block instance through a reference: that reference.
            ExprKind::Deref(base) => {
                if let Some((r, to @ (RefTo::Block(_) | RefTo::Std(_)))) = self.ref_of(base, scope)? {
                    return Ok(self.checked_ref(&r, &to));
                }
            }
            _ => {}
        }
        if let Some((lv, _)) = self.data_path(x, scope)? {
            return Ok(lv_ref(&lv));
        }
        // An elementary input of a function block instance.
        if let ExprKind::Member(base, m) = &x.kind
            && let Some(p) = self.fb_place(base, scope)?
            && let Some(fb) = self.iface(&p)
            && let Some((_, field, _, Dir::Input | Dir::Output)) = fb.param(&m.text)
        {
            return Ok(format!("iecf_ref_c(&({}.{field}))", fb.lv));
        }
        self.not_yet(x.span, "REF() of this")
    }

    /// `r` checked, an `iecf_ref_t` that is not NULL: a NULL one faults
    /// and gives `ctx->nowhere`.
    pub(crate) fn checked_ref(&mut self, r: &str, to: &RefTo) -> String {
        let member = match to {
            RefTo::Data(d) => self.declare(d, "@"),
            RefTo::Block(k) => format!("B_{k}_t @"),
            RefTo::Std(fb) => format!("iecf_{}_t @", fb.stem),
        };
        self.nowhere.insert(member);
        self.may_fault = true;
        format!("iecf_deref({FAULT}, {r}, &ctx->nowhere)")
    }

    /// What a checked reference `r` to `d` reaches: through the runtime's
    /// helpers if `d` can lie in the image, else a C lvalue.
    pub(crate) fn data_at(&self, r: &str, d: &DTy) -> Lv {
        if self.packed_size(d).is_some() {
            Lv::Ref(r.to_string())
        } else {
            Lv::C(format!("(*({}){r}.p)", self.declare(d, "(*)")))
        }
    }

    /// `base^` of a reference to data: where it is, checked inline - for
    /// an expression, whose statement stops before it stores.
    pub(crate) fn deref_path(&mut self, base: &Expr, scope: &Scope) -> R<Option<(Lv, DTy)>> {
        let Some((r, to)) = self.ref_of(base, scope)? else {
            return self.not_yet(base.span, "dereferencing this");
        };
        let RefTo::Data(d) = &to else { return Ok(None) };
        let chk = self.checked_ref(&r, &to);
        Ok(Some((self.data_at(&chk, d), d.clone())))
    }

    /// A function block instance `base` names: by its name, or through a
    /// reference, checked inline.
    pub(crate) fn fb_place(&mut self, base: &Expr, scope: &Scope) -> R<Option<Place>> {
        match &base.kind {
            ExprKind::Paren(x) => self.fb_place(x, scope),
            ExprKind::Name(id) => Ok(match scope.vars.get(&id.text.to_ascii_uppercase()) {
                Some(p @ (Place::Fb(..) | Place::Block(..))) => Some(p.clone()),
                _ => None,
            }),
            ExprKind::Deref(b) => match self.ref_of(b, scope)? {
                Some((r, to @ (RefTo::Block(_) | RefTo::Std(_)))) => {
                    let chk = self.checked_ref(&r, &to);
                    Ok(Some(self.fb_through(&chk, &to)))
                }
                _ => Ok(None),
            },
            // An instance in an instance, `f.k`: a PUBLIC one, as the
            // checker allowed (Ed.3 6.6.5.2 rule 7).
            ExprKind::Member(b, m) => {
                let Some(Place::Block(lv, key)) = self.fb_place(b, scope)? else {
                    // A structure's field (inst.rs).
                    return self.data_instance(base, scope);
                };
                let k = m.text.to_ascii_uppercase();
                // Its field as the type's own code reaches it, `self->V_K`
                // or, inherited, `self->base.V_K`.
                let field = |f: &str| format!("{lv}.{}", f.strip_prefix("self->").unwrap_or(f));
                Ok(match self.block_scopes.get(&key).and_then(|s| s.vars.get(&k)) {
                    Some(Place::Block(f, inner)) => Some(Place::Block(field(f), inner.clone())),
                    Some(Place::Fb(f, fb)) => Some(Place::Fb(field(f), fb.clone())),
                    // An array or structure of instances in it.
                    Some(Place::Data(Lv::C(f), t)) => match &t.kind {
                        crate::derived::Kind::Block(k) => Some(Place::Block(field(f), k.clone())),
                        crate::derived::Kind::Std(fb) => Some(Place::Fb(field(f), fb.clone())),
                        _ => None,
                    },
                    _ => None,
                })
            }
            // An element (inst.rs); a FUNCTION's result that is an instance.
            ExprKind::Index(..) | ExprKind::Call { .. } => self.data_instance(base, scope),
            _ => Ok(None),
        }
    }

    /// The instance a checked reference `r` to a function block reaches.
    pub(crate) fn fb_through(&self, r: &str, to: &RefTo) -> Place {
        match to {
            RefTo::Block(k) => Place::Block(format!("(*(B_{k}_t *){r}.p)"), k.clone()),
            RefTo::Std(fb) => Place::Fb(format!("(*(iecf_{}_t *){r}.p)", fb.stem), fb.clone()),
            RefTo::Data(d) => Place::Data(self.data_at(r, d), d.clone()),
        }
    }

    /// `a = b` or `a <> b` of references, or with NULL.
    pub(crate) fn ref_compare(&mut self, eq: bool, a: &Expr, b: &Expr, scope: &Scope) -> R<String> {
        let (x, y) = (self.ref_value(a, scope)?, self.ref_value(b, scope)?);
        Ok(if eq { format!("iecf_ref_eq({x}, {y})") } else { format!("((IEC_BOOL)!iecf_ref_eq({x}, {y}))") })
    }

    /// The subrange `e`'s reference references, when known: `Some(None)`
    /// for a target that is no subrange.
    fn target_range(&mut self, e: &Expr, scope: &Scope) -> R<Option<Option<(i128, i128)>>> {
        match &e.kind {
            ExprKind::Paren(x) => self.target_range(x, scope),
            ExprKind::Call { callee, args } if is_ref_call(callee) => match args.as_slice() {
                [Arg::Positional(x)] => Ok(match &x.kind {
                    ExprKind::Name(id) => match scope.vars.get(&id.text.to_ascii_uppercase()) {
                        Some(Place::Data(_, t)) => Some(t.range),
                        Some(_) => Some(None),
                        None => None,
                    },
                    _ => self.data_path(x, scope)?.map(|(_, t)| t.range),
                }),
                _ => Ok(None),
            },
            ExprKind::Name(id) if id.is("NULL") => Ok(None),
            _ => Ok(match self.ref_of(e, scope)? {
                Some((_, RefTo::Data(d))) => Some(d.range),
                Some(_) => Some(None),
                None => None,
            }),
        }
    }

    /// A reference to `to` given `e`: a subrange and its base are two
    /// types to a reference (Ed.3 6.4.4.10.1), though the checker holds a
    /// subrange as its base.
    pub(crate) fn check_ref_target(&mut self, to: &RefTo, e: &Expr, scope: &Scope) -> R<()> {
        let want = match to {
            RefTo::Data(d) => d.range,
            _ => None,
        };
        let saved = (self.may_fault, self.may_flag);
        let got = self.target_range(e, scope)?;
        (self.may_fault, self.may_flag) = saved;
        match got {
            Some(r) if r != want => {
                let name = |r: Option<(i128, i128)>| match r {
                    Some((lo, hi)) => format!("a subrange {lo}..{hi}"),
                    None => "no subrange".to_string(),
                };
                self.diags.push(Diagnostic::new(
                    Code::TypeMismatch,
                    e.span,
                    format!(
                        "this references {}, the reference {}: a reference takes only its own type (Ed.3 6.4.4.10.1)",
                        name(r),
                        name(want)
                    ),
                ));
                Err(crate::Skip)
            }
            _ => Ok(()),
        }
    }

    /// The initial values `REF(x)` left in `text`, now that `scope` has
    /// the POU's variables.
    pub(crate) fn fill_ref_inits(&mut self, text: &mut String, scope: &Scope) -> R<()> {
        for n in 0..self.ref_inits.len() {
            let m = marker(n);
            if !text.contains(&m) {
                continue;
            }
            let (lv, x, to) = self.ref_inits[n].clone();
            self.check_ref_target(&to, &x, scope)?;
            self.may_fault = false;
            self.may_flag = false;
            let v = self.ref_value(&x, scope)?;
            if self.may_fault || self.may_flag {
                return self.not_yet(x.span, "an initial value that can fault or be flagged");
            }
            *text = text.replace(&m, &format!("{lv} = {v};"));
        }
        self.fill_itf_inits(text, scope)
    }

    /// `ctx->nowhere`: where a NULL reference points while its statement
    /// stops, as big as anything a reference is dereferenced to.
    pub(crate) fn nowhere_field(&self) -> String {
        if self.nowhere.is_empty() {
            return String::new();
        }
        let mut s = String::from("    union {\n        uint64_t align_;\n        double real_;\n        void *ptr_;\n");
        for (i, m) in self.nowhere.iter().enumerate() {
            s.push_str(&format!("        {};\n", m.replacen('@', &format!("t{i}"), 1)));
        }
        s.push_str("    } nowhere; /* what a NULL reference gives while its statement stops */\n");
        s
    }
}

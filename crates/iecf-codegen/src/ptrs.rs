//! Pointers (the pointers extension, POINTERS.md): `POINTER TO T`,
//! `ADR(x)`, `SIZEOF(x)`, `p^`, arithmetic and comparison in bytes.
//!
//! A pointer is runtime/iecf.h's `iecf_ptr_t`: the variable it was taken
//! in, that variable's size, and an offset in bytes. `ADR(x)` takes the
//! variable `x` is a part of whole, so arithmetic may move across it.
//! Through `p^` an elementary value is `Lv::Ptr`, read and written by
//! `iecf_pget_T` and `iecf_pset_T`, which check it lies in the variable;
//! an element or a field moves the offset by `sizeof` and `offsetof`.

use crate::derived::{DTy, Kind};
use crate::names::{Lv, Place, Scope};
use crate::{Gen, R};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;

const FAULT: &str = "&ctx->fault";
const FLAG: &str = "&ctx->flag";

/// The line an initial value `ADR(x)` leaves, until `fill_ptr_inits`.
pub(crate) fn marker(n: usize) -> String {
    format!("/*@PTR{n}@*/")
}

/// Whether `callee` names `n`.
fn named(callee: &Expr, n: &str) -> bool {
    matches!(&callee.kind, ExprKind::Name(id) if id.is(n))
}

impl Gen<'_> {
    /// Whether `e` is of a pointer type.
    pub(crate) fn is_pointer(&mut self, e: &Expr) -> bool {
        matches!(self.types.expr(e.span), Some(Ty::Pointer(_)))
    }

    /// `e`, a pointer, 0 or NULL, as a C `iecf_ptr_t`.
    pub(crate) fn ptr_value(&mut self, e: &Expr, scope: &Scope) -> R<String> {
        match &e.kind {
            ExprKind::Paren(x) => self.ptr_value(x, scope),
            ExprKind::Name(id) if id.is("NULL") && !scope.vars.contains_key("NULL") => Ok("iecf_ptr_null()".into()),
            _ if crate::expr::const_int(e) == Some(0) => Ok("iecf_ptr_null()".into()),
            ExprKind::Call { callee, args } if named(callee, "ADR") => match args.as_slice() {
                [Arg::Positional(x)] => self.make_ptr(x, scope),
                _ => self.not_yet(e.span, "this ADR"),
            },
            // p + n, n + p, p - n: the offset moved by n bytes.
            ExprKind::Binary(op @ (BinaryOp::Add | BinaryOp::Sub), a, b) => {
                let (p, n) = if self.is_pointer(a) { (a, b) } else { (b, a) };
                let pv = self.ptr_value(p, scope)?;
                let nv = self.value_as(n, Elem::Lint, scope)?;
                let f = if *op == BinaryOp::Add { "iecf_ptr_add" } else { "iecf_ptr_sub" };
                Ok(format!("{f}({pv}, (int64_t)({nv}))"))
            }
            _ => match self.data_path(e, scope)? {
                Some((lv, DTy { kind: Kind::Ptr(_), .. })) => match lv.c() {
                    Some(c) => Ok(c.to_string()),
                    None => self.not_yet(e.span, "this pointer"),
                },
                _ => self.not_yet(e.span, "this pointer"),
            },
        }
    }

    /// What `e`, a pointer, points to: its type as the generator holds it.
    pub(crate) fn ptr_target(&mut self, e: &Expr, scope: &Scope) -> R<Option<DTy>> {
        match &e.kind {
            ExprKind::Paren(x) => self.ptr_target(x, scope),
            ExprKind::Binary(_, a, b) => {
                let p = if self.is_pointer(a) { a } else { b };
                self.ptr_target(p, scope)
            }
            ExprKind::Call { callee, args } if named(callee, "ADR") => match args.as_slice() {
                [Arg::Positional(x)] => self.place_type(x, scope),
                _ => Ok(None),
            },
            _ => Ok(match self.data_path(e, scope)? {
                Some((_, DTy { kind: Kind::Ptr(to), .. })) => Some(*to),
                _ => None,
            }),
        }
    }

    /// The type of the place `x`: its derived type, or its elementary one.
    pub(crate) fn place_type(&mut self, x: &Expr, scope: &Scope) -> R<Option<DTy>> {
        if let Some((_, t)) = self.data_path(x, scope)? {
            return Ok(Some(t));
        }
        Ok(match self.ty_of(x)? {
            Ty::Elem(e) => Some(DTy { kind: Kind::Elem(e), init: None, range: None }),
            _ => None,
        })
    }

    /// `ADR(x)`: a pointer into the variable `x` is part of, at `x`. The
    /// place is found once: finding it writes its indices' lines.
    pub(crate) fn make_ptr(&mut self, x: &Expr, scope: &Scope) -> R<String> {
        let x = crate::expr::strip_paren(x);
        // A variable at an address: its area and byte, found through the
        // image at each use, as the platform may move it.
        if let ExprKind::Name(id) = &x.kind
            && let Some(Place::Located { area, byte, .. }) = scope.vars.get(&id.text.to_ascii_uppercase())
        {
            return Ok(format!("iecf_ptr_image((uint8_t)'{area}', {byte}u)"));
        }
        let part = match self.data_path(x, scope)? {
            // Through a pointer: the place's own pointer.
            Some((Lv::Ptr(p), _)) => return Ok(p),
            Some((Lv::C(c), _)) => c,
            // An element or a field of a variable at an address.
            Some((Lv::Bytes { area: crate::names::Area::Image(a), off, bit: None }, _)) => {
                return Ok(format!("iecf_ptr_image((uint8_t)'{a}', {off})"));
            }
            Some(_) => {
                return self
                    .not_yet(x.span, "ADR of a bit, of a structure with relative locations, or through a reference");
            }
            None => self.c_name(x, scope)?,
        };
        // The whole variable, a name: no index to find again.
        let whole = match root(x) {
            Some(r) if !std::ptr::eq(r, x) => match &r.kind {
                ExprKind::Name(id) => match scope.vars.get(&id.text.to_ascii_uppercase()) {
                    Some(Place::Field(f, _)) => f.clone(),
                    Some(Place::Data(Lv::C(c), _)) => c.clone(),
                    _ => part.clone(),
                },
                _ => part.clone(),
            },
            _ => part.clone(),
        };
        Ok(format!("iecf_ptr_in(&({whole}), sizeof({whole}), &({part}))"))
    }

    /// The C lvalue of a variable or a block's elementary input or output,
    /// where `data_path` has none.
    fn c_name(&mut self, x: &Expr, scope: &Scope) -> R<String> {
        if let ExprKind::Name(id) = &x.kind
            && let Some(Place::Field(f, _)) = scope.vars.get(&id.text.to_ascii_uppercase())
        {
            return Ok(f.clone());
        }
        if let ExprKind::Member(base, m) = &x.kind
            && let Some(p) = self.fb_place(base, scope)?
            && let Some(fb) = self.iface(&p)
            && let Some((_, field, _, _)) = fb.param(&m.text)
        {
            return Ok(format!("{}.{field}", fb.lv));
        }
        self.not_yet(x.span, "ADR of this")
    }

    /// `base^`, `base` a pointer: what it points to, `Lv::Ptr`, each use
    /// checked as it reads or writes.
    pub(crate) fn ptr_deref(&mut self, base: &Expr, scope: &Scope) -> R<Option<(Lv, DTy)>> {
        let Some(to) = self.ptr_target(base, scope)? else {
            return self.not_yet(base.span, "dereferencing this pointer");
        };
        let p = self.ptr_value(base, scope)?;
        self.may_fault = true;
        if let Kind::Str(..) = to.kind {
            return self.ptr_chars(base, &p, &to).map(Some);
        }
        Ok(Some((Lv::Ptr(p), to)))
    }

    /// A STRING through the pointer `p`: its characters as a C `char`
    /// array where they lie, which C lets alias any bytes and needs no
    /// alignment for; out of its variable a fault and scratch memory
    /// (`ctx->nowhere`), the statement stopped before it ends. A WSTRING's
    /// units are 16 bits, whose alignment no pointer promises: refused.
    pub(crate) fn ptr_chars(&mut self, base: &Expr, p: &str, to: &DTy) -> R<(Lv, DTy)> {
        let Kind::Str(n, false) = to.kind else {
            return self.not_yet(base.span, "a WSTRING through a pointer: read its units through a POINTER TO WORD");
        };
        self.nowhere.insert(format!("char @[{}]", n + 1));
        self.may_fault = true;
        let c =
            format!("(*(char (*)[{}])iecf_ptr_chars(&ctx->fault, &ctx->image, {p}, {}u, &ctx->nowhere))", n + 1, n + 1);
        Ok((Lv::C(c), to.clone()))
    }

    /// `a op b` where an operand is a pointer, as a C value of the
    /// result's type: None when neither is.
    pub(crate) fn ptr_binary(&mut self, op: BinaryOp, a: &Expr, b: &Expr, scope: &Scope) -> R<Option<String>> {
        use BinaryOp::*;
        if !self.is_pointer(a) && !self.is_pointer(b) {
            return Ok(None);
        }
        let (x, y) = (self.ptr_value(a, scope), self.ptr_value(b, scope));
        Ok(Some(match op {
            Eq => format!("iecf_ptr_eq({}, {})", x?, y?),
            Ne => format!("((IEC_BOOL)!iecf_ptr_eq({}, {}))", x?, y?),
            Lt | Le | Gt | Ge => {
                self.may_fault = true;
                let sym = match op {
                    Lt => "<",
                    Le => "<=",
                    Gt => ">",
                    _ => ">=",
                };
                format!("((IEC_BOOL)(iecf_ptr_cmp({FAULT}, {}, {}) {sym} 0))", x?, y?)
            }
            Sub if self.is_pointer(a) && self.is_pointer(b) => {
                self.may_fault = true;
                self.may_flag = true;
                format!("iecf_ptr_diff({FAULT}, {FLAG}, {}, {})", x?, y?)
            }
            _ => return self.not_yet(a.span, "this operation on a pointer"),
        }))
    }

    /// The initial values `ADR(x)` left in `text`, now that `scope` has
    /// the POU's variables.
    pub(crate) fn fill_ptr_inits(&mut self, text: &mut String, scope: &Scope) -> R<()> {
        for n in 0..self.ptr_inits.len() {
            let m = marker(n);
            if !text.contains(&m) {
                continue;
            }
            let (lv, x) = self.ptr_inits[n].clone();
            self.may_fault = false;
            self.may_flag = false;
            let v = self.ptr_value(&x, scope)?;
            if self.may_fault || self.may_flag {
                return self.not_yet(x.span, "an initial value that can fault or be flagged");
            }
            *text = text.replace(&m, &format!("{lv} = {v};"));
        }
        Ok(())
    }

    /// `SIZEOF(x)`: the C size of `x`'s type on the platform, as the
    /// checker's unsigned type.
    pub(crate) fn sizeof_value(&mut self, e: &Expr, x: &Expr, scope: &Scope) -> R<String> {
        let t = self.elem_of(e)?;
        let d = match self.types.sizeof_type(x.span).cloned() {
            Some(Ty::Elem(el)) => Some(DTy { kind: Kind::Elem(el), init: None, range: None }),
            Some(_) => match &x.kind {
                ExprKind::Name(id) => self.derived(&TypeSpec::Named(id.clone()))?,
                _ => None,
            },
            None => self.place_type(x, scope)?,
        };
        let Some(d) = d else { return self.not_yet(x.span, "SIZEOF of this") };
        let c = self.declare(&d, "");
        // The type was chosen from iecf's layout; that the platform's size
        // fits it is checked as the C compiles: an array of -1 elements
        // otherwise, never a value cut.
        let max = t.int_range().map_or(u64::MAX as i128, |(_, hi)| hi);
        Ok(format!(
            "(({})(sizeof({c}) + 0u * sizeof(char[(sizeof({c}) <= UINT64_C({max})) ? 1 : -1])))",
            crate::c_type(t)
        ))
    }
}

/// The variable a place is a part of: its name, unless through a pointer
/// or a reference.
fn root(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        ExprKind::Name(_) => Some(e),
        ExprKind::Paren(b) | ExprKind::Index(b, _) | ExprKind::Member(b, _) => root(b),
        _ => None,
    }
}

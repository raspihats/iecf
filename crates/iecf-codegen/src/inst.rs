//! Function block and class instances where Ed.3 allows them besides a
//! POU's own variable: with an initial value (6.6.3.3, Table 41 row 2;
//! 6.6.5.3, Table 49), as an array's element or a structure's field
//! (Table 11 rows 5a-7b), and in VAR_GLOBAL (6.5.2.2).
//!
//! An instance is set up by its type's `_init` (`B_<NAME>_init`, or the
//! runtime's `iecf_<stem>_init`), then each variable its initial value
//! names is set: an input, an output or a PUBLIC variable, as the checker
//! allowed. A variable the instance keeps across a warm start keeps it
//! then, its initial value with it, as the type's own initial values are
//! kept (lib.rs `block`, docs/c-output.md §7). An element or a field is
//! reached as any other (`arr[i]`, `s.t`), its index checked, and is an
//! instance of exactly its type: no derived one, so its methods and its
//! body are bound statically.

use crate::derived::{DTy, Kind};
use crate::names::{Lv, Place, Scope};
use crate::{Gen, R};
use iecf_syntax::ast::{Expr, ExprKind, Init, PouKind, Retain, TypeDef, TypeSpec, VarKind};
use std::fmt::Write as _;

impl Gen<'_> {
    /// The instance at `lv`, of `kind`, set up and given `given`: into
    /// `out`. `self.init_keep` is its `_init`'s `keep`.
    pub(crate) fn init_instance(&mut self, lv: &str, kind: &Kind, given: Option<&Init>, out: &mut String) -> R<()> {
        let set = match given {
            None => Vec::new(),
            Some(Init::Struct { fields, .. }) => fields.clone(),
            Some(Init::Expr(e)) => return self.not_yet(e.span, "this initial value of an instance"),
            Some(Init::Array { span, .. }) => return self.not_yet(*span, "this initial value of an instance"),
        };
        match kind {
            Kind::Std(fb) => {
                let _ = writeln!(out, "    iecf_{}_init(&{lv});", fb.stem);
                for (n, i) in &set {
                    let (Some((name, t, _)), Init::Expr(x)) = (fb.param(&n.text).cloned(), i) else {
                        return self.not_yet(n.span, format!("this initial value of {} of {}", n.text, fb.stem));
                    };
                    let v = self.init_value(x, t)?;
                    let _ = writeln!(out, "    {lv}.{name} = {v};");
                }
            }
            Kind::Block(k) => {
                let keep = self.init_keep.clone();
                let _ = writeln!(out, "    B_{k}_init(ctx, &{lv}, {keep});");
                let Some(b) = self.blocks.get(k).cloned() else {
                    // Its block is being written: it reached this one
                    // through an in-out of this one's type.
                    return self.not_yet(
                        self.pous[k.as_str()].name.span,
                        format!("an instance of {k} held by a block that {k} reaches through an in-out"),
                    );
                };
                for (n, i) in &set {
                    let name = n.text.to_ascii_uppercase();
                    let field = format!("{lv}.{}", b.field(&name));
                    let mut code = String::new();
                    if let Some((_, t, _)) = b.params.iter().find(|p| p.0 == name) {
                        let Init::Expr(x) = i else {
                            return self.not_yet(n.span, format!("this initial value of {}", n.text));
                        };
                        if let (Some((lo, hi)), Some(v)) = (b.ranges.get(&name), crate::expr::const_int(x))
                            && (v < *lo || v > *hi)
                        {
                            self.diags.push(iecf_diagnostics::Diagnostic::new(
                                iecf_diagnostics::Code::LiteralRange,
                                x.span,
                                format!("{v} is outside the subrange {lo}..{hi}"),
                            ));
                            return Err(crate::Skip);
                        }
                        let v = self.init_value(x, *t)?;
                        let _ = writeln!(code, "    {field} = {v};");
                    } else if let Some((_, t, _)) = b.data.iter().find(|d| d.0 == name) {
                        self.init_data(&field, t, Some(i), &mut code)?;
                    } else {
                        return self.not_yet(n.span, format!("an initial value of {}", n.text));
                    }
                    // Kept across a warm start as the type keeps it; a
                    // `keep` that is never 2 keeps only the RETAIN ones.
                    let guard = match (keep.as_str(), self.member_retain(k, &name)) {
                        ("0", _) | (_, Some(Retain::NonRetain)) => None,
                        (_, Some(Retain::Retain)) => Some(format!("({keep}) == 0")),
                        (_, None) if !keep.contains('2') => None,
                        (_, None) => Some(format!("({keep}) != 2")),
                    };
                    out.push_str(&guarded(guard.as_deref(), &code));
                }
            }
            _ => unreachable!("an instance's kind"),
        }
        Ok(())
    }

    /// Whether the variable `name` of the type `key`, or of a base, is in
    /// a RETAIN or NON_RETAIN section.
    fn member_retain(&self, key: &str, name: &str) -> Option<Retain> {
        let mut k = Some(key.to_string());
        while let Some(t) = k {
            let pou = self.pous.get(t.as_str())?;
            for b in &pou.var_blocks {
                if b.decls.iter().any(|d| d.names.iter().any(|n| n.text.eq_ignore_ascii_case(name))) {
                    return b.retain;
                }
            }
            k = self.types.base(&t).map(str::to_string);
        }
        None
    }

    /// The instance an element or a field `e` is, `arr[i]` or `s.t`:
    /// reached as data (expr.rs `data_path`), the index checked there.
    pub(crate) fn data_instance(&mut self, e: &Expr, scope: &Scope) -> R<Option<Place>> {
        if !matches!(e.kind, ExprKind::Index(..) | ExprKind::Member(..) | ExprKind::Call { .. }) {
            return Ok(None);
        }
        Ok(match self.data_path(e, scope)? {
            Some((Lv::C(lv), t)) => match t.kind {
                Kind::Block(k) => Some(Place::Block(lv, k)),
                Kind::Std(fb) => Some(Place::Fb(lv, fb)),
                _ => None,
            },
            _ => None,
        })
    }
}

impl Gen<'_> {
    /// The instance `e` names, for a call on it or one it is given: an
    /// index that can fault checked on a line of its own first, so one
    /// out of bounds stops the statement before anything runs on another
    /// element (its address hoisted, `pre_ok`). A reference's `^` is its
    /// callers' to check (refs.rs).
    pub(crate) fn call_instance(&mut self, e: &Expr, scope: &Scope) -> R<Option<Place>> {
        let was = std::mem::replace(&mut self.may_fault, false);
        let p = self.fb_place(e, scope)?;
        let deref = matches!(crate::expr::strip_paren(e).kind, ExprKind::Deref(_));
        let out = match p {
            Some(p @ (Place::Block(..) | Place::Fb(..))) if self.may_fault && !deref => {
                if !self.pre_ok {
                    return self.not_yet(e.span, "an instance at an index that is checked, here");
                }
                let (ty, lv) = match &p {
                    Place::Block(lv, k) => (format!("B_{k}_t"), lv.clone()),
                    Place::Fb(lv, fb) => (format!("iecf_{}_t", fb.stem), lv.clone()),
                    _ => unreachable!("matched above"),
                };
                let o = self.fresh("o");
                self.str_temps.push(format!("{ty} *{o} = &({lv});"));
                self.str_temps.push(crate::enable::STOP.to_string());
                self.may_fault = false;
                Some(match p {
                    Place::Block(_, k) => Place::Block(format!("(*{o})"), k),
                    Place::Fb(_, fb) => Place::Fb(format!("(*{o})"), fb),
                    p => p,
                })
            }
            p => p,
        };
        self.may_fault |= was;
        Ok(out)
    }
}

/// `code`'s lines run only when `guard` holds.
pub(crate) fn guarded(guard: Option<&str>, code: &str) -> String {
    match guard {
        None => code.to_string(),
        Some(g) => {
            let mut s = format!("    if ({g}) {{\n");
            for l in code.lines() {
                let _ = writeln!(s, "    {l}");
            }
            s.push_str("    }\n");
            s
        }
    }
}

impl Gen<'_> {
    /// Whether `spec` names a FUNCTION_BLOCK or CLASS anywhere in it, a
    /// REF_TO one too: a global of it is placed after the others, as its
    /// type's body may name them (lib.rs `configuration`). `depth` stops
    /// a type that contains itself, which the checker refused.
    pub(crate) fn names_block(&self, spec: &TypeSpec, depth: u32) -> bool {
        if depth > 64 {
            return false;
        }
        match spec {
            TypeSpec::Named(id) => {
                let key = id.text.to_ascii_uppercase();
                if self.pous.get(&key).is_some_and(|p| matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)) {
                    return true;
                }
                match self.type_decls.get(&key).map(|d| &d.def) {
                    Some(TypeDef::Spec(s)) => self.names_block(s, depth + 1),
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().any(|f| self.names_block(&f.ty, depth + 1)),
                    None => false,
                }
            }
            TypeSpec::Array { elem, .. } => self.names_block(elem, depth + 1),
            TypeSpec::Ref { target, .. } => self.names_block(target, depth + 1),
            _ => false,
        }
    }

    /// Whether an instance of the user's `key` keeps something across a
    /// warm start, read from the declarations: lib.rs `block`'s
    /// `retentive`, known before the type is written.
    pub(crate) fn type_keeps(&self, key: &str, depth: u32) -> bool {
        let Some(pou) = self.pous.get(key).filter(|_| depth <= 64) else { return false };
        if self.types.base(key).is_some_and(|b| self.type_keeps(b, depth + 1)) {
            return true;
        }
        pou.var_blocks.iter().any(|b| {
            b.retain == Some(Retain::Retain)
                || (b.kind == VarKind::Var && b.decls.iter().any(|d| self.spec_keeps(&d.ty, depth + 1)))
        })
    }

    fn spec_keeps(&self, spec: &TypeSpec, depth: u32) -> bool {
        if depth > 64 {
            return false;
        }
        match spec {
            TypeSpec::Named(id) => {
                let key = id.text.to_ascii_uppercase();
                if self.pous.get(&key).is_some_and(|p| matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)) {
                    return self.type_keeps(&key, depth + 1);
                }
                match self.type_decls.get(&key).map(|d| &d.def) {
                    Some(TypeDef::Spec(s)) => self.spec_keeps(s, depth + 1),
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().any(|f| self.spec_keeps(&f.ty, depth + 1)),
                    None => false,
                }
            }
            TypeSpec::Array { elem, .. } => self.spec_keeps(elem, depth + 1),
            _ => false,
        }
    }
}

/// A whole instance at `p` as a value: where it is and its type, to be
/// copied as a structure is (Ed.3 Table 72, 1d; docs/dependencies.md).
pub(crate) fn whole(p: &Place) -> Option<(Lv, DTy)> {
    let (lv, kind) = match p {
        Place::Block(lv, k) => (lv, Kind::Block(k.clone())),
        Place::Fb(lv, fb) => (lv, Kind::Std(fb.clone())),
        _ => return None,
    };
    Some((Lv::C(lv.clone()), DTy { kind, init: None, range: None }))
}

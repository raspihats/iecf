//! A block's or class's constant read through its type's name (the
//! type-constants extension): `T.C`, C a `VAR CONSTANT` of the
//! FUNCTION_BLOCK or CLASS T with an initial value, is that value, as
//! TwinCAT reads one (`STRING(TcoMessengerEnv.MAX_MESSAGE_TEXT_LENGHT)`).
//! Written so before the check, the value typed as C is: anywhere a
//! constant may be - a STRING's length, an array's bounds, a body. A
//! value naming another of T's constants names it the same way. A POU
//! that declares a variable named T means its variable.

use crate::codesys_oo::{expr, var_names, walk_pou};
use iecf_diagnostics::Span;
use iecf_syntax::ast::*;
use std::collections::HashMap;

/// Each block's and class's constants with values: by the type's
/// upper-cased name, each constant's upper-cased name, its type and its
/// value.
type Consts = HashMap<String, Vec<(String, TypeSpec, Expr)>>;

fn consts(units: &[SourceUnit]) -> Consts {
    let mut out = Consts::new();
    for unit in units {
        for item in &unit.items {
            let Item::Pou(p) = item else { continue };
            if !matches!(p.kind, PouKind::FunctionBlock | PouKind::Class) {
                continue;
            }
            for b in p.var_blocks.iter().filter(|b| b.constant && b.kind == VarKind::Var) {
                for d in &b.decls {
                    let Some(Init::Expr(e)) = &d.init else { continue };
                    for n in &d.names {
                        out.entry(p.name.text.to_ascii_uppercase()).or_default().push((
                            n.text.to_ascii_uppercase(),
                            d.ty.clone(),
                            e.clone(),
                        ));
                    }
                }
            }
        }
    }
    out
}

/// `units` with each `T.C` written as C's value.
pub fn rewrite(units: &mut [SourceUnit]) {
    let all = consts(units);
    if all.is_empty() {
        return;
    }
    let names: Vec<String> = all.keys().cloned().collect();
    let mut f = |e: &mut Expr| {
        if let Some(v) = value(&all, e, 0) {
            *e = v;
        }
    };
    for unit in units.iter_mut() {
        for item in unit.items.iter_mut() {
            match item {
                Item::Pou(p) => {
                    let mut own = var_names(&p.var_blocks);
                    for m in &p.methods {
                        own.extend(var_names(&m.var_blocks));
                    }
                    if own.iter().any(|v| names.contains(v)) {
                        continue;
                    }
                    for b in p.var_blocks.iter_mut().chain(p.methods.iter_mut().flat_map(|m| m.var_blocks.iter_mut())) {
                        for d in b.decls.iter_mut() {
                            spec(&mut d.ty, &mut f);
                            if let Some(Init::Expr(e)) = &mut d.init {
                                expr(e, &mut f);
                            }
                        }
                    }
                    if let Some(r) = &mut p.return_type {
                        spec(r, &mut f);
                    }
                    for m in p.methods.iter_mut() {
                        if let Some(r) = &mut m.return_type {
                            spec(r, &mut f);
                        }
                    }
                    walk_pou(p, &mut f);
                }
                Item::Globals(b) => {
                    for d in b.decls.iter_mut() {
                        spec(&mut d.ty, &mut f);
                        if let Some(Init::Expr(e)) = &mut d.init {
                            expr(e, &mut f);
                        }
                    }
                }
                Item::Types(ts) => {
                    for t in ts.iter_mut() {
                        match &mut t.def {
                            TypeDef::Spec(s) => spec(s, &mut f),
                            TypeDef::Struct { fields, .. } => fields.iter_mut().for_each(|d| spec(&mut d.ty, &mut f)),
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// The lengths and bounds a type written in place has.
fn spec(t: &mut TypeSpec, f: &mut dyn FnMut(&mut Expr)) {
    match t {
        TypeSpec::String { len: Some(l), .. } => expr(l, f),
        TypeSpec::Array { dims, elem, .. } => {
            for (lo, hi) in dims.iter_mut().flatten() {
                expr(lo, f);
                expr(hi, f);
            }
            spec(elem, f);
        }
        TypeSpec::Subrange { low, high, .. } => {
            expr(low, f);
            expr(high, f);
        }
        TypeSpec::Ref { target, .. } | TypeSpec::Pointer { target, .. } => spec(target, f),
        _ => {}
    }
}

/// `T.C` of `all`: C's value, at `e`'s place; its own `D` of T's
/// constants written so too, `depth` deep.
fn value(all: &Consts, e: &Expr, depth: u32) -> Option<Expr> {
    let ExprKind::Member(base, c) = &e.kind else { return None };
    let ExprKind::Name(t) = &base.kind else { return None };
    // By its whole name, or - a library's, used by its short one - by
    // the last part of exactly one.
    let key = t.text.to_ascii_uppercase();
    let list = all.get(&key).or_else(|| {
        let tail = format!("{}{key}", crate::namespaces::SEP);
        let mut hits = all.iter().filter(|(k, _)| k.ends_with(&tail));
        match (hits.next(), hits.next()) {
            (Some((_, l)), None) => Some(l),
            _ => None,
        }
    })?;
    let (_, ty, v) = list.iter().find(|(n, _, _)| c.text.eq_ignore_ascii_case(n))?;
    if depth > 16 {
        return None;
    }
    let mut v = v.clone();
    // Each part at a place of its own inside the use: what is checked is
    // keyed by it.
    let mut n = 0u32;
    let mut parts = |x: &mut Expr| {
        // A bare name of another of T's constants: T's.
        if let ExprKind::Name(id) = &x.kind
            && list.iter().any(|(k, _, _)| id.text.eq_ignore_ascii_case(k))
        {
            let qualified = Expr {
                kind: ExprKind::Member(Box::new(Expr { kind: ExprKind::Name(t.clone()), span: x.span }), id.clone()),
                span: x.span,
            };
            if let Some(w) = value(all, &qualified, depth + 1) {
                *x = w;
            }
        }
        n += 1;
        let o = e.span.start + n.min(e.span.end - e.span.start);
        x.span = Span::new(e.span.file, o, o);
    };
    expr(&mut v, &mut parts);
    v.span = e.span;
    // An untyped literal typed as the constant is.
    if let (ExprKind::Literal(l), TypeSpec::Named(tn)) = (&mut v.kind, ty)
        && l.type_prefix.is_none()
        && matches!(l.kind, LiteralKind::Int | LiteralKind::Real | LiteralKind::Based)
        && crate::types::Elem::from_name(&tn.text).is_some()
    {
        l.type_prefix = Some(Ident { text: tn.text.clone(), span: e.span });
        l.span = e.span;
    }
    Some(v)
}

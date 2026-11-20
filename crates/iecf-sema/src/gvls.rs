//! Global variable lists (`--gvl NAME=FILE`), as CODESYS and TwinCAT
//! have them: the VAR_GLOBAL sections of a file are the list NAME, and
//! `NAME.x` names its variable x anywhere. A list is an object of a
//! CODESYS project, which no ST text names; the command line does.
//!
//! Written so before the check: `NAME.x` is `x`, a global as any other;
//! a qualified-only list's (`--qualified-gvl`, CODESYS's
//! `{attribute 'qualified_only'}`) is the global `NAME__x`, shown
//! `NAME.x` (namespaces.rs), which no plain `x` names. A POU that
//! declares a variable of the list's name means its variable.

use crate::codesys_oo::{expr, var_names, walk_pou};
use crate::namespaces::SEP;
use iecf_syntax::ast::*;

/// A list: its name, whether only `NAME.x` reaches it, and its file's
/// place among the units.
pub struct Gvl<'a> {
    pub name: &'a str,
    pub qualified: bool,
    pub unit: usize,
}

/// `units` with the lists `gvls` named as each says.
pub fn rewrite(units: &mut [SourceUnit], gvls: &[Gvl]) {
    // Each list: its upper-cased name, its variables, and what each is
    // then called.
    let mut lists: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for g in gvls {
        let Some(unit) = units.get_mut(g.unit) else { continue };
        let mut vars = Vec::new();
        globals(&mut unit.items, &mut |b| {
            for d in b.decls.iter_mut() {
                for n in d.names.iter_mut() {
                    let to = if g.qualified { format!("{}{SEP}{}", g.name, n.text) } else { n.text.clone() };
                    vars.push((n.text.to_ascii_uppercase(), to.clone()));
                    n.text = to;
                }
            }
        });
        // Inside the list a variable is named bare: `ARRAY[1..N]`.
        if g.qualified {
            globals(&mut unit.items, &mut |b| {
                inits(std::slice::from_mut(b), &mut |e| {
                    if let ExprKind::Name(id) = &mut e.kind
                        && let Some((_, to)) = vars.iter().find(|(v, _)| id.text.eq_ignore_ascii_case(v))
                    {
                        id.text = to.clone();
                    }
                })
            });
        }
        match lists.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(g.name)) {
            Some((_, v)) => v.extend(vars),
            None => lists.push((g.name.to_ascii_uppercase(), vars)),
        }
    }
    if lists.is_empty() {
        return;
    }
    let mut f = |e: &mut Expr| {
        if let ExprKind::Member(base, m) = &e.kind
            && let ExprKind::Name(g) = &base.kind
            && let Some((_, vars)) = lists.iter().find(|(n, _)| g.text.eq_ignore_ascii_case(n))
            && let Some((_, to)) = vars.iter().find(|(v, _)| m.text.eq_ignore_ascii_case(v))
        {
            e.kind = ExprKind::Name(Ident { text: to.clone(), span: m.span });
        }
    };
    let names: Vec<String> = lists.iter().map(|(n, _)| n.clone()).collect();
    for unit in units.iter_mut() {
        items(&mut unit.items, &names, &mut f);
    }
}

/// Each VAR_GLOBAL section of `items`, at any depth of namespace.
fn globals(items: &mut [Item], f: &mut dyn FnMut(&mut VarBlock)) {
    for i in items {
        match i {
            Item::Globals(b) => f(b),
            Item::Namespace(n) => globals(&mut n.items, f),
            _ => {}
        }
    }
}

/// Every expression of `items` - POUs, their initial values, the
/// globals' - but in a POU declaring a variable one of `lists` names.
fn items(list: &mut [Item], lists: &[String], f: &mut dyn FnMut(&mut Expr)) {
    for i in list {
        match i {
            Item::Pou(p) => {
                let mut own = var_names(&p.var_blocks);
                for m in &p.methods {
                    own.extend(var_names(&m.var_blocks));
                }
                if own.iter().any(|v| lists.contains(v)) {
                    continue;
                }
                inits(&mut p.var_blocks, f);
                for m in p.methods.iter_mut() {
                    inits(&mut m.var_blocks, f);
                }
                walk_pou(p, f);
            }
            Item::Globals(b) => inits(std::slice::from_mut(b), f),
            Item::Types(ts) => {
                for t in ts.iter_mut() {
                    match &mut t.def {
                        TypeDef::Spec(s) => spec(s, f),
                        TypeDef::Struct { fields, .. } => fields.iter_mut().for_each(|d| decl(d, f)),
                    }
                }
            }
            Item::Namespace(n) => items(&mut n.items, lists, f),
            _ => {}
        }
    }
}

/// The lengths, bounds and values a type written in place names.
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
        TypeSpec::Ref { target, .. } | TypeSpec::Pointer { target, .. } => spec(target, f),
        TypeSpec::Subrange { low, high, .. } => {
            expr(low, f);
            expr(high, f);
        }
        TypeSpec::Enum { values, .. } => values.iter_mut().filter_map(|v| v.value.as_mut()).for_each(|e| expr(e, f)),
        TypeSpec::Named(_) | TypeSpec::String { len: None, .. } => {}
    }
}

/// A declaration's type and initial value.
fn decl(d: &mut VarDecl, f: &mut dyn FnMut(&mut Expr)) {
    spec(&mut d.ty, f);
    if let Some(i) = &mut d.init {
        init(i, f);
    }
}

fn inits(blocks: &mut [VarBlock], f: &mut dyn FnMut(&mut Expr)) {
    for b in blocks {
        b.decls.iter_mut().for_each(|d| decl(d, f));
    }
}

fn init(i: &mut Init, f: &mut dyn FnMut(&mut Expr)) {
    {
        match i {
            Init::Expr(e) => expr(e, f),
            Init::Array { elems, .. } => {
                for el in elems.iter_mut() {
                    if let Some(r) = &mut el.repeat {
                        expr(r, f);
                    }
                    if let Some(v) = &mut el.value {
                        init(v, f);
                    }
                    for m in el.more.iter_mut() {
                        init(m, f);
                    }
                }
            }
            Init::Struct { fields, .. } => fields.iter_mut().for_each(|(_, v)| init(v, f)),
        }
    }
}

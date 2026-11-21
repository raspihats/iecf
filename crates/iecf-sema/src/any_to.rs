//! CODESYS's and TwinCAT's `ANY_TO_<type>(x)` (the any-to extension):
//! x converted to <type>, whatever x's type, which is Ed.3's overloaded
//! conversion `TO_<type>(x)` (Table 22, feature 1b). Written so before
//! anything is checked, so what `TO_<type>` takes and gives is what
//! `ANY_TO_<type>` does; a FUNCTION the project declares with such a
//! name is called as it is.

use crate::codesys_oo::{expr, walk_pou};
use iecf_syntax::ast::*;

const PREFIX: &str = "ANY_TO_";

/// Whether `units` call some `ANY_TO_<type>`.
pub fn needed(units: &[&SourceUnit]) -> bool {
    let mut copy: Vec<SourceUnit> = units.iter().map(|u| (*u).clone()).collect();
    let mut found = false;
    each(&mut copy, &mut |e| found |= callee(e).is_some());
    found
}

/// Each call of `ANY_TO_<type>` written `TO_<type>`, unless the project
/// declares a POU of that name.
pub fn rewrite(units: &mut [SourceUnit]) {
    let declared: Vec<String> = units
        .iter()
        .flat_map(|u| u.items.iter())
        .filter_map(|i| match i {
            Item::Pou(p) => Some(p.name.text.to_ascii_uppercase()),
            _ => None,
        })
        .collect();
    each(units, &mut |e| {
        if let Some(id) = callee(e)
            && !declared.contains(&id.text.to_ascii_uppercase())
        {
            id.text = format!("TO_{}", &id.text[PREFIX.len()..]);
        }
    });
}

/// The name a call of `ANY_TO_<type>` calls.
fn callee(e: &mut Expr) -> Option<&mut Ident> {
    let ExprKind::Call { callee, .. } = &mut e.kind else { return None };
    let ExprKind::Name(id) = &mut callee.kind else { return None };
    let upper = id.text.to_ascii_uppercase();
    (upper.len() > PREFIX.len() && upper.starts_with(PREFIX)).then_some(id)
}

/// Every expression of `units`: the POUs' and the initial values.
fn each(units: &mut [SourceUnit], f: &mut dyn FnMut(&mut Expr)) {
    fn inits(blocks: &mut [VarBlock], f: &mut dyn FnMut(&mut Expr)) {
        for d in blocks.iter_mut().flat_map(|b| b.decls.iter_mut()) {
            if let Some(i) = &mut d.init {
                init(i, f);
            }
        }
    }
    fn init(i: &mut Init, f: &mut dyn FnMut(&mut Expr)) {
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
                    el.more.iter_mut().for_each(|m| init(m, f));
                }
            }
            Init::Struct { fields, .. } => fields.iter_mut().for_each(|(_, v)| init(v, f)),
        }
    }
    for unit in units.iter_mut() {
        for item in unit.items.iter_mut() {
            match item {
                Item::Pou(p) => {
                    inits(&mut p.var_blocks, f);
                    for m in p.methods.iter_mut() {
                        inits(&mut m.var_blocks, f);
                    }
                    walk_pou(p, f);
                }
                Item::Globals(b) => inits(std::slice::from_mut(b), f),
                _ => {}
            }
        }
    }
}

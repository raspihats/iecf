//! CODESYS's `__SYSTEM.IQueryInterface` (the query-interface extension):
//! the interface CODESYS has every interface that `__QUERYINTERFACE`
//! is given extend. It declares nothing:
//!
//! ```text
//! INTERFACE __SYSTEM.IQueryInterface END_INTERFACE
//! ```
//!
//! Written so before the check: each `__SYSTEM.IQueryInterface` is named
//! `__SYSTEM__IQueryInterface`, as namespaces.rs names an element, and
//! the interface declared where one is named and it is not yet. The
//! checker takes `__QUERYINTERFACE(a, b)` (check.rs `query_itf_call`).

use iecf_diagnostics::{FileId, Span};
use iecf_syntax::ast::*;

/// The interface, as the check sees it.
pub const IQUERY: &str = "__SYSTEM__IQueryInterface";
/// As a program writes it.
const WRITTEN: &str = "__SYSTEM.IQueryInterface";

/// Whether `units` name `__SYSTEM.IQueryInterface`.
pub fn needed(units: &[&SourceUnit]) -> bool {
    let mut copy: Vec<SourceUnit> = units.iter().map(|u| (*u).clone()).collect();
    let mut file = None;
    for u in copy.iter_mut() {
        items(&mut u.items, &mut file);
    }
    file.is_some()
}

/// `units` with `__SYSTEM.IQueryInterface` named as above, and the
/// interface declared where some name needs it and it is not yet.
pub fn rewrite(units: &mut [SourceUnit]) {
    let mut file = None;
    for unit in units.iter_mut() {
        items(&mut unit.items, &mut file);
    }
    let Some(file) = file else { return };
    let declared = units
        .iter()
        .flat_map(|u| u.items.iter())
        .any(|i| matches!(i, Item::Pou(p) if p.name.text.eq_ignore_ascii_case(IQUERY)));
    if declared {
        return;
    }
    // A place of its own, past any text, any.rs's and var_info.rs's.
    let at = Span::new(file, u32::MAX - 12288, u32::MAX - 12288);
    let itf = Pou {
        kind: PouKind::Interface,
        name: Ident { text: IQUERY.into(), span: at },
        return_type: None,
        var_blocks: Vec::new(),
        accesses: Vec::new(),
        body: Vec::new(),
        sfc: Vec::new(),
        internal: false,
        using: Vec::new(),
        specs: Specs::default(),
        methods: Vec::new(),
        extends: None,
        interfaces: Vec::new(),
        header_semicolon: None,
        span: at,
    };
    if let Some(u) = units.first_mut() {
        u.items.insert(0, Item::Pou(itf));
    }
}

/// Each `__SYSTEM.IQueryInterface` of `list` named as above; `file`,
/// the first file one is in.
fn items(list: &mut [Item], file: &mut Option<FileId>) {
    for i in list {
        match i {
            Item::Pou(p) => pou(p, file),
            Item::Globals(b) => b.decls.iter_mut().for_each(|d| spec(&mut d.ty, file)),
            Item::Types(ts) => {
                for t in ts.iter_mut() {
                    match &mut t.def {
                        TypeDef::Struct { fields, .. } => fields.iter_mut().for_each(|d| spec(&mut d.ty, file)),
                        TypeDef::Spec(s) => spec(s, file),
                    }
                }
            }
            Item::Namespace(n) => items(&mut n.items, file),
            _ => {}
        }
    }
}

fn pou(p: &mut Pou, file: &mut Option<FileId>) {
    for i in p.interfaces.iter_mut() {
        name(i, file);
    }
    if let Some(t) = p.return_type.as_mut() {
        spec(t, file);
    }
    for b in p.var_blocks.iter_mut() {
        b.decls.iter_mut().for_each(|d| spec(&mut d.ty, file));
    }
    for m in p.methods.iter_mut() {
        pou(m, file);
    }
}

fn name(id: &mut Ident, file: &mut Option<FileId>) {
    if id.text.eq_ignore_ascii_case(WRITTEN) {
        file.get_or_insert(id.span.file);
        id.text = IQUERY.into();
    }
}

fn spec(t: &mut TypeSpec, file: &mut Option<FileId>) {
    match t {
        TypeSpec::Named(id) => name(id, file),
        TypeSpec::Array { elem, .. } => spec(elem, file),
        TypeSpec::Ref { target, .. } | TypeSpec::Pointer { target, .. } => spec(target, file),
        _ => {}
    }
}

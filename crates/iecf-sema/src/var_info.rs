//! CODESYS's and TwinCAT's `__VARINFO(x)` (the var-info extension): what
//! is known of the variable `x`, as a structure of CODESYS's:
//!
//! ```text
//! TYPE __SYSTEM.MEMORY_AREA : (MEM_GLOBAL, MEM_LOCAL, MEM_MEMORY,
//!     MEM_INPUT, MEM_OUTPUT, MEM_RETAIN); END_TYPE
//! TYPE __SYSTEM.VAR_INFO : STRUCT
//!     ByteAddress : DWORD;  ByteOffset : DWORD;  Area : DINT;
//!     BitNo : INT;  BitSize : INT;  BitAddress : UDINT;
//!     TypeClass : __SYSTEM.TYPE_CLASS;  TypeName : STRING(79);
//!     NumElements : UDINT;  BaseTypeClass : __SYSTEM.TYPE_CLASS;
//!     ElemBitSize : UDINT;
//!     MemoryArea : __SYSTEM.MEMORY_AREA := MEM_MEMORY;
//!     Symbol : STRING(39);  Comment : STRING(79);
//! END_STRUCT END_TYPE
//! ```
//!
//! Written so before the check: `__SYSTEM.VAR_INFO` is named
//! `__SYSTEM__VAR_INFO`, as namespaces.rs names an element, and the
//! types declared (`__SYSTEM.TYPE_CLASS` as any.rs declares it, where it
//! does not). The checker takes `__VARINFO(x)` of a variable, an element
//! or a field (check.rs `var_info_call`), and the generator fills the
//! structure from what it worked out.

use crate::any::{CLASSES, TYPE_CLASS, enum_decl};
use crate::codesys_oo::walk_pou;
use iecf_diagnostics::{FileId, Span};
use iecf_syntax::ast::*;

/// The structure `__VARINFO` gives.
pub const VAR_INFO: &str = "__SYSTEM__VAR_INFO";
/// The enumeration of memory areas.
pub const MEMORY_AREA: &str = "__SYSTEM__MEMORY_AREA";
/// `__SYSTEM.MEMORY_AREA`'s values, in CODESYS's order.
pub const AREAS: &[&str] = &["MEM_GLOBAL", "MEM_LOCAL", "MEM_MEMORY", "MEM_INPUT", "MEM_OUTPUT", "MEM_RETAIN"];
/// `__SYSTEM.VAR_INFO`'s members, in CODESYS's order, and their types.
pub const MEMBERS: &[(&str, &str)] = &[
    ("ByteAddress", "DWORD"),
    ("ByteOffset", "DWORD"),
    ("Area", "DINT"),
    ("BitNo", "INT"),
    ("BitSize", "INT"),
    ("BitAddress", "UDINT"),
    ("TypeClass", TYPE_CLASS),
    ("TypeName", "STRING79"),
    ("NumElements", "UDINT"),
    ("BaseTypeClass", TYPE_CLASS),
    ("ElemBitSize", "UDINT"),
    ("MemoryArea", MEMORY_AREA),
    ("Symbol", "STRING39"),
    ("Comment", "STRING79"),
];

/// Whether `units` call `__VARINFO` or name `__SYSTEM.VAR_INFO`.
pub fn needed(units: &[&SourceUnit]) -> bool {
    let mut copy: Vec<SourceUnit> = units.iter().map(|u| (*u).clone()).collect();
    let mut file = None;
    for u in copy.iter_mut() {
        items(&mut u.items, &mut file);
    }
    file.is_some()
}

/// `units` with `__SYSTEM.VAR_INFO` and `__SYSTEM.MEMORY_AREA` named as
/// above, and the types declared where some name or `__VARINFO` call
/// needs them and they are not yet.
pub fn rewrite(units: &mut [SourceUnit]) {
    let mut file = None;
    for unit in units.iter_mut() {
        items(&mut unit.items, &mut file);
    }
    let Some(file) = file else { return };
    let declared = |name: &str| {
        units
            .iter()
            .flat_map(|u| u.items.iter())
            .any(|i| matches!(i, Item::Types(ts) if ts.iter().any(|t| t.name.text.eq_ignore_ascii_case(name))))
    };
    if declared(VAR_INFO) {
        return;
    }
    // Places of their own, past any text and any.rs's: what is checked
    // is keyed by them.
    let n = std::cell::Cell::new(0u32);
    let at = || {
        n.set(n.get() + 1);
        let o = u32::MAX - 8192 + n.get();
        Span::new(file, o, o)
    };
    let id = |text: &str| Ident { text: text.to_string(), span: at() };
    let mut types = Vec::new();
    if !declared(TYPE_CLASS) {
        types.push(enum_decl(TYPE_CLASS, CLASSES, &at));
    }
    types.push(enum_decl(MEMORY_AREA, AREAS, &at));
    let fields = MEMBERS
        .iter()
        .map(|(name, ty)| {
            let spec = match *ty {
                "STRING39" | "STRING79" => {
                    let len = Expr {
                        kind: ExprKind::Literal(Literal {
                            kind: LiteralKind::Int,
                            type_prefix: None,
                            negative: false,
                            text: ty[6..].to_string(),
                            span: at(),
                        }),
                        span: at(),
                    };
                    TypeSpec::String { name: id("STRING"), len: Some(Box::new(len)), parens: false, span: at() }
                }
                t => TypeSpec::Named(id(t)),
            };
            let init = (*name == "MemoryArea").then(|| {
                Init::Expr(Expr {
                    kind: ExprKind::EnumValue { ty: id(MEMORY_AREA), value: id("MEM_MEMORY") },
                    span: at(),
                })
            });
            VarDecl { names: vec![id(name)], location: None, ty: spec, edge: None, init, fb_args: None, span: at() }
        })
        .collect();
    types.push(TypeDecl {
        name: id(VAR_INFO),
        access: None,
        extends: None,
        union_kw: None,
        abstract_kw: None,
        def: TypeDef::Struct { overlap: false, fields, span: at() },
        init: None,
        no_semi: None,
        span: at(),
    });
    if let Some(u) = units.first_mut() {
        crate::put_first(u, Item::Types(types));
    }
}

/// Each `__SYSTEM.VAR_INFO` and `__SYSTEM.MEMORY_AREA` of `list` named as
/// above; `file`, the first file one, or a `__VARINFO` call, is in.
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
    for b in p.var_blocks.iter_mut().chain(p.methods.iter_mut().flat_map(|m| m.var_blocks.iter_mut())) {
        b.decls.iter_mut().for_each(|d| spec(&mut d.ty, file));
    }
    walk_pou(p, &mut |e| {
        if let ExprKind::Call { callee, .. } = &e.kind
            && let ExprKind::Name(id) = &callee.kind
            && id.text.eq_ignore_ascii_case("__VARINFO")
        {
            file.get_or_insert(id.span.file);
        }
        value(e, file);
    });
}

/// `__SYSTEM.MEMORY_AREA.MEM_GLOBAL`: the value,
/// `__SYSTEM__MEMORY_AREA#MEM_GLOBAL`.
fn value(e: &mut Expr, file: &mut Option<FileId>) {
    if let ExprKind::Member(base, v) = &e.kind
        && let ExprKind::Member(sys, area) = &base.kind
        && let ExprKind::Name(s) = &sys.kind
        && s.text.eq_ignore_ascii_case("__SYSTEM")
        && area.text.eq_ignore_ascii_case("MEMORY_AREA")
    {
        file.get_or_insert(s.span.file);
        let ty = Ident { text: MEMORY_AREA.into(), span: Span::new(s.span.file, s.span.start, area.span.end) };
        e.kind = ExprKind::EnumValue { ty, value: v.clone() };
    }
}

fn spec(t: &mut TypeSpec, file: &mut Option<FileId>) {
    match t {
        TypeSpec::Named(id) => {
            for (written, name) in [("__SYSTEM.VAR_INFO", VAR_INFO), ("__SYSTEM.MEMORY_AREA", MEMORY_AREA)] {
                if id.text.eq_ignore_ascii_case(written) {
                    file.get_or_insert(id.span.file);
                    id.text = name.into();
                }
            }
        }
        TypeSpec::Array { elem, .. } => spec(elem, file),
        TypeSpec::Ref { target, .. } | TypeSpec::Pointer { target, .. } => spec(target, file),
        _ => {}
    }
}

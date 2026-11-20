//! CODESYS's ANY as an input (the any-input extension): a FUNCTION's,
//! method's or block's `VAR_INPUT x : ANY` takes a value of any type,
//! and is, as CODESYS passes it, a structure of the value's type class,
//! a pointer to it and its size:
//!
//! ```text
//! TYPE __SYSTEM.TYPE_CLASS : (TYPE_BOOL, TYPE_BIT, ...); END_TYPE
//! TYPE __SYSTEM.ANYTYPE : STRUCT
//!     TypeClass : __SYSTEM.TYPE_CLASS;
//!     pValue : POINTER TO BYTE;
//!     diSize : DINT;
//! END_STRUCT END_TYPE
//! ```
//!
//! Written so before the check: the input's type is `__SYSTEM.ANYTYPE`
//! (named `__SYSTEM__ANYTYPE`, as namespaces.rs names an element), and
//! `__SYSTEM.TYPE_CLASS.TYPE_REAL` the enumerated value. The checker
//! takes any value for such an input (check.rs `coerce`), keeping its
//! type class, and the generator builds the structure at the call.

use crate::codesys_oo::{expr, walk_pou};
use iecf_diagnostics::Span;
use iecf_syntax::ast::*;

/// The enumeration of type classes, as written in a program.
pub const TYPE_CLASS: &str = "__SYSTEM__TYPE_CLASS";
/// The structure an ANY input is.
pub const ANY_TYPE: &str = "__SYSTEM__ANYTYPE";

/// `__SYSTEM.TYPE_CLASS`'s values, in CODESYS's order as far as known;
/// a program names them, and their numbers are iecf's own. LDATE, LTOD,
/// LDT, CHAR and WCHAR, which CODESYS's list does not name, come last.
pub const CLASSES: &[&str] = &[
    "TYPE_BOOL",
    "TYPE_BIT",
    "TYPE_BYTE",
    "TYPE_WORD",
    "TYPE_DWORD",
    "TYPE_LWORD",
    "TYPE_SINT",
    "TYPE_INT",
    "TYPE_DINT",
    "TYPE_LINT",
    "TYPE_USINT",
    "TYPE_UINT",
    "TYPE_UDINT",
    "TYPE_ULINT",
    "TYPE_REAL",
    "TYPE_LREAL",
    "TYPE_STRING",
    "TYPE_WSTRING",
    "TYPE_TIME",
    "TYPE_DATE",
    "TYPE_DATEANDTIME",
    "TYPE_TIMEOFDAY",
    "TYPE_POINTER",
    "TYPE_REFERENCE",
    "TYPE_SUBRANGE",
    "TYPE_ENUM",
    "TYPE_ARRAY",
    "TYPE_PARAMS",
    "TYPE_USERDEF",
    "TYPE_NONE",
    "TYPE_ANY",
    "TYPE_ANYBIT",
    "TYPE_ANYDATE",
    "TYPE_ANYINT",
    "TYPE_ANYNUM",
    "TYPE_ANYREAL",
    "TYPE_LAZY",
    "TYPE_LTIME",
    "TYPE_BITCONST",
    "TYPE_INTERFACE",
    "TYPE_LDATE",
    "TYPE_LTIMEOFDAY",
    "TYPE_LDATEANDTIME",
    "TYPE_CHAR",
    "TYPE_WCHAR",
];

/// `units` with each ANY input, `__SYSTEM.TYPE_CLASS` and its values
/// written as above, and the two types declared when any is.
pub fn rewrite(units: &mut [SourceUnit]) {
    let mut file = None;
    for unit in units.iter_mut() {
        items(&mut unit.items, &mut file);
    }
    let Some(file) = file else { return };
    // Places of their own, past any text: what is checked is keyed by them.
    let n = std::cell::Cell::new(0u32);
    let at = || {
        n.set(n.get() + 1);
        let o = u32::MAX - 4096 + n.get();
        Span::new(file, o, o)
    };
    let id = |text: &str| Ident { text: text.to_string(), span: at() };
    let values = CLASSES.iter().map(|c| EnumValue { name: id(c), value: None }).collect();
    let class = TypeDecl {
        name: id(TYPE_CLASS),
        access: None,
        extends: None,
        union_kw: None,
        def: TypeDef::Spec(TypeSpec::Enum { base: None, values, base_after: false, span: at() }),
        init: None,
        no_semi: None,
        span: at(),
    };
    let field = |name: &str, ty: TypeSpec| VarDecl {
        names: vec![id(name)],
        location: None,
        ty,
        edge: None,
        init: None,
        fb_args: None,
        span: at(),
    };
    let fields = vec![
        field("TypeClass", TypeSpec::Named(Ident { text: TYPE_CLASS.into(), span: at() })),
        field(
            "pValue",
            TypeSpec::Pointer {
                target: Box::new(TypeSpec::Named(Ident { text: "BYTE".into(), span: at() })),
                span: at(),
            },
        ),
        field("diSize", TypeSpec::Named(Ident { text: "DINT".into(), span: at() })),
    ];
    let any = TypeDecl {
        name: Ident { text: ANY_TYPE.into(), span: at() },
        access: None,
        extends: None,
        union_kw: None,
        def: TypeDef::Struct { overlap: false, fields, span: at() },
        init: None,
        no_semi: None,
        span: at(),
    };
    if let Some(u) = units.first_mut() {
        u.items.insert(0, Item::Types(vec![class, any]));
    }
}

/// Each ANY input and each `__SYSTEM` name of `list`; `file`, the first
/// file one is in.
fn items(list: &mut [Item], file: &mut Option<iecf_diagnostics::FileId>) {
    for i in list {
        match i {
            Item::Pou(p) => pou(p, file),
            Item::Globals(b) => block(b, false, file),
            Item::Types(ts) => {
                for t in ts.iter_mut() {
                    if let TypeDef::Struct { fields, .. } = &mut t.def {
                        fields.iter_mut().for_each(|d| spec(&mut d.ty, file));
                    }
                }
            }
            Item::Namespace(n) => items(&mut n.items, file),
            _ => {}
        }
    }
}

fn pou(p: &mut Pou, file: &mut Option<iecf_diagnostics::FileId>) {
    let takes = matches!(p.kind, PouKind::Function | PouKind::FunctionBlock | PouKind::Method);
    for b in p.var_blocks.iter_mut() {
        block(b, takes, file);
    }
    for m in p.methods.iter_mut() {
        for b in m.var_blocks.iter_mut() {
            block(b, true, file);
        }
    }
    walk_pou(p, &mut |e| value(e, file));
    for b in p.var_blocks.iter_mut().chain(p.methods.iter_mut().flat_map(|m| m.var_blocks.iter_mut())) {
        for d in b.decls.iter_mut() {
            if let Some(Init::Expr(e)) = &mut d.init {
                expr(e, &mut |e| value(e, file));
            }
        }
    }
}

/// A section's declarations: an input of ANY, where `inputs` take one;
/// `__SYSTEM.TYPE_CLASS` anywhere.
fn block(b: &mut VarBlock, inputs: bool, file: &mut Option<iecf_diagnostics::FileId>) {
    for d in b.decls.iter_mut() {
        if inputs
            && b.kind == VarKind::Input
            && let TypeSpec::Named(id) = &mut d.ty
            && id.text.eq_ignore_ascii_case("ANY")
        {
            file.get_or_insert(id.span.file);
            id.text = ANY_TYPE.into();
        }
        spec(&mut d.ty, file);
    }
}

fn spec(t: &mut TypeSpec, file: &mut Option<iecf_diagnostics::FileId>) {
    match t {
        TypeSpec::Named(id) if id.text.eq_ignore_ascii_case("__SYSTEM.TYPE_CLASS") => {
            file.get_or_insert(id.span.file);
            id.text = TYPE_CLASS.into();
        }
        TypeSpec::Array { elem, .. } => spec(elem, file),
        TypeSpec::Ref { target, .. } | TypeSpec::Pointer { target, .. } => spec(target, file),
        _ => {}
    }
}

/// `__SYSTEM.TYPE_CLASS.TYPE_REAL`: the value, `__SYSTEM__TYPE_CLASS#TYPE_REAL`.
fn value(e: &mut Expr, file: &mut Option<iecf_diagnostics::FileId>) {
    if let ExprKind::Member(base, v) = &e.kind
        && let ExprKind::Member(sys, class) = &base.kind
        && let ExprKind::Name(s) = &sys.kind
        && s.text.eq_ignore_ascii_case("__SYSTEM")
        && class.text.eq_ignore_ascii_case("TYPE_CLASS")
    {
        file.get_or_insert(s.span.file);
        let ty = Ident { text: TYPE_CLASS.into(), span: Span::new(s.span.file, s.span.start, class.span.end) };
        e.kind = ExprKind::EnumValue { ty, value: v.clone() };
    }
}

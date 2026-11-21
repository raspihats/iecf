//! CODESYS's and TwinCAT's BIT (the bit extension): a structure's member
//! one bit wide, or a function block's variable.
//!
//! Written so before the check. A structure with a BIT member becomes a
//! structure with relative locations (Ed.3 6.4.4.7), each BIT a BOOL at
//! its bit:
//!
//! ```text
//! STRUCT a : BIT; b : BIT; n : INT; c : BIT; END_STRUCT
//! STRUCT a AT %X0.0 : BOOL; b AT %X0.1 : BOOL; n AT %B2 : INT;
//!        c AT %X4.0 : BOOL; END_STRUCT
//! ```
//!
//! BITs in a row fill a byte from bit 0 up, and the next byte after
//! eight; a member of another type starts at the next whole byte,
//! aligned to its own size (an array to its element's, a string to 1,
//! none to more than 8), as TwinCAT's default packing aligns one. A
//! structure whose members already have places of their own (a UNION's,
//! a STRUCT OVERLAP's, `AT %I*`) keeps them; a BIT there is a BOOL, at
//! bit 0 of the byte where it is placed. A member whose size is known
//! only from a declaration elsewhere (a named type) beside a BIT leaves
//! the structure as written: its BIT is then an unknown type. A function
//! block's BIT variable is a BOOL: nothing reads its bytes.

use crate::types::Elem;
use iecf_syntax::ast::*;

/// Whether `units` name BIT where the extension rewrites it.
pub fn needed(units: &[&SourceUnit]) -> bool {
    !declares_bit(units.iter().copied()) && units.iter().any(|u| items_need(&u.items))
}

/// `units` with each structure's BIT members placed and each function
/// block's BIT variables BOOL, as above.
pub fn rewrite(units: &mut [SourceUnit]) {
    if declares_bit(units.iter()) {
        return;
    }
    for u in units.iter_mut() {
        items(&mut u.items);
    }
}

fn declares_bit<'a>(mut units: impl Iterator<Item = &'a SourceUnit>) -> bool {
    fn declared(list: &[Item]) -> bool {
        list.iter().any(|i| match i {
            Item::Types(ts) => ts.iter().any(|t| t.name.text.eq_ignore_ascii_case("BIT")),
            Item::Namespace(n) => declared(&n.items),
            _ => false,
        })
    }
    units.any(|u| declared(&u.items))
}

fn is_bit(t: &TypeSpec) -> bool {
    matches!(t, TypeSpec::Named(id) if id.text.eq_ignore_ascii_case("BIT"))
}

fn items_need(list: &[Item]) -> bool {
    list.iter().any(|i| match i {
        Item::Pou(p) => {
            p.kind == PouKind::FunctionBlock && p.var_blocks.iter().any(|b| b.decls.iter().any(|d| is_bit(&d.ty)))
        }
        Item::Types(ts) => {
            ts.iter().any(|t| matches!(&t.def, TypeDef::Struct { fields, .. } if fields.iter().any(|f| is_bit(&f.ty))))
        }
        Item::Namespace(n) => items_need(&n.items),
        _ => false,
    })
}

fn items(list: &mut [Item]) {
    for i in list {
        match i {
            Item::Pou(p) if p.kind == PouKind::FunctionBlock => {
                for d in p.var_blocks.iter_mut().flat_map(|b| b.decls.iter_mut()) {
                    to_bool(&mut d.ty);
                }
            }
            Item::Types(ts) => {
                for t in ts.iter_mut() {
                    if let TypeDef::Struct { fields, .. } = &mut t.def {
                        structure(fields);
                    }
                }
            }
            Item::Namespace(n) => items(&mut n.items),
            _ => {}
        }
    }
}

fn to_bool(t: &mut TypeSpec) {
    if let TypeSpec::Named(id) = t
        && is_bit(&TypeSpec::Named(id.clone()))
    {
        id.text = "BOOL".into();
    }
}

/// A structure's members, its BITs placed as above.
fn structure(fields: &mut Vec<VarDecl>) {
    if !fields.iter().any(|f| is_bit(&f.ty)) {
        return;
    }
    if fields.iter().any(|f| f.location.is_some()) {
        // Placed already: a BIT at a byte is at its bit 0.
        for f in fields.iter_mut().filter(|f| is_bit(&f.ty)) {
            to_bool(&mut f.ty);
            if let Some(l) = &mut f.location
                && l.is_relative()
                && let Some((byte, None)) = l.relative_place()
            {
                l.text = format!("%X{byte}.0");
            }
        }
        return;
    }
    // Each member's size and alignment, or the structure stays as written.
    let Some(sizes) =
        fields.iter().map(|f| if is_bit(&f.ty) { Some((0, 1)) } else { size(&f.ty) }).collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let mut placed = Vec::new();
    let mut bit = 0u64;
    for (f, (n, align)) in std::mem::take(fields).into_iter().zip(sizes) {
        let bitty = is_bit(&f.ty);
        for name in &f.names {
            let text = if bitty {
                let at = format!("%X{}.{}", bit / 8, bit % 8);
                bit += 1;
                at
            } else {
                let byte = bit.div_ceil(8).next_multiple_of(align);
                bit = (byte + n) * 8;
                format!("%B{byte}")
            };
            let mut one = f.clone();
            one.names = vec![name.clone()];
            one.location = Some(DirectAddress { text, span: name.span });
            to_bool(&mut one.ty);
            placed.push(one);
        }
    }
    *fields = placed;
}

/// A member's size in bytes and its alignment, from what is written: an
/// elementary type, a string, an array of either with literal bounds.
fn size(t: &TypeSpec) -> Option<(u64, u64)> {
    match t {
        TypeSpec::Named(id) => {
            let n = u64::from(Elem::from_name(&id.text)?.bits()?.max(8) / 8);
            Some((n, n.min(8)))
        }
        TypeSpec::String { name, len, .. } => {
            let n = match len {
                Some(e) => int(e)?,
                None => 80,
            } + 1;
            Some(if name.text.eq_ignore_ascii_case("WSTRING") { (n * 2, 2) } else { (n, 1) })
        }
        TypeSpec::Array { dims, elem, .. } => {
            let (mut n, align) = size(elem)?;
            for d in dims {
                let (lo, hi) = d.as_ref()?;
                let count = int_signed(hi)?.checked_sub(int_signed(lo)?)?.checked_add(1)?;
                n = n.checked_mul(u64::try_from(count).ok()?)?;
            }
            Some((n, align))
        }
        _ => None,
    }
}

fn int(e: &Expr) -> Option<u64> {
    u64::try_from(int_signed(e)?).ok()
}

/// A literal integer, `-3` or `16#FF`.
fn int_signed(e: &Expr) -> Option<i64> {
    match &e.kind {
        ExprKind::Literal(l) if l.kind == LiteralKind::Int && l.type_prefix.is_none() => {
            let text = l.text.replace('_', "");
            let v = match text.split_once('#') {
                Some((base, digits)) => i64::from_str_radix(digits, base.parse().ok()?).ok()?,
                None => text.parse().ok()?,
            };
            Some(if l.negative { -v } else { v })
        }
        ExprKind::Unary(UnaryOp::Neg, operand) => int_signed(operand).map(|v| -v),
        _ => None,
    }
}

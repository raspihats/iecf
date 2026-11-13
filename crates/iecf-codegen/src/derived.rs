//! Derived data types (Ed.3 6.4.4): enumerated, subrange, ARRAY and
//! STRUCT, as the generator holds them.
//!
//! An enumerated value is held as its base integer (INT when none is
//! named), its values numbered from 0 in order unless given (Ed.3
//! 6.4.4.3). A subrange is held as its base type; a value stored into it
//! outside its range is clamped to the bound it passed and flagged
//! (decided 2026-11-06, docs/dependencies.md). A structure is a C
//! structure `T_<NAME>` with a field `V_<FIELD>` for each of its own; an
//! array is a C array, of as many dimensions, each counted from its lower
//! bound. Each index is checked: out of its bounds it is a fault
//! (IECF_FAULT_INDEX), and the statement changes nothing (stmt.rs). A
//! REF_TO is an `iecf_ref_t` (refs.rs).
//!
//! A structure whose components have relative locations (Ed.3 6.4.4.7)
//! is its bytes, `T_<NAME>` holding `uint8_t b[n]`: each field at its
//! byte, little-endian, read and written as the process image is
//! (names.rs `Lv::Bytes`), so OVERLAP and a BOOL at a bit mean the same
//! in C memory as at an address (docs/c-output.md §4).

use crate::expr::{const_int, zero};
use crate::names::{Area, Lv};
use crate::{Gen, R, c_type};
use iecf_diagnostics::Span;
use iecf_sema::types::Elem;
use iecf_syntax::ast::*;
use std::collections::HashMap;
use std::fmt::Write as _;

/// A type, with the initial value its declaration gives it.
#[derive(Clone, Debug)]
pub(crate) struct DTy {
    pub(crate) kind: Kind,
    pub(crate) init: Option<Init>,
    /// A subrange's bounds.
    pub(crate) range: Option<(i128, i128)>,
}

#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Elem(Elem),
    /// An enumerated type, by its upper-cased name, and its base.
    Enum(String, Elem),
    /// A structure type, by its upper-cased name.
    Struct(String),
    /// Elements and each dimension's bounds.
    Array(Box<DTy>, Vec<(i64, i64)>),
    /// `ARRAY[*]` of as many dimensions (flex.rs): an in-out's
    /// `iecf_flex_t`, the bounds the caller's.
    Flex(Box<DTy>, usize),
    /// `STRING(n)`, `true` a `WSTRING(n)`: up to n characters (strings.rs).
    Str(u32, bool),
    /// `REF_TO` (refs.rs): an `iecf_ref_t`; `true` for a `REFERENCE TO`
    /// (reference.rs), which only its name in the symbols tells apart.
    Ref(Box<RefTo>, bool),
    /// An instance of the user's FUNCTION_BLOCK or CLASS, by its
    /// upper-cased name, as an element or a field (Ed.3 Table 11 5a, 7a):
    /// its `B_<NAME>_t`.
    Block(String),
    /// An instance of a standard function block there: its `iecf_<stem>_t`.
    Std(crate::names::StdFb),
    /// A variable of an INTERFACE, by its upper-cased name, as an element
    /// or a field: an `iecf_itf_t` (itf.rs).
    Itf(String),
}

impl DTy {
    /// A variable of the interface `i` (itf.rs).
    pub(crate) fn itf(i: &str) -> DTy {
        DTy { kind: Kind::Itf(i.to_string()), init: None, range: None }
    }
}

/// What a reference references.
#[derive(Clone, Debug)]
pub(crate) enum RefTo {
    Data(DTy),
    /// The user's FUNCTION_BLOCK, by its upper-cased name.
    Block(String),
    Std(crate::names::StdFb),
}

pub(crate) struct EnumInfo {
    pub(crate) base: Elem,
    /// Upper-cased value names and their numbers.
    pub(crate) values: Vec<(String, i128)>,
}

/// A structure's fields, in order: upper-cased name and type, the field's
/// own initial value in its `init`.
pub(crate) type Fields = Vec<(String, DTy)>;

/// A structure with relative locations (Ed.3 6.4.4.7): its size, from
/// byte 0 to the last byte a component takes, and each field's byte and
/// the bit of a BOOL at a bit.
#[derive(Clone, Debug)]
pub(crate) struct Layout {
    pub(crate) size: u64,
    pub(crate) at: HashMap<String, (u64, Option<u8>)>,
    /// STRUCT OVERLAP: a field's value is in its bytes, which another's
    /// may have written, never its own initial value alone.
    pub(crate) overlap: bool,
}

/// Count of elements in `lo..hi`, if it is one or more.
pub(crate) fn count(lo: i64, hi: i64) -> Option<u64> {
    if hi < lo { None } else { Some((hi as i128 - lo as i128 + 1) as u64) }
}

impl DTy {
    /// The elementary type it is held in, for an elementary, enumerated or
    /// subrange type.
    pub(crate) fn scalar(&self) -> Option<Elem> {
        match &self.kind {
            Kind::Elem(e) | Kind::Enum(_, e) => Some(*e),
            _ => None,
        }
    }
}

/// `off` plus `n` bytes, folded when `off` is a constant.
pub(crate) fn off_plus(off: &str, n: u64) -> String {
    match off.strip_suffix('u').and_then(|k| k.parse::<u64>().ok()) {
        Some(k) => format!("{}u", k + n),
        None if n == 0 => off.to_string(),
        None => format!("({off} + {n}u)"),
    }
}

impl Gen<'_> {
    /// Its size in the process image, its parts back to back with no gap,
    /// a BOOL a byte, a STRING(n) n + 1 (docs/dependencies.md); `None` for
    /// what has no place there.
    pub(crate) fn packed_size(&self, t: &DTy) -> Option<u64> {
        match &t.kind {
            Kind::Elem(e) | Kind::Enum(_, e) => e.bits().map(|b| u64::from(b.max(8) / 8)),
            Kind::Str(n, false) => Some(u64::from(*n) + 1),
            // Its units, 16 bits each, read and written byte by byte.
            Kind::Str(n, true) => Some((u64::from(*n) + 1) * 2),
            Kind::Array(elem, bounds) => {
                bounds.iter().try_fold(self.packed_size(elem)?, |a, (lo, hi)| a.checked_mul(count(*lo, *hi)?))
            }
            Kind::Struct(k) if self.layouts.contains_key(k) => self.layouts.get(k).map(|l| l.size),
            // Its `*` components are where VAR_CONFIG says, not in it.
            Kind::Struct(k) if self.struct_stars.contains_key(k) => None,
            Kind::Struct(k) => {
                self.structs.get(k)?.iter().try_fold(0u64, |a, (_, f)| a.checked_add(self.packed_size(f)?))
            }
            Kind::Ref(..) | Kind::Block(_) | Kind::Std(_) | Kind::Itf(_) | Kind::Flex(..) => None,
        }
    }

    /// Whether `t` holds an instance of a function block or class,
    /// anywhere in it.
    pub(crate) fn has_instance(&self, t: &DTy) -> bool {
        match &t.kind {
            Kind::Block(_) | Kind::Std(_) => true,
            Kind::Array(elem, _) | Kind::Flex(elem, _) => self.has_instance(elem),
            Kind::Struct(k) => self.structs.get(k).is_some_and(|fs| fs.iter().any(|(_, f)| self.has_instance(f))),
            _ => false,
        }
    }

    /// Whether an instance in `t` keeps something across a warm start
    /// (lib.rs `block`'s `retentive`).
    pub(crate) fn keeps_in(&self, t: &DTy) -> bool {
        match &t.kind {
            Kind::Block(k) => self.retentive.contains(k),
            Kind::Array(elem, _) => self.keeps_in(elem),
            Kind::Struct(k) => self.structs.get(k).is_some_and(|fs| fs.iter().any(|(_, f)| self.keeps_in(f))),
            _ => false,
        }
    }

    /// Whether `t` gives an initial value of its own, anywhere in it, or
    /// one that is not 0: an enumeration starts at its first value (Ed.3
    /// 6.4.4.2.2), a subrange at its lower limit (6.4.4.4.2).
    pub(crate) fn gives_init(&self, t: &DTy) -> bool {
        if t.init.is_some() {
            return true;
        }
        match &t.kind {
            Kind::Block(_) | Kind::Std(_) => true,
            Kind::Enum(k, _) => self.enums.get(k).and_then(|i| i.values.first()).is_some_and(|(_, n)| *n != 0),
            Kind::Elem(_) => t.range.is_some_and(|(lo, _)| lo != 0),
            Kind::Array(elem, _) => self.gives_init(elem),
            Kind::Struct(k) => self.structs.get(k).is_some_and(|fs| fs.iter().any(|(_, f)| self.gives_init(f))),
            _ => false,
        }
    }

    /// Field `key` of the structure `t` at `lv`: where it is, and its type.
    pub(crate) fn lv_member(&mut self, lv: &Lv, t: &DTy, key: &str, at: Span) -> R<(Lv, DTy)> {
        let Kind::Struct(k) = &t.kind else { return self.not_yet(at, "this member") };
        let Some(fields) = self.structs.get(k) else { return self.not_yet(at, "this member") };
        // A component at a `*` address: through the address the variable
        // holds for it (config.rs); such a structure is in C memory only.
        if self.star_fields.get(k).is_some_and(|s| s.contains_key(key)) {
            let Some((_, ft)) = fields.iter().find(|(n, _)| n == key) else { return self.not_yet(at, "this member") };
            let out = match lv {
                Lv::C(s) => Lv::Ref(format!("{s}.A_{key}")),
                Lv::Ref(r) => Lv::Ref(format!("((T_{k} *)iecf_ref_ptr(&ctx->image, {r}))->A_{key}")),
                _ => return self.not_yet(at, "this member"),
            };
            return Ok((out, ft.clone()));
        }
        // A structure with relative locations: the field at its byte, in
        // its bytes wherever they are.
        if let Some((byte, bit)) = self.layouts.get(k).and_then(|l| l.at.get(key)).copied() {
            let Some((_, ft)) = fields.iter().find(|(n, _)| n == key) else { return self.not_yet(at, "this member") };
            let out = match lv {
                Lv::C(s) => Lv::Bytes { area: Area::Mem(format!("{s}.b")), off: format!("{byte}u"), bit },
                Lv::Bytes { area, off, .. } => Lv::Bytes { area: area.clone(), off: off_plus(off, byte), bit },
                Lv::Ref(r) => {
                    let bit = bit.map_or("IECF_NO_BIT".to_string(), |b| format!("{b}u"));
                    Lv::Ref(format!("iecf_ref_in({r}, {byte}u, {bit})"))
                }
                Lv::Part { .. } => return self.not_yet(at, "this member"),
            };
            return Ok((out, ft.clone()));
        }
        let mut before = 0u64;
        for (n, ft) in fields {
            if n == key {
                let out = match lv {
                    Lv::C(s) => Lv::C(format!("{s}.V_{key}")),
                    Lv::Bytes { area, off, .. } => {
                        Lv::Bytes { area: area.clone(), off: off_plus(off, before), bit: None }
                    }
                    Lv::Ref(r) => Lv::Ref(format!("iecf_ref_at({r}, 1u, offsetof(T_{k}, V_{key}), {before}u)")),
                    // A part of a bit string has no fields.
                    Lv::Part { .. } => return self.not_yet(at, "this member"),
                };
                return Ok((out, ft.clone()));
            }
            before += self.packed_size(ft).unwrap_or(0);
        }
        self.not_yet(at, format!("the field {key}"))
    }

    /// The value of a constant expression the standard requires one for:
    /// literals, or CONSTANT variables too, as the checker worked out.
    pub(crate) fn cint(&self, e: &Expr) -> Option<i128> {
        const_int(e).or_else(|| self.types.const_value(e.span))
    }

    /// Element `offs` (each counted from its dimension's lower bound) of
    /// the array `t` at `lv`; with fewer of them than it has dimensions,
    /// the array of the others (Ed.3 6.5.3, Example 2's `A2[2]`), which
    /// lies there as a whole: every layout is row-major.
    pub(crate) fn lv_index(&mut self, lv: &Lv, t: &DTy, offs: &[String], at: Span) -> R<(Lv, DTy)> {
        if let (Kind::Flex(elem, n), Lv::C(d)) = (&t.kind, lv) {
            if offs.len() < *n {
                return Ok((
                    Lv::C(self.flex_part(d, elem, *n, offs)),
                    DTy { kind: Kind::Flex(elem.clone(), n - offs.len()), init: None, range: None },
                ));
            }
            return Ok((self.flex_elem(d, elem, offs), (**elem).clone()));
        }
        let Kind::Array(elem, bounds) = &t.kind else { return self.not_yet(at, "this index") };
        let part = match bounds.get(offs.len()..) {
            Some(rest) if !rest.is_empty() => {
                DTy { kind: Kind::Array(elem.clone(), rest.to_vec()), init: None, range: None }
            }
            _ => (**elem).clone(),
        };
        match lv {
            Lv::C(s) => {
                let mut s = s.clone();
                for o in offs {
                    s = format!("{s}[{o}]");
                }
                Ok((Lv::C(s), part))
            }
            Lv::Bytes { area, off, .. } => {
                let Some(size) = self.packed_size(elem) else { return self.not_yet(at, "this index") };
                let mut off = off.clone();
                for (d, o) in offs.iter().enumerate() {
                    let stride =
                        bounds[d + 1..].iter().map(|(lo, hi)| count(*lo, *hi).unwrap_or(0)).product::<u64>() * size;
                    off = match o.strip_suffix('u').and_then(|k| k.parse::<u64>().ok()) {
                        Some(k) => off_plus(&off, k * stride),
                        None => format!("({off} + (uint32_t)({o}) * {stride}u)"),
                    };
                }
                Ok((Lv::Bytes { area: area.clone(), off, bit: None }, part))
            }
            // The element's number, row-major: the same in C as packed.
            Lv::Ref(r) => {
                let Some(size) = self.packed_size(elem) else { return self.not_yet(at, "this index") };
                let mut terms = Vec::new();
                for (d, o) in offs.iter().enumerate() {
                    let n = bounds[d + 1..].iter().map(|(lo, hi)| count(*lo, *hi).unwrap_or(0)).product::<u64>();
                    terms.push(if n == 1 { format!("(uint32_t)({o})") } else { format!("(uint32_t)({o}) * {n}u") });
                }
                let c = self.declare(elem, "");
                let out = Lv::Ref(format!("iecf_ref_at({r}, {}, sizeof({c}), {size}u)", terms.join(" + ")));
                Ok((out, part))
            }
            // A part of a bit string has no elements.
            Lv::Part { .. } => self.not_yet(at, "this index"),
        }
    }
}

impl Gen<'_> {
    /// C statements copying a whole `t` from `src` to `dst`: as bytes
    /// when both are C objects or both in the image (the same layout),
    /// part by part between the two, an array in a loop per dimension.
    pub(crate) fn copy_data(&mut self, dst: &Lv, src: &Lv, t: &DTy, at: Span) -> R<String> {
        if let Kind::Flex(..) = t.kind {
            return self.flex_whole(at);
        }
        // An instance, or what holds one, is in C memory only: copied
        // whole, every variable of it (docs/dependencies.md).
        if self.has_instance(t) && !matches!((dst, src), (Lv::C(_), Lv::C(_))) {
            return self.not_yet(at, "copying an instance that is not in memory");
        }
        match (dst, src) {
            (Lv::C(d), Lv::C(s)) if !self.star_parts(t).is_empty() || self.holds_pointers(t, 0) => {
                return self.copy_keeping(d, s, t, at);
            }
            (Lv::C(d), Lv::C(s)) => return Ok(format!("iecf_bytes(&({d}), &({s}), sizeof ({d}));")),
            (Lv::Bytes { area: da, off: d, bit: None }, Lv::Bytes { area: sa, off: s, bit: None }) => {
                let Some(n) = self.packed_size(t) else { return self.not_yet(at, "copying this") };
                return Ok(format!("iecf_move({} + {d}, {} + {s}, {n}u);", da.ptr(), sa.ptr()));
            }
            _ => {}
        }
        // A structure with relative locations is its bytes wherever it is:
        // all of them, the gaps too.
        if let Kind::Struct(k) = &t.kind
            && let Some(n) = self.layouts.get(k).map(|l| l.size)
        {
            let start = |lv: &Lv| match lv {
                Lv::C(c) => Some(format!("({c}).b")),
                Lv::Bytes { area, off, .. } => Some(format!("({} + {off})", area.ptr())),
                Lv::Ref(r) => Some(format!("((uint8_t *)iecf_ref_ptr(&ctx->image, {r}))")),
                Lv::Part { .. } => None,
            };
            if let (Some(d), Some(s)) = (start(dst), start(src)) {
                return Ok(format!("iecf_move({d}, {s}, {n}u);"));
            }
            return self.not_yet(at, "copying this");
        }
        match &t.kind {
            Kind::Elem(_) | Kind::Enum(..) => {
                let e = t.scalar().unwrap_or(Elem::Dint);
                Ok(dst.store(e, &src.load(e)))
            }
            // A WSTRING in the image or through a reference (strings.rs).
            Kind::Str(n, w) => Ok(match (crate::strings::wide_at(dst, *w), crate::strings::wide_at(src, *w)) {
                (Some(d), Some(s)) => {
                    let b = self.fresh("w");
                    format!("{{ IEC_WCHAR {b}[{}]; {} }}", *n as u64 + 1, d.put(*n, &s.get(&b, *n)))
                }
                (Some(d), None) => d.put(*n, &src.string(*n, *w)),
                // Into the C array itself, or from where it is (the same
                // bytes, when a reference reaches this array).
                (None, Some(s)) => {
                    let d = dst.string(*n, *w);
                    format!("iecf_move(&{d}, {}, sizeof {d});", s.get(&d, *n))
                }
                _ => format!(
                    "iecf_bytes(&{}, &{}, sizeof {});",
                    dst.string(*n, *w),
                    src.string(*n, *w),
                    dst.string(*n, *w)
                ),
            }),
            // A reference is in C memory only: no other pair gets here.
            Kind::Ref(..) | Kind::Itf(_) => match (dst.c(), src.c()) {
                (Some(d), Some(s)) => Ok(format!("{d} = {s};")),
                _ => self.not_yet(at, "copying this reference"),
            },
            Kind::Block(_) | Kind::Std(_) | Kind::Flex(..) => unreachable!("refused above"),
            Kind::Struct(k) => {
                let names: Vec<String> =
                    self.structs.get(k).map(|f| f.iter().map(|(n, _)| n.clone()).collect()).unwrap_or_default();
                let mut out = String::new();
                for n in names {
                    let (d, ft) = self.lv_member(dst, t, &n, at)?;
                    let (s, _) = self.lv_member(src, t, &n, at)?;
                    out.push_str(&self.copy_data(&d, &s, &ft, at)?);
                    out.push(' ');
                }
                Ok(out.trim_end().to_string())
            }
            Kind::Array(_, bounds) => {
                let ks: Vec<String> = bounds.iter().map(|_| self.fresh("k")).collect();
                let (d, et) = self.lv_index(dst, t, &ks, at)?;
                let (s, _) = self.lv_index(src, t, &ks, at)?;
                let mut body = self.copy_data(&d, &s, &et, at)?;
                for (k, (lo, hi)) in ks.iter().zip(bounds).rev() {
                    let n = count(*lo, *hi).unwrap_or(0);
                    body = format!("for (uint32_t {k} = 0; {k} < {n}u; {k}++) {{ {body} }}");
                }
                Ok(body)
            }
        }
    }
}

impl<'a> Gen<'a> {
    /// The type a declaration's type names, when it is a derived one: a
    /// TYPE of the project, or ARRAY, a subrange or an enumeration written
    /// in place. `None` for an elementary type or a function block.
    pub(crate) fn derived(&mut self, spec: &TypeSpec) -> R<Option<DTy>> {
        match spec {
            TypeSpec::Named(id) => {
                let key = id.text.to_ascii_uppercase();
                if self.type_decls.contains_key(&key) { self.named(&key, id.span).map(Some) } else { Ok(None) }
            }
            TypeSpec::Array { .. } | TypeSpec::Subrange { .. } | TypeSpec::String { .. } | TypeSpec::Ref { .. } => {
                self.spec(spec).map(Some)
            }
            // Declared in place: under the name the checker gave it.
            TypeSpec::Enum { base, values, span } => {
                let key = iecf_sema::inplace_enum_name(base.as_ref(), values, *span).to_ascii_uppercase();
                let base = self.enumeration(&key, base.as_ref(), values)?;
                Ok(Some(DTy { kind: Kind::Enum(key, base), init: None, range: None }))
            }
        }
    }

    /// The integer an enumeration `key` is held in, worked out if not yet.
    pub(crate) fn enum_base(&mut self, key: &str, at: Span) -> R<Elem> {
        if let Some(i) = self.enums.get(key) {
            return Ok(i.base);
        }
        match self.named(key, at)?.kind {
            Kind::Enum(_, b) => Ok(b),
            _ => self.not_yet(at, format!("the enumeration {key}")),
        }
    }

    fn named(&mut self, key: &str, at: Span) -> R<DTy> {
        let Some(decl) = self.type_decls.get(key).copied() else {
            return self.not_yet(at, format!("the type {key}"));
        };
        let kind = match &decl.def {
            TypeDef::Struct { fields, .. } => {
                self.structure(key, fields, decl.name.span)?;
                Kind::Struct(key.to_string())
            }
            TypeDef::Spec(TypeSpec::Enum { base, values, .. }) => {
                let base = self.enumeration(key, base.as_ref(), values)?;
                Kind::Enum(key.to_string(), base)
            }
            TypeDef::Spec(spec) => {
                // An alias: its own initial value first, then its base's.
                let inner = self.spec(spec)?;
                return Ok(DTy { kind: inner.kind, init: decl.init.clone().or(inner.init), range: inner.range });
            }
        };
        Ok(DTy { kind, init: decl.init.clone(), range: None })
    }

    fn spec(&mut self, spec: &TypeSpec) -> R<DTy> {
        match spec {
            TypeSpec::Named(id) => {
                if let Some(e) = Elem::from_name(&id.text)
                    && crate::expr::held(e)
                {
                    return Ok(DTy { kind: Kind::Elem(e), init: None, range: None });
                }
                let key = id.text.to_ascii_uppercase();
                if self.type_decls.contains_key(&key) {
                    return self.named(&key, id.span);
                }
                // An instance as an element or a field (Ed.3 Table 11 5a,
                // 7a): its type written first, as C needs it whole.
                if self.pous.get(&key).is_some_and(|p| matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)) {
                    self.block(&key, id.span)?;
                    return Ok(DTy { kind: Kind::Block(key), init: None, range: None });
                }
                if let Some(fb) = crate::names::StdFb::named(&id.text) {
                    return Ok(DTy { kind: Kind::Std(fb), init: None, range: None });
                }
                if self.is_itf(&key) {
                    self.itfs_used.insert(key.clone());
                    return Ok(DTy { kind: Kind::Itf(key), init: None, range: None });
                }
                self.not_yet(id.span, format!("an element or field of type {}", id.text))
            }
            TypeSpec::Subrange { base, low, high, .. } => {
                let range = (self.cint(low), self.cint(high));
                match (Elem::from_name(&base.text), range) {
                    (Some(e), (Some(lo), Some(hi))) if e.is_int() && lo <= hi => {
                        Ok(DTy { kind: Kind::Elem(e), init: None, range: Some((lo, hi)) })
                    }
                    _ => self.not_yet(base.span, "this subrange"),
                }
            }
            TypeSpec::Array { dims, elem, span } => {
                if dims.iter().all(Option::is_none) {
                    let elem = self.spec(elem)?;
                    return Ok(DTy { kind: Kind::Flex(Box::new(elem), dims.len()), init: None, range: None });
                }
                let mut bounds = Vec::new();
                for d in dims {
                    let b = d.as_ref().and_then(|(lo, hi)| {
                        let (lo, hi) = (i64::try_from(self.cint(lo)?).ok()?, i64::try_from(self.cint(hi)?).ok()?);
                        count(lo, hi).filter(|n| *n <= u32::MAX as u64).map(|_| (lo, hi))
                    });
                    match b {
                        Some(b) => bounds.push(b),
                        None => return self.not_yet(*span, "an array whose bounds are not constant"),
                    }
                }
                let elem = self.spec(elem)?;
                Ok(DTy { kind: Kind::Array(Box::new(elem), bounds), init: None, range: None })
            }
            TypeSpec::Ref { target, implicit, .. } => {
                let to = match &**target {
                    // By name when it is being written: a reference is
                    // opaque in C, so a block or structure may reference
                    // its own type.
                    TypeSpec::Named(id)
                        if self
                            .pous
                            .get(&id.text.to_ascii_uppercase())
                            .is_some_and(|p| matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)) =>
                    {
                        let key = id.text.to_ascii_uppercase();
                        if !self.block_busy.contains(&key) {
                            self.block(&key, id.span)?;
                        }
                        RefTo::Block(key)
                    }
                    TypeSpec::Named(id) if self.struct_busy.contains(&id.text.to_ascii_uppercase()) => {
                        RefTo::Data(DTy { kind: Kind::Struct(id.text.to_ascii_uppercase()), init: None, range: None })
                    }
                    TypeSpec::Named(id) if crate::names::StdFb::named(&id.text).is_some() => {
                        RefTo::Std(crate::names::StdFb::named(&id.text).expect("checked"))
                    }
                    t => RefTo::Data(self.spec(t)?),
                };
                Ok(DTy { kind: Kind::Ref(Box::new(to), *implicit), init: None, range: None })
            }
            TypeSpec::String { name, len, span } => {
                let wide = name.is("WSTRING");
                if !wide && !name.is("STRING") {
                    return self.not_yet(*span, format!("a variable of type {}", name.text.to_ascii_uppercase()));
                }
                let n = match len {
                    None => 80,
                    Some(l) => match self.cint(l).and_then(|v| u32::try_from(v).ok()) {
                        Some(v) if (1..=crate::strings::MAX).contains(&v) => v,
                        _ => return self.not_yet(l.span, "this length of a STRING"),
                    },
                };
                Ok(DTy { kind: Kind::Str(n, wide), init: None, range: None })
            }
            _ => self.not_yet(spec.span(), "this type"),
        }
    }

    fn enumeration(&mut self, key: &str, base: Option<&Ident>, values: &[EnumValue]) -> R<Elem> {
        if let Some(i) = self.enums.get(key) {
            return Ok(i.base);
        }
        let base = match base {
            None => Elem::Int,
            Some(b) => match Elem::from_name(&b.text) {
                Some(e) if e.int_range().is_some() && e != Elem::Bool => e,
                _ => return self.not_yet(b.span, "an enumeration of this type"),
            },
        };
        // A value not given is one more than the one before it, from 0.
        let mut list = Vec::new();
        let mut next = 0i128;
        for v in values {
            let n = match &v.value {
                None => next,
                Some(e) => match const_int(e).or_else(|| self.types.const_value(e.span)) {
                    Some(n) => n,
                    None => return self.not_yet(e.span, "this named value"),
                },
            };
            next = n + 1;
            list.push((v.name.text.to_ascii_uppercase(), n));
        }
        self.enums.insert(key.to_string(), EnumInfo { base, values: list });
        Ok(base)
    }

    /// A structure: its fields, and its C type `T_<NAME>` written after the
    /// types it contains.
    fn structure(&mut self, key: &str, fields: &[VarDecl], at: Span) -> R<()> {
        if self.structs.contains_key(key) {
            return Ok(());
        }
        if self.struct_busy.iter().any(|k| k == key) {
            return self.not_yet(at, "a structure that contains itself");
        }
        self.struct_busy.push(key.to_string());
        let mut list = Vec::new();
        let mut body = String::new();
        let mut result = Ok(());
        // Relative locations (Ed.3 6.4.4.7): each field's byte and bit; the
        // checker saw that every field has one, each where its type fits.
        let placed = fields.iter().any(|f| f.location.as_ref().is_some_and(|l| l.is_relative()));
        let overlap = matches!(self.type_decls.get(key).map(|d| &d.def), Some(TypeDef::Struct { overlap: true, .. }));
        let mut layout = Layout { size: 0, at: HashMap::new(), overlap };
        // Components at `*` addresses (Ed.3 6.4.4.8): each an address
        // VAR_CONFIG gives every variable of the type (config.rs).
        let mut stars: Vec<(String, DTy)> = Vec::new();
        let mut star_names: HashMap<String, char> = HashMap::new();
        for f in fields {
            let partly = f.location.as_ref().is_some_and(|l| l.is_partly());
            let place = match &f.location {
                None if !placed => None,
                Some(_) if partly && !placed => None,
                Some(l) if placed => match l.relative_place() {
                    Some(p) => Some(p),
                    None => {
                        result = self.not_yet(l.span, format!("the relative location {}", l.text));
                        break;
                    }
                },
                _ => {
                    result = self.not_yet(f.span, "a structure field at an address");
                    break;
                }
            };
            let t = match self.spec(&f.ty) {
                Ok(t) => t,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            };
            if let Some((byte, bit)) = place {
                let width = if bit.is_some() { Some(1) } else { self.packed_size(&t) };
                let Some(end) = width.and_then(|w| u64::from(byte).checked_add(w)) else {
                    result = self.not_yet(f.span, "this type at a relative location");
                    break;
                };
                layout.size = layout.size.max(end);
                for n in &f.names {
                    layout.at.insert(n.text.to_ascii_uppercase(), (u64::from(byte), bit));
                }
            }
            if partly && self.packed_size(&t).is_none() {
                result = self.not_yet(f.span, "this type at a `*` address");
                break;
            }
            for n in &f.names {
                let k = n.text.to_ascii_uppercase();
                let ft = DTy { kind: t.kind.clone(), init: f.init.clone().or(t.init.clone()), range: t.range };
                if partly {
                    let _ = writeln!(body, "    iecf_ref_t A_{k};");
                    let area = f.location.as_ref().and_then(|l| l.area()).unwrap_or('I');
                    star_names.insert(k.clone(), area);
                    stars.push((format!("A_{k}"), ft.clone()));
                } else {
                    let _ = writeln!(body, "    {};", self.declare(&t, &format!("V_{k}")));
                    for (p, pt) in self.star_parts(&t) {
                        stars.push((format!("V_{k}.{p}"), pt));
                    }
                }
                list.push((k, ft));
            }
        }
        self.struct_busy.retain(|k| k != key);
        result?;
        if !stars.is_empty() {
            self.struct_stars.insert(key.to_string(), stars);
        }
        if !star_names.is_empty() {
            self.star_fields.insert(key.to_string(), star_names);
        }
        if placed {
            if u32::try_from(layout.size).is_err() {
                return self.not_yet(at, "a structure of 4 GB or more");
            }
            body = format!("    uint8_t b[{}];\n", layout.size.max(1));
            self.layouts.insert(key.to_string(), layout);
        }
        // One that holds an instance after the instance's type, among the
        // blocks' (lib.rs), which may hold the others.
        let t = format!("typedef struct {{\n{body}}} T_{key};\n\n");
        let inst = list.iter().any(|(_, f)| self.has_instance(f));
        self.structs.insert(key.to_string(), list);
        if inst {
            self.block_types.push_str(&t)
        } else {
            self.data_types.push_str(&t)
        }
        Ok(())
    }

    /// A C declaration of `name` as `t`: `IEC_INT V_R[4]`.
    pub(crate) fn declare(&self, t: &DTy, name: &str) -> String {
        let mut dims = String::new();
        let mut t = t;
        while let Kind::Array(elem, bounds) = &t.kind {
            for (lo, hi) in bounds {
                let _ = write!(dims, "[{}]", count(*lo, *hi).unwrap_or(1));
            }
            t = elem;
        }
        let base = match &t.kind {
            Kind::Elem(e) | Kind::Enum(_, e) => c_type(*e),
            Kind::Struct(k) => format!("T_{k}"),
            Kind::Str(n, w) => {
                let _ = write!(dims, "[{}]", *n as u64 + 1);
                crate::strings::unit(*w).to_string()
            }
            Kind::Ref(..) => "iecf_ref_t".to_string(),
            Kind::Itf(_) => "iecf_itf_t".to_string(),
            Kind::Block(k) => format!("B_{k}_t"),
            Kind::Std(fb) => format!("iecf_{}_t", fb.stem),
            Kind::Flex(..) => "iecf_flex_t".to_string(),
            Kind::Array(..) => unreachable!("arrays unwound above"),
        };
        format!("{base} {name}{dims}")
    }

    /// The value of an enumerated value, `idle` or `mode#idle`, in the
    /// enumeration `key`.
    /// What a variable of the enumerated type `t` starts as: `given`, the
    /// type's own initial value, or the first of its values (Ed.3
    /// 6.4.4.2.2).
    pub(crate) fn enum_initial(&mut self, t: &DTy, given: Option<&Init>) -> R<String> {
        let Kind::Enum(k, e) = &t.kind else { return Ok("0".into()) };
        match given.or(t.init.as_ref()) {
            // A constant: nothing of the caller's statement is in it.
            Some(Init::Expr(x)) => self.init_scalar(x, *e),
            Some(i) => self.not_yet(init_span(i), "this initial value"),
            None => Ok(self
                .enums
                .get(k)
                .and_then(|i| i.values.first())
                .and_then(|(_, n)| crate::expr::lit(*n, *e))
                .unwrap_or_else(|| zero(*e))),
        }
    }

    pub(crate) fn enum_value(&self, key: &str, name: &str) -> Option<String> {
        let info = self.enums.get(key)?;
        let up = name.to_ascii_uppercase();
        let (_, n) = info.values.iter().find(|(v, _)| *v == up)?;
        crate::expr::lit(*n, info.base)
    }

    /// Code setting `lv`, of type `t`, to `given` or else its type's
    /// initial value, into `out`.
    pub(crate) fn init_data(&mut self, lv: &str, t: &DTy, given: Option<&Init>, out: &mut String) -> R<()> {
        // Beside an instance kept across a warm start, a part is set only
        // when the instance is not (inst.rs); a standard block's state is
        // all it has, kept as a RETAIN one is (lib.rs `program`).
        let laid = matches!(&t.kind, Kind::Struct(k) if self.layouts.contains_key(k));
        if let (
            Some(g),
            Kind::Elem(_) | Kind::Enum(..) | Kind::Str(..) | Kind::Ref(..) | Kind::Itf(_) | Kind::Std(_),
            _,
        )
        | (Some(g), _, true) = (self.init_guard.clone(), &t.kind, laid)
        {
            self.init_guard = None;
            let mut code = String::new();
            let r = self.init_data(lv, t, given, &mut code);
            self.init_guard = Some(g.clone());
            r?;
            out.push_str(&crate::inst::guarded(Some(&g), &code));
            return Ok(());
        }
        let mut given = given.or(t.init.as_ref()).cloned();
        // A string, an array or a structure given a CONSTANT's name: that
        // constant's own initial value (consts.rs), through any number
        // of them, each once.
        if !matches!(t.kind, Kind::Elem(_) | Kind::Enum(..)) {
            let mut seen = 0;
            while let Some(Init::Expr(x)) = &given {
                let x = x.clone();
                let next = match self.const_init(&x) {
                    Some(i) => Some(i.clone()),
                    // A part of a constant array or structure: its own.
                    None => match self.const_whole_part(&x)? {
                        Some(i) => i,
                        None => break,
                    },
                };
                seen += 1;
                if seen > 64 {
                    return self.const_cycle(x.span);
                }
                given = next;
            }
        }
        match &t.kind {
            // The caller's array: an output's elements are set by flex.rs.
            Kind::Flex(..) => unreachable!("an ARRAY[*] is the caller's array, never initialised whole"),
            Kind::Elem(_) | Kind::Enum(..) => {
                let e = t.scalar().unwrap_or(Elem::Dint);
                let v = match (&given, &t.kind) {
                    (Some(Init::Expr(x)), _) => {
                        if let (Some((lo, hi)), Some(v)) = (t.range, self.cint(x))
                            && (v < lo || v > hi)
                        {
                            self.diags.push(iecf_diagnostics::Diagnostic::new(
                                iecf_diagnostics::Code::LiteralRange,
                                x.span,
                                format!("{v} is outside the subrange {lo}..{hi}"),
                            ));
                            return Err(crate::Skip);
                        }
                        self.init_value(x, e)?
                    }
                    (Some(i), _) => return self.not_yet(init_span(i), "this initial value"),
                    (None, Kind::Enum(k, _)) => match self.enums.get(k).and_then(|i| i.values.first()) {
                        Some((_, n)) => crate::expr::lit(*n, e).unwrap_or_else(|| zero(e)),
                        None => zero(e),
                    },
                    // A subrange starts at its lower limit (Ed.3 6.4.4.4.2).
                    (None, _) => match t.range {
                        Some((lo, _)) => crate::expr::lit(lo, e).unwrap_or_else(|| zero(e)),
                        None => zero(e),
                    },
                };
                let _ = writeln!(out, "    {lv} = {v};");
            }
            Kind::Str(n, w) => {
                let w = *w;
                let units = match &given {
                    None => Vec::new(),
                    Some(Init::Expr(x)) => match &x.kind {
                        ExprKind::Literal(l) if matches!(l.kind, LiteralKind::String | LiteralKind::WString) => {
                            match crate::strings::literal_units(&l.text, w) {
                                Ok(u) => u.into_iter().take_while(|c| *c != 0).collect(),
                                Err(_) => return self.not_yet(x.span, "this initial value"),
                            }
                        }
                        // A constant expression: Table 34's functions
                        // of literals and constants, worked out here
                        // (consts.rs; decided 2026-11-13, florin).
                        _ => {
                            self.fold_bad = None;
                            match self.fold_string(x, 0) {
                                Some(u) => u,
                                None => {
                                    if let Some((code, at, why)) = self.fold_bad.take() {
                                        self.diags.push(iecf_diagnostics::Diagnostic::new(code, at, why));
                                        return Err(crate::Skip);
                                    }
                                    if let Some((at, why)) = self.not_constant(x) {
                                        self.diags.push(iecf_diagnostics::Diagnostic::new(
                                            iecf_diagnostics::Code::NotConstant,
                                            at,
                                            format!("an initial value is a constant expression: {why}"),
                                        ));
                                        return Err(crate::Skip);
                                    }
                                    return self.not_yet(x.span, "this function in a string's initial value");
                                }
                            }
                        }
                    },
                    Some(i) => return self.not_yet(init_span(i), "this initial value of a string"),
                };
                let ty = if w { "WSTRING" } else { "STRING" };
                if let Some(i) = &given
                    && units.len() > *n as usize
                {
                    let at = init_span(i);
                    self.diags.push(iecf_diagnostics::Diagnostic::new(
                        iecf_diagnostics::Code::LiteralRange,
                        at,
                        format!("this string has {} characters, more than {ty}({n}) holds", units.len()),
                    ));
                    return Err(crate::Skip);
                }
                let p = crate::strings::infix(w);
                let _ = writeln!(out, "    iecf_{p}sinit({lv}, {n}u, {});", crate::strings::c_units(&units, w));
            }
            // NULL, or REF() of a variable: filled in when the POU's
            // variables are all known (refs.rs).
            Kind::Ref(to, _) => match &given {
                None => {
                    let _ = writeln!(out, "    {lv} = iecf_ref_null();");
                }
                Some(Init::Expr(x)) if matches!(&x.kind, ExprKind::Name(id) if id.is("NULL")) => {
                    let _ = writeln!(out, "    {lv} = iecf_ref_null();");
                }
                Some(Init::Expr(x)) => {
                    let n = self.ref_inits.len();
                    self.ref_inits.push((lv.to_string(), x.clone(), (**to).clone()));
                    let _ = writeln!(out, "    {}", crate::refs::marker(n));
                }
                Some(i) => return self.not_yet(init_span(i), "this initial value of a reference"),
            },
            // NULL, or an instance or interface: filled in as REF() is.
            Kind::Itf(i) => self.init_itf(lv, i, given.as_ref(), out)?,
            Kind::Block(_) | Kind::Std(_) => self.init_instance(lv, &t.kind, given.as_ref(), out)?,
            Kind::Struct(k) => {
                let fields = self.structs.get(k).cloned().unwrap_or_default();
                let mut set: HashMap<String, Init> = HashMap::new();
                match &given {
                    Some(Init::Struct { fields, .. }) => {
                        for (n, i) in fields {
                            set.insert(n.text.to_ascii_uppercase(), i.clone());
                        }
                    }
                    Some(i) => return self.not_yet(init_span(i), "this initial value of a structure"),
                    None => {}
                }
                if self.layouts.contains_key(k) {
                    return self.init_layout(lv, t, &fields, &set, out);
                }
                let stars = self.star_fields.get(k).cloned().unwrap_or_default();
                let at = self.type_decls.get(k).map(|d| d.name.span);
                for (name, ft) in &fields {
                    match (stars.get(name), at) {
                        // Its address stays; its value is the image's at it.
                        (Some(area), Some(at)) => {
                            let lv = Lv::Ref(format!("{lv}.A_{name}"));
                            self.init_at(&lv, ft, set.get(name), *area == 'I', out, at)?;
                        }
                        _ => self.init_data(&format!("{lv}.V_{name}"), ft, set.get(name), out)?,
                    }
                }
            }
            Kind::Array(elem, bounds) => {
                let sizes: Vec<u64> = bounds.iter().map(|(lo, hi)| count(*lo, *hi).unwrap_or(1)).collect();
                let total: u64 = sizes.iter().product();
                let mut items: Vec<Option<Init>> = Vec::new();
                match &given {
                    Some(Init::Array { elems, .. }) => {
                        for el in elems {
                            let n = match &el.repeat {
                                Some(r) => match self.cint(r).and_then(|n| u64::try_from(n).ok()) {
                                    Some(n) => n,
                                    None => return self.not_yet(r.span, "this repetition"),
                                },
                                None => 1,
                            };
                            for _ in 0..n.min(total) {
                                items.push(el.value.clone());
                            }
                        }
                    }
                    Some(i) => return self.not_yet(init_span(i), "this initial value of an array"),
                    None => {}
                }
                items.truncate(total as usize);
                // Element `n` in row-major order: its C indices.
                let at = |n: u64| {
                    let mut s = String::new();
                    let mut rest = n;
                    let mut idx = vec![0u64; sizes.len()];
                    for (j, size) in sizes.iter().enumerate().rev() {
                        idx[j] = rest % size;
                        rest /= size;
                    }
                    for i in idx {
                        let _ = write!(s, "[{i}]");
                    }
                    s
                };
                for (n, item) in items.iter().enumerate() {
                    self.init_data(&format!("{lv}{}", at(n as u64)), elem, item.as_ref(), out)?;
                }
                let from = items.len() as u64;
                if from < total {
                    // The rest, each its type's initial value, in one loop.
                    let i = self.fresh("i");
                    let mut index = String::new();
                    if sizes.len() == 1 {
                        let _ = write!(index, "[{i}]");
                    } else {
                        let mut stride: u64 = total;
                        for size in &sizes {
                            stride /= size;
                            let _ = write!(index, "[({i} / {stride}u) % {size}u]");
                        }
                    }
                    let mut inner = String::new();
                    self.init_data(&format!("{lv}{index}"), elem, None, &mut inner)?;
                    let _ = writeln!(
                        out,
                        "    {{\n        uint32_t {i};\n        for ({i} = {from}u; {i} < {total}u; {i}++) {{"
                    );
                    for line in inner.lines() {
                        let _ = writeln!(out, "        {line}");
                    }
                    let _ = writeln!(out, "        }}\n    }}");
                }
            }
        }
        Ok(())
    }

    /// A structure with relative locations at the C lvalue `lv`: its
    /// bytes 0, the gaps too, then each field its initial value. Where
    /// two of an OVERLAP share bytes, the one declared first has its
    /// own there, as the first member of a C union has: they are set
    /// from the last (docs/c-output.md §4).
    fn init_layout(
        &mut self,
        lv: &str,
        t: &DTy,
        fields: &Fields,
        set: &HashMap<String, Init>,
        out: &mut String,
    ) -> R<()> {
        let Kind::Struct(k) = &t.kind else { return Ok(()) };
        // Only a declared TYPE has a layout.
        let Some(at) = self.type_decls.get(k).map(|d| d.name.span) else { return Ok(()) };
        let _ = writeln!(out, "    iecf_zero(({lv}).b, sizeof ({lv}).b);");
        for (name, ft) in fields.iter().rev() {
            let (dst, _) = self.lv_member(&Lv::C(lv.to_string()), t, name, at)?;
            let given = set.get(name);
            if let Some(e) = ft.scalar() {
                let v = self.scalar_initial(ft, given, at)?;
                let _ = writeln!(out, "    {}", dst.store(e, &v));
                continue;
            }
            // Built in a C object of its type, copied in.
            let tmp = self.fresh("init");
            let _ = writeln!(out, "    {{\n    {};", self.declare(ft, &tmp));
            self.init_data(&tmp, ft, given, out)?;
            let copy = self.copy_data(&dst, &Lv::C(tmp), ft, at)?;
            let _ = writeln!(out, "    {copy}\n    }}");
        }
        Ok(())
    }

    /// `v`, a C value of `e`, as a subrange `range` holds it: clamped to
    /// the bound it passed and flagged (runtime/iecf.h). `v` unchanged
    /// when there is no range.
    pub(crate) fn clamp(&mut self, v: &str, e: Elem, range: Option<(i128, i128)>) -> String {
        let Some((lo, hi)) = range else { return v.to_string() };
        self.may_flag = true;
        let ct = c_type(e);
        if e.is_signed_int() {
            format!("(({ct})iecf_clamp_s(&ctx->flag, (int64_t)({v}), INT64_C({lo}), INT64_C({hi})))")
        } else {
            format!("(({ct})iecf_clamp_u(&ctx->flag, (uint64_t)({v}), UINT64_C({lo}), UINT64_C({hi})))")
        }
    }

    /// What a variable of the subrange or plain type `t` starts as, as
    /// C: `given`, the type's own initial value, or the subrange's lower
    /// limit (Ed.3 6.4.4.4.2). One outside the subrange is E0020.
    pub(crate) fn scalar_initial(&mut self, t: &DTy, given: Option<&Init>, at: Span) -> R<String> {
        let mut out = String::new();
        self.init_data("@", t, given, &mut out)?;
        match out.trim().strip_prefix("@ = ").and_then(|v| v.strip_suffix(';')) {
            Some(v) => Ok(v.to_string()),
            None => self.not_yet(given.map_or(at, init_span), "this initial value"),
        }
    }

    /// A constant initial value as `e`: no fault and no flag possible.
    pub(crate) fn init_value(&mut self, x: &Expr, e: Elem) -> R<String> {
        self.init_scalar(x, e)
    }
}

fn init_span(i: &Init) -> Span {
    match i {
        Init::Expr(e) => e.span,
        Init::Array { span, .. } | Init::Struct { span, .. } => *span,
    }
}

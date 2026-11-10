//! Derived data types (Ed.3 6.4.4): enumerated, subrange, ARRAY and
//! STRUCT, as the generator holds them.
//!
//! An enumerated value is held as its base integer (DINT when none is
//! named), its values numbered from 0 in order unless given (Ed.3
//! 6.4.4.3). A subrange is held as its base type; a value stored into it
//! outside its range is clamped to the bound it passed and flagged
//! (decided 2026-11-06, docs/dependencies.md). A structure is a C
//! structure `T_<NAME>` with a field `V_<FIELD>` for each of its own; an
//! array is a C array, of as many dimensions, each counted from its lower
//! bound. Each index is checked: out of its bounds it is a fault
//! (IECF_FAULT_INDEX), and the statement changes nothing (stmt.rs). A
//! REF_TO is an `iecf_ref_t` (refs.rs).

use crate::expr::{const_int, zero};
use crate::names::Lv;
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

/// Count of elements in `lo..hi`, if it is one or more.
fn count(lo: i64, hi: i64) -> Option<u64> {
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
            Kind::Struct(k) => {
                self.structs.get(k)?.iter().try_fold(0u64, |a, (_, f)| a.checked_add(self.packed_size(f)?))
            }
            Kind::Ref(..) | Kind::Block(_) | Kind::Std(_) => None,
        }
    }

    /// Whether `t` holds an instance of a function block or class,
    /// anywhere in it.
    pub(crate) fn has_instance(&self, t: &DTy) -> bool {
        match &t.kind {
            Kind::Block(_) | Kind::Std(_) => true,
            Kind::Array(elem, _) => self.has_instance(elem),
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

    /// Whether `t` gives an initial value of its own, anywhere in it.
    pub(crate) fn gives_init(&self, t: &DTy) -> bool {
        if t.init.is_some() {
            return true;
        }
        match &t.kind {
            Kind::Block(_) | Kind::Std(_) => true,
            Kind::Array(elem, _) => self.gives_init(elem),
            Kind::Struct(k) => self.structs.get(k).is_some_and(|fs| fs.iter().any(|(_, f)| self.gives_init(f))),
            _ => false,
        }
    }

    /// Field `key` of the structure `t` at `lv`: where it is, and its type.
    pub(crate) fn lv_member(&mut self, lv: &Lv, t: &DTy, key: &str, at: Span) -> R<(Lv, DTy)> {
        let Kind::Struct(k) = &t.kind else { return self.not_yet(at, "this member") };
        let Some(fields) = self.structs.get(k) else { return self.not_yet(at, "this member") };
        let mut before = 0u64;
        for (n, ft) in fields {
            if n == key {
                let out = match lv {
                    Lv::C(s) => Lv::C(format!("{s}.V_{key}")),
                    Lv::Image { area, off } => Lv::Image { area: *area, off: off_plus(off, before) },
                    Lv::Ref(r) => Lv::Ref(format!("iecf_ref_at({r}, 1u, offsetof(T_{k}, V_{key}), {before}u)")),
                };
                return Ok((out, ft.clone()));
            }
            before += self.packed_size(ft).unwrap_or(0);
        }
        self.not_yet(at, format!("the field {key}"))
    }

    /// Element `offs` (each counted from its dimension's lower bound) of
    /// the array `t` at `lv`.
    pub(crate) fn lv_index(&mut self, lv: &Lv, t: &DTy, offs: &[String], at: Span) -> R<(Lv, DTy)> {
        let Kind::Array(elem, bounds) = &t.kind else { return self.not_yet(at, "this index") };
        match lv {
            Lv::C(s) => {
                let mut s = s.clone();
                for o in offs {
                    s = format!("{s}[{o}]");
                }
                Ok((Lv::C(s), (**elem).clone()))
            }
            Lv::Image { area, off } => {
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
                Ok((Lv::Image { area: *area, off }, (**elem).clone()))
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
                Ok((out, (**elem).clone()))
            }
        }
    }
}

impl Gen<'_> {
    /// C statements copying a whole `t` from `src` to `dst`: as bytes
    /// when both are C objects or both in the image (the same layout),
    /// part by part between the two, an array in a loop per dimension.
    pub(crate) fn copy_data(&mut self, dst: &Lv, src: &Lv, t: &DTy, at: Span) -> R<String> {
        if self.has_instance(t) {
            return self.not_yet(at, "copying an array or structure that holds an instance");
        }
        match (dst, src) {
            (Lv::C(d), Lv::C(s)) => return Ok(format!("iecf_bytes(&({d}), &({s}), sizeof ({d}));")),
            (Lv::Image { area: da, off: d }, Lv::Image { area: sa, off: s }) => {
                let Some(n) = self.packed_size(t) else { return self.not_yet(at, "copying this") };
                let (dp, sp) = (crate::names::image(*da), crate::names::image(*sa));
                return Ok(format!("iecf_move({dp} + {d}, {sp} + {s}, {n}u);"));
            }
            _ => {}
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
            Kind::Ref(..) => match (dst.c(), src.c()) {
                (Some(d), Some(s)) => Ok(format!("{d} = {s};")),
                _ => self.not_yet(at, "copying this reference"),
            },
            Kind::Block(_) | Kind::Std(_) => unreachable!("refused above"),
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
            TypeSpec::Enum { span, .. } => self.not_yet(*span, "an enumeration declared in place"),
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
            TypeDef::Struct { overlap: true, span, .. } => return self.not_yet(*span, "STRUCT OVERLAP"),
            TypeDef::Struct { fields, .. } => {
                self.structure(key, fields, decl.name.span)?;
                Kind::Struct(key.to_string())
            }
            TypeDef::Spec(TypeSpec::Enum { base, values, span }) => {
                let base = self.enumeration(key, base.as_ref(), values, *span)?;
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
                self.not_yet(id.span, format!("an element or field of type {}", id.text))
            }
            TypeSpec::Subrange { base, low, high, .. } => {
                let range = (const_int(low), const_int(high));
                match (Elem::from_name(&base.text), range) {
                    (Some(e), (Some(lo), Some(hi))) if e.is_int() && lo <= hi => {
                        Ok(DTy { kind: Kind::Elem(e), init: None, range: Some((lo, hi)) })
                    }
                    _ => self.not_yet(base.span, "this subrange"),
                }
            }
            TypeSpec::Array { dims, elem, span } => {
                let mut bounds = Vec::new();
                for d in dims {
                    let b = d.as_ref().and_then(|(lo, hi)| {
                        let (lo, hi) = (i64::try_from(const_int(lo)?).ok()?, i64::try_from(const_int(hi)?).ok()?);
                        count(lo, hi).filter(|n| *n <= u32::MAX as u64).map(|_| (lo, hi))
                    });
                    match b {
                        Some(b) => bounds.push(b),
                        None => return self.not_yet(*span, "an array whose bounds are not integer literals"),
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
                    Some(l) => match const_int(l).and_then(|v| u32::try_from(v).ok()) {
                        Some(v) if (1..=crate::strings::MAX).contains(&v) => v,
                        _ => return self.not_yet(l.span, "this length of a STRING"),
                    },
                };
                Ok(DTy { kind: Kind::Str(n, wide), init: None, range: None })
            }
            _ => self.not_yet(spec.span(), "this type"),
        }
    }

    fn enumeration(&mut self, key: &str, base: Option<&Ident>, values: &[EnumValue], at: Span) -> R<Elem> {
        if let Some(i) = self.enums.get(key) {
            return Ok(i.base);
        }
        let base = match base {
            None => Elem::Dint,
            Some(b) => match Elem::from_name(&b.text) {
                Some(e) if e.is_int() => e,
                _ => return self.not_yet(b.span, "an enumeration of this type"),
            },
        };
        let mut list = Vec::new();
        for (i, v) in values.iter().enumerate() {
            let n = match &v.value {
                None if base == Elem::Dint || values.iter().all(|v| v.value.is_none()) => i as i128,
                None => return self.not_yet(at, "an enumeration with some values given and some not"),
                Some(e) => match const_int(e) {
                    Some(n) => n,
                    None => return self.not_yet(e.span, "an enumerated value that is not an integer literal"),
                },
            };
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
        for f in fields {
            if f.location.is_some() {
                result = self.not_yet(f.span, "a structure field at an address");
                break;
            }
            let t = match self.spec(&f.ty) {
                Ok(t) => t,
                Err(e) => {
                    result = Err(e);
                    break;
                }
            };
            for n in &f.names {
                let k = n.text.to_ascii_uppercase();
                let _ = writeln!(body, "    {};", self.declare(&t, &format!("V_{k}")));
                list.push((k, DTy { kind: t.kind.clone(), init: f.init.clone().or(t.init.clone()), range: t.range }));
            }
        }
        self.struct_busy.retain(|k| k != key);
        result?;
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
            Kind::Block(k) => format!("B_{k}_t"),
            Kind::Std(fb) => format!("iecf_{}_t", fb.stem),
            Kind::Array(..) => unreachable!("arrays unwound above"),
        };
        format!("{base} {name}{dims}")
    }

    /// The value of an enumerated value, `idle` or `mode#idle`, in the
    /// enumeration `key`.
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
        if let (Some(g), Kind::Elem(_) | Kind::Enum(..) | Kind::Str(..) | Kind::Ref(..) | Kind::Std(_)) =
            (self.init_guard.clone(), &t.kind)
        {
            self.init_guard = None;
            let mut code = String::new();
            let r = self.init_data(lv, t, given, &mut code);
            self.init_guard = Some(g.clone());
            r?;
            out.push_str(&crate::inst::guarded(Some(&g), &code));
            return Ok(());
        }
        let given = given.or(t.init.as_ref()).cloned();
        match &t.kind {
            Kind::Elem(_) | Kind::Enum(..) => {
                let e = t.scalar().unwrap_or(Elem::Dint);
                let v = match (&given, &t.kind) {
                    (Some(Init::Expr(x)), _) => {
                        if let (Some((lo, hi)), Some(v)) = (t.range, const_int(x))
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
                    (None, _) => zero(e),
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
                        _ => return self.not_yet(x.span, "an initial value of a string that is not a literal"),
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
                for (name, ft) in &fields {
                    self.init_data(&format!("{lv}.V_{name}"), ft, set.get(name), out)?;
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
                                Some(r) => match const_int(r).and_then(|n| u64::try_from(n).ok()) {
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

    /// A constant initial value as `e`: no fault and no flag possible.
    pub(crate) fn init_value(&mut self, x: &Expr, e: Elem) -> R<String> {
        self.may_fault = false;
        self.may_flag = false;
        let code = self.value_as(x, e, &crate::names::Scope::default())?;
        if self.may_fault || self.may_flag {
            return self.not_yet(x.span, "an initial value that can fault or be flagged");
        }
        Ok(code)
    }
}

fn init_span(i: &Init) -> Span {
    match i {
        Init::Expr(e) => e.span,
        Init::Array { span, .. } | Init::Struct { span, .. } => *span,
    }
}

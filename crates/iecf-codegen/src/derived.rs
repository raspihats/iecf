//! Derived data types (Ed.3 6.4.4): enumerated, subrange, ARRAY and
//! STRUCT, as the generator holds them.
//!
//! An enumerated value is held as its base integer (DINT when none is
//! named), its values numbered from 0 in order unless given (Ed.3
//! 6.4.4.3). A subrange is held as its base type. A structure is a C
//! structure `T_<NAME>` with a field `V_<FIELD>` for each of its own; an
//! array is a C array, of as many dimensions, each counted from its lower
//! bound. Each index is checked: out of its bounds it is a fault
//! (IECF_FAULT_INDEX), and the statement changes nothing (stmt.rs).

use crate::expr::{const_int, zero};
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
            TypeSpec::Array { .. } | TypeSpec::Subrange { .. } => self.spec(spec).map(Some),
            TypeSpec::Enum { span, .. } => self.not_yet(*span, "an enumeration declared in place"),
            _ => Ok(None),
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
                return Ok(DTy { kind: inner.kind, init: decl.init.clone().or(inner.init) });
            }
        };
        Ok(DTy { kind, init: decl.init.clone() })
    }

    fn spec(&mut self, spec: &TypeSpec) -> R<DTy> {
        match spec {
            TypeSpec::Named(id) => {
                if let Some(e) = Elem::from_name(&id.text)
                    && crate::expr::held(e)
                {
                    return Ok(DTy { kind: Kind::Elem(e), init: None });
                }
                let key = id.text.to_ascii_uppercase();
                if self.type_decls.contains_key(&key) {
                    return self.named(&key, id.span);
                }
                self.not_yet(id.span, format!("an element or field of type {}", id.text))
            }
            TypeSpec::Subrange { base, .. } => match Elem::from_name(&base.text) {
                Some(e) if e.is_int() => Ok(DTy { kind: Kind::Elem(e), init: None }),
                _ => self.not_yet(base.span, "this subrange"),
            },
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
                Ok(DTy { kind: Kind::Array(Box::new(elem), bounds), init: None })
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
                list.push((k, DTy { kind: t.kind.clone(), init: f.init.clone().or(t.init.clone()) }));
            }
        }
        self.struct_busy.retain(|k| k != key);
        result?;
        let _ = writeln!(self.data_types, "typedef struct {{\n{body}}} T_{key};\n");
        self.structs.insert(key.to_string(), list);
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
        let given = given.or(t.init.as_ref()).cloned();
        match &t.kind {
            Kind::Elem(_) | Kind::Enum(..) => {
                let e = t.scalar().unwrap_or(Elem::Dint);
                let v = match (&given, &t.kind) {
                    (Some(Init::Expr(x)), _) => self.init_value(x, e)?,
                    (Some(i), _) => return self.not_yet(init_span(i), "this initial value"),
                    (None, Kind::Enum(k, _)) => match self.enums.get(k).and_then(|i| i.values.first()) {
                        Some((_, n)) => crate::expr::lit(*n, e).unwrap_or_else(|| zero(e)),
                        None => zero(e),
                    },
                    (None, _) => zero(e),
                };
                let _ = writeln!(out, "    {lv} = {v};");
            }
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

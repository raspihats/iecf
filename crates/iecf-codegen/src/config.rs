//! Variables at `*` addresses (Ed.3 6.5.5.4, 6.4.4.8) and the
//! CONFIGURATION's VAR_CONFIG (Ed.3 6.8.2): locations and initial values
//! given per instance.
//!
//! A variable declared `AT %I*`, `%Q*` or `%M*` is where VAR_CONFIG says,
//! for each instance of the PROGRAM, FUNCTION_BLOCK or structure that
//! declares it - and the instances of one type share its code. So each
//! instance holds the address, `iecf_ref_t A_<NAME>` beside its other
//! variables, set by the configuration as it starts, before any initial
//! value is written; the variable is read and written through it, as
//! through a reference into the image (names.rs `Lv::Ref`). A copy of a
//! whole instance or structure copies such a variable's value from the
//! one address to the other, and leaves each its own address.
//!
//! VAR_CONFIG's initial values "always prevail type specific initial
//! values" (Ed.3 6.8.2): set after every other, by `<p>_config`, at a
//! cold start, and at a warm one for each variable the start does not
//! keep (docs/c-output.md §7).

use crate::derived::{DTy, Kind};
use crate::names::{Lv, Place, Scope};
use crate::{Decl, Gen, R};
use iecf_diagnostics::Span;
use iecf_sema::types::Elem;
use iecf_syntax::ast::*;
use std::fmt::Write as _;

/// A program instance as VAR_CONFIG's paths name it: `R.P` (or `P`
/// when the configuration names no resource), its C lvalue in the
/// context, and its PROGRAM.
pub(crate) struct Instance<'s> {
    pub(crate) path: &'s str,
    pub(crate) lv: String,
    pub(crate) pou: &'s str,
}

/// Where a VAR_CONFIG path ends: the variable, whether a warm start
/// keeps it, and whether it is in the input image, the platform's.
struct Target {
    place: Place,
    kept: bool,
    input: bool,
}

impl Gen<'_> {
    /// `k AT %Q* : T` in VAR of a PROGRAM or FUNCTION_BLOCK: the field
    /// holding its address into `fields`, its initial value written
    /// through it into `init` (not an input's: the platform's), and its
    /// place and type.
    pub(crate) fn partly_var(
        &mut self,
        k: &str,
        decl: &Decl,
        d: &VarDecl,
        a: &DirectAddress,
        fields: &mut String,
        init: &mut String,
    ) -> R<(Place, DTy)> {
        let t = match decl {
            Decl::Elem(e) => DTy { kind: Kind::Elem(*e), init: None, range: None },
            Decl::Data(t) if self.packed_size(t).is_some() => t.clone(),
            _ => return self.not_yet(a.span, format!("this type at {}", a.text)),
        };
        let _ = writeln!(fields, "    iecf_ref_t A_{k};");
        let lv = Lv::Ref(format!("self->A_{k}"));
        self.init_at(&lv, &t, d.init.as_ref(), a.area() == Some('I'), init, d.span)?;
        Ok((Place::Data(lv, t.clone()), t))
    }

    /// The initial value of `t` at `lv`, which is not a C object:
    /// `given`, or its type's when it has one, built in a C object and
    /// copied in; nothing for an input, whose image is the platform's.
    pub(crate) fn init_at(
        &mut self,
        lv: &Lv,
        t: &DTy,
        given: Option<&Init>,
        input: bool,
        out: &mut String,
        at: Span,
    ) -> R<()> {
        if input || !(given.is_some() || self.gives_init(t)) {
            return Ok(());
        }
        if let Some(e) = t.scalar() {
            let v = self.scalar_initial(t, given, at)?;
            let _ = writeln!(out, "    {}", lv.store(e, &v));
            return Ok(());
        }
        let tmp = self.fresh("init");
        let _ = writeln!(out, "    {{\n    {};", self.declare(t, &tmp));
        self.init_data(&tmp, t, given, out)?;
        let copy = self.copy_data(lv, &Lv::C(tmp), t, at)?;
        let _ = writeln!(out, "    {copy}\n    }}");
        Ok(())
    }

    /// The `*` variables in a value of `t`, anywhere in it: each one's
    /// field path from the value (`A_X`, `V_S.A_IN`, `base.A_X`), and
    /// its type.
    pub(crate) fn star_parts(&self, t: &DTy) -> Vec<(String, DTy)> {
        match &t.kind {
            Kind::Block(k) => self.block_stars.get(k).cloned().unwrap_or_default(),
            Kind::Struct(k) => self.struct_stars.get(k).cloned().unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// The `*` variables of the FUNCTION_BLOCK or CLASS `key`, from its
    /// scope `scope` (its own variables `self->V_X`, its `*` ones
    /// `self->A_X`) and what its base has.
    pub(crate) fn note_block_stars(&mut self, key: &str, scope: &Scope) {
        let mut list: Vec<(String, DTy)> = Vec::new();
        if let Some(base) = self.types.base(key) {
            let base = base.to_ascii_uppercase();
            for (p, t) in self.block_stars.get(&base).cloned().unwrap_or_default() {
                list.push((format!("base.{p}"), t));
            }
        }
        let mut names: Vec<&String> = scope.vars.keys().collect();
        names.sort();
        for name in names {
            let own = format!("self->V_{name}");
            match &scope.vars[name] {
                Place::Data(Lv::Ref(r), t) if *r == format!("self->A_{name}") => {
                    list.push((format!("A_{name}"), t.clone()));
                }
                Place::Block(lv, inner) if *lv == own => {
                    let t = DTy { kind: Kind::Block(inner.clone()), init: None, range: None };
                    for (p, pt) in self.star_parts(&t) {
                        list.push((format!("V_{name}.{p}"), pt));
                    }
                }
                Place::Data(Lv::C(lv), t) if *lv == own => {
                    for (p, pt) in self.star_parts(t) {
                        list.push((format!("V_{name}.{p}"), pt));
                    }
                }
                _ => {}
            }
        }
        if !list.is_empty() {
            self.block_stars.insert(key.to_string(), list);
        }
    }

    /// A whole `t` holding `*` variables copied from the C object `s` to
    /// `d`: each such variable's value from the one's address to the
    /// other's, then the rest as bytes, each keeping its own addresses.
    pub(crate) fn copy_with_stars(&mut self, d: &str, s: &str, t: &DTy, at: Span) -> R<String> {
        let stars = self.star_parts(t);
        let mut out = String::from("{ ");
        for (p, pt) in &stars {
            let c = self.copy_data(&Lv::Ref(format!("({d}).{p}")), &Lv::Ref(format!("({s}).{p}")), pt, at)?;
            out.push_str(&c);
            out.push(' ');
        }
        let keep: Vec<String> = stars.iter().map(|_| self.fresh("a")).collect();
        for (k, (p, _)) in keep.iter().zip(&stars) {
            let _ = write!(out, "iecf_ref_t {k} = ({d}).{p}; ");
        }
        let _ = write!(out, "iecf_bytes(&({d}), &({s}), sizeof ({d})); ");
        for (k, (p, _)) in keep.iter().zip(&stars) {
            let _ = write!(out, "({d}).{p} = {k}; ");
        }
        out.push('}');
        Ok(out)
    }

    /// VAR_CONFIG's entries for `instances`: each address, set in `links`
    /// as the configuration starts (the image's extent grown to hold
    /// it); each initial value, into the body of `<p>_config`, which
    /// runs after every other initial value is set - an entry for a
    /// variable a warm start keeps, at a cold start only.
    pub(crate) fn var_config(
        &mut self,
        config: &Configuration,
        instances: &[Instance],
        globals: &Scope,
        links: &mut String,
    ) -> R<String> {
        let mut out = String::new();
        for e in &config.config_inits {
            let names: Vec<String> = e.path.iter().map(|i| i.text.to_ascii_uppercase()).collect();
            let found = instances.iter().find_map(|i| {
                let n = i.path.split('.').count();
                (names.len() > n && names[..n].join(".") == i.path).then(|| (n, i.lv.clone(), i.pou.to_string()))
            });
            let Some((n, lv, pou)) = found else { return self.not_yet(e.span, "this VAR_CONFIG path") };
            let target = self.config_target(&lv, &pou, &e.path[n..])?;
            if let Some(a) = &e.location {
                let Place::Data(Lv::Ref(r), t) = &target.place else {
                    return self.not_yet(a.span, "this VAR_CONFIG address");
                };
                let Some((area, size, byte, bit)) = crate::names::parse_address(&a.text) else {
                    return self.not_yet(a.span, format!("the address {}", a.text));
                };
                let width = match t.scalar() {
                    Some(Elem::Bool) => Some(1),
                    _ => self.packed_size(t),
                };
                let Some(end) = width.and_then(|w| u32::try_from(u64::from(byte) + w).ok()) else {
                    return self.not_yet(a.span, format!("this type at {}", a.text));
                };
                let slot = match area {
                    'I' => 0,
                    'Q' => 1,
                    _ => 2,
                };
                self.extent[slot] = self.extent[slot].max(end);
                let bit = if size == 'X' { format!("{bit}u") } else { "IECF_NO_BIT".into() };
                let _ = writeln!(links, "    {r} = iecf_ref_image((uint8_t)'{area}', {byte}u, {bit});");
            }
            let Some(init) = &e.init else { continue };
            let mut code = String::new();
            match &target.place {
                Place::Data(Lv::C(c), t) => self.init_data(c, t, Some(init), &mut code)?,
                Place::Data(lv, t) => self.init_at(lv, t, Some(init), target.input, &mut code, e.span)?,
                Place::Field(_, ty) | Place::Located { ty, .. } => {
                    let Init::Expr(x) = init else { return self.not_yet(e.span, "this initial value") };
                    if !target.input {
                        let v = self.init_scalar(x, *ty)?;
                        let _ = writeln!(code, "    {}", target.place.store(&v));
                    }
                }
                Place::Block(lv, k) => {
                    self.instance_members(lv, &Kind::Block(k.clone()), Some(init), "0", &mut code)?
                }
                Place::Fb(lv, fb) => self.instance_members(lv, &Kind::Std(fb.clone()), Some(init), "0", &mut code)?,
                Place::Itf(lv, i) => self.init_itf(lv, i, Some(init), &mut code)?,
            }
            if target.kept && !code.is_empty() {
                let _ = writeln!(out, "    if (!warm) {{\n{code}    }}");
            } else {
                out.push_str(&code);
            }
        }
        self.fill_ref_inits(&mut out, globals)?;
        Ok(out)
    }

    /// The variable `path` names in the program instance `inst` of the
    /// PROGRAM `pou`: through its function block and class instances,
    /// structure fields, a standard block's inputs and outputs.
    fn config_target(&mut self, inst: &str, pou: &str, path: &[Ident]) -> R<Target> {
        let mut place: Option<Place> = None;
        let mut kept = false;
        let mut input = false;
        for id in path {
            let name = id.text.to_ascii_uppercase();
            let next = match place.take() {
                None => self.member_of(pou, inst, &name, &mut kept, &mut input),
                Some(Place::Block(lv, k)) => self.member_of(&k, &lv, &name, &mut kept, &mut input),
                Some(Place::Fb(lv, fb)) => {
                    fb.param(&name).map(|(field, e, _)| Place::Field(format!("{lv}.{field}"), *e))
                }
                Some(Place::Data(lv, t)) if matches!(t.kind, Kind::Struct(_)) => {
                    let Kind::Struct(k) = &t.kind else { unreachable!("matched") };
                    input = self.field_decl(k, &name).is_some_and(at_input);
                    let (lv, ft) = self.lv_member(&lv, &t, &name, id.span)?;
                    Some(match (&lv, &ft.kind) {
                        (Lv::C(c), Kind::Block(k)) => Place::Block(c.clone(), k.clone()),
                        (Lv::C(c), Kind::Std(fb)) => Place::Fb(c.clone(), fb.clone()),
                        _ => Place::Data(lv, ft),
                    })
                }
                Some(_) => None,
            };
            match next {
                Some(p) => place = Some(p),
                None => return self.not_yet(id.span, format!("'{}' in this VAR_CONFIG path", id.text)),
            }
        }
        match place {
            Some(place) => {
                let input = input
                    || matches!(&place, Place::Located { area: 'I', .. })
                    || matches!(&place, Place::Data(Lv::Bytes { area: crate::names::Area::Image('I'), .. }, _));
                Ok(Target { place, kept, input })
            }
            None => self.not_yet(
                path.first().map_or(Span::new(iecf_diagnostics::FileId(0), 0, 0), |i| i.span),
                "this VAR_CONFIG path",
            ),
        }
    }

    /// The variable `name` of the instance at `lv` of the POU `key` (a
    /// PROGRAM, FUNCTION_BLOCK or CLASS), as C reaches it there; `kept`
    /// becomes whether a warm start keeps it (a RETAIN section, or one
    /// inside a RETAIN instance, NON_RETAIN undoing it), `input` whether
    /// it is declared at an input address.
    fn member_of(&self, key: &str, lv: &str, name: &str, kept: &mut bool, input: &mut bool) -> Option<Place> {
        let scope = match self.pous.get(key).map(|p| p.kind) {
            Some(PouKind::Program) => self.program_scopes.get(key)?,
            _ => self.block_scopes.get(key)?,
        };
        let place = scope.vars.get(name)?.rebased(lv);
        match self.member_retain(key, name) {
            Some(Retain::Retain) => *kept = true,
            Some(Retain::NonRetain) => *kept = false,
            None => {}
        }
        *input = self.member_decl(key, name).is_some_and(at_input);
        Some(place)
    }

    /// The declaration of the variable `name` of the POU `key`, or of a
    /// base it inherits it from.
    fn member_decl(&self, key: &str, name: &str) -> Option<&VarDecl> {
        let mut k = Some(key.to_string());
        while let Some(t) = k {
            let pou = self.pous.get(t.as_str())?;
            let found = pou
                .var_blocks
                .iter()
                .flat_map(|b| &b.decls)
                .find(|d| d.names.iter().any(|n| n.text.eq_ignore_ascii_case(name)));
            if found.is_some() {
                return found;
            }
            k = self.types.base(&t).map(str::to_string);
        }
        None
    }

    /// The declaration of the field `name` of the structure type `key`.
    fn field_decl(&self, key: &str, name: &str) -> Option<&VarDecl> {
        match &self.type_decls.get(key)?.def {
            TypeDef::Struct { fields, .. } => {
                fields.iter().find(|f| f.names.iter().any(|n| n.text.eq_ignore_ascii_case(name)))
            }
            TypeDef::Spec(_) => None,
        }
    }
}

/// Whether a declaration is at an input address, `%I` or `%I*`.
fn at_input(d: &VarDecl) -> bool {
    d.location.as_ref().is_some_and(|l| l.area() == Some('I'))
}

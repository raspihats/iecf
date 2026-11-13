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
use iecf_sema::Binding;
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
    /// Declared RETAIN: a warm start keeps its variables of sections
    /// neither RETAIN nor NON_RETAIN.
    pub(crate) kept: bool,
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

    /// Whether a value of `t` holds an instance with pointers to globals
    /// (`pointer_names`), anywhere in it.
    pub(crate) fn holds_pointers(&self, t: &DTy, depth: usize) -> bool {
        if depth > 32 {
            return false;
        }
        match &t.kind {
            Kind::Block(k) => {
                let mut level = Some(k.clone());
                while let Some(key) = level {
                    if !self.pointer_names(&key).is_empty() {
                        return true;
                    }
                    let scope = self.block_scopes.get(&key);
                    let held = scope.is_some_and(|s| {
                        s.vars.iter().any(|(n, p)| match p {
                            Place::Block(lv, inner) if *lv == format!("self->V_{n}") => self.holds_pointers(
                                &DTy { kind: Kind::Block(inner.clone()), init: None, range: None },
                                depth + 1,
                            ),
                            Place::Data(Lv::C(lv), dt) if *lv == format!("self->V_{n}") => {
                                self.holds_pointers(dt, depth + 1)
                            }
                            _ => false,
                        })
                    });
                    if held {
                        return true;
                    }
                    level = self.types.base(&key).map(str::to_ascii_uppercase);
                }
                false
            }
            Kind::Array(elem, _) => self.holds_pointers(elem, depth + 1),
            Kind::Struct(k) => {
                self.structs.get(k).is_some_and(|fs| fs.iter().any(|(_, f)| self.holds_pointers(f, depth + 1)))
            }
            _ => false,
        }
    }

    /// A whole `t` holding `*` variables or pointers to globals copied
    /// from the C object `s` to `d`: each `*` variable's value from the
    /// one's address to the other's, then the rest as bytes, `d` keeping
    /// its own addresses and pointers.
    pub(crate) fn copy_keeping(&mut self, d: &str, s: &str, t: &DTy, at: Span) -> R<String> {
        let stars = self.star_parts(t);
        let mut out = String::from("{ ");
        for (p, pt) in &stars {
            let c = self.copy_data(&Lv::Ref(format!("({d}).{p}")), &Lv::Ref(format!("({s}).{p}")), pt, at)?;
            out.push_str(&c);
            out.push(' ');
        }
        let keep = self.fresh("keep");
        let _ = write!(out, "{} = {d}; ", self.declare(t, &keep));
        let _ = write!(out, "iecf_bytes(&({d}), &({s}), sizeof ({d})); ");
        for (p, _) in &stars {
            let _ = write!(out, "({d}).{p} = {keep}.{p}; ");
        }
        let mut restore = String::new();
        let value = |_: &mut Self, _: &str, field: &str| Ok(field.replacen(d, &keep, 1));
        self.set_held(d, t, &value, &mut restore)?;
        for line in restore.lines() {
            out.push_str(line.trim());
            out.push(' ');
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
                (names.len() > n && names[..n].join(".") == i.path)
                    .then(|| (n, i.lv.clone(), i.pou.to_string(), i.kept))
            });
            let Some((n, lv, pou, kept)) = found else { return self.not_yet(e.span, "this VAR_CONFIG path") };
            let target = self.config_target(&lv, &pou, &e.path[n..], kept)?;
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
    fn config_target(&mut self, inst: &str, pou: &str, path: &[Ident], kept: bool) -> R<Target> {
        let mut place: Option<Place> = None;
        let mut kept = kept;
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
            // An instance in a section that says neither keeps what its
            // own type says, whatever holds it (Ed.3 6.5.6.2).
            None if matches!(place, Place::Block(..) | Place::Fb(..)) => *kept = false,
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

/// The place of the global at `p` reached through the pointer `ptr` to
/// it, which `pointer_to` gives: a value as through a reference, an
/// instance or interface through a C pointer.
fn pointed(p: &Place, ptr: &str) -> Place {
    match p {
        Place::Field(_, e) | Place::Located { ty: e, .. } => {
            Place::Data(Lv::Ref(ptr.to_string()), DTy { kind: Kind::Elem(*e), init: None, range: None })
        }
        Place::Data(_, t) => Place::Data(Lv::Ref(ptr.to_string()), DTy { init: None, ..t.clone() }),
        Place::Block(_, k) => Place::Block(format!("(*{ptr})"), k.clone()),
        Place::Fb(_, fb) => Place::Fb(format!("(*{ptr})"), fb.clone()),
        Place::Itf(_, i) => Place::Itf(format!("(*{ptr})"), i.clone()),
    }
}

/// The C declaration of a pointer `X_<NAME>` to the global at `p`.
fn pointer_decl(p: &Place, name: &str) -> String {
    match p {
        Place::Block(_, k) => format!("B_{k}_t *X_{name}"),
        Place::Fb(_, fb) => format!("iecf_{}_t *X_{name}", fb.stem),
        Place::Itf(..) => format!("iecf_itf_t *X_{name}"),
        _ => format!("iecf_ref_t X_{name}"),
    }
}

/// What a pointer to the global at `p` is set to.
fn pointer_to(p: &Place) -> Option<String> {
    Some(match p {
        Place::Block(lv, _) | Place::Fb(lv, _) | Place::Itf(lv, _) => format!("&({lv})"),
        Place::Field(lv, _) | Place::Data(Lv::C(lv), _) => format!("iecf_ref_c(&({lv}))"),
        Place::Located { area, size, byte, bit, .. } => {
            let bit = if *size == 'X' { format!("{bit}u") } else { "IECF_NO_BIT".into() };
            format!("iecf_ref_image((uint8_t)'{area}', {byte}u, {bit})")
        }
        Place::Data(Lv::Bytes { area: crate::names::Area::Image(a), off, bit }, _) => {
            let bit = bit.map_or("IECF_NO_BIT".to_string(), |b| format!("{b}u"));
            format!("iecf_ref_image((uint8_t)'{a}', {off}, {bit})")
        }
        _ => return None,
    })
}

/// The prefix of a global declared in a VAR_GLOBAL list outside any
/// CONFIGURATION, in the keys of the globals and in its C field,
/// `V_0__<NAME>`: no IEC name starts with a digit.
pub(crate) const TOP: &str = "0__";

/// The symbol table's path before a global's name, from its prefix:
/// `R.` for a resource's, `R.P.` for a program instance's, none for the
/// configuration's and a global variable list's.
pub(crate) fn symbol_prefix(rp: &str) -> String {
    if rp == TOP { String::new() } else { rp.replace("__", ".") }
}

/// The globals of `all` (keyed `<NAME>` for the configuration's,
/// `<RESOURCE>__<NAME>` for a resource's, `<RESOURCE>__<INSTANCE>__<NAME>`
/// for a program instance's own) as what runs under the prefix `rp` - a
/// resource's `R__`, a program instance's `R__P__` - sees them, by their
/// names (Ed.3 6.5.2.2). IEC names hold no `__`.
pub(crate) fn global_view(all: &Scope, rp: &str) -> Scope {
    let mut v = Scope { vars: Default::default(), poly: all.poly.clone() };
    // A global variable list's, the configuration's, then each level
    // further in, one named as one further out hiding it.
    let mut prefix = String::new();
    let mut levels = vec![TOP.to_string(), prefix.clone()];
    for part in rp.split("__").filter(|s| !s.is_empty() && rp != TOP) {
        prefix = format!("{prefix}{part}__");
        levels.push(prefix.clone());
    }
    for level in levels {
        for (k, p) in &all.vars {
            if let Some(n) = k.strip_prefix(level.as_str()).filter(|n| !n.contains("__")) {
                v.vars.insert(n.to_string(), p.clone());
            }
        }
    }
    v
}

impl Gen<'_> {
    /// The global the VAR_EXTERNAL `name` of the POU or method `key`
    /// reaches here: the one it reaches wherever it runs in this
    /// configuration, or, where its instances reach different ones, the
    /// instance's own pointer to its, `self->X_<NAME>` (Ed.3 6.5.2.2;
    /// decided 2026-11-13, florin; docs/c-output.md §3).
    pub(crate) fn external(&mut self, key: &str, name: &str, at: Span) -> R<Place> {
        let owner = iecf_sema::owner_of(key).to_string();
        let set: Vec<Binding> = self
            .types
            .bindings(&self.config_key, &owner, name)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        match set.as_slice() {
            [one] => self.bound(one, at),
            [first, ..] => {
                let p = self.bound(first, at)?;
                Ok(pointed(&p, &format!("self->X_{name}")))
            }
            // A type the checker found running nowhere here, written as
            // an in-out's or a reference's: what it names, if this
            // configuration has one global of the name.
            [] => {
                let mut keys: Vec<&String> =
                    self.globals.keys().filter(|k| *k == name || k.rsplit("__").next() == Some(name)).collect();
                keys.sort();
                match keys.first() {
                    Some(k) => Ok(self.globals[*k].clone()),
                    None => self.not_yet(at, "this VAR_EXTERNAL"),
                }
            }
        }
    }

    /// The place of the global `b`.
    fn bound(&mut self, b: &Binding, at: Span) -> R<Place> {
        let k = match b {
            Binding::Config(n) => n.clone(),
            Binding::Resource(r, n) if r.is_empty() => n.clone(),
            Binding::Resource(r, n) => format!("{r}__{n}"),
            Binding::Program(r, i, n) if r.is_empty() => format!("{i}__{n}"),
            Binding::Program(r, i, n) => format!("{r}__{i}__{n}"),
            Binding::Top(n) => format!("{TOP}{n}"),
        };
        match self.globals.get(&k) {
            Some(p) => Ok(p.clone()),
            None => self.not_yet(at, "this VAR_EXTERNAL"),
        }
    }

    /// The names the instances of `key` reach different globals by here:
    /// each one's pointer in the instance (`X_<NAME>`).
    pub(crate) fn pointer_names(&self, key: &str) -> Vec<String> {
        let key = key.to_ascii_uppercase();
        let key = key.as_str();
        self.types
            .bound_names(&self.config_key, key)
            .into_iter()
            .filter(|n| self.types.bindings(&self.config_key, key, n).is_some_and(|s| s.len() > 1))
            .collect()
    }

    /// The C fields of `key`'s pointers (`pointer_names`).
    pub(crate) fn pointer_fields(&mut self, key: &str, at: Span) -> R<String> {
        let mut out = String::new();
        for n in self.pointer_names(key) {
            let first = self.types.bindings(&self.config_key, key, &n).and_then(|s| s.first().cloned());
            let Some(first) = first else { continue };
            let p = self.bound(&first, at)?;
            let _ = writeln!(out, "    {};", pointer_decl(&p, &n));
        }
        Ok(out)
    }

    /// Code setting the pointers (`pointer_names`) of the instance at `lv`
    /// of the PROGRAM, FUNCTION_BLOCK or CLASS `key`, and of every
    /// instance it holds, its bases' too: each to what `value(name)`
    /// gives, the global the name reaches where the instance runs.
    pub(crate) fn set_pointers(
        &mut self,
        lv: &str,
        key: &str,
        value: &dyn Fn(&mut Self, &str, &str) -> R<String>,
        out: &mut String,
        depth: usize,
    ) -> R<()> {
        if depth > 32 {
            return Ok(());
        }
        let mut level = Some((key.to_string(), lv.to_string()));
        while let Some((k, at)) = level {
            for n in self.pointer_names(&k) {
                let field = format!("{at}.X_{n}");
                let v = value(self, &n, &field)?;
                let _ = writeln!(out, "    {field} = {v};");
            }
            let scope = match self.pous.get(k.as_str()).map(|p| p.kind) {
                Some(PouKind::Program) => self.program_scopes.get(&k).cloned(),
                _ => self.block_scopes.get(&k).cloned(),
            }
            .unwrap_or_default();
            let mut names: Vec<&String> = scope.vars.keys().collect();
            names.sort();
            for name in names {
                let own = format!("self->V_{name}");
                match &scope.vars[name] {
                    Place::Block(clv, inner) if *clv == own => {
                        self.set_pointers(&format!("{at}.V_{name}"), inner, value, out, depth + 1)?;
                    }
                    Place::Data(Lv::C(clv), t) if *clv == own && self.has_instance(t) => {
                        self.set_held_pointers(&format!("{at}.V_{name}"), t, value, out, depth + 1)?;
                    }
                    _ => {}
                }
            }
            level = self.types.base(&k).map(|b| (b.to_ascii_uppercase(), format!("{at}.base")));
        }
        Ok(())
    }

    /// `set_pointers` for each instance a value of `t` at `lv` holds.
    pub(crate) fn set_held(
        &mut self,
        lv: &str,
        t: &DTy,
        value: &dyn Fn(&mut Self, &str, &str) -> R<String>,
        out: &mut String,
    ) -> R<()> {
        self.set_held_pointers(lv, t, value, out, 0)
    }

    /// `set_pointers` for each instance a value of `t` at `lv` holds: an
    /// array's elements in a loop, a structure's fields.
    fn set_held_pointers(
        &mut self,
        lv: &str,
        t: &DTy,
        value: &dyn Fn(&mut Self, &str, &str) -> R<String>,
        out: &mut String,
        depth: usize,
    ) -> R<()> {
        match &t.kind {
            Kind::Block(k) => self.set_pointers(lv, k, value, out, depth),
            Kind::Array(elem, bounds) => {
                let ks: Vec<String> = bounds.iter().map(|_| self.fresh("p")).collect();
                let mut at = lv.to_string();
                for k in &ks {
                    at = format!("{at}[{k}]");
                }
                let mut inner = String::new();
                self.set_held_pointers(&at, elem, value, &mut inner, depth)?;
                if inner.is_empty() {
                    return Ok(());
                }
                let mut body = inner.trim_end().to_string();
                for (k, (lo, hi)) in ks.iter().zip(bounds).rev() {
                    let n = crate::derived::count(*lo, *hi).unwrap_or(0);
                    body = format!("for (uint32_t {k} = 0; {k} < {n}u; {k}++) {{\n{body}\n    }}");
                }
                let _ = writeln!(out, "    {body}");
                Ok(())
            }
            Kind::Struct(k) => {
                let fields = self.structs.get(k).cloned().unwrap_or_default();
                for (n, ft) in fields {
                    if self.has_instance(&ft) {
                        self.set_held_pointers(&format!("{lv}.V_{n}"), &ft, value, out, depth)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The value a pointer `name` of an instance created as `key` runs
    /// (a VAR_TEMP, a FUNCTION's or method's variable) is set to: `key`'s
    /// own, if it has one by the name, else the one global it reaches.
    pub(crate) fn pointer_here(&mut self, key: &str, name: &str, at: Span) -> R<String> {
        let owner = iecf_sema::owner_of(key).to_string();
        if self.pointer_names(&owner).iter().any(|n| n == name) {
            return Ok(format!("self->X_{name}"));
        }
        let p = self.external(key, name, at)?;
        match pointer_to(&p) {
            Some(v) => Ok(v),
            None => self.not_yet(at, "this VAR_EXTERNAL"),
        }
    }

    /// The value a pointer `name` of an instance running in the resource
    /// whose globals `view` holds is set to.
    pub(crate) fn pointer_in(&mut self, view: &Scope, name: &str, at: Span) -> R<String> {
        match view.vars.get(name).and_then(pointer_to) {
            Some(v) => Ok(v),
            None => self.not_yet(at, format!("the global {name} here")),
        }
    }
}

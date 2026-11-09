//! Names and types: every name resolves, every value meets its target
//! by the standard's conversion rules, every call fits what it calls.
//!
//! One error per mistake: an expression whose type could not be worked
//! out is `Ty::Unknown`, and nothing is reported against it again.

use crate::stdlib::{Dir, reserved_name, std_fb, std_sig, unify};

mod stdcall;
use crate::types::{Conversion, Elem, Ty};
use crate::{Extension, Options};
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_syntax::ast::*;
use std::collections::{HashMap, HashSet};

fn upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

/// A variable as a POU sees it.
#[derive(Clone, Debug)]
struct Var {
    ty: Ty,
    constant: bool,
    /// Gone when its POU returns: VAR_TEMP, or any variable of a FUNCTION
    /// but a VAR_EXTERNAL (Ed.3 6.4.4.10.3, for REF()).
    temp: bool,
    /// At an input address, `%I` (or `%I*`), its own or its global's.
    input: bool,
}

impl Var {
    fn of(ty: Ty, constant: bool) -> Var {
        Var { ty, constant, temp: false, input: false }
    }
}

/// Whether a declaration is at an input address.
fn at_input(d: &VarDecl) -> bool {
    d.location.as_ref().is_some_and(|l| l.area() == Some('I'))
}

/// A parameter of a function or function block.
#[derive(Clone, Debug)]
struct Param {
    name: String,
    dir: Dir,
    ty: Ty,
}

/// A variable declared `AT %I*` (or `%Q*`, `%M*`) somewhere inside an
/// instance: its path from the instance, and what VAR_CONFIG must give.
#[derive(Clone, Debug)]
struct Partly {
    path: Vec<String>,
    area: char,
    ty: Ty,
}

/// Where VAR_CONFIG paths start: a program instance, under the names
/// that lead to it (`cpu.main`; `main`; `PLC_PRG` for a VAR_CONFIG list).
struct ProgInstance {
    prefix: Vec<String>,
    program: String,
    span: Span,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Callee {
    Function,
    FunctionBlock,
    /// A program instance's connections in a resource (Ed.3 6.8.2).
    Program,
}

pub(crate) struct Checker<'a> {
    options: Options,
    pous: HashMap<String, &'a Pou>,
    types: HashMap<String, &'a TypeDecl>,
    globals: HashMap<String, (&'a VarDecl, bool)>,
    /// enumerated value -> the enum types that have it
    enum_values: HashMap<String, Vec<String>>,
    type_cache: HashMap<String, Ty>,
    resolving: Vec<String>,
    struct_fields: HashMap<String, HashMap<String, Ty>>,
    params_cache: HashMap<String, Vec<Param>>,
    /// POU or structure type -> its `*` addresses, through nested instances.
    partly_cache: HashMap<String, Vec<Partly>>,
    /// Untyped one-character STRING literals, by span: each is a CHAR
    /// where a CHAR is wanted (Ed.3 Table 6, 1b: "String of length one
    /// or character CHAR").
    one_char: std::collections::HashSet<Span>,
    partly_visiting: Vec<String>,
    diags: Vec<Diagnostic>,
    // The POU being checked.
    locals: HashMap<String, Var>,
    pou_kind: Option<PouKind>,
    /// The POU being checked, upper-cased.
    pou_key: String,
    steps: HashMap<String, Span>,
    actions: HashMap<String, Span>,
    loop_depth: usize,
    /// What was worked out, for the code generator (`crate::Types`).
    pub(crate) out: crate::Types,
}

impl<'a> Checker<'a> {
    pub(crate) fn new(options: Options) -> Self {
        Checker {
            options,
            pous: HashMap::new(),
            types: HashMap::new(),
            globals: HashMap::new(),
            enum_values: HashMap::new(),
            type_cache: HashMap::new(),
            resolving: Vec::new(),
            struct_fields: HashMap::new(),
            params_cache: HashMap::new(),
            partly_cache: HashMap::new(),
            one_char: std::collections::HashSet::new(),
            partly_visiting: Vec::new(),
            diags: Vec::new(),
            locals: HashMap::new(),
            pou_kind: None,
            pou_key: String::new(),
            steps: HashMap::new(),
            actions: HashMap::new(),
            loop_depth: 0,
            out: crate::Types::default(),
        }
    }

    fn error(&mut self, code: Code, span: Span, message: impl Into<String>) {
        self.diags.push(Diagnostic::new(code, span, message));
    }

    pub(crate) fn finish(self) -> (Vec<Diagnostic>, crate::Types) {
        (self.diags, self.out)
    }

    // ---- the project ----------------------------------------------

    /// Every POU, type and global of every file, before any body is read:
    /// a name may be used above its declaration, or in another file.
    pub(crate) fn collect(&mut self, units: &[&'a SourceUnit]) {
        let mut top: HashMap<String, Span> = HashMap::new();
        let mut declare_top = |c: &mut Self, id: &Ident, what: &str| {
            if let Some(why) = reserved_name(&id.text) {
                c.error(
                    Code::ReservedName,
                    id.span,
                    format!(
                        "'{}' is reserved: it names {why} (Ed.3 6.1.3); choose another name for this {what}",
                        id.text
                    ),
                );
            }
            if top.insert(upper(&id.text), id.span).is_some() {
                c.error(
                    Code::Duplicate,
                    id.span,
                    format!("'{}' is declared twice: POUs and types share one namespace", id.text),
                );
                return false;
            }
            true
        };
        for unit in units {
            for item in &unit.items {
                match item {
                    Item::Pou(p) if !p.name.text.is_empty() => {
                        let what = match p.kind {
                            PouKind::Program => "PROGRAM",
                            PouKind::Function => "FUNCTION",
                            PouKind::FunctionBlock => "FUNCTION_BLOCK",
                        };
                        if declare_top(self, &p.name, what) {
                            self.pous.insert(upper(&p.name.text), p);
                        }
                    }
                    Item::Types(decls) => {
                        for d in decls {
                            if d.name.text.is_empty() || !declare_top(self, &d.name, "type") {
                                continue;
                            }
                            self.types.insert(upper(&d.name.text), d);
                            if let TypeDef::Spec(TypeSpec::Enum { values, .. }) = &d.def {
                                for v in values {
                                    self.enum_values.entry(upper(&v.name.text)).or_default().push(upper(&d.name.text));
                                }
                            }
                        }
                    }
                    Item::Globals(block) => self.collect_globals(block),
                    Item::Configuration(c) => {
                        for b in &c.globals {
                            self.collect_globals(b);
                        }
                        for r in &c.resources {
                            for b in &r.globals {
                                self.collect_globals(b);
                            }
                        }
                    }
                    Item::Pou(_) | Item::ConfigList(..) => {}
                }
            }
        }
    }

    fn collect_globals(&mut self, block: &'a VarBlock) {
        for d in &block.decls {
            for n in &d.names {
                if self.globals.insert(upper(&n.text), (d, block.constant)).is_some() {
                    self.error(Code::Duplicate, n.span, format!("global variable '{}' is declared twice", n.text));
                }
            }
        }
    }

    pub(crate) fn check_all(&mut self, units: &[&'a SourceUnit]) {
        let mut lists: Vec<(&'a [ConfigInit], Span)> = Vec::new();
        let mut configured = false;
        for unit in units {
            for item in &unit.items {
                match item {
                    Item::Types(decls) => {
                        for d in decls {
                            self.check_type_decl(d);
                        }
                    }
                    Item::Pou(p) => self.check_pou(p),
                    Item::Globals(b) => self.check_global_block(b),
                    Item::Configuration(c) => {
                        configured = true;
                        self.check_configuration(c);
                    }
                    Item::ConfigList(entries, at) => lists.push((entries, *at)),
                }
            }
        }
        self.check_config_lists(&lists, configured);
    }

    /// VAR_CONFIG outside a CONFIGURATION, with the var-config-list
    /// extension (docs/extensions.md): as CODESYS, each PROGRAM is its
    /// own one instance, and its `*` addresses are completed here.
    fn check_config_lists(&mut self, lists: &[(&'a [ConfigInit], Span)], configured: bool) {
        if !self.options.has(Extension::VarConfigList) {
            for (_, at) in lists {
                self.error(
                    Code::ConfigOutsideConfiguration,
                    *at,
                    "VAR_CONFIG belongs inside a CONFIGURATION (Ed.3 6.8.2); a list of its own, as CODESYS writes it, needs --extension var-config-list",
                );
            }
            return;
        }
        if configured {
            for (_, at) in lists {
                self.error(
                    Code::ConfigOutsideConfiguration,
                    *at,
                    "this project has a CONFIGURATION: its VAR_CONFIG goes inside it",
                );
            }
            return;
        }
        let mut programs: Vec<&'a Pou> = self.pous.values().copied().filter(|p| p.kind == PouKind::Program).collect();
        programs.sort_by_key(|p| (p.name.span.file, p.name.span.start));
        let instances: Vec<ProgInstance> = programs
            .iter()
            .map(|p| ProgInstance {
                prefix: vec![p.name.text.clone()],
                program: upper(&p.name.text),
                span: p.name.span,
            })
            .collect();
        let entries: Vec<&ConfigInit> = lists.iter().flat_map(|(e, _)| e.iter()).collect();
        self.locals.clear();
        let globals: Vec<(String, &'a VarDecl, bool)> =
            self.globals.iter().map(|(k, (d, c))| (k.clone(), *d, *c)).collect();
        for (key, d, constant) in globals {
            let ty = self.resolve_spec(&d.ty, false);
            self.locals.insert(key, Var::of(ty, constant));
        }
        self.check_config_inits(&entries, &instances, &["PROGRAM"]);
    }

    // ---- types ----------------------------------------------------

    /// The type a declaration names. `report` false resolves silently:
    /// the declaration's own check reports its mistakes once.
    fn resolve_spec(&mut self, spec: &TypeSpec, report: bool) -> Ty {
        match spec {
            TypeSpec::Named(id) => {
                if id.text.is_empty() {
                    return Ty::Unknown;
                }
                if let Some(e) = Elem::from_name(&id.text) {
                    return Ty::Elem(e);
                }
                let key = upper(&id.text);
                if self.types.contains_key(&key) {
                    return self.named_type(&key);
                }
                if let Some(p) = self.pous.get(&key) {
                    if p.kind == PouKind::FunctionBlock {
                        return Ty::Fb(key);
                    }
                    if report {
                        self.error(Code::UnknownType, id.span, format!("'{}' is a {:?}, not a type", id.text, p.kind));
                    }
                    return Ty::Unknown;
                }
                if std_fb(&key).is_some() {
                    return Ty::Fb(key);
                }
                if report {
                    self.error(Code::UnknownType, id.span, format!("there is no type '{}'", id.text));
                }
                Ty::Unknown
            }
            TypeSpec::String { name, len, .. } => {
                if name.is("USTRING") {
                    if report {
                        self.error(Code::NotYetSupported, name.span, "USTRING (Ed.4) is not compiled by iecf yet");
                    }
                    return Ty::Unknown;
                }
                let len = len.as_ref().and_then(|e| self.const_int(e)).and_then(|v| u64::try_from(v).ok());
                Ty::Str { wide: name.is("WSTRING"), len }
            }
            TypeSpec::Array { dims, elem, .. } => {
                if report {
                    for (lo, hi) in dims.iter().flatten() {
                        for e in [lo, hi] {
                            let t = self.expr(e);
                            if !t.is_unknown() && !t.is_int() {
                                self.error(
                                    Code::OperandTypes,
                                    e.span,
                                    format!("an array bound is an integer, not {t}"),
                                );
                            }
                        }
                    }
                }
                let bounds = dims
                    .iter()
                    .map(|d| d.as_ref().and_then(|(lo, hi)| Some((self.const_int(lo)?, self.const_int(hi)?))))
                    .collect();
                let elem_spec = elem;
                let elem = self.resolve_spec(elem, report);
                if report && matches!(elem, Ty::Reference(_)) {
                    self.error(
                        Code::ReferenceNotKept,
                        elem_spec.span(),
                        "an array cannot hold REFERENCE TO (CODESYS, TwinCAT): use REF_TO",
                    );
                }
                Ty::Array { elem: Box::new(elem), dims: bounds }
            }
            // A structure may reference its own type: a reference does not
            // contain what it references.
            TypeSpec::Ref { target, implicit, span } => {
                let to = match &**target {
                    TypeSpec::Named(id)
                        if self.resolving.contains(&upper(&id.text))
                            && matches!(
                                self.types.get(&upper(&id.text)).map(|t| &t.def),
                                Some(TypeDef::Struct { .. })
                            ) =>
                    {
                        Ty::Struct(upper(&id.text))
                    }
                    t => self.resolve_spec(t, report),
                };
                if report && *implicit && !self.options.has(Extension::ReferenceTo) {
                    self.error(
                        Code::UnknownType,
                        *span,
                        "REFERENCE TO is CODESYS's, not IEC 61131-3: write REF_TO, or --extension reference-to",
                    );
                }
                // A reference's base is not a REFERENCE TO (CODESYS, TwinCAT).
                if report && matches!(to, Ty::Reference(_)) {
                    self.error(Code::ReferenceNotKept, *span, "a reference cannot reference a REFERENCE TO");
                }
                if report && *implicit && matches!(to, Ty::Ref(_)) {
                    self.error(Code::ReferenceNotKept, *span, "a REFERENCE TO cannot reference a reference");
                }
                if *implicit { Ty::Reference(Box::new(to)) } else { Ty::Ref(Box::new(to)) }
            }
            TypeSpec::Subrange { base, .. } => match Elem::from_name(&base.text) {
                Some(e) if e.is_int() => Ty::Elem(e),
                _ => {
                    if report {
                        self.error(
                            Code::UnknownType,
                            base.span,
                            format!("a subrange is of an integer type, not '{}'", base.text),
                        );
                    }
                    Ty::Unknown
                }
            },
            TypeSpec::Enum { values, .. } => {
                // An enum declared in place: its values are known in this
                // POU under a name made of them.
                let name = format!("({})", values.iter().map(|v| v.name.text.as_str()).collect::<Vec<_>>().join(", "));
                for v in values {
                    self.enum_values.entry(upper(&v.name.text)).or_default().push(upper(&name));
                }
                Ty::Enum(upper(&name))
            }
        }
    }

    fn named_type(&mut self, key: &str) -> Ty {
        if let Some(t) = self.type_cache.get(key) {
            return t.clone();
        }
        let Some(decl) = self.types.get(key).copied() else { return Ty::Unknown };
        if self.resolving.iter().any(|k| k == key) {
            self.error(
                Code::UnknownType,
                decl.name.span,
                format!("type '{}' is defined in terms of itself", decl.name.text),
            );
            self.type_cache.insert(key.to_string(), Ty::Unknown);
            return Ty::Unknown;
        }
        self.resolving.push(key.to_string());
        let ty = match &decl.def {
            TypeDef::Struct { fields, .. } => {
                let mut map = HashMap::new();
                for f in fields {
                    let t = self.resolve_spec(&f.ty, false);
                    for n in &f.names {
                        map.insert(upper(&n.text), t.clone());
                    }
                }
                self.struct_fields.insert(key.to_string(), map);
                Ty::Struct(key.to_string())
            }
            TypeDef::Spec(TypeSpec::Enum { .. }) => Ty::Enum(key.to_string()),
            TypeDef::Spec(spec) => self.resolve_spec(spec, false),
        };
        self.resolving.pop();
        self.type_cache.insert(key.to_string(), ty.clone());
        ty
    }

    fn check_type_decl(&mut self, d: &TypeDecl) {
        let key = upper(&d.name.text);
        match &d.def {
            TypeDef::Struct { fields, .. } => {
                let mut seen: HashMap<String, Span> = HashMap::new();
                for f in fields {
                    let t = self.resolve_spec(&f.ty, true);
                    for n in &f.names {
                        if seen.insert(upper(&n.text), n.span).is_some() {
                            self.error(Code::Duplicate, n.span, format!("field '{}' is declared twice", n.text));
                        }
                    }
                    if let Some(init) = &f.init {
                        self.check_init(init, &t);
                    }
                }
            }
            TypeDef::Spec(TypeSpec::Enum { values, .. }) => {
                let mut seen: HashMap<String, Span> = HashMap::new();
                for v in values {
                    if seen.insert(upper(&v.name.text), v.name.span).is_some() {
                        self.error(
                            Code::Duplicate,
                            v.name.span,
                            format!("value '{}' is in this enumeration twice", v.name.text),
                        );
                    }
                    if let Some(e) = &v.value {
                        let t = self.expr(e);
                        if !t.is_unknown() && !t.is_int() {
                            self.error(
                                Code::TypeMismatch,
                                e.span,
                                format!("an enumerated value is an integer, not {t}"),
                            );
                        }
                    }
                }
            }
            TypeDef::Spec(spec) => {
                self.resolve_spec(spec, true);
            }
        }
        if let Some(init) = &d.init {
            let t = if self.types.contains_key(&key) { self.named_type(&key) } else { Ty::Unknown };
            self.check_init(init, &t);
        }
    }

    // ---- POUs -----------------------------------------------------

    /// The parameters of a function or function block, in declaration
    /// order: inputs, in-outs, outputs.
    fn params(&mut self, key: &str) -> Vec<Param> {
        if let Some(p) = self.params_cache.get(key) {
            return p.clone();
        }
        let params = if let Some(pou) = self.pous.get(key).copied() {
            let mut out = Vec::new();
            for b in &pou.var_blocks {
                let dir = match b.kind {
                    VarKind::Input => Dir::Input,
                    VarKind::Output => Dir::Output,
                    VarKind::InOut => Dir::InOut,
                    _ => continue,
                };
                for d in &b.decls {
                    let ty = self.resolve_spec(&d.ty, false);
                    for n in &d.names {
                        out.push(Param { name: upper(&n.text), dir, ty: ty.clone() });
                    }
                }
            }
            out
        } else if let Some(ps) = std_fb(key) {
            ps.into_iter().map(|p| Param { name: p.name.to_string(), dir: p.dir, ty: p.ty }).collect()
        } else {
            Vec::new()
        };
        self.params_cache.insert(key.to_string(), params.clone());
        params
    }

    /// The internal variables of a project FUNCTION_BLOCK, which the
    /// outside may not touch.
    fn is_internal_member(&self, fb: &str, member: &str) -> bool {
        self.pous.get(fb).is_some_and(|p| {
            p.var_blocks.iter().any(|b| {
                matches!(b.kind, VarKind::Var | VarKind::Temp | VarKind::External)
                    && b.decls.iter().any(|d| d.names.iter().any(|n| n.is(member)))
            })
        })
    }

    fn check_pou(&mut self, pou: &'a Pou) {
        self.locals.clear();
        self.steps.clear();
        self.actions.clear();
        self.pou_kind = Some(pou.kind);
        self.pou_key = upper(&pou.name.text);
        if let Some(rt) = &pou.return_type {
            let t = self.resolve_spec(rt, true);
            self.out.decls.insert(rt.span(), t.clone());
            let temp = pou.kind == PouKind::Function;
            self.locals.insert(upper(&pou.name.text), Var { temp, ..Var::of(t, false) });
        } else if pou.kind == PouKind::Function {
            self.locals.insert(upper(&pou.name.text), Var { temp: true, ..Var::of(Ty::Void, true) });
        }
        for b in &pou.var_blocks {
            self.declare_block(b, Some(pou.kind));
        }
        for el in &pou.sfc {
            match el {
                SfcElement::Step(s) => {
                    if self.steps.insert(upper(&s.name.text), s.name.span).is_some() {
                        self.error(Code::Duplicate, s.name.span, format!("step '{}' is declared twice", s.name.text));
                    }
                }
                SfcElement::Action(a) => {
                    if self.actions.insert(upper(&a.name.text), a.name.span).is_some() {
                        self.error(Code::Duplicate, a.name.span, format!("action '{}' is declared twice", a.name.text));
                    }
                }
                SfcElement::Transition(_) => {}
            }
        }
        // Inits after every declaration: an initial value may name a
        // constant declared below it.
        for b in &pou.var_blocks {
            for d in &b.decls {
                if let Some(init) = &d.init {
                    let t = self.resolve_spec(&d.ty, false);
                    self.check_init(init, &t);
                }
            }
        }
        self.stmts(&pou.body);
        let initial = pou.sfc.iter().filter(|e| matches!(e, SfcElement::Step(s) if s.initial)).count();
        if !pou.sfc.is_empty() && initial != 1 && pou.sfc.iter().any(|e| matches!(e, SfcElement::Step(_))) {
            self.error(
                Code::BadCall,
                pou.name.span,
                format!("an SFC network has exactly one INITIAL_STEP; this one has {initial}"),
            );
        }
        for el in &pou.sfc {
            match el {
                SfcElement::Step(s) => {
                    for a in &s.actions {
                        let key = upper(&a.action.text);
                        let is_bool_var = self.variable(&key).is_some_and(|v| v.ty.is_bool());
                        if !self.actions.contains_key(&key) && !is_bool_var {
                            self.error(
                                Code::UnknownName,
                                a.action.span,
                                format!("no action or BOOL variable '{}'", a.action.text),
                            );
                        }
                        if let Some(t) = &a.time {
                            let ty = self.expr(t);
                            self.coerce(&ty, &Ty::Elem(Elem::Time), t.span);
                        }
                    }
                }
                SfcElement::Transition(t) => {
                    for s in t.from.iter().chain(&t.to) {
                        if !s.text.is_empty() && !self.steps.contains_key(&upper(&s.text)) {
                            self.error(Code::UnknownName, s.span, format!("no step '{}'", s.text));
                        }
                    }
                    self.condition(&t.condition);
                }
                SfcElement::Action(a) => self.stmts(&a.body),
            }
        }
        self.pou_kind = None;
    }

    fn declare_block(&mut self, b: &VarBlock, pou: Option<PouKind>) {
        self.check_block_form(b, pou);
        for d in &b.decls {
            let ty = self.resolve_spec(&d.ty, true);
            self.out.decls.insert(d.span, ty.clone());
            self.check_location(b, d, &ty, pou);
            self.check_reference_kept(b, d, &ty);
            if d.edge.is_some() && !(b.kind == VarKind::Input && ty.is_bool()) {
                self.error(Code::BadCall, d.span, "R_EDGE and F_EDGE are for BOOL inputs (Ed.3 6.5.1)");
            }
            for n in &d.names {
                if let Some(why) = reserved_name(&n.text) {
                    self.error(
                        Code::ReservedName,
                        n.span,
                        format!("'{}' is reserved: it names {why} (Ed.3 6.1.3); choose another name", n.text),
                    );
                }
                let key = upper(&n.text);
                if self.locals.contains_key(&key) {
                    self.error(Code::Duplicate, n.span, format!("'{}' is declared twice in this POU", n.text));
                    continue;
                }
                let mut input = at_input(d);
                if b.kind == VarKind::External {
                    input = self.globals.get(&key).is_some_and(|(g, _)| at_input(g));
                    match self.globals.get(&key).copied() {
                        None => self.error(
                            Code::ExternalWithoutGlobal,
                            n.span,
                            format!("there is no global variable '{}'", n.text),
                        ),
                        Some((g, g_const)) => {
                            let gt = self.resolve_spec(&g.ty, false);
                            if !gt.is_unknown() && !ty.is_unknown() && gt != ty {
                                self.error(
                                    Code::ExternalWithoutGlobal,
                                    d.ty.span(),
                                    format!("global '{}' is {gt}, so its VAR_EXTERNAL must be {gt}, not {ty}", n.text),
                                );
                            }
                            if g_const && !b.constant {
                                self.error(
                                    Code::ExternalWithoutGlobal,
                                    n.span,
                                    format!("global '{}' is CONSTANT: declare it VAR_EXTERNAL CONSTANT", n.text),
                                );
                            }
                        }
                    }
                }
                let temp = b.kind == VarKind::Temp || (pou == Some(PouKind::Function) && b.kind != VarKind::External);
                self.locals.insert(key, Var { temp, input, ..Var::of(ty.clone(), b.constant) });
            }
        }
    }

    /// A `VAR` of a PROGRAM or FUNCTION_BLOCK is one of three forms, each
    /// its own production (Ed.3 Annex A; Ed.2 B.1.4.3 the same): plain
    /// (`Var_Decls`), addressed in full (`Loc_Var_Decls`), or with `*`
    /// addresses (`Loc_Partly_Var_Decl`). A block that mixes them is
    /// refused at its first declaration of another form.
    fn check_block_form(&mut self, b: &VarBlock, pou: Option<PouKind>) {
        if b.kind != VarKind::Var || !matches!(pou, Some(PouKind::Program | PouKind::FunctionBlock)) {
            return;
        }
        let form = |d: &VarDecl| match &d.location {
            None => 0,
            Some(l) if l.is_partly() => 2,
            Some(_) => 1,
        };
        const NAMES: [&str; 3] = ["plain variables", "located variables", "`*` addresses"];
        let Some(first) = b.decls.first().map(form) else { return };
        if let Some(d) = b.decls.iter().find(|d| form(d) != first) {
            let what = d.names.first().map(|n| format!("'{}'", n.text)).unwrap_or_else(|| "this one".into());
            self.error(
                Code::BadLocation,
                d.span,
                format!(
                    "this VAR block holds {}, so {what} goes in a VAR block of its own: \
                     the standard keeps plain, located and `*` declarations in separate blocks (Ed.3 Annex A)",
                    NAMES[first]
                ),
            );
        }
    }

    /// An elementary variable at `loc`: the address is where it starts,
    /// and it takes its own type's size (Ed.3 6.5.5.3: "does not need to
    /// be of equal or bigger size"), warned when the size prefix differs,
    /// as CODESYS's "should match" (docs/dependencies.md). A BOOL at a byte
    /// takes the whole byte. Only a BOOL starts at a bit.
    fn check_location_size(&mut self, loc: &DirectAddress, ty: &Ty) {
        let Some(size) = loc.size() else { return };
        // An array, structure or string lies from a byte, its parts back
        // to back (docs/dependencies.md); the size prefix is compared with
        // its first element's or field's.
        if !matches!(ty, Ty::Elem(_)) {
            if size == 'X' && matches!(ty, Ty::Array { .. } | Ty::Struct(_) | Ty::Str { .. }) {
                self.error(
                    Code::BadLocation,
                    loc.span,
                    format!("{ty} at {}: an array, structure or string starts at a byte", loc.text),
                );
                return;
            }
            let Some(first) = self.first_part(ty, 0) else { return };
            let Some(bits) = first.bits() else { return };
            let named = match size {
                'X' => 1,
                'B' => 8,
                'W' => 16,
                'D' => 32,
                _ => 64,
            };
            // A BOOL inside takes a byte.
            if bits.max(8) != named && !(bits == 1 && named == 1) {
                self.error(
                    Code::LocationSize,
                    loc.span,
                    format!(
                        "{ty} at {}: its first part is {}, {}, and {size} names {}",
                        loc.text,
                        first.name(),
                        bytes(bits.max(8) / 8),
                        bytes(named / 8)
                    ),
                );
            }
            return;
        }
        let Ty::Elem(e) = ty else { return };
        let t = e.name();
        let Some(bits) = e.bits() else { return };
        let named = match size {
            'X' => 1,
            'B' => 8,
            'W' => 16,
            'D' => 32,
            _ => 64,
        };
        let digits = loc.text.get(3..).unwrap_or_default();
        if size == 'X' && bits > 1 {
            self.error(
                Code::BadLocation,
                loc.span,
                format!("{t} at {}: only a BOOL starts at a bit; a {bits}-bit value starts at a byte", loc.text),
            );
        } else if bits == 1 && named != 1 {
            self.error(
                Code::LocationSize,
                loc.span,
                format!(
                    "a BOOL at {} takes byte {} whole: TRUE is 1, and any value but 0 reads TRUE",
                    loc.text, digits
                ),
            );
        } else if bits != named {
            let at = match digits.parse::<u64>() {
                Ok(b) => format!("bytes {b} to {}", b + u64::from(bits / 8) - 1),
                Err(_) => format!("{} bytes", bits / 8),
            };
            self.error(
                Code::LocationSize,
                loc.span,
                format!(
                    "{t} at {} takes {at}: {t} is {}, {size} names {}",
                    loc.text,
                    bytes(bits / 8),
                    bytes(named / 8)
                ),
            );
        }
    }

    /// Whether a value of `ty` holds a reference anywhere: itself, an
    /// element, a field, or a variable of a function block instance.
    fn holds_ref(&mut self, ty: &Ty, depth: usize) -> bool {
        if depth > 32 {
            return false;
        }
        match ty {
            Ty::Ref(_) | Ty::Reference(_) => true,
            Ty::Array { elem, .. } => self.holds_ref(elem, depth + 1),
            Ty::Struct(k) => {
                let fields: Vec<&'a VarDecl> = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().collect(),
                    _ => Vec::new(),
                };
                fields.into_iter().any(|f| {
                    let t = self.resolve_spec(&f.ty, false);
                    self.holds_ref(&t, depth + 1)
                })
            }
            Ty::Fb(k) => {
                let decls: Vec<&'a VarDecl> = match self.pous.get(k) {
                    Some(p) => p
                        .var_blocks
                        .iter()
                        .filter(|b| !matches!(b.kind, VarKind::Temp | VarKind::External | VarKind::InOut))
                        .flat_map(|b| b.decls.iter())
                        .collect(),
                    None => Vec::new(),
                };
                decls.into_iter().any(|d| {
                    let t = self.resolve_spec(&d.ty, false);
                    self.holds_ref(&t, depth + 1)
                })
            }
            _ => false,
        }
    }

    /// A reference where it cannot be kept (E0027): VAR_IN_OUT (Ed.3
    /// 6.4.4.10.3), RETAIN, or an address (docs/dependencies.md).
    fn check_reference_kept(&mut self, b: &VarBlock, d: &VarDecl, ty: &Ty) {
        if b.kind == VarKind::InOut && matches!(ty, Ty::Ref(_) | Ty::Reference(_)) {
            self.error(
                Code::ReferenceNotKept,
                d.ty.span(),
                "a reference is not a VAR_IN_OUT (Ed.3 6.4.4.10.3): pass it as a VAR_INPUT",
            );
            return;
        }
        let retained = b.retain == Some(Retain::Retain);
        if !(retained || d.location.is_some()) || !self.holds_ref(ty, 0) {
            return;
        }
        let what = if matches!(ty, Ty::Ref(_) | Ty::Reference(_)) {
            "a reference".to_string()
        } else {
            format!("{ty}, which holds a reference,")
        };
        let message = if retained {
            format!("{what} is not RETAIN: after a restart it would reference nothing; retain what it references")
        } else {
            format!("{what} has no place at an address: the process image holds values")
        };
        self.error(Code::ReferenceNotKept, d.span, message);
    }

    /// `REF(x)` (Ed.3 6.4.4.10.3): a reference to the variable `x`.
    fn ref_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let [Arg::Positional(x)] = args else {
            self.error(Code::BadCall, span, "REF takes one variable: REF(x)");
            self.args_only(args);
            return Ty::Unknown;
        };
        let t = self.expr(x);
        if t.is_unknown() {
            return Ty::Unknown;
        }
        if let Err(why) = self.referable(x) {
            self.error(Code::BadReference, x.span, why);
            return Ty::Unknown;
        }
        Ty::Ref(Box::new(t))
    }

    /// Whether `REF(x)` may be taken (E0026), warning when `x` is at an
    /// input address (W0005).
    fn referable(&mut self, x: &Expr) -> Result<(), String> {
        match &x.kind {
            ExprKind::Paren(e) => self.referable(e),
            ExprKind::Deref(_) => Ok(()),
            ExprKind::Name(id) => {
                let key = upper(&id.text);
                if (key == "EN" || key == "ENO")
                    && matches!(self.pou_kind, Some(PouKind::Function | PouKind::FunctionBlock))
                {
                    return Err(format!("REF({}) is an error (Ed.3 6.6.1.5, rule 8)", id.text));
                }
                let Some(v) = self.variable(&key) else {
                    return Err(format!("'{}' is not a variable: REF() takes a variable", id.text));
                };
                if matches!(v.ty, Ty::Struct(ref k) if k.starts_with('<')) {
                    return Err(format!("'{}' is not a variable: REF() takes a variable", id.text));
                }
                if v.temp {
                    let what = if self.pou_kind == Some(PouKind::Function) {
                        "a variable of a FUNCTION"
                    } else {
                        "a VAR_TEMP"
                    };
                    return Err(format!(
                        "'{}' is {what}, gone when the POU returns: REF() of a temporary is not permitted \
                         (Ed.3 6.4.4.10.3)",
                        id.text
                    ));
                }
                if v.constant {
                    return Err(format!("'{}' is CONSTANT: a reference could write it", id.text));
                }
                if v.input {
                    self.error(
                        Code::ReferenceToInput,
                        x.span,
                        format!(
                            "'{}' is at an input address: the next inputs overwrite whatever is written \
                             through this reference",
                            id.text
                        ),
                    );
                }
                Ok(())
            }
            ExprKind::Member(base, m) => {
                if let Some(Ty::Fb(fb)) = self.out.exprs.get(&base.span).cloned()
                    && let Some(p) = self.params(&fb).into_iter().find(|p| p.name == upper(&m.text))
                    && p.dir == Dir::Output
                {
                    return Err(format!("{} is an output of {fb}: only the block writes it", m.text));
                }
                self.referable(base)
            }
            ExprKind::Index(base, _) => self.referable(base),
            ExprKind::BitAccess(..) => Err("REF() of a single bit: a reference is to a whole variable".into()),
            ExprKind::DirectAddress(d) => {
                Err(format!("REF({}): declare a variable AT {} and take REF() of it", d.text, d.text))
            }
            _ => Err("REF() takes a variable, not a value".into()),
        }
    }

    /// The elementary type of an array's first element or a structure's
    /// first field, through nested ones; an enumeration's base, a
    /// string's character.
    fn first_part(&mut self, ty: &Ty, depth: usize) -> Option<Elem> {
        if depth > 32 {
            return None;
        }
        match ty {
            Ty::Elem(e) => Some(*e),
            Ty::Str { wide, .. } => Some(if *wide { Elem::Wchar } else { Elem::Char }),
            Ty::Array { elem, .. } => self.first_part(elem, depth + 1),
            Ty::Enum(k) => match self.types.get(k).map(|t| &t.def) {
                Some(TypeDef::Spec(TypeSpec::Enum { base: Some(b), .. })) => Elem::from_name(&b.text),
                Some(_) => Some(Elem::Dint),
                None => None,
            },
            Ty::Struct(k) => {
                let first = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.first(),
                    _ => None,
                }?;
                let t = self.resolve_spec(&first.ty, false);
                self.first_part(&t, depth + 1)
            }
            _ => None,
        }
    }

    /// Where an address may stand (Ed.3 6.5.5): not in a FUNCTION; a
    /// `*` address only in VAR of a PROGRAM or
    /// FUNCTION_BLOCK, under a name, where VAR_CONFIG can reach it.
    fn check_location(&mut self, b: &VarBlock, d: &VarDecl, ty: &Ty, pou: Option<PouKind>) {
        let reachable = matches!(pou, Some(PouKind::Program | PouKind::FunctionBlock)) && b.kind == VarKind::Var;
        if let Some(loc) = &d.location {
            if pou == Some(PouKind::Function) {
                self.error(
                    Code::BadLocation,
                    loc.span,
                    "a FUNCTION has no located variables: it has no memory between calls (Ed.2 Table E.1)",
                );
            } else if loc.is_partly() && !reachable {
                let why = if pou.is_none() {
                    "a global has one instance, so its address is written here in full"
                } else {
                    "only VAR of a PROGRAM or FUNCTION_BLOCK declares one"
                };
                self.error(
                    Code::BadLocation,
                    loc.span,
                    format!("{} is not allowed here: {why} (Ed.3 6.5.5.4)", loc.text),
                );
            } else if loc.is_partly() && d.names.is_empty() {
                self.error(
                    Code::BadLocation,
                    loc.span,
                    format!("AT {} needs a name: VAR_CONFIG completes it by its name (Ed.3 6.5.5.4)", loc.text),
                );
            } else {
                self.check_location_size(loc, ty);
            }
        }
        if reachable && !matches!(ty, Ty::Array { .. }) {
            return;
        }
        if self.has_partly(ty) {
            let why = if matches!(ty, Ty::Array { .. }) {
                "VAR_CONFIG names no array elements"
            } else if pou.is_none() {
                "VAR_CONFIG reaches only instances inside programs"
            } else {
                "VAR_CONFIG reaches only instances declared in VAR of a PROGRAM or FUNCTION_BLOCK"
            };
            self.error(
                Code::BadLocation,
                d.ty.span(),
                format!("{ty} has `*` addresses, which could not be completed here: {why} (Ed.3 6.5.5.4)"),
            );
        }
    }

    fn has_partly(&mut self, ty: &Ty) -> bool {
        match ty {
            Ty::Array { elem, .. } => self.has_partly(elem),
            Ty::Fb(k) | Ty::Struct(k) => {
                let k = k.clone();
                !self.partly_of(&k).is_empty()
            }
            _ => false,
        }
    }

    /// The `*` addresses inside a PROGRAM, FUNCTION_BLOCK or structure
    /// type, through the instances and structures in it.
    fn partly_of(&mut self, key: &str) -> Vec<Partly> {
        if let Some(p) = self.partly_cache.get(key) {
            return p.clone();
        }
        if self.partly_visiting.iter().any(|k| k == key) {
            return Vec::new();
        }
        let decls: Vec<&'a VarDecl> = if let Some(pou) = self.pous.get(key).copied() {
            pou.var_blocks.iter().filter(|b| b.kind == VarKind::Var).flat_map(|b| &b.decls).collect()
        } else if let Some(TypeDef::Struct { fields, .. }) = self.types.get(key).map(|t| &t.def) {
            fields.iter().collect()
        } else {
            Vec::new()
        };
        self.partly_visiting.push(key.to_string());
        let mut out = Vec::new();
        for d in decls {
            let ty = self.resolve_spec(&d.ty, false);
            for n in &d.names {
                if let Some(loc) = d.location.as_ref().filter(|l| l.is_partly()) {
                    let area = loc.area().unwrap_or('I');
                    out.push(Partly { path: vec![n.text.clone()], area, ty: ty.clone() });
                } else if let Ty::Fb(k) | Ty::Struct(k) = &ty {
                    for mut inner in self.partly_of(&k.clone()) {
                        inner.path.insert(0, n.text.clone());
                        out.push(inner);
                    }
                }
            }
        }
        self.partly_visiting.pop();
        self.partly_cache.insert(key.to_string(), out.clone());
        out
    }

    fn check_global_block(&mut self, b: &VarBlock) {
        self.locals.clear();
        // Globals see each other, for constants in initial values.
        for d in &b.decls {
            let ty = self.resolve_spec(&d.ty, true);
            self.out.decls.insert(d.span, ty.clone());
            self.check_location(b, d, &ty, None);
            self.check_reference_kept(b, d, &ty);
            for n in &d.names {
                if let Some(why) = reserved_name(&n.text) {
                    self.error(
                        Code::ReservedName,
                        n.span,
                        format!("'{}' is reserved: it names {why} (Ed.3 6.1.3); choose another name", n.text),
                    );
                }
                self.locals.insert(upper(&n.text), Var { input: at_input(d), ..Var::of(ty.clone(), b.constant) });
            }
        }
        for d in &b.decls {
            if let Some(init) = &d.init {
                let t = self.resolve_spec(&d.ty, false);
                self.check_init(init, &t);
            }
        }
    }

    fn check_configuration(&mut self, c: &Configuration) {
        for b in &c.globals {
            self.check_global_block(b);
        }
        for r in &c.resources {
            for b in &r.globals {
                self.check_global_block(b);
            }
            self.locals.clear();
            let mut tasks: HashMap<String, Span> = HashMap::new();
            for t in &r.tasks {
                if tasks.insert(upper(&t.name.text), t.name.span).is_some() {
                    self.error(Code::Duplicate, t.name.span, format!("task '{}' is declared twice", t.name.text));
                }
                for a in &t.params {
                    let Arg::Input(name, value) = a else {
                        self.error(Code::BadCall, t.span, "a TASK takes SINGLE, INTERVAL and PRIORITY by name");
                        continue;
                    };
                    let want = match upper(&name.text).as_str() {
                        "SINGLE" => Ty::Elem(Elem::Bool),
                        "INTERVAL" => Ty::Elem(Elem::Time),
                        "PRIORITY" => Ty::Elem(Elem::Uint),
                        _ => {
                            self.error(
                                Code::BadCall,
                                name.span,
                                format!("a TASK has no parameter '{}': SINGLE, INTERVAL, PRIORITY", name.text),
                            );
                            continue;
                        }
                    };
                    let got = self.expr(value);
                    self.coerce(&got, &want, value.span);
                }
            }
            // A program's connections (Ed.3 6.8.2, Figure 15): its inputs
            // given and its outputs taken by name, from and to the
            // configuration's and the resource's globals, addresses and
            // literals - checked as a function block call's.
            self.locals.clear();
            for b in c.globals.iter().chain(&r.globals) {
                for d in &b.decls {
                    let ty = self.resolve_spec(&d.ty, false);
                    for n in &d.names {
                        self.locals
                            .insert(upper(&n.text), Var { input: at_input(d), ..Var::of(ty.clone(), b.constant) });
                    }
                }
            }
            let mut instances: HashMap<String, Span> = HashMap::new();
            for p in &r.programs {
                if instances.insert(upper(&p.name.text), p.name.span).is_some() {
                    self.error(
                        Code::Duplicate,
                        p.name.span,
                        format!("program instance '{}' is declared twice", p.name.text),
                    );
                }
                if let Some(t) = &p.task
                    && !tasks.contains_key(&upper(&t.text))
                {
                    self.error(Code::UnknownName, t.span, format!("no task '{}' in this resource", t.text));
                }
                match self.pous.get(&upper(&p.program_type.text)) {
                    Some(pou) if pou.kind == PouKind::Program => {
                        self.call_args(&upper(&p.program_type.text), Callee::Program, &p.args, p.span);
                    }
                    Some(pou) => self.error(
                        Code::BadCall,
                        p.program_type.span,
                        format!("'{}' is a {:?}; a resource runs PROGRAMs", p.program_type.text, pou.kind),
                    ),
                    None => self.error(
                        Code::UnknownName,
                        p.program_type.span,
                        format!("no PROGRAM '{}'", p.program_type.text),
                    ),
                }
            }
        }
        self.check_configuration_inits(c);
    }

    /// VAR_CONFIG (Ed.3 6.8.2) of a configuration: its program instances
    /// are the resources' PROGRAMs, under the resource's name if it has
    /// one.
    fn check_configuration_inits(&mut self, c: &Configuration) {
        self.locals.clear();
        for b in &c.globals {
            for d in &b.decls {
                let ty = self.resolve_spec(&d.ty, false);
                for n in &d.names {
                    self.locals.insert(upper(&n.text), Var { input: at_input(d), ..Var::of(ty.clone(), b.constant) });
                }
            }
        }
        let mut instances = Vec::new();
        for r in &c.resources {
            for p in &r.programs {
                let mut prefix: Vec<String> = r.name.iter().map(|n| n.text.clone()).collect();
                prefix.push(p.name.text.clone());
                instances.push(ProgInstance { prefix, program: upper(&p.program_type.text), span: p.name.span });
            }
        }
        let named = c.resources.iter().any(|r| r.name.is_some());
        let labels: &[&str] = if named { &["resource", "program instance"] } else { &["program instance"] };
        let entries: Vec<&ConfigInit> = c.config_inits.iter().collect();
        self.check_config_inits(&entries, &instances, labels);
    }

    /// Each entry names a variable of a program instance and gives it a
    /// location, an initial value, or both; every `*` address of every
    /// program instance gets its location. `labels` name the parts of an
    /// instance's prefix, for messages.
    fn check_config_inits(&mut self, entries: &[&ConfigInit], instances: &[ProgInstance], labels: &[&str]) {
        let mut given: HashMap<String, Span> = HashMap::new();
        let mut located: HashSet<String> = HashSet::new();
        for e in entries {
            let key = e.path.iter().map(|i| upper(&i.text)).collect::<Vec<_>>().join(".");
            if given.insert(key.clone(), e.span).is_some() {
                let path = e.path.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join(".");
                self.error(Code::Duplicate, e.span, format!("'{path}' has two VAR_CONFIG entries"));
                continue;
            }
            if self.check_config_init(e, instances, labels) {
                located.insert(key);
            }
        }
        for inst in instances {
            if !self.pous.get(&inst.program).is_some_and(|pou| pou.kind == PouKind::Program) {
                continue;
            }
            for leaf in self.partly_of(&inst.program) {
                let path = inst.prefix.iter().chain(&leaf.path).cloned().collect::<Vec<_>>().join(".");
                if !located.contains(&upper(&path)) {
                    self.error(
                        Code::UnconfiguredLocation,
                        inst.span,
                        format!(
                            "'{path}' is declared AT %{}* and VAR_CONFIG gives it no address: add `{path} AT %{}...: {};`",
                            leaf.area, leaf.area, leaf.ty
                        ),
                    );
                }
            }
        }
    }

    /// Checks one VAR_CONFIG entry; true if it completes a `*` address.
    fn check_config_init(&mut self, e: &ConfigInit, instances: &[ProgInstance], labels: &[&str]) -> bool {
        let matches = |inst: &ProgInstance, depth: usize| {
            inst.prefix.len() >= depth
                && e.path.len() >= depth
                && inst.prefix[..depth].iter().zip(&e.path[..depth]).all(|(a, b)| b.is(a))
        };
        let Some(inst) = instances.iter().find(|i| matches(i, i.prefix.len())) else {
            // The first name that leads nowhere.
            let depth = (0..labels.len()).find(|&d| !instances.iter().any(|i| matches(i, d + 1))).unwrap_or(0);
            match e.path.get(depth) {
                Some(id) => self.error(
                    Code::UnknownName,
                    id.span,
                    format!("no {} '{}': the path starts with {}", labels[depth], id.text, labels.join(", then ")),
                ),
                None => self.error(
                    Code::BadConfigEntry,
                    e.span,
                    format!("the path names {}, then a variable in it", labels.join(", then ")),
                ),
            }
            return false;
        };
        let mut segs = e.path[inst.prefix.len()..].iter();
        let prog_name = inst.prefix.last().cloned().unwrap_or_default();
        let program = inst.program.clone();
        let rest: Vec<&Ident> = segs.by_ref().collect();
        if rest.is_empty() {
            self.error(
                Code::BadConfigEntry,
                e.span,
                format!("'{prog_name}' is a program instance: VAR_CONFIG names a variable inside it"),
            );
            return false;
        }
        let path = e.path.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join(".");
        let mut container = program;
        let mut target: Option<(&'a VarDecl, Option<VarKind>, bool)> = None;
        for (i, id) in rest.iter().enumerate() {
            let found = if let Some(pou) = self.pous.get(&container).copied() {
                pou.var_blocks.iter().find_map(|b| {
                    b.decls
                        .iter()
                        .find(|d| d.names.iter().any(|n| n.is(&id.text)))
                        .map(|d| (d, Some(b.kind), b.constant))
                })
            } else if let Some(TypeDef::Struct { fields, .. }) = self.types.get(&container).map(|t| &t.def) {
                fields.iter().find(|d| d.names.iter().any(|n| n.is(&id.text))).map(|d| (d, None, false))
            } else {
                None
            };
            let Some(found) = found else {
                if !self.pous.contains_key(&container) && !self.types.contains_key(&container) {
                    return false; // reported where the type is declared or used
                }
                self.error(Code::UnknownName, id.span, format!("{container} has no variable '{}'", id.text));
                return false;
            };
            if i + 1 < rest.len() {
                match self.resolve_spec(&found.0.ty, false) {
                    Ty::Fb(k) | Ty::Struct(k) => container = k,
                    Ty::Unknown => return false,
                    t => {
                        self.error(
                            Code::UnknownName,
                            rest[i + 1].span,
                            format!("'{}' is {t}: it has no parts to name", id.text),
                        );
                        return false;
                    }
                }
            }
            target = Some(found);
        }
        let Some((decl, kind, constant)) = target else { return false };
        let declared = self.resolve_spec(&decl.ty, false);
        let written = self.resolve_spec(&e.ty, true);
        if !declared.is_unknown() && !written.is_unknown() && declared != written {
            self.error(
                Code::TypeMismatch,
                e.ty.span(),
                format!("'{path}' is declared {declared}: VAR_CONFIG repeats its type, not {written}"),
            );
        }
        // An address given to a `*` variable, even a wrong one, is not
        // also reported missing: one error per mistake.
        let completes = e.location.is_some() && decl.location.as_ref().is_some_and(|d| d.is_partly());
        if let Some(loc) = &e.location {
            match &decl.location {
                _ if loc.is_partly() => self.error(
                    Code::BadConfigEntry,
                    loc.span,
                    format!("VAR_CONFIG gives the complete address, not {}", loc.text),
                ),
                Some(d) if d.is_partly() && d.area() != loc.area() => self.error(
                    Code::BadConfigEntry,
                    loc.span,
                    format!("'{path}' is declared AT {}: its address is a %{} one", d.text, d.area().unwrap_or('?')),
                ),
                Some(d) if d.is_partly() => self.check_location_size(loc, &declared),
                Some(d) => self.error(
                    Code::BadConfigEntry,
                    loc.span,
                    format!("'{path}' is declared AT {}: VAR_CONFIG gives addresses only to `*` ones", d.text),
                ),
                None => self.error(
                    Code::BadConfigEntry,
                    loc.span,
                    format!("'{path}' is not declared AT %I*, %Q* or %M*: VAR_CONFIG gives addresses only to those"),
                ),
            }
        }
        if let Some(init) = &e.init {
            let refused = match kind {
                Some(VarKind::Temp) => Some("VAR_TEMP"),
                Some(VarKind::External) => Some("VAR_EXTERNAL"),
                Some(VarKind::InOut) => Some("VAR_IN_OUT"),
                _ if constant => Some("CONSTANT"),
                _ => None,
            };
            if let Some(what) = refused {
                self.error(
                    Code::BadConfigEntry,
                    e.span,
                    format!("'{path}' is {what}: it takes no instance-specific initial value (Ed.3 6.8.2)"),
                );
            } else {
                self.check_init(init, &declared);
            }
        }
        completes
    }

    // ---- statements -----------------------------------------------

    fn stmts(&mut self, body: &[Stmt]) {
        for s in body {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Assign { target, value } => {
                let want = self.place(target);
                let got = self.expr(value);
                if got == Ty::Void {
                    self.error(Code::BadCall, value.span, "this call gives no value to assign");
                } else {
                    self.coerce(&got, &want, value.span);
                }
            }
            StmtKind::Call(e) => {
                self.expr(e);
            }
            StmtKind::If { branches, else_body } => {
                for (cond, body) in branches {
                    self.condition(cond);
                    self.stmts(body);
                }
                if let Some(b) = else_body {
                    self.stmts(b);
                }
            }
            StmtKind::Case { selector, arms, else_body } => {
                let sel = self.expr(selector);
                let ok = sel.is_unknown() || sel.is_int() || matches!(sel, Ty::Enum(_));
                if !ok {
                    self.error(
                        Code::OperandTypes,
                        selector.span,
                        format!("CASE selects on an integer or an enumeration, not {sel}"),
                    );
                }
                for arm in arms {
                    for label in &arm.labels {
                        match label {
                            CaseLabel::Value(e) => {
                                let t = self.expr(e);
                                if ok {
                                    self.coerce(&t, &sel, e.span);
                                }
                            }
                            CaseLabel::Range(lo, hi) => {
                                for e in [lo, hi] {
                                    let t = self.expr(e);
                                    if ok {
                                        self.coerce(&t, &sel, e.span);
                                    }
                                }
                            }
                        }
                    }
                    self.stmts(&arm.body);
                }
                if let Some(b) = else_body {
                    self.stmts(b);
                }
            }
            StmtKind::For { var, from, to, by, body } => {
                let control = Expr { kind: ExprKind::Name(var.clone()), span: var.span };
                let t = self.place(&control);
                if !t.is_unknown() && !t.is_int() {
                    self.error(Code::OperandTypes, var.span, format!("a FOR variable is an integer, not {t}"));
                }
                for e in [Some(from), Some(to), by.as_ref()].into_iter().flatten() {
                    let et = self.expr(e);
                    self.coerce(&et, &t, e.span);
                }
                self.loop_depth += 1;
                self.stmts(body);
                self.loop_depth -= 1;
            }
            StmtKind::While { cond, body } => {
                self.condition(cond);
                self.loop_depth += 1;
                self.stmts(body);
                self.loop_depth -= 1;
            }
            StmtKind::Repeat { body, until } => {
                self.loop_depth += 1;
                self.stmts(body);
                self.loop_depth -= 1;
                self.condition(until);
            }
            StmtKind::Exit | StmtKind::Continue => {
                if self.loop_depth == 0 {
                    let which = if s.kind == StmtKind::Exit { "EXIT" } else { "CONTINUE" };
                    self.error(
                        Code::LoopControlOutsideLoop,
                        s.span,
                        format!("{which} is outside any FOR, WHILE or REPEAT"),
                    );
                }
            }
            StmtKind::RefAssign { target, value } => {
                if !self.options.has(Extension::ReferenceTo) {
                    self.error(
                        Code::UnexpectedToken,
                        s.span,
                        "REF= is CODESYS's, not IEC 61131-3: write r := REF(x) for a REF_TO, or --extension reference-to",
                    );
                    self.expr(target);
                    self.expr(value);
                    return;
                }
                match self.raw_expr(target) {
                    Ty::Reference(to) => self.bind_reference(&to, value, true),
                    Ty::Unknown => {}
                    other => {
                        self.error(Code::TypeMismatch, target.span, format!("REF= binds a REFERENCE TO, not {other}"));
                        self.expr(value);
                    }
                }
            }
            StmtKind::Return | StmtKind::Empty => {}
        }
    }

    fn condition(&mut self, e: &Expr) {
        let t = self.expr(e);
        if !t.is_unknown() && !t.is_bool() {
            self.error(Code::NotBool, e.span, format!("a condition is BOOL, not {t}"));
        }
    }

    // ---- conversions ----------------------------------------------

    /// Does a value of type `from` meet a place of type `to`? Reports
    /// E0013 or E0020 if not.
    fn coerce(&mut self, from: &Ty, to: &Ty, span: Span) -> bool {
        match (from, to) {
            (Ty::Unknown, _) | (_, Ty::Unknown) => true,
            _ if from == to => true,
            (Ty::IntLit(v), Ty::Elem(e)) if e.is_num() || e.is_bit() => {
                if let (Some(v), Some((lo, hi))) = (v, e.int_range())
                    && (*v < lo || *v > hi)
                {
                    self.error(Code::LiteralRange, span, format!("{v} is outside {} ({lo}..{hi})", e.name()));
                    return false;
                }
                true
            }
            (Ty::RealLit, Ty::Elem(e)) if e.is_real() => true,
            (Ty::Elem(a), Ty::Elem(b)) => match a.conversion_to(*b) {
                Conversion::Same | Conversion::Implicit => true,
                Conversion::Explicit => {
                    self.error(
                        Code::TypeMismatch,
                        span,
                        format!(
                            "{} does not convert to {} implicitly: use {}_TO_{}",
                            a.name(),
                            b.name(),
                            a.name(),
                            b.name()
                        ),
                    );
                    false
                }
                Conversion::None => {
                    self.error(Code::TypeMismatch, span, format!("{} does not convert to {}", a.name(), b.name()));
                    false
                }
            },
            // 'a' where a CHAR is wanted (Ed.3 Table 6, 1b).
            (Ty::Str { wide: false, .. }, Ty::Elem(Elem::Char)) if self.one_char.contains(&span) => {
                self.out.exprs.insert(span, Ty::Elem(Elem::Char));
                true
            }
            (Ty::Elem(Elem::Char), Ty::Str { wide: false, .. })
            | (Ty::Elem(Elem::Wchar), Ty::Str { wide: true, .. }) => true,
            (Ty::Str { wide: a, .. }, Ty::Str { wide: b, .. }) if a == b => true,
            (Ty::Null, Ty::Ref(_)) => true,
            (Ty::Ref(a), Ty::Ref(b)) if a.is_unknown() || b.is_unknown() => true,
            // A bound not known (`*`, or already reported) meets any.
            (Ty::Array { elem: a, dims: da }, Ty::Array { elem: b, dims: db })
                if a == b
                    && da.len() == db.len()
                    && da.iter().zip(db).all(|(x, y)| x.is_none() || y.is_none() || x == y) =>
            {
                true
            }
            _ => {
                let hint = match (from, to) {
                    (Ty::Str { wide: false, .. }, Ty::Str { wide: true, .. }) => ": use STRING_TO_WSTRING",
                    (Ty::Str { wide: true, .. }, Ty::Str { wide: false, .. }) => ": use WSTRING_TO_STRING",
                    _ => "",
                };
                self.error(Code::TypeMismatch, span, format!("{from} does not convert to {to}{hint}"));
                false
            }
        }
    }

    fn check_init(&mut self, init: &Init, ty: &Ty) {
        if let Ty::Reference(_) = ty {
            let at = match init {
                Init::Expr(e) => e.span,
                Init::Array { span, .. } | Init::Struct { span, .. } => *span,
            };
            self.error(Code::TypeMismatch, at, "a REFERENCE TO is bound with REF=, not given an initial value");
            return;
        }
        match init {
            Init::Expr(e) => {
                let t = self.expr(e);
                self.coerce(&t, ty, e.span);
            }
            Init::Array { elems, span } => {
                let elem = match ty {
                    Ty::Array { elem, .. } => (**elem).clone(),
                    Ty::Unknown => Ty::Unknown,
                    other => {
                        self.error(Code::TypeMismatch, *span, format!("an array initial value for {other}"));
                        Ty::Unknown
                    }
                };
                for el in elems {
                    if let Some(r) = &el.repeat {
                        let rt = self.expr(r);
                        if !rt.is_unknown() && !rt.is_int() {
                            self.error(Code::OperandTypes, r.span, "a repeat count is an integer");
                        }
                    }
                    if let Some(v) = &el.value {
                        self.check_init(v, &elem);
                    }
                }
            }
            Init::Struct { fields, span } => {
                for (name, value) in fields {
                    let ft = match ty {
                        Ty::Struct(s) => match self.struct_fields.get(s).and_then(|m| m.get(&upper(&name.text))) {
                            Some(t) => t.clone(),
                            None => {
                                self.error(Code::UnknownName, name.span, format!("{ty} has no field '{}'", name.text));
                                Ty::Unknown
                            }
                        },
                        Ty::Fb(fb) => {
                            let fb = fb.clone();
                            match self.params(&fb).into_iter().find(|p| p.name == upper(&name.text)) {
                                Some(p) if p.dir == Dir::Input => p.ty,
                                _ => {
                                    self.error(
                                        Code::UnknownName,
                                        name.span,
                                        format!("{ty} has no input '{}'", name.text),
                                    );
                                    Ty::Unknown
                                }
                            }
                        }
                        Ty::Unknown => Ty::Unknown,
                        other => {
                            self.error(Code::TypeMismatch, *span, format!("a structure initial value for {other}"));
                            return;
                        }
                    };
                    self.check_init(value, &ft);
                }
            }
        }
    }

    // ---- places ---------------------------------------------------

    /// The type of something assigned to, after checking it may be.
    fn place(&mut self, e: &Expr) -> Ty {
        match &e.kind {
            ExprKind::Name(id) => {
                let t = self.expr(e);
                if let Some(v) = self.locals.get(&upper(&id.text)) {
                    if v.constant {
                        self.error(Code::NotAssignable, e.span, format!("'{}' is CONSTANT", id.text));
                    }
                } else if !t.is_unknown() {
                    self.error(Code::NotAssignable, e.span, format!("'{}' is not a variable", id.text));
                }
                t
            }
            ExprKind::Member(base, m) => {
                let bt = self.expr(base);
                if let Ty::Fb(fb) = &bt {
                    let fb = fb.clone();
                    if let Some(p) = self.params(&fb).into_iter().find(|p| p.name == upper(&m.text))
                        && p.dir == Dir::Output
                    {
                        self.error(
                            Code::NotAssignable,
                            e.span,
                            format!("{} is an output of {fb}: only the block writes it", m.text),
                        );
                        return p.ty;
                    }
                }
                if matches!(bt, Ty::Struct(_)) {
                    self.place(base);
                }
                self.expr(e)
            }
            ExprKind::Index(base, _) => {
                self.place(base);
                self.expr(e)
            }
            ExprKind::BitAccess(base, _) => {
                self.place(base);
                self.expr(e)
            }
            ExprKind::Deref(_) | ExprKind::Paren(_) => self.expr(e),
            ExprKind::DirectAddress(d) => {
                if d.text.get(1..2).is_some_and(|c| c.eq_ignore_ascii_case("I")) {
                    self.error(
                        Code::NotAssignable,
                        e.span,
                        format!("{} is an input address: the program reads it, never writes it", d.text),
                    );
                }
                self.expr(e)
            }
            ExprKind::Error => Ty::Unknown,
            _ => {
                let t = self.expr(e);
                self.error(Code::NotAssignable, e.span, "only a variable can be assigned to");
                t
            }
        }
    }

    // ---- expressions ----------------------------------------------

    /// The value of a constant integer expression, when it is one.
    fn const_int(&mut self, e: &Expr) -> Option<i128> {
        match &e.kind {
            ExprKind::Literal(l) if matches!(l.kind, LiteralKind::Int | LiteralKind::Based) => literal_int(l),
            ExprKind::Paren(x) => self.const_int(x),
            ExprKind::Unary(UnaryOp::Neg, x) => self.const_int(x).map(|v| -v),
            ExprKind::Binary(op, a, b) => {
                let (a, b) = (self.const_int(a)?, self.const_int(b)?);
                match op {
                    BinaryOp::Add => a.checked_add(b),
                    BinaryOp::Sub => a.checked_sub(b),
                    BinaryOp::Mul => a.checked_mul(b),
                    BinaryOp::Div if b != 0 => Some(a / b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The type of `e`, recorded for the code generator.
    pub(crate) fn expr(&mut self, e: &Expr) -> Ty {
        let t = self.expr_kind(e);
        // A REFERENCE TO stands for what it references.
        let t = match t {
            Ty::Reference(to) => {
                self.out.refs.insert(e.span, crate::RefUse::Deref);
                *to
            }
            t => t,
        };
        self.out.exprs.insert(e.span, t.clone());
        t
    }

    /// `e`'s own type, a REFERENCE TO as itself: for REF=, __ISVALIDREF and
    /// a call's binding. Recorded as `expr` records it.
    fn raw_expr(&mut self, e: &Expr) -> Ty {
        let t = self.expr_kind(e);
        let shown = match &t {
            Ty::Reference(to) => (**to).clone(),
            t => t.clone(),
        };
        self.out.exprs.insert(e.span, shown);
        self.out.refs.remove(&e.span);
        t
    }

    /// A REFERENCE TO `to` bound to `x` (REF=, or a call's input): `x` a
    /// variable of type `to` that REF() could take, another REFERENCE TO
    /// `to` (bound to what it references), or, for REF=, 0 (unbound).
    fn bind_reference(&mut self, to: &Ty, x: &Expr, zero_ok: bool) {
        if zero_ok && matches!(&x.kind, ExprKind::Literal(l) if l.text == "0") {
            self.expr(x);
            self.out.refs.insert(x.span, crate::RefUse::Unbind);
            return;
        }
        let t = self.raw_expr(x);
        let got = match &t {
            Ty::Reference(inner) => (**inner).clone(),
            t => t.clone(),
        };
        if got.is_unknown() || to.is_unknown() {
            return;
        }
        if got != *to {
            self.error(Code::TypeMismatch, x.span, format!("a REFERENCE TO {to} binds a {to}, not {got}"));
            return;
        }
        if !matches!(t, Ty::Reference(_)) {
            match self.referable(x) {
                Err(why) => self.error(Code::BadReference, x.span, why),
                Ok(_) => {
                    self.out.refs.insert(x.span, crate::RefUse::Bind);
                }
            }
        }
    }

    fn expr_kind(&mut self, e: &Expr) -> Ty {
        match &e.kind {
            ExprKind::Literal(l) => self.literal(l),
            ExprKind::Name(id) => self.name(id),
            ExprKind::DirectAddress(d) if d.is_partly() => {
                if self.pou_kind.is_none() {
                    // In a body the parser has said so already.
                    self.error(
                        Code::BadLocation,
                        d.span,
                        format!("{} is a declaration's placeholder, completed by VAR_CONFIG: it has no value", d.text),
                    );
                }
                Ty::Unknown
            }
            ExprKind::DirectAddress(d) => {
                let size = d.text.chars().nth(2).map(|c| c.to_ascii_uppercase());
                Ty::Elem(match size {
                    Some('B') => Elem::Byte,
                    Some('W') => Elem::Word,
                    Some('D') => Elem::Dword,
                    Some('L') => Elem::Lword,
                    _ => Elem::Bool,
                })
            }
            ExprKind::EnumValue { ty, value } => {
                let key = upper(&ty.text);
                match self.types.get(&key).map(|d| &d.def) {
                    Some(TypeDef::Spec(TypeSpec::Enum { values, .. })) => {
                        if values.iter().any(|v| v.name.is(&value.text)) {
                            Ty::Enum(key)
                        } else {
                            self.error(
                                Code::UnknownName,
                                value.span,
                                format!("{} has no value '{}'", ty.text, value.text),
                            );
                            Ty::Unknown
                        }
                    }
                    Some(_) => {
                        self.error(Code::UnknownType, ty.span, format!("'{}' is not an enumeration", ty.text));
                        Ty::Unknown
                    }
                    None => {
                        self.error(Code::UnknownType, ty.span, format!("there is no type '{}'", ty.text));
                        Ty::Unknown
                    }
                }
            }
            ExprKind::Member(base, m) => {
                let bt = self.expr(base);
                self.member(&bt, m)
            }
            ExprKind::BitAccess(base, bit) => {
                let bt = self.expr(base);
                let width = match bt.elem() {
                    Some(Elem::Byte) => 8,
                    Some(Elem::Word) => 16,
                    Some(Elem::Dword) => 32,
                    Some(Elem::Lword) => 64,
                    _ if bt.is_unknown() => return Ty::Unknown,
                    _ => {
                        self.error(
                            Code::OperandTypes,
                            e.span,
                            format!("a bit is taken from BYTE, WORD, DWORD or LWORD, not {bt}"),
                        );
                        return Ty::Unknown;
                    }
                };
                if literal_int(bit).is_some_and(|b| b >= width) {
                    self.error(Code::OperandTypes, bit.span, format!("{bt} has bits 0..{}", width - 1));
                }
                Ty::Elem(Elem::Bool)
            }
            ExprKind::Index(base, index) => {
                let bt = self.expr(base);
                for i in index {
                    let it = self.expr(i);
                    if !it.is_unknown() && !it.is_int() {
                        self.error(Code::OperandTypes, i.span, format!("an array index is an integer, not {it}"));
                    }
                }
                match bt {
                    Ty::Array { elem, dims } => {
                        let dims = dims.len();
                        if dims != index.len() {
                            self.error(
                                Code::OperandTypes,
                                e.span,
                                format!("this array has {dims} dimension(s), indexed with {}", index.len()),
                            );
                        }
                        *elem
                    }
                    Ty::Unknown => Ty::Unknown,
                    other => {
                        self.error(Code::OperandTypes, base.span, format!("{other} is not an array"));
                        Ty::Unknown
                    }
                }
            }
            ExprKind::Deref(base) => match self.expr(base) {
                Ty::Ref(t) => *t,
                Ty::Unknown => Ty::Unknown,
                other => {
                    self.error(Code::OperandTypes, e.span, format!("'^' dereferences a REF_TO, not {other}"));
                    Ty::Unknown
                }
            },
            ExprKind::Call { callee, args } => self.call(callee, args, e.span),
            ExprKind::Unary(op, x) => {
                let t = self.expr(x);
                match op {
                    UnaryOp::Neg | UnaryOp::Plus => match t {
                        Ty::IntLit(v) if *op == UnaryOp::Neg => Ty::IntLit(v.map(|v| -v)),
                        Ty::Elem(e) if e.is_unsigned_int() && *op == UnaryOp::Neg => {
                            self.error(
                                Code::OperandTypes,
                                x.span,
                                format!("{} is unsigned: it has no negative", e.name()),
                            );
                            Ty::Unknown
                        }
                        t if t.is_num() || t.elem().is_some_and(Elem::is_duration) || t.is_unknown() => t,
                        t => {
                            self.error(Code::OperandTypes, x.span, format!("'-' and '+' take a number, not {t}"));
                            Ty::Unknown
                        }
                    },
                    UnaryOp::Not => {
                        if t.is_bit() || t.is_unknown() || matches!(t, Ty::IntLit(_)) {
                            t
                        } else {
                            self.error(Code::OperandTypes, x.span, format!("NOT takes BOOL or a bit string, not {t}"));
                            Ty::Unknown
                        }
                    }
                }
            }
            ExprKind::Binary(op, a, b) => {
                let (ta, tb) = (self.expr(a), self.expr(b));
                // 'a' against a CHAR is a CHAR (Ed.3 Table 6, 1b).
                let char = Ty::Elem(Elem::Char);
                let ta = if tb == char && self.char_literal(a) { char.clone() } else { ta };
                let tb = if ta == char && self.char_literal(b) { char.clone() } else { tb };
                self.binary(*op, &ta, &tb, e.span)
            }
            ExprKind::Paren(x) => self.expr(x),
            ExprKind::Error => Ty::Unknown,
        }
    }

    /// Whether `e` is an untyped one-character STRING literal, which it
    /// then records as a CHAR for the generator.
    fn char_literal(&mut self, e: &Expr) -> bool {
        let inner = match &e.kind {
            ExprKind::Paren(x) => return self.char_literal(x),
            ExprKind::Literal(l) => l.span,
            _ => return false,
        };
        if !self.one_char.contains(&inner) && !self.one_char.contains(&e.span) {
            return false;
        }
        self.out.exprs.insert(e.span, Ty::Elem(Elem::Char));
        self.out.exprs.insert(inner, Ty::Elem(Elem::Char));
        true
    }

    fn literal(&mut self, l: &Literal) -> Ty {
        let prefixed = |c: &mut Self| -> Option<Ty> {
            let p = l.type_prefix.as_ref()?;
            match upper(&p.text).as_str() {
                "STRING" => return Some(Ty::Str { wide: false, len: None }),
                "WSTRING" => return Some(Ty::Str { wide: true, len: None }),
                _ => {}
            }
            match Elem::from_name(&p.text) {
                Some(e) => Some(Ty::Elem(e)),
                None => {
                    c.error(Code::UnknownType, p.span, format!("'{}#' names no elementary type", p.text));
                    Some(Ty::Unknown)
                }
            }
        };
        match l.kind {
            LiteralKind::Int | LiteralKind::Based => {
                let v = literal_int(l).map(|v| if l.negative { -v } else { v });
                match prefixed(self) {
                    // `CHAR#16#41`: the character of that code (Ed.3 Table 5).
                    Some(Ty::Elem(Elem::Char)) => {
                        if let Some(v) = v
                            && !(0..=255).contains(&v)
                        {
                            self.error(Code::LiteralRange, l.span, format!("{v} is outside CHAR (0..255)"));
                        }
                        Ty::Elem(Elem::Char)
                    }
                    Some(t) => {
                        let lit = Ty::IntLit(v);
                        self.coerce(&lit, &t, l.span);
                        t
                    }
                    None => Ty::IntLit(v),
                }
            }
            LiteralKind::Real => match prefixed(self) {
                Some(t) => {
                    self.coerce(&Ty::RealLit, &t, l.span);
                    t
                }
                None => Ty::RealLit,
            },
            LiteralKind::Bool => {
                if let Some(t) = prefixed(self) {
                    self.coerce(&Ty::Elem(Elem::Bool), &t, l.span);
                    return t;
                }
                Ty::Elem(Elem::Bool)
            }
            LiteralKind::String | LiteralKind::WString => {
                let wide = l.kind == LiteralKind::WString;
                if !wide && let Err(ch) = crate::strings::string_bytes(&l.text) {
                    self.error(
                        Code::LiteralRange,
                        l.span,
                        format!("{ch} is not in Latin-1, the character set of a STRING: use a WSTRING (docs/dependencies.md)"),
                    );
                }
                let bytes = crate::strings::string_bytes(&l.text).map(|b| b.len()).unwrap_or(0);
                match prefixed(self) {
                    // `CHAR#'X'`: one character (Ed.3 Table 6, 3b).
                    Some(Ty::Elem(Elem::Char)) if wide || bytes != 1 => {
                        self.error(Code::LiteralRange, l.span, "a CHAR literal holds one character: CHAR#'X'");
                        Ty::Elem(Elem::Char)
                    }
                    Some(t) => t,
                    None => {
                        if !wide && bytes == 1 {
                            self.one_char.insert(l.span);
                        }
                        Ty::Str { wide, len: None }
                    }
                }
            }
            LiteralKind::Time => {
                if let Some(Err(why)) = crate::datetime::literal(&l.text) {
                    self.error(Code::LiteralRange, l.span, format!("{}: {why}", l.text));
                }
                let prefix = l.text.split('#').next().unwrap_or_default().to_ascii_uppercase();
                Ty::Elem(match prefix.as_str() {
                    "T" | "TIME" => Elem::Time,
                    "LT" | "LTIME" => Elem::Ltime,
                    "D" | "DATE" => Elem::Date,
                    "LD" | "LDATE" => Elem::Ldate,
                    "TOD" | "TIME_OF_DAY" => Elem::Tod,
                    "LTOD" | "LTIME_OF_DAY" => Elem::Ltod,
                    "DT" | "DATE_AND_TIME" => Elem::Dt,
                    _ => Elem::Ldt,
                })
            }
        }
    }

    fn name(&mut self, id: &Ident) -> Ty {
        if id.text.is_empty() {
            return Ty::Unknown;
        }
        let key = upper(&id.text);
        if let Some(v) = self.variable(&key) {
            return v.ty;
        }
        if self.steps.contains_key(&key) {
            return Ty::Struct("<step>".into());
        }
        // An action's flag, ACTION1.Q (Ed.3 6.7.4.6 a), read-only.
        if self.actions.contains_key(&key) {
            return Ty::Struct("<action>".into());
        }
        if id.is("NULL") {
            return Ty::Null;
        }
        if let Some(enums) = self.enum_values.get(&key).cloned() {
            if enums.len() == 1 {
                return Ty::Enum(enums[0].clone());
            }
            self.error(
                Code::UnknownName,
                id.span,
                format!("'{}' is a value of {} enumerations: write Type#{}", id.text, enums.len(), id.text),
            );
            return Ty::Unknown;
        }
        let message = if self.globals.contains_key(&key) {
            format!("'{}' is a global variable: declare it VAR_EXTERNAL to use it here", id.text)
        } else if let Some(p) = self.pous.get(&key) {
            format!("'{}' is a {:?}: it is called, not read", id.text, p.kind)
        } else {
            format!("'{}' is not declared", id.text)
        };
        self.error(Code::UnknownName, id.span, message);
        Ty::Unknown
    }

    /// A variable the POU sees by name: its own, or, with the
    /// implicit-externals extension, a global it did not declare
    /// VAR_EXTERNAL - which then counts as declared, CONSTANT and all.
    fn variable(&mut self, key: &str) -> Option<Var> {
        if let Some(v) = self.locals.get(key) {
            return Some(v.clone());
        }
        // ENO, in a FUNCTION or FUNCTION_BLOCK that does not declare it
        // (Ed.3 6.6.1.5 rules 2 and 7): a BOOL the body may read and set,
        // TRUE as it starts (docs/dependencies.md).
        if key == "ENO" && matches!(self.pou_kind, Some(PouKind::Function | PouKind::FunctionBlock)) {
            self.out.implicit_eno.insert(self.pou_key.clone());
            let v = Var { temp: self.pou_kind == Some(PouKind::Function), ..Var::of(Ty::Elem(Elem::Bool), false) };
            self.locals.insert(key.to_string(), v.clone());
            return Some(v);
        }
        if self.pou_kind.is_none() || !self.options.has(Extension::ImplicitExternals) {
            return None;
        }
        let (decl, constant) = self.globals.get(key).copied()?;
        let ty = self.resolve_spec(&decl.ty, false);
        let v = Var { input: at_input(decl), ..Var::of(ty, constant) };
        self.locals.insert(key.to_string(), v.clone());
        Some(v)
    }

    fn member(&mut self, base: &Ty, m: &Ident) -> Ty {
        let key = upper(&m.text);
        match base {
            Ty::Unknown => Ty::Unknown,
            Ty::Struct(s) if s == "<action>" => match key.as_str() {
                "Q" => Ty::Elem(Elem::Bool),
                _ => {
                    self.error(Code::UnknownName, m.span, format!("an action has Q, not '{}'", m.text));
                    Ty::Unknown
                }
            },
            Ty::Struct(s) if s == "<step>" => match key.as_str() {
                "X" => Ty::Elem(Elem::Bool),
                "T" => Ty::Elem(Elem::Time),
                _ => {
                    self.error(Code::UnknownName, m.span, format!("a step has X and T, not '{}'", m.text));
                    Ty::Unknown
                }
            },
            Ty::Struct(s) => match self.struct_fields.get(s).and_then(|f| f.get(&key)) {
                Some(t) => t.clone(),
                None => {
                    self.error(Code::UnknownName, m.span, format!("{s} has no field '{}'", m.text));
                    Ty::Unknown
                }
            },
            Ty::Fb(fb) => {
                let fb = fb.clone();
                if let Some(p) = self.params(&fb).into_iter().find(|p| p.name == key) {
                    return p.ty;
                }
                if matches!(key.as_str(), "ENO" | "EN") {
                    return Ty::Elem(Elem::Bool);
                }
                if self.is_internal_member(&fb, &key) {
                    self.error(
                        Code::UnknownName,
                        m.span,
                        format!("'{}' is internal to {fb}: only its inputs and outputs are seen outside", m.text),
                    );
                } else {
                    self.error(Code::UnknownName, m.span, format!("{fb} has no input or output '{}'", m.text));
                }
                Ty::Unknown
            }
            other => {
                self.error(Code::UnknownName, m.span, format!("{other} has no members"));
                Ty::Unknown
            }
        }
    }

    fn binary(&mut self, op: BinaryOp, a: &Ty, b: &Ty, span: Span) -> Ty {
        use BinaryOp::*;
        if a.is_unknown() || b.is_unknown() {
            return match op {
                Eq | Ne | Lt | Le | Gt | Ge => Ty::Elem(Elem::Bool),
                _ => Ty::Unknown,
            };
        }
        let sym = match op {
            Or => "OR",
            Xor => "XOR",
            And => "AND",
            Eq => "=",
            Ne => "<>",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Mod => "MOD",
            Power => "**",
        };
        let fail = |c: &mut Self, what: &str| {
            // Suggest the conversion that loses least: an integer to a
            // real rather than a real to an integer.
            let hint = match (a.elem(), b.elem()) {
                (Some(x), Some(y)) if x.is_real() && !y.is_real() && y.conversion_to(x) == Conversion::Explicit => {
                    format!(": convert one side, e.g. {}_TO_{}", y.name(), x.name())
                }
                (Some(x), Some(y)) if x.conversion_to(y) == Conversion::Explicit => {
                    format!(": convert one side, e.g. {}_TO_{}", x.name(), y.name())
                }
                _ => String::new(),
            };
            c.error(Code::OperandTypes, span, format!("{sym} {what}, not {a} and {b}{hint}"));
            Ty::Unknown
        };
        match op {
            Or | Xor | And => {
                let ok = |t: &Ty| t.is_bit() || matches!(t, Ty::IntLit(_));
                if !(ok(a) && ok(b)) {
                    return fail(self, "takes BOOL or bit strings");
                }
                unify(a, b).unwrap_or_else(|| fail(self, "takes two operands of one type"))
            }
            // References: = and <> only, of one type or with NULL
            // (Ed.3 Table 12 2c; docs/dependencies.md).
            Eq | Ne | Lt | Le | Gt | Ge if matches!(a, Ty::Ref(_) | Ty::Null) || matches!(b, Ty::Ref(_) | Ty::Null) => {
                if !matches!(op, Eq | Ne) {
                    return fail(self, "compares references with = and <> only");
                }
                let ok = match (a, b) {
                    (Ty::Null, Ty::Ref(_) | Ty::Null) | (Ty::Ref(_), Ty::Null) => true,
                    (Ty::Ref(x), Ty::Ref(y)) => x == y || x.is_unknown() || y.is_unknown(),
                    _ => false,
                };
                if !ok {
                    fail(self, "compares two references to one type, or one with NULL");
                }
                Ty::Elem(Elem::Bool)
            }
            Eq | Ne | Lt | Le | Gt | Ge => {
                if unify(a, b).is_none() {
                    fail(self, "compares two values of one type");
                }
                Ty::Elem(Elem::Bool)
            }
            Add | Sub | Mul | Div => {
                if let Some(t) = time_arithmetic(op, a, b) {
                    return t;
                }
                if !(a.is_num() && b.is_num()) {
                    return fail(self, "takes numbers");
                }
                unify(a, b).unwrap_or_else(|| fail(self, "takes two numbers of one type"))
            }
            Mod => {
                if !(a.is_int() && b.is_int()) {
                    return fail(self, "takes integers");
                }
                unify(a, b).unwrap_or_else(|| fail(self, "takes two integers of one type"))
            }
            Power => {
                let base_ok = matches!(a, Ty::RealLit | Ty::IntLit(_)) || a.elem().is_some_and(Elem::is_real);
                if !base_ok || !b.is_num() {
                    return fail(self, "raises a REAL or LREAL to a number");
                }
                if matches!(a, Ty::IntLit(_)) { Ty::RealLit } else { a.clone() }
            }
        }
    }

    // ---- calls ----------------------------------------------------

    fn call(&mut self, callee: &Expr, args: &[Arg], span: Span) -> Ty {
        if let ExprKind::Name(id) = &callee.kind {
            let key = upper(&id.text);
            if !self.locals.contains_key(&key) {
                if let Some(pou) = self.pous.get(&key).copied() {
                    return match pou.kind {
                        PouKind::Function => {
                            let ret = match &pou.return_type {
                                Some(rt) => self.resolve_spec(rt, false),
                                None => Ty::Void,
                            };
                            self.call_args(&key, Callee::Function, args, span);
                            ret
                        }
                        PouKind::FunctionBlock => {
                            self.error(
                                Code::BadCall,
                                id.span,
                                format!(
                                    "'{}' is a FUNCTION_BLOCK type: declare an instance and call the instance",
                                    id.text
                                ),
                            );
                            self.args_only(args);
                            Ty::Unknown
                        }
                        PouKind::Program => {
                            self.error(Code::BadCall, id.span, format!("'{}' is a PROGRAM: a task runs it", id.text));
                            self.args_only(args);
                            Ty::Unknown
                        }
                    };
                }
                if std_fb(&key).is_some() {
                    self.error(
                        Code::BadCall,
                        id.span,
                        format!("'{}' is a function block type: declare an instance and call the instance", id.text),
                    );
                    self.args_only(args);
                    return Ty::Unknown;
                }
                if key == "REF" {
                    return self.ref_call(args, span);
                }
                if key == "__ISVALIDREF" && self.options.has(Extension::ReferenceTo) {
                    let [Arg::Positional(x)] = args else {
                        self.error(Code::BadCall, span, "__ISVALIDREF takes one REFERENCE TO: __ISVALIDREF(r)");
                        self.args_only(args);
                        return Ty::Elem(Elem::Bool);
                    };
                    let t = self.raw_expr(x);
                    if !matches!(t, Ty::Reference(_) | Ty::Unknown) {
                        self.error(Code::OperandTypes, x.span, format!("__ISVALIDREF takes a REFERENCE TO, not {t}"));
                    }
                    return Ty::Elem(Elem::Bool);
                }
                if let Some(sig) = std_sig(&key) {
                    if key == "TRUNC" {
                        self.error(
                            Code::TruncDeprecated,
                            id.span,
                            "TRUNC without a type is deprecated since Ed.3 and gives DINT: write TRUNC_DINT, \
                             or TRUNC_INT and the others for another type",
                        );
                    }
                    return self.std_call(&key, sig, args, span);
                }
            }
        }
        let t = self.expr(callee);
        match t {
            Ty::Fb(fb) => {
                self.call_args(&fb, Callee::FunctionBlock, args, span);
                Ty::Void
            }
            Ty::Unknown => {
                self.args_only(args);
                Ty::Unknown
            }
            other => {
                self.error(Code::BadCall, callee.span, format!("{other} cannot be called"));
                self.args_only(args);
                Ty::Unknown
            }
        }
    }

    fn args_only(&mut self, args: &[Arg]) -> Vec<Ty> {
        args.iter()
            .map(|a| match a {
                Arg::Positional(e) | Arg::Input(_, e) => self.expr(e),
                Arg::Output { target, .. } => {
                    self.place(target);
                    Ty::Unknown
                }
            })
            .collect()
    }

    fn call_args(&mut self, key: &str, kind: Callee, args: &[Arg], span: Span) {
        let params = self.params(key);
        let formal = args.iter().any(|a| !matches!(a, Arg::Positional(_)));
        let positional = args.iter().any(|a| matches!(a, Arg::Positional(_)));
        if formal && positional {
            self.error(
                Code::BadCall,
                span,
                "a call passes its arguments all by name or all in order, not both (Ed.3 6.6.1.4)",
            );
            self.args_only(args);
            return;
        }
        if positional {
            if kind != Callee::Function {
                let what = if kind == Callee::Program {
                    "a program's connections are given by name: IN := ..., OUT => ..."
                } else {
                    "a function block is called with its parameters by name: IN := ..., Q => ..."
                };
                self.error(Code::BadCall, span, what);
                self.args_only(args);
                return;
            }
            let ins: Vec<&Param> = params.iter().filter(|p| p.dir != Dir::Output).collect();
            if args.len() < ins.len() || args.len() > params.len() {
                self.error(
                    Code::BadCall,
                    span,
                    format!("{key} takes {} argument(s) in order, not {} (Ed.3 6.6.1.4)", ins.len(), args.len()),
                );
            }
            let ordered: Vec<&Param> = params
                .iter()
                .filter(|p| p.dir != Dir::Output)
                .chain(params.iter().filter(|p| p.dir == Dir::Output))
                .collect();
            for (a, p) in args.iter().zip(ordered) {
                let Arg::Positional(e) = a else { continue };
                if p.dir == Dir::Input {
                    let t = self.expr(e);
                    self.coerce(&t, &p.ty, e.span);
                } else {
                    let t = self.place(e);
                    self.coerce(&p.ty, &t, e.span);
                }
            }
            return;
        }
        let mut seen: HashMap<String, Span> = HashMap::new();
        for a in args {
            let name = match a {
                Arg::Input(n, _) => n,
                Arg::Output { name, .. } => name,
                Arg::Positional(_) => unreachable!("positional arguments returned above"),
            };
            let pname = upper(&name.text);
            if seen.insert(pname.clone(), name.span).is_some() {
                self.error(Code::BadCall, name.span, format!("'{}' is passed twice", name.text));
            }
            let param = match pname.as_str() {
                "EN" => Some(Param { name: pname.clone(), dir: Dir::Input, ty: Ty::Elem(Elem::Bool) }),
                "ENO" => Some(Param { name: pname.clone(), dir: Dir::Output, ty: Ty::Elem(Elem::Bool) }),
                _ => params.iter().find(|p| p.name == pname).cloned(),
            };
            let Some(p) = param else {
                self.error(Code::BadCall, name.span, format!("{key} has no parameter '{}'", name.text));
                self.args_only(std::slice::from_ref(a));
                continue;
            };
            match a {
                Arg::Input(_, e) => match p.dir {
                    // A REFERENCE TO input is bound by the call (TwinCAT).
                    Dir::Input if matches!(p.ty, Ty::Reference(_)) => {
                        if let Ty::Reference(to) = &p.ty {
                            self.bind_reference(to, e, false);
                        }
                    }
                    Dir::Input => {
                        let t = self.expr(e);
                        self.coerce(&t, &p.ty, e.span);
                    }
                    Dir::InOut => {
                        let t = self.place(e);
                        if !t.is_unknown() && !p.ty.is_unknown() && t != p.ty {
                            self.error(
                                Code::TypeMismatch,
                                e.span,
                                format!("in-out '{}' is {}: pass a {} variable, not {t}", name.text, p.ty, p.ty),
                            );
                        }
                    }
                    Dir::Output => {
                        self.error(
                            Code::BadCall,
                            name.span,
                            format!("'{}' is an output: take it with {} => ...", name.text, name.text),
                        );
                        self.expr(e);
                    }
                },
                Arg::Output { target, .. } => {
                    if p.dir != Dir::Output {
                        self.error(
                            Code::BadCall,
                            name.span,
                            format!("'{}' is an input: pass it with {} := ...", name.text, name.text),
                        );
                    }
                    let t = self.place(target);
                    self.coerce(&p.ty, &t, target.span);
                }
                Arg::Positional(_) => {}
            }
        }
        // Every in-out "properly mapped" (Ed.3 6.6.1.4 rule 2): a
        // function's at each call, a program's in its connections.
        if kind != Callee::FunctionBlock {
            for p in params.iter().filter(|p| p.dir == Dir::InOut) {
                if !seen.contains_key(&p.name) {
                    let how = if kind == Callee::Program { "connected to a global variable" } else { "passed" };
                    self.error(Code::BadCall, span, format!("in-out '{}' of {key} must be {how}", p.name));
                }
            }
        }
    }
}

fn literal_int(l: &Literal) -> Option<i128> {
    let text = l.text.replace('_', "");
    if let Some((base, digits)) = text.split_once('#') {
        let radix = base.parse::<u32>().ok()?;
        return i128::from_str_radix(digits, radix).ok();
    }
    text.parse::<i128>().ok()
}

/// TIME and date arithmetic by operator (Ed.3 Table 36).
fn time_arithmetic(op: BinaryOp, a: &Ty, b: &Ty) -> Option<Ty> {
    use Elem::*;
    let (x, y) = (a.elem(), b.elem());
    let dur = |e: Option<Elem>| e.is_some_and(Elem::is_duration);
    let num = |t: &Ty| t.is_num();
    Some(match (op, x, y) {
        (BinaryOp::Add | BinaryOp::Sub, Some(Time), Some(Time)) => Ty::Elem(Time),
        (BinaryOp::Add | BinaryOp::Sub, Some(Ltime), Some(Ltime | Time))
        | (BinaryOp::Add | BinaryOp::Sub, Some(Time), Some(Ltime)) => Ty::Elem(Ltime),
        // A short date and an LTIME: the date widened to its long type
        // (Ed.3 6.6.1.6, rule 7), as the LTIME cannot narrow.
        (BinaryOp::Add | BinaryOp::Sub, Some(d @ (Tod | Dt)), Some(Time)) => Ty::Elem(d),
        (BinaryOp::Add | BinaryOp::Sub, Some(Tod | Ltod), Some(Ltime))
        | (BinaryOp::Add | BinaryOp::Sub, Some(Ltod), Some(Time)) => Ty::Elem(Ltod),
        (BinaryOp::Add | BinaryOp::Sub, Some(Dt | Ldt), Some(Ltime))
        | (BinaryOp::Add | BinaryOp::Sub, Some(Ldt), Some(Time)) => Ty::Elem(Ldt),
        (BinaryOp::Sub, Some(Date), Some(Date))
        | (BinaryOp::Sub, Some(Tod), Some(Tod))
        | (BinaryOp::Sub, Some(Dt), Some(Dt)) => Ty::Elem(Time),
        // Two dates, one long: the other widened.
        (BinaryOp::Sub, Some(Ldate | Date), Some(Ldate | Date))
        | (BinaryOp::Sub, Some(Ltod | Tod), Some(Ltod | Tod))
        | (BinaryOp::Sub, Some(Ldt | Dt), Some(Ldt | Dt)) => Ty::Elem(Ltime),
        (BinaryOp::Mul | BinaryOp::Div, Some(d), _) if dur(Some(d)) && num(b) => Ty::Elem(d),
        (BinaryOp::Mul, _, Some(d)) if dur(Some(d)) && num(a) => Ty::Elem(d),
        _ => return None,
    })
}

/// `1 byte`, `4 bytes`.
fn bytes(n: u32) -> String {
    if n == 1 { "1 byte".into() } else { format!("{n} bytes") }
}

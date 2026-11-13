//! Names and types: every name resolves, every value meets its target
//! by the standard's conversion rules, every call fits what it calls.
//!
//! One error per mistake: an expression whose type could not be worked
//! out is `Ty::Unknown`, and nothing is reported against it again.

use crate::stdlib::{Dir, reserved_name, std_fb, std_sig, unify};

mod stdcall;
use crate::Binding;
use crate::types::{Conversion, Elem, Ty};
use crate::{Extension, Options};
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_syntax::ast::*;
use std::collections::{HashMap, HashSet};

fn upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

/// An enumeration declared in place: its base type and its values.
type InPlaceEnum = std::rc::Rc<(Option<Ident>, Vec<EnumValue>)>;

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
    /// An `ARRAY[*]` input: the caller's array itself, read and never
    /// written (docs/dependencies.md).
    flex_in: bool,
    /// An input that is a function block instance or holds one, in an
    /// array or a structure: the instance's variables are read, never
    /// written, and it is not called (Ed.3 6.6.1.4 rule 4, 6.6.3.4 rules
    /// 6 and 7).
    inst_in: bool,
}

impl Var {
    fn of(ty: Ty, constant: bool) -> Var {
        Var { ty, constant, temp: false, input: false, flex_in: false, inst_in: false }
    }
}

/// A name a POU declares, as a constant expression sees it.
#[derive(Clone, Copy)]
enum Konst<'a> {
    /// A CONSTANT with an initial value.
    Value(&'a VarDecl, &'a Expr),
    /// A VAR_EXTERNAL: the global's.
    Global,
    /// A variable: no constant, and no global of its name seen.
    Not,
}

/// Whether a declaration is of an integer type, which a constant
/// integer expression may name.
fn int_decl(d: &VarDecl) -> bool {
    matches!(&d.ty, TypeSpec::Named(id) if Elem::from_name(&id.text).is_some_and(|e| e.is_int()))
}

/// Whether a parameter of type `param` takes a variable of type `arg` as
/// an `ARRAY[*]` does (Ed.3 6.5.3): an array of as many dimensions and
/// the same element type, whatever its bounds.
fn flex_takes(param: &Ty, arg: &Ty) -> bool {
    match (param, arg) {
        (Ty::Array { elem: a, dims: da }, Ty::Array { elem: b, dims: db }) => {
            a == b && da.len() == db.len() && da.iter().all(Option::is_none)
        }
        _ => false,
    }
}

/// Whether `t` is a variable-length array, `ARRAY[*]`.
fn is_flex(t: &Ty) -> bool {
    matches!(t, Ty::Array { dims, .. } if dims.iter().any(Option::is_none))
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

/// A global's declaration, and whether it is CONSTANT.
type GlobalDecl<'a> = (&'a VarDecl, bool);

/// A CONFIGURATION as its POUs' VAR_EXTERNALs see it (Ed.3 6.5.2.2).
struct ConfigScope<'a> {
    /// Upper-cased.
    name: String,
    /// Its own globals, which each of its resources sees.
    globals: HashMap<String, GlobalDecl<'a>>,
    resources: Vec<ResourceScope<'a>>,
}

/// A RESOURCE: its name (none when the configuration names none), its
/// own globals, the program instances it runs and their PROGRAMs,
/// upper-cased.
struct ResourceScope<'a> {
    name: Option<String>,
    globals: HashMap<String, GlobalDecl<'a>>,
    programs: Vec<(String, String)>,
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

/// The name a call is written with, for a message.
fn callee_name(callee: &Expr) -> String {
    match &callee.kind {
        ExprKind::Name(id) => id.text.clone(),
        _ => String::new(),
    }
}

/// What a call may run, for the recursion check.
#[derive(Clone)]
enum CallTo {
    /// A FUNCTION, a block's base body (`SUPER()`), a base's method
    /// (`SUPER.m()`): that one.
    Exactly(String),
    /// A block instance called: its type's body, or a derived type's,
    /// as the instance may be one (Ed.3 6.6.8).
    Body(String),
    /// `x.m()`, `THIS.m()`: the method, or an override of it in a type
    /// derived from `x`'s.
    Method(String, String),
    /// A method through an interface: any implementation of it.
    Itf(String, String),
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
    /// Every global, the first of each name: what a POU's VAR_EXTERNAL
    /// is typed and folded by. Which one a VAR_EXTERNAL is depends on
    /// where its POU runs: `top_globals` and `configs` (Ed.3 6.5.2.2).
    globals: HashMap<String, (&'a VarDecl, bool)>,
    /// VAR_GLOBAL at the top of a file, a global variable list: seen in
    /// every configuration.
    top_globals: HashMap<String, (&'a VarDecl, bool)>,
    /// Each CONFIGURATION: its name, its globals and its resources', and
    /// the PROGRAMs its resources run.
    configs: Vec<ConfigScope<'a>>,
    /// enumerated value -> the enum types that have it
    enum_values: HashMap<String, Vec<std::rc::Rc<str>>>,
    type_cache: HashMap<String, Ty>,
    resolving: Vec<String>,
    struct_fields: HashMap<String, HashMap<String, Ty>>,
    /// The enumerations declared in place, by the name the checker gives
    /// each (`resolve_spec`): its base type and its values.
    inplace: HashMap<String, InPlaceEnum>,
    params_cache: HashMap<String, Vec<Param>>,
    /// POU or structure type -> its `*` addresses, through nested instances.
    partly_cache: HashMap<String, Vec<Partly>>,
    /// Untyped one-character STRING literals, by span: each is a CHAR
    /// where a CHAR is wanted (Ed.3 Table 6, 1b: "String of length one
    /// or character CHAR").
    /// Untyped one-character string literals, `true` for a double-byte
    /// one: a CHAR or WCHAR where one is wanted (Ed.3 Table 6, 1b and 2b).
    one_char: std::collections::HashMap<Span, bool>,
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
    /// The type whose instance `THIS` is: in a FUNCTION_BLOCK's body, and
    /// in a method of a CLASS or FUNCTION_BLOCK (Ed.3 6.6.5.7.2).
    this_type: Option<String>,
    /// How deep in `place` the checker is: the outermost one checks what
    /// is written as a whole.
    place_depth: u32,
    /// The instance's variables the POU being checked reaches as its own:
    /// `THIS.x` is `x` for each.
    own_vars: std::collections::HashSet<String>,
    /// In a method: its type's variables it cannot reach, and why (Ed.3
    /// 6.6.7.2.3: not its VAR_TEMP, VAR_IN_OUT, R_EDGE or F_EDGE inputs).
    hidden: HashMap<String, &'static str>,
    /// Each CLASS's and FUNCTION_BLOCK's base type, when it EXTENDS one
    /// that may be its base (Ed.3 6.6.5.5, 6.6.7.2.7): upper-cased keys.
    bases: HashMap<String, String>,
    /// The variables a derived type inherits, while its declarations
    /// are checked: one declared again is 6.6.5.5.5's error 1.
    inherited: std::collections::HashSet<String>,
    /// The `SUPER()` already in the body being checked (Ed.3 6.6.7.2.9:
    /// once).
    super_body: Option<Span>,
    /// Every call of the project's POUs and methods, for the recursion
    /// check (`check_recursion`): the caller's key, what it calls, where.
    calls: Vec<(String, CallTo, Span)>,
    /// A method call written `SUPER.m()`: the base's method itself.
    static_call: bool,
    /// The names the POU being checked declares, for constant expressions
    /// (Ed.3 6.6.1.2.4): its constants' values, and what hides a global.
    consts: HashMap<String, Konst<'a>>,
    /// How deep a constant's value is being worked out, through others.
    const_depth: u8,
    /// Whether the declaration being resolved may be `ARRAY[*]` (Ed.3
    /// 6.5.3): taken by the first `resolve_spec`.
    star_ok: bool,
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
            top_globals: HashMap::new(),
            configs: Vec::new(),
            enum_values: HashMap::new(),
            type_cache: HashMap::new(),
            resolving: Vec::new(),
            struct_fields: HashMap::new(),
            inplace: HashMap::new(),
            params_cache: HashMap::new(),
            partly_cache: HashMap::new(),
            one_char: std::collections::HashMap::new(),
            partly_visiting: Vec::new(),
            diags: Vec::new(),
            locals: HashMap::new(),
            pou_kind: None,
            pou_key: String::new(),
            steps: HashMap::new(),
            actions: HashMap::new(),
            loop_depth: 0,
            this_type: None,
            place_depth: 0,
            own_vars: std::collections::HashSet::new(),
            hidden: HashMap::new(),
            bases: HashMap::new(),
            inherited: std::collections::HashSet::new(),
            super_body: None,
            calls: Vec::new(),
            static_call: false,
            consts: HashMap::new(),
            const_depth: 0,
            star_ok: false,
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
                        let what = p.kind.keyword();
                        if declare_top(self, &p.name, what) {
                            let owner = upper(&p.name.text);
                            self.pous.insert(owner.clone(), p);
                            // Each method by `T__M`, as a FUNCTION is by its name.
                            let mut seen = std::collections::HashSet::new();
                            for m in p.methods.iter().filter(|m| !m.name.text.is_empty()) {
                                if !seen.insert(upper(&m.name.text)) {
                                    self.error(
                                        Code::Duplicate,
                                        m.name.span,
                                        format!(
                                            "{} has two methods '{}': a method is not overloaded (Ed.3 6.6.5.4.3 NOTE 1)",
                                            p.name.text, m.name.text
                                        ),
                                    );
                                    continue;
                                }
                                self.pous.insert(crate::method_key(&owner, &m.name.text), m);
                            }
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
                                    self.enum_values
                                        .entry(upper(&v.name.text))
                                        .or_default()
                                        .push(upper(&d.name.text).into());
                                }
                            }
                        }
                    }
                    Item::Globals(block) => {
                        let mut top = std::mem::take(&mut self.top_globals);
                        self.collect_globals(block, &mut top);
                        self.top_globals = top;
                    }
                    Item::Configuration(c) => self.collect_configuration(c),
                    Item::Pou(_) | Item::ConfigList(..) | Item::Namespace(_) | Item::Using(_) => {}
                }
            }
        }
    }

    /// The globals of `block` into `scope`, one name once there, and the
    /// first of each name into `globals`.
    fn collect_globals(&mut self, block: &'a VarBlock, scope: &mut HashMap<String, (&'a VarDecl, bool)>) {
        for d in &block.decls {
            for n in &d.names {
                let key = upper(&n.text);
                if scope.insert(key.clone(), (d, block.constant)).is_some() {
                    self.error(Code::Duplicate, n.span, format!("global variable '{}' is declared twice", n.text));
                }
                self.globals.entry(key).or_insert((d, block.constant));
            }
        }
    }

    /// A CONFIGURATION: its name once in the project, and not IECF, which
    /// names the runtime's header and C (docs/c-output.md §2); its
    /// globals and its resources', one name once (Ed.3 6.5.2.2).
    fn collect_configuration(&mut self, c: &'a Configuration) {
        let name = upper(&c.name.text);
        if name == "IECF" {
            self.error(
                Code::ReservedName,
                c.name.span,
                "'IECF' is reserved for a CONFIGURATION: iecf.h and the C prefix iecf_ are the runtime's; choose another name",
            );
        }
        if self.configs.iter().any(|s| s.name == name) {
            self.error(Code::Duplicate, c.name.span, format!("CONFIGURATION '{}' is declared twice", c.name.text));
        }
        let mut globals = HashMap::new();
        for b in &c.globals {
            self.collect_globals(b, &mut globals);
        }
        // A resource's globals are its own (Ed.3 6.5.2.2): two resources
        // may name theirs alike, and one named as its configuration's
        // hides that one in the resource, as an inner name does in
        // CODESYS (decided 2026-11-13, florin).
        let mut resources = Vec::new();
        for r in &c.resources {
            let mut own = HashMap::new();
            for b in &r.globals {
                self.collect_globals(b, &mut own);
            }
            let programs = r.programs.iter().map(|p| (upper(&p.name.text), upper(&p.program_type.text))).collect();
            resources.push(ResourceScope { name: r.name.as_ref().map(|n| upper(&n.text)), globals: own, programs });
        }
        self.configs.push(ConfigScope { name, globals, resources });
    }

    pub(crate) fn check_all(&mut self, units: &[&'a SourceUnit]) {
        self.check_bases();
        self.check_interface_names();
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
                    // Taken out before the check (namespaces.rs).
                    Item::Namespace(_) | Item::Using(_) => {}
                }
            }
        }
        self.check_config_lists(&lists, configured);
        self.check_blocks_contain_themselves();
        self.check_recursion();
        self.check_externals();
    }

    /// E0017: each VAR_EXTERNAL against the globals where its POU runs
    /// (Ed.3 6.5.2.2): in each RESOURCE that runs it, the resource's, its
    /// configuration's and the top-level ones; a POU no configuration
    /// runs, a library's, against any global of the name. A global named
    /// as one further out hides it.
    ///
    /// Which global each name reaches, in each place, goes to the
    /// generator (`Types::bindings`): where one type's instances reach
    /// different ones, each holds its own (docs/dependencies.md). A
    /// FUNCTION has no instance to hold which, so it reaches one; and
    /// CONSTANT globals of one name hold one value (`check_one_constant`).
    fn check_externals(&mut self) {
        let configs = std::mem::take(&mut self.configs);
        let top = self.top_globals.clone();
        let mut seen: std::collections::HashSet<(Span, String)> = std::collections::HashSet::new();
        let mut ran: std::collections::HashSet<String> = std::collections::HashSet::new();
        let none = HashMap::new();
        self.check_one_constant(&configs, &top);
        for c in &configs {
            // Per type and name: each global reached, and where.
            let mut found: std::collections::BTreeMap<
                (String, String),
                std::collections::BTreeMap<Binding, Vec<String>>,
            > = std::collections::BTreeMap::new();
            // The configuration's own instances, its globals', run where
            // they are declared: they see its globals and the top-level
            // ones, no resource's.
            let decls: Vec<&'a VarDecl> = c.globals.values().map(|(d, _)| *d).collect();
            let mut own: Vec<String> = Vec::new();
            for d in decls {
                let t = self.resolve_spec(&d.ty, false);
                own.extend(self.instance_types(&t, 0));
            }
            if !own.is_empty() {
                let mut scope = top.clone();
                let mut binds: HashMap<String, Binding> =
                    top.keys().map(|k| (k.clone(), Binding::Top(k.clone()))).collect();
                for (k, v) in &c.globals {
                    scope.insert(k.clone(), *v);
                    binds.insert(k.clone(), Binding::Config(k.clone()));
                }
                let where_ = format!("CONFIGURATION {}", c.name);
                for key in self.reached(&own) {
                    ran.insert(key.clone());
                    let place =
                        format!("in {where_}, which declares an instance running {}", crate::namespaces::display(&key));
                    self.check_pou_externals(&key, &none, &scope, &place, &mut seen);
                    let owner = crate::owner_of(&key).to_string();
                    for n in self.needs(&key, 0) {
                        if let Some(b) = binds.get(&n) {
                            let at = found.entry((owner.clone(), n)).or_default().entry(b.clone()).or_default();
                            if !at.contains(&where_) {
                                at.push(where_.clone());
                            }
                        }
                    }
                }
            }
            for r in &c.resources {
                let rname = r.name.clone().unwrap_or_default();
                let mut scope = top.clone();
                let mut binds: HashMap<String, Binding> =
                    top.keys().map(|k| (k.clone(), Binding::Top(k.clone()))).collect();
                for (k, v) in &c.globals {
                    scope.insert(k.clone(), *v);
                    binds.insert(k.clone(), Binding::Config(k.clone()));
                }
                for (k, v) in &r.globals {
                    scope.insert(k.clone(), *v);
                    binds.insert(k.clone(), Binding::Resource(rname.clone(), k.clone()));
                }
                let where_ = match &r.name {
                    Some(n) => format!("resource {n} of CONFIGURATION {}", c.name),
                    None => format!("CONFIGURATION {}", c.name),
                };
                // A PROGRAM's own VAR_GLOBALs (Ed.3 Table 47, 8a and 8b)
                // are seen by what each instance of it contains: the
                // instances it holds and the FUNCTIONs it calls, beside
                // what the resource sees. Each instance has its own.
                for (inst, ptype) in &r.programs {
                    let own = self.program_globals(ptype);
                    if own.is_empty() {
                        continue;
                    }
                    let mut pscope = scope.clone();
                    let mut pbinds = binds.clone();
                    for (k, v) in &own {
                        pscope.insert(k.clone(), *v);
                        pbinds.insert(k.clone(), Binding::Program(rname.clone(), inst.clone(), k.clone()));
                    }
                    let pwhere = format!("program {inst} of {where_}");
                    for key in self.reached(std::slice::from_ref(ptype)) {
                        ran.insert(key.clone());
                        let place = format!("in {pwhere}, which runs {}", crate::namespaces::display(&key));
                        self.check_pou_externals(&key, &none, &pscope, &place, &mut seen);
                        let owner = crate::owner_of(&key).to_string();
                        for n in self.needs(&key, 0) {
                            if let Some(b) = pbinds.get(&n) {
                                let at = found.entry((owner.clone(), n)).or_default().entry(b.clone()).or_default();
                                if !at.contains(&pwhere) {
                                    at.push(pwhere.clone());
                                }
                            }
                        }
                    }
                }
                // Its other programs, and the instances its own globals hold.
                let mut roots: Vec<String> = Vec::new();
                for (_, ptype) in &r.programs {
                    if self.program_globals(ptype).is_empty() {
                        roots.push(ptype.clone());
                    }
                }
                let held: Vec<&'a VarDecl> = r.globals.values().map(|(d, _)| *d).collect();
                for d in held {
                    let t = self.resolve_spec(&d.ty, false);
                    roots.extend(self.instance_types(&t, 0));
                }
                for key in self.reached(&roots) {
                    ran.insert(key.clone());
                    let place = format!("in {where_}, which runs {}", crate::namespaces::display(&key));
                    self.check_pou_externals(&key, &none, &scope, &place, &mut seen);
                    let owner = crate::owner_of(&key).to_string();
                    for n in self.needs(&key, 0) {
                        if let Some(b) = binds.get(&n) {
                            let at = found.entry((owner.clone(), n)).or_default().entry(b.clone()).or_default();
                            if !at.contains(&where_) {
                                at.push(where_.clone());
                            }
                        }
                    }
                }
            }
            for ((owner, n), bs) in &found {
                self.out
                    .bindings
                    .entry((c.name.clone(), owner.clone()))
                    .or_default()
                    .insert(n.clone(), bs.keys().cloned().collect());
                if bs.len() > 1 {
                    let places: Vec<String> = bs.values().flatten().cloned().collect();
                    self.refuse_two_bindings(owner, n, &places);
                }
            }
        }
        // A POU no configuration runs: against every global of the name.
        let mut all: HashMap<String, Vec<GlobalDecl<'a>>> = HashMap::new();
        let mut programs: Vec<String> =
            self.pous.iter().filter(|(_, p)| p.kind == PouKind::Program).map(|(k, _)| k.clone()).collect();
        programs.sort();
        for k in programs {
            for (n, g) in self.program_globals(&k) {
                all.entry(n).or_default().push(g);
            }
        }
        for (k, v) in top.iter().chain(
            configs.iter().flat_map(|c| c.globals.iter().chain(c.resources.iter().flat_map(|r| r.globals.iter()))),
        ) {
            all.entry(k.clone()).or_default().push(*v);
        }
        let mut keys: Vec<String> = self.pous.keys().filter(|k| !ran.contains(*k)).cloned().collect();
        keys.sort();
        for key in keys {
            let empty = HashMap::new();
            self.check_pou_externals(&key, &all, &empty, "", &mut seen);
        }
        self.configs = configs;
    }

    /// The VAR_GLOBALs the PROGRAM `key` declares itself, by upper-cased
    /// name.
    fn program_globals(&self, key: &str) -> HashMap<String, GlobalDecl<'a>> {
        let mut out = HashMap::new();
        let Some(pou) = self.pous.get(key).copied().filter(|p| p.kind == PouKind::Program) else { return out };
        for b in pou.var_blocks.iter().filter(|b| b.kind == VarKind::Global) {
            for d in &b.decls {
                for n in &d.names {
                    out.entry(upper(&n.text)).or_insert((d, b.constant));
                }
            }
        }
        out
    }

    /// E0012: globals of one name in different scopes - two
    /// configurations', two resources' - of which one is CONSTANT are all
    /// CONSTANT, of one integer value. A constant is worked out once,
    /// for a bound or an initial value of a type or a POU, which is one
    /// whichever scope runs it (docs/dependencies.md).
    fn check_one_constant(&mut self, configs: &[ConfigScope<'a>], top: &HashMap<String, GlobalDecl<'a>>) {
        let mut all: std::collections::BTreeMap<String, Vec<(GlobalDecl<'a>, String)>> =
            std::collections::BTreeMap::new();
        for (k, g) in top {
            all.entry(k.clone()).or_default().push((*g, "the top level".to_string()));
        }
        for c in configs {
            for (k, g) in &c.globals {
                all.entry(k.clone()).or_default().push((*g, format!("CONFIGURATION {}", c.name)));
            }
            for r in &c.resources {
                let place = match &r.name {
                    Some(n) => format!("resource {n} of CONFIGURATION {}", c.name),
                    None => format!("CONFIGURATION {}", c.name),
                };
                for (k, g) in &r.globals {
                    all.entry(k.clone()).or_default().push((*g, place.clone()));
                }
            }
        }
        let mut programs: Vec<String> =
            self.pous.iter().filter(|(_, p)| p.kind == PouKind::Program).map(|(k, _)| k.clone()).collect();
        programs.sort();
        for p in programs {
            for (k, g) in self.program_globals(&p) {
                all.entry(k).or_default().push((g, format!("PROGRAM {p}")));
            }
        }
        for (k, mut found) in all {
            if found.len() < 2 || !found.iter().any(|((_, c), _)| *c) {
                continue;
            }
            let span = |d: &VarDecl| d.names.iter().find(|n| upper(&n.text) == k).map_or(d.ty.span(), |n| n.span);
            found.sort_by_key(|((d, _), _)| (span(d).file.0, span(d).start));
            let ((first, first_const), first_place) = found[0].clone();
            let value = |c: &mut Self, d: &VarDecl| match d.init.as_ref() {
                Some(Init::Expr(e)) => c.const_int(e),
                _ => None,
            };
            let v0 = value(self, first);
            for ((d, constant), place) in &found[1..] {
                let name = d.names.iter().find(|n| upper(&n.text) == k).map_or(k.as_str(), |n| n.text.as_str());
                if *constant != first_const {
                    self.error(
                        Code::Duplicate,
                        span(d),
                        format!(
                            "global variable '{name}' is CONSTANT in one of {first_place} and {place}, and not in the \
                             other: a constant is worked out once, so globals of one name are all CONSTANT or none is"
                        ),
                    );
                    continue;
                }
                let v = value(self, d);
                if v0.is_none() || v != v0 {
                    self.error(
                        Code::Duplicate,
                        span(d),
                        format!(
                            "CONSTANT '{name}' of {place} is not the integer value it has in {first_place}: a constant is \
                             worked out once, so the CONSTANT globals of one name hold one integer value (name them apart)"
                        ),
                    );
                }
            }
        }
    }

    /// The name `n` reaches two globals or more where the FUNCTION
    /// `owner` runs (`places`): it has no instance to hold which (E0017).
    fn refuse_two_bindings(&mut self, owner: &str, n: &str, places: &[String]) {
        let Some(pou) = self.pous.get(owner).copied().filter(|p| p.kind == PouKind::Function) else { return };
        let own = pou
            .var_blocks
            .iter()
            .filter(|b| b.kind == VarKind::External)
            .find_map(|b| b.decls.iter().find_map(|d| d.names.iter().find(|x| upper(&x.text) == n)));
        let at = own.map_or(pou.name.span, |x| x.span);
        // As it is written, where this POU writes it.
        let n = own.map_or(n, |x| x.text.as_str());
        let shown = places.join(" and in ");
        self.error(
            Code::ExternalWithoutGlobal,
            at,
            format!(
                "'{n}' is a different global in {shown}, which run FUNCTION {}: a FUNCTION has no instance to hold \
                 which, so the globals it reaches are one wherever it runs (name them apart, or pass the value in)",
                crate::namespaces::display(owner)
            ),
        );
    }

    /// The external names the code of the POU or method `key` reaches as
    /// it runs, through itself: its VAR_EXTERNALs (and the implicit ones
    /// of the implicit-externals extension), and every name the code of
    /// an instance declared in it reaches - its type's, its bases', their
    /// methods', through the instances in those - as an instance it
    /// creates while running takes them from it.
    fn needs(&mut self, key: &str, depth: usize) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let Some(pou) = self.pous.get(key).copied() else { return out };
        if depth > 32 {
            return out;
        }
        for b in &pou.var_blocks {
            for d in &b.decls {
                if b.kind == VarKind::External {
                    out.extend(d.names.iter().map(|n| upper(&n.text)));
                    continue;
                }
                if b.kind == VarKind::InOut {
                    continue;
                }
                // A PROGRAM's own global: each instance of it has its.
                if b.kind == VarKind::Global && pou.kind == PouKind::Program {
                    out.extend(d.names.iter().map(|n| upper(&n.text)));
                }
                let t = self.resolve_spec(&d.ty, false);
                for inst in self.instance_types(&t, 0) {
                    out.extend(self.instance_needs(&inst, depth + 1));
                }
            }
        }
        out.extend(self.out.implicit_externals(key).iter().map(|(k, _)| k.clone()));
        out.sort();
        out.dedup();
        out
    }

    /// Every external name the code of an instance of the FUNCTION_BLOCK
    /// or CLASS `key` reaches: its own, its bases', their methods'.
    fn instance_needs(&mut self, key: &str, depth: usize) -> Vec<String> {
        let mut out = Vec::new();
        let types: Vec<String> = std::iter::once(key.to_string()).chain(self.ancestors(key)).collect();
        for t in types {
            out.extend(self.needs(&t, depth));
            let methods: Vec<String> = self
                .pous
                .get(&t)
                .map(|p| p.methods.iter().map(|m| crate::method_key(&t, &m.name.text)).collect())
                .unwrap_or_default();
            for m in methods {
                out.extend(self.needs(&m, depth));
            }
        }
        out
    }

    /// The VAR_EXTERNALs of the POU or method `key` against `scope`, or,
    /// where `scope` has not the name, any of `any`'s of the name.
    fn check_pou_externals(
        &mut self,
        key: &str,
        any: &HashMap<String, Vec<GlobalDecl<'a>>>,
        scope: &HashMap<String, GlobalDecl<'a>>,
        place: &str,
        seen: &mut std::collections::HashSet<(Span, String)>,
    ) {
        let Some(pou) = self.pous.get(key).copied() else { return };
        for b in pou.var_blocks.iter().filter(|b| b.kind == VarKind::External) {
            for d in &b.decls {
                let ty = self.resolve_spec(&d.ty, false);
                for n in &d.names {
                    let k = upper(&n.text);
                    let found: Vec<GlobalDecl<'a>> = match scope.get(&k) {
                        Some(g) => vec![*g],
                        None => any.get(&k).cloned().unwrap_or_default(),
                    };
                    let mut report = |c: &mut Self, code: Code, at: Span, message: String| {
                        if seen.insert((at, message.clone())) {
                            c.error(code, at, message);
                        }
                    };
                    let where_ = if place.is_empty() { String::new() } else { format!(" {place}") };
                    if found.is_empty() {
                        report(
                            self,
                            Code::ExternalWithoutGlobal,
                            n.span,
                            format!("there is no global variable '{}'{where_}", n.text),
                        );
                        continue;
                    }
                    let typed: Vec<(Ty, bool)> =
                        found.iter().map(|(g, c)| (self.resolve_spec(&g.ty, false), *c)).collect();
                    let matching = typed.iter().find(|(gt, _)| gt.is_unknown() || ty.is_unknown() || *gt == ty);
                    match matching {
                        None => {
                            let gt = &typed[0].0;
                            report(
                                self,
                                Code::ExternalWithoutGlobal,
                                d.ty.span(),
                                format!(
                                    "global '{}' is {gt}{where_}, so its VAR_EXTERNAL must be {gt}, not {ty}",
                                    n.text
                                ),
                            );
                        }
                        Some((_, true)) if !b.constant => report(
                            self,
                            Code::ExternalWithoutGlobal,
                            n.span,
                            format!("global '{}' is CONSTANT{where_}: declare it VAR_EXTERNAL CONSTANT", n.text),
                        ),
                        Some(_) => {}
                    }
                }
            }
        }
    }

    /// The POUs and methods whose code a program instance of each of
    /// `programs` brings in, to run with the globals seen there: the
    /// programs, the types of the instances declared in any of them and
    /// their bases, and the FUNCTIONs called there. Not what a call of an
    /// instance may run: an instance is declared somewhere, and reaches
    /// the globals seen there, wherever it is called from - through a
    /// reference or an in-out, from another resource.
    fn reached(&mut self, programs: &[String]) -> Vec<String> {
        let mut edges: HashMap<String, Vec<String>> = HashMap::new();
        for (from, to, _) in self.calls.clone() {
            if let CallTo::Exactly(k) = &to
                && self.pous.get(k).is_some_and(|p| p.kind == PouKind::Function)
            {
                edges.entry(from).or_default().push(k.clone());
            }
        }
        let mut out: Vec<String> = Vec::new();
        let mut todo: Vec<String> = programs.to_vec();
        while let Some(k) = todo.pop() {
            if out.contains(&k) || !self.pous.contains_key(&k) {
                continue;
            }
            out.push(k.clone());
            let Some(pou) = self.pous.get(&k).copied() else { continue };
            for b in pou.var_blocks.iter().filter(|b| !matches!(b.kind, VarKind::External | VarKind::InOut)) {
                for d in &b.decls {
                    let t = self.resolve_spec(&d.ty, false);
                    todo.extend(self.instance_types(&t, 0));
                }
            }
            todo.extend(self.ancestors(&k));
            for m in &pou.methods {
                todo.push(crate::method_key(&k, &m.name.text));
            }
            todo.extend(edges.get(&k).cloned().unwrap_or_default());
        }
        out
    }

    /// The FUNCTION_BLOCK and CLASS types a value of `t` holds instances
    /// of: itself, an array's elements, a structure's fields.
    fn instance_types(&mut self, t: &Ty, depth: usize) -> Vec<String> {
        if depth > 32 {
            return Vec::new();
        }
        match t {
            Ty::Fb(k) if self.pous.contains_key(&upper(k)) => vec![upper(k)],
            Ty::Array { elem, .. } => self.instance_types(elem, depth + 1),
            Ty::Struct(k) => {
                let fields: Vec<&'a VarDecl> = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().collect(),
                    _ => Vec::new(),
                };
                let mut out = Vec::new();
                for f in fields {
                    let ft = self.resolve_spec(&f.ty, false);
                    out.extend(self.instance_types(&ft, depth + 1));
                }
                out
            }
            _ => Vec::new(),
        }
    }

    fn record_call(&mut self, to: CallTo, at: Span) {
        if !self.pou_key.is_empty() {
            self.calls.push((self.pou_key.clone(), to, at));
        }
    }

    /// What a call may run: the keys of the POUs and methods.
    fn call_targets(&self, to: &CallTo) -> Vec<String> {
        let blocks = || {
            self.pous.iter().filter(|(_, p)| matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)).map(|(k, _)| k)
        };
        let mut out: Vec<String> = match to {
            CallTo::Exactly(k) => vec![k.clone()],
            CallTo::Body(t) => blocks()
                .filter(|d| self.derives(d, t) && self.pous.get(*d).is_some_and(|p| p.kind == PouKind::FunctionBlock))
                .cloned()
                .collect(),
            CallTo::Method(owner, m) => blocks()
                .filter(|d| self.derives(d, owner))
                .filter_map(|d| self.find_method(d, m).map(|(def, _)| crate::method_key(&def, m)))
                .collect(),
            CallTo::Itf(i, m) => blocks()
                .filter(|d| self.out.implemented(d).contains(i))
                .filter_map(|d| self.find_method(d, m).map(|(def, _)| crate::method_key(&def, m)))
                .collect(),
        };
        out.sort();
        out.dedup();
        out
    }

    /// E0016: a POU or method that can call itself, directly or through
    /// others (Ed.3 6.6.1: "The recursive call of POUs and methods is
    /// Implementer specific"). iecf refuses it, as PLCnext does, and as
    /// CODESYS does a FUNCTION's: its stack would be unbounded, and C's
    /// overflow is no fault the program could report (docs/extensions.md,
    /// recursion, proposed). A call that may run an override or another
    /// implementation counts as calling each.
    fn check_recursion(&mut self) {
        let mut edges: HashMap<String, Vec<(String, Span)>> = HashMap::new();
        for (from, to, at) in self.calls.clone() {
            for t in self.call_targets(&to) {
                edges.entry(from.clone()).or_default().push((t, at));
            }
        }
        let mut nodes: Vec<String> = edges.keys().cloned().collect();
        nodes.sort();
        let mut done: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seen_cycles: std::collections::HashSet<Vec<String>> = std::collections::HashSet::new();
        for start in nodes {
            if done.contains(&start) {
                continue;
            }
            // Depth first, the path kept: an edge back into it closes a cycle.
            let mut path: Vec<String> = vec![start.clone()];
            let mut next: Vec<usize> = vec![0];
            while let Some(top) = path.last().cloned() {
                let i = *next.last().unwrap_or(&0);
                let out = edges.get(&top).cloned().unwrap_or_default();
                if i >= out.len() {
                    done.insert(top);
                    path.pop();
                    next.pop();
                    continue;
                }
                if let Some(n) = next.last_mut() {
                    *n += 1;
                }
                let (to, at) = &out[i];
                if let Some(pos) = path.iter().position(|p| p == to) {
                    let mut cycle: Vec<String> = path[pos..].to_vec();
                    let mut key = cycle.clone();
                    key.sort();
                    if seen_cycles.insert(key) {
                        cycle.push(to.clone());
                        let shown: Vec<String> = cycle.iter().map(|k| crate::namespaces::display(k)).collect();
                        self.error(
                            Code::BadCall,
                            *at,
                            format!(
                                "a recursive call, {}: Ed.3 6.6.1 leaves recursion to the implementer, and iecf does not allow it - a stack without a bound (docs/extensions.md)",
                                shown.join(" -> ")
                            ),
                        );
                    }
                } else if !done.contains(to) {
                    path.push(to.clone());
                    next.push(0);
                }
            }
        }
    }

    /// E0011: a FUNCTION_BLOCK or CLASS whose instance holds one of
    /// itself - in a variable, an input or output, a structure's field or
    /// an array, or through its base - has no end, as a structure defined
    /// in terms of itself has none. An in-out, a reference, an interface
    /// and a VAR_TEMP hold no instance.
    fn check_blocks_contain_themselves(&mut self) {
        let blocks: Vec<(String, &'a Pou)> = self
            .pous
            .iter()
            .filter(|(_, p)| matches!(p.kind, PouKind::FunctionBlock | PouKind::Class))
            .map(|(k, p)| (k.clone(), *p))
            .collect();
        for (key, pou) in &blocks {
            let mut seen = vec![key.clone()];
            let mut todo = vec![key.clone()];
            let mut found = false;
            while let Some(k) = todo.pop() {
                for held in self.held_blocks(&k) {
                    if held == *key {
                        found = true;
                    } else if !seen.contains(&held) {
                        seen.push(held.clone());
                        todo.push(held);
                    }
                }
            }
            if found {
                self.error(
                    Code::UnknownType,
                    pou.name.span,
                    format!(
                        "{} holds an instance of itself, in its variables or theirs: it would have no end (as a type defined in terms of itself)",
                        pou.name.text
                    ),
                );
            }
        }
    }

    /// The blocks an instance of the block `key` holds by value, its
    /// base's included.
    fn held_blocks(&self, key: &str) -> Vec<String> {
        fn of(c: &Checker<'_>, t: &Ty, out: &mut Vec<String>, depth: usize) {
            if depth > 32 {
                return;
            }
            match t {
                Ty::Fb(n) => out.push(upper(n)),
                Ty::Array { elem, .. } => of(c, elem, out, depth + 1),
                Ty::Struct(k) => {
                    if let Some(fields) = c.struct_fields.get(k) {
                        for f in fields.values() {
                            of(c, f, out, depth + 1);
                        }
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        let Some(pou) = self.pous.get(key) else { return out };
        for b in pou.var_blocks.iter().filter(|b| matches!(b.kind, VarKind::Var | VarKind::Input | VarKind::Output)) {
            for d in &b.decls {
                if let Some(t) = self.out.decls.get(&d.span) {
                    of(self, t, &mut out, 0);
                }
            }
        }
        if let Some(base) = self.out.base(key) {
            out.push(base.to_string());
        }
        out
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
        let star_ok = std::mem::replace(&mut self.star_ok, false);
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
                    if matches!(p.kind, PouKind::FunctionBlock | PouKind::Class) {
                        return Ty::Fb(key);
                    }
                    if p.kind == PouKind::Interface {
                        return Ty::Itf(key);
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
                let len = match len {
                    Some(e) => match self.const_bound(e, report, "a STRING's length") {
                        Some(v) => u64::try_from(v).ok(),
                        None => return Ty::Unknown,
                    },
                    None => None,
                };
                Ty::Str { wide: name.is("WSTRING"), len }
            }
            TypeSpec::Array { dims, elem, span } => {
                let star = dims.iter().any(Option::is_none);
                if report && star && !star_ok {
                    self.error(
                        Code::VariableLengthArray,
                        *span,
                        "ARRAY[*] is for a FUNCTION's or method's input, output or in-out, or a FUNCTION_BLOCK's in-out (Ed.3 6.5.3)",
                    );
                }
                if report && star && dims.iter().any(Option::is_some) {
                    self.error(
                        Code::VariableLengthArray,
                        *span,
                        "a variable-length array has `*` for every dimension (Ed.3 Table 15): ARRAY[*, *]",
                    );
                }
                let mut bounds = Vec::new();
                let mut known = true;
                for d in dims {
                    let Some((lo, hi)) = d else {
                        bounds.push(None);
                        continue;
                    };
                    let (lo_v, hi_v) = (
                        self.const_bound(lo, report, "an array bound"),
                        self.const_bound(hi, report, "an array bound"),
                    );
                    let (Some(a), Some(b)) = (lo_v, hi_v) else {
                        known = false;
                        continue;
                    };
                    let dint = i128::from(i32::MIN)..=i128::from(i32::MAX);
                    if !dint.contains(&a) || !dint.contains(&b) {
                        if report {
                            self.error(
                                Code::OperandTypes,
                                if dint.contains(&a) { hi.span } else { lo.span },
                                "an array bound is within DINT, as in CODESYS (LOWER_BOUND and UPPER_BOUND give DINT)",
                            );
                        }
                        known = false;
                    } else if a > b {
                        if report {
                            self.error(
                                Code::OperandTypes,
                                lo.span,
                                format!("this array's lower bound {a} is above its upper bound {b}"),
                            );
                        }
                        known = false;
                    }
                    bounds.push(Some((a, b)));
                }
                let elem_spec = elem;
                let elem = self.resolve_spec(elem, report);
                if report && matches!(elem, Ty::Reference(_)) {
                    self.error(
                        Code::ReferenceNotKept,
                        elem_spec.span(),
                        "an array cannot hold REFERENCE TO (CODESYS, TwinCAT): use REF_TO",
                    );
                }
                if !known {
                    return Ty::Unknown;
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
            TypeSpec::Subrange { base, low, high, .. } => match Elem::from_name(&base.text) {
                Some(e) if e.is_int() => {
                    let lo = self.const_bound(low, report, "a subrange's limit");
                    let hi = self.const_bound(high, report, "a subrange's limit");
                    if lo.is_none() || hi.is_none() {
                        return Ty::Unknown;
                    }
                    Ty::Elem(e)
                }
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
            TypeSpec::Enum { values, base, span } => {
                // An enum declared in place: its values are known in this
                // POU under a name made of them. One with named values
                // (Ed.3 6.4.4.3) is its declaration's own: another with
                // the same names may give them other numbers.
                let name = crate::inplace_enum_name(base.as_ref(), values, *span);
                // The name is as long as the declaration: one copy of it,
                // shared by every value.
                let key: std::rc::Rc<str> = upper(&name).into();
                if !self.inplace.contains_key(&*key) {
                    self.inplace.insert(key.to_string(), std::rc::Rc::new((base.clone(), values.clone())));
                }
                if report {
                    self.check_enum(&key, values, base.as_ref());
                }
                // Once, however often the declaration is resolved.
                for v in values {
                    let of = self.enum_values.entry(upper(&v.name.text)).or_default();
                    if !of.iter().any(|k| **k == *key) {
                        of.push(key.clone());
                    }
                }
                Ty::Enum(key.to_string())
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
            TypeDef::Struct { overlap, fields, span } => {
                self.check_layout(*overlap, fields, *span);
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
            TypeDef::Spec(TypeSpec::Enum { values, base, .. }) => self.check_enum(&key, values, base.as_ref()),
            TypeDef::Spec(spec) => {
                self.resolve_spec(spec, true);
            }
        }
        if let Some(init) = &d.init {
            let t = if self.types.contains_key(&key) { self.named_type(&key) } else { Ty::Unknown };
            self.check_init(init, &t);
        }
    }

    /// A structure's components at relative locations (Ed.3 6.4.4.7):
    /// every one placed, or none; each where its type can lie, a BOOL
    /// alone at a bit; none sharing a bit with another unless the
    /// structure is OVERLAP, whose components have no initial value of
    /// their own (6.4.4.7.2). A field at a full address would put every
    /// variable of the type there.
    fn check_layout(&mut self, overlap: bool, fields: &[VarDecl], span: Span) {
        for f in fields {
            if let Some(l) = f.location.as_ref().filter(|l| !l.is_relative() && !l.is_partly()) {
                self.error(
                    Code::BadLocation,
                    l.span,
                    format!(
                        "a field of a structure type has no address of its own: every variable of the type would be at {}. \
                         Its place in the structure is a relative location, as %B2 or %X3.0 (Ed.3 6.4.4.7); \
                         %I*, %Q* or %M* leaves its address to VAR_CONFIG (Ed.3 6.4.4.8)",
                        l.text
                    ),
                );
            }
        }
        let placed = |f: &VarDecl| f.location.as_ref().is_some_and(|l| l.is_relative());
        if !fields.iter().any(placed) {
            if overlap {
                self.error(
                    Code::BadLayout,
                    span,
                    "STRUCT OVERLAP places its components: give each one AT and its byte, as `head AT %B0 : INT;`; \
                     components at the same byte share it (Ed.3 6.4.4.7)",
                );
            }
            return;
        }
        // Each component's bits, from and to, to find two that share one.
        let mut taken: Vec<(u64, u64, String, String)> = Vec::new();
        for f in fields {
            let name = f.names.first().map(|n| n.text.clone()).unwrap_or_default();
            let Some(l) = f.location.as_ref().filter(|l| l.is_relative()) else {
                if f.location.as_ref().is_none_or(|l| l.is_partly()) {
                    self.error(
                        Code::BadLayout,
                        f.span,
                        format!(
                            "'{name}' has no place in the structure: when one component has a relative location, \
                             each one does (Ed.3 6.4.4.7)"
                        ),
                    );
                }
                continue;
            };
            if overlap && let Some(init) = &f.init {
                self.error(
                    Code::BadLayout,
                    init_span(init),
                    format!(
                        "'{name}' is a component of a STRUCT OVERLAP: it has no initial value of its own, \
                         an overlapped structure is not initialised explicitly (Ed.3 6.4.4.7.2)"
                    ),
                );
            }
            let ty = self.resolve_spec(&f.ty, false);
            let Some((byte, bit)) = l.relative_place() else {
                self.error(
                    Code::BadLocation,
                    l.span,
                    format!(
                        "{} is not a relative location: %X, a byte and a bit from 0 to 7 (%X3.0), \
                         or %B, %W, %D or %L and a byte (%B2) (Ed.3 6.4.4.7)",
                        l.text
                    ),
                );
                continue;
            };
            let (from, to) = match bit {
                Some(bit) => {
                    if !matches!(ty, Ty::Elem(Elem::Bool) | Ty::Unknown) {
                        self.error(
                            Code::BadLocation,
                            l.span,
                            format!(
                                "{ty} at {}: only a BOOL is at a bit; a wider component starts at a byte, %B{byte}",
                                l.text
                            ),
                        );
                        continue;
                    }
                    let b = u64::from(byte) * 8 + u64::from(bit);
                    (b, b + 1)
                }
                None => {
                    if ty.is_unknown() {
                        continue;
                    }
                    let Some(n) = self.packed_size(&ty, 0) else {
                        let why = match &ty {
                            Ty::Fb(_) => "a function block instance has no address",
                            Ty::Ref(_) | Ty::Reference(_) | Ty::Itf(_) => "a reference is not a value bytes hold",
                            _ => {
                                "a component there is elementary, a string, an enumeration, a subrange, \
                                  or an array or structure of those"
                            }
                        };
                        self.error(
                            Code::BadLocation,
                            l.span,
                            format!("{ty} has no place at {}: {why} (Ed.3 6.4.4.7)", l.text),
                        );
                        continue;
                    };
                    if let (Ty::Elem(e), Some(size @ ('W' | 'D' | 'L'))) = (&ty, l.size())
                        && let Some(bits) = e.bits()
                    {
                        let named = match size {
                            'W' => 16,
                            'D' => 32,
                            _ => 64,
                        };
                        if bits.max(8) != named {
                            self.error(
                                Code::LocationSize,
                                l.span,
                                format!(
                                    "{ty} at {} takes {}: {ty} is {}, {size} names {}; %B{byte} names its byte alone",
                                    l.text,
                                    span_of_bytes(u64::from(byte), n),
                                    bytes(bits.max(8) / 8),
                                    bytes(named / 8)
                                ),
                            );
                        }
                    }
                    let b = u64::from(byte) * 8;
                    (b, b + n * 8)
                }
            };
            if to / 8 > u64::from(u32::MAX) {
                self.error(Code::BadLocation, l.span, format!("{ty} at {} ends past 4 GB", l.text));
                continue;
            }
            for n in &f.names {
                taken.push((from, to, n.text.clone(), l.text.clone()));
            }
        }
        if overlap {
            return;
        }
        let mut order: Vec<usize> = (0..taken.len()).collect();
        order.sort_by_key(|&i| (taken[i].0, i));
        let mut reach: Option<usize> = None;
        for i in order {
            if let Some(r) = reach
                && taken[r].1 > taken[i].0
            {
                let (a, b) = (&taken[r], &taken[i]);
                let message = format!(
                    "'{}' at {} shares {} with '{}' at {}: the components of a STRUCT each have their own, \
                     unless it is STRUCT OVERLAP (Ed.3 6.4.4.7)",
                    b.2,
                    b.3,
                    if (a.1 - a.0) == 1 || (b.1 - b.0) == 1 { "a bit" } else { "bytes" },
                    a.2,
                    a.3
                );
                let at = fields
                    .iter()
                    .find(|f| f.names.iter().any(|n| n.text == b.2))
                    .and_then(|f| f.location.as_ref())
                    .map_or(span, |l| l.span);
                self.error(Code::BadLayout, at, message);
                return;
            }
            if reach.is_none_or(|r| taken[i].1 > taken[r].1) {
                reach = Some(i);
            }
        }
    }

    /// Whether the structure type `key` places its components (Ed.3
    /// 6.4.4.7), and whether it is OVERLAP.
    pub(crate) fn layout_kind(&self, key: &str) -> Option<bool> {
        match self.types.get(key).map(|t| &t.def) {
            Some(TypeDef::Struct { overlap, fields, .. })
                if fields.iter().any(|f| f.location.as_ref().is_some_and(|l| l.is_relative())) =>
            {
                Some(*overlap)
            }
            _ => None,
        }
    }

    /// Its size in bytes laid out as in the process image: each part its
    /// type's width, back to back, a BOOL a byte, a STRING(n) n + 1, a
    /// structure with relative locations up to its last byte
    /// (docs/c-output.md §4). `None` for what has no place there.
    fn packed_size(&mut self, ty: &Ty, depth: usize) -> Option<u64> {
        if depth > 32 {
            return None;
        }
        match ty {
            Ty::Elem(e) => e.bits().map(|b| u64::from(b.max(8) / 8)),
            Ty::Str { wide, len } => {
                let n = len.unwrap_or(80) + 1;
                Some(if *wide { n * 2 } else { n })
            }
            Ty::Enum(_) => self.first_part(ty, depth).and_then(Elem::bits).map(|b| u64::from(b.max(8) / 8)),
            Ty::Array { elem, dims } => {
                let mut n = self.packed_size(elem, depth + 1)?;
                for d in dims {
                    let (lo, hi) = (*d)?;
                    n = n.checked_mul(u64::try_from(hi.checked_sub(lo)?.checked_add(1)?).ok()?)?;
                }
                Some(n)
            }
            Ty::Struct(k) => {
                let fields: Vec<&'a VarDecl> = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().collect(),
                    _ => return None,
                };
                let placed = self.layout_kind(k).is_some();
                let mut n = 0u64;
                for f in fields {
                    let t = self.resolve_spec(&f.ty, false);
                    let size = self.packed_size(&t, depth + 1)?;
                    let names = f.names.len() as u64;
                    match &f.location {
                        None if !placed => n = n.checked_add(size.checked_mul(names)?)?,
                        Some(l) if placed && l.is_relative() => {
                            let (byte, bit) = l.relative_place()?;
                            let end = u64::from(byte) + if bit.is_some() { 1 } else { size };
                            n = n.max(end);
                        }
                        // A field at an address of its own, or one with none
                        // among placed ones: already an error.
                        _ => return None,
                    }
                }
                Some(n)
            }
            _ => None,
        }
    }

    /// An enumeration's declaration, a TYPE's or one in place: each value
    /// once, a number given a constant its base type holds.
    fn check_enum(&mut self, key: &str, values: &[EnumValue], base: Option<&Ident>) {
        {
            {
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
                        } else if !t.is_unknown() {
                            self.const_bound(e, true, "a named value");
                        }
                    }
                }
                // Each value is one its base type holds.
                if let Some(base) = self.named_values(key) {
                    for v in values {
                        if let Some(n) = self.named_value(key, &v.name.text) {
                            self.coerce(&Ty::IntLit(Some(n)), &Ty::Elem(base), v.name.span);
                        }
                    }
                } else if let Some(b) = base {
                    self.error(
                        Code::TypeMismatch,
                        b.span,
                        format!("named values are of an integer or bit string type, not '{}'", b.text),
                    );
                } else {
                    self.enum_fits(values);
                }
            }
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
            // A derived block's inherited inputs, outputs and in-outs
            // first, as its structure has them.
            let mut out = match self.bases.get(key).cloned() {
                Some(b) => self.params(&b),
                None => Vec::new(),
            };
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
    /// The type of `key`'s own variable `member`.
    fn var_type(&mut self, key: &str, member: &str) -> Option<Ty> {
        let d = std::iter::once(key.to_string()).chain(self.ancestors(key)).find_map(|k| {
            let p = self.pous.get(&k).copied()?;
            p.var_blocks.iter().flat_map(|b| &b.decls).find(|d| d.names.iter().any(|n| n.is(member)))
        })?;
        Some(self.resolve_spec(&d.ty, false))
    }

    fn is_internal_member(&self, fb: &str, member: &str) -> bool {
        std::iter::once(fb.to_string()).chain(self.ancestors(fb)).any(|k| {
            self.pous.get(&k).is_some_and(|p| {
                p.var_blocks.iter().any(|b| {
                    matches!(b.kind, VarKind::Var | VarKind::Temp | VarKind::External)
                        && b.decls.iter().any(|d| d.names.iter().any(|n| n.is(member)))
                })
            })
        })
    }

    fn check_pou(&mut self, pou: &'a Pou) {
        // VAR_ACCESS stands in a CONFIGURATION, and in a PROGRAM for its
        // own variables (Ed.3 Table 47, 9).
        if pou.kind == PouKind::Program {
            let key = upper(&pou.name.text);
            self.check_accesses(&pou.accesses, None, Some(&key));
        } else if let Some(a) = pou.accesses.first() {
            self.error(
                Code::UnexpectedToken,
                a.span,
                "VAR_ACCESS stands in a CONFIGURATION, or in a PROGRAM for its own variables (Ed.3 6.8.1, Table 47)",
            );
        }
        if pou.kind == PouKind::Interface {
            self.check_interface(pou);
            return;
        }
        self.locals.clear();
        self.own_vars.clear();
        self.hidden.clear();
        self.this_type = matches!(pou.kind, PouKind::FunctionBlock | PouKind::Class).then(|| upper(&pou.name.text));
        let key = upper(&pou.name.text);
        // A derived type's body sees what it inherits as its own (Ed.3
        // 6.6.5.5.2 rule 2).
        self.inherited.clear();
        if self.this_type.is_some() {
            for (b, d, n) in self.inherited_vars(&key) {
                let k = upper(&n.text);
                let v = self.var_of(b, d, &k);
                self.inherited.insert(k.clone());
                self.locals.insert(k, v);
            }
        }
        self.super_body = None;
        self.check_pou_in(pou, key);
        self.inherited.clear();
        if matches!(pou.kind, PouKind::FunctionBlock | PouKind::Class) {
            self.check_type_specs(pou);
            self.check_methods(pou);
        }
        self.this_type = None;
        self.consts.clear();
    }

    /// A type's variable as its body and methods see it.
    fn var_of(&mut self, b: &VarBlock, d: &VarDecl, key: &str) -> Var {
        let ty = self.resolve_spec(&d.ty, false);
        let mut v = Var::of(ty, b.constant);
        v.input = if b.kind == VarKind::External {
            self.globals.get(key).is_some_and(|(g, _)| at_input(g))
        } else {
            at_input(d)
        };
        v
    }

    /// The POU's declarations and bodies, `self.locals` holding what it
    /// sees before its own declarations (a method: its type's variables).
    /// A method is checked as a FUNCTION is: its variables temporary
    /// (Ed.3 6.6.5.4.3 rule 9), its result its name's.
    fn check_pou_in(&mut self, pou: &'a Pou, key: String) {
        self.steps.clear();
        self.actions.clear();
        let fn_like = matches!(pou.kind, PouKind::Function | PouKind::Method);
        self.pou_kind = Some(if pou.kind == PouKind::Method { PouKind::Function } else { pou.kind });
        self.pou_key = key;
        self.pou_consts(pou);
        self.check_specs(pou);
        let name = upper(&pou.name.text);
        if pou.kind == PouKind::Method && self.locals.contains_key(&name) {
            self.error(
                Code::Duplicate,
                pou.name.span,
                format!("method '{}' has the name of a variable of its type (Ed.3 6.6.5.5.5, error 2)", pou.name.text),
            );
        }
        if let Some(rt) = &pou.return_type {
            let t = self.resolve_spec(rt, true);
            self.out.decls.insert(rt.span(), t.clone());
            self.locals.insert(name, Var { temp: fn_like, ..Var::of(t, false) });
        } else if fn_like {
            self.locals.insert(name, Var { temp: true, ..Var::of(Ty::Void, true) });
        }
        for b in &pou.var_blocks {
            self.check_section(pou, b);
            self.declare_block(b, self.pou_kind);
        }
        if pou.kind == PouKind::FunctionBlock {
            self.own_vars = self.locals.keys().filter(|k| *k != "ENO").cloned().collect();
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
        // SFC structures a PROGRAM or a FUNCTION_BLOCK (Ed.3 6.7.1): a
        // FUNCTION or a method keeps no state for its steps.
        if !pou.sfc.is_empty() && matches!(pou.kind, PouKind::Function | PouKind::Method) {
            let what = if pou.kind == PouKind::Function { "FUNCTION" } else { "METHOD" };
            self.error(
                Code::UnexpectedToken,
                pou.name.span,
                format!(
                    "a {what} has no SFC: steps keep their state from one call to the next, which only a PROGRAM or a \
                     FUNCTION_BLOCK does (Ed.3 6.7.1)"
                ),
            );
            return;
        }
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
                        // Indicator variables (Ed.3 6.7.4.2, Table 58 "c",
                        // deprecated): BOOL variables the action may set;
                        // they do nothing of their own.
                        for ind in &a.indicators {
                            match self.variable(&upper(&ind.text)).map(|v| v.ty.clone()) {
                                None => self.error(
                                    Code::UnknownName,
                                    ind.span,
                                    format!("no variable '{}' for this indicator", ind.text),
                                ),
                                Some(t) if !t.is_bool() && !t.is_unknown() => self.error(
                                    Code::TypeMismatch,
                                    ind.span,
                                    format!("an indicator variable is a BOOL, and '{}' is {t}", ind.text),
                                ),
                                Some(_) => self.error(
                                    Code::IndicatorDeprecated,
                                    ind.span,
                                    format!(
                                        "'{}' is an indicator variable, deprecated since Ed.3: it does nothing of its \
                                         own, so name the action alone",
                                        ind.text
                                    ),
                                ),
                            }
                        }
                        if let Some(t) = &a.time {
                            let ty = self.expr(t);
                            self.coerce(&ty, &Ty::Elem(Elem::Time), t.span);
                            // Ed.3 Annex A, Action_Time: `Duration |
                            // Variable_Name` - a literal or a variable,
                            // one of an instance or a structure too.
                            fn variable(e: &Expr) -> bool {
                                match &e.kind {
                                    ExprKind::Name(_) => true,
                                    ExprKind::Member(b, _) | ExprKind::Paren(b) => variable(b),
                                    _ => false,
                                }
                            }
                            let literal = matches!(&t.kind, ExprKind::Literal(_))
                                || matches!(&t.kind, ExprKind::Unary(UnaryOp::Neg, x) if matches!(x.kind, ExprKind::Literal(_)));
                            if !literal && !variable(t) {
                                self.error(
                                    Code::UnexpectedToken,
                                    t.span,
                                    "an action's time is a duration literal or a variable (Ed.3 Annex A, Action_Time): \
                                     assign the expression to a TIME variable and name that",
                                );
                            }
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

    /// The methods of `owner`, each seeing its type's variables but those
    /// 6.6.7.2.3 keeps from it.
    fn check_methods(&mut self, owner: &'a Pou) {
        let okey = upper(&owner.name.text);
        let mut seen: HashMap<String, Var> = HashMap::new();
        let mut hidden: HashMap<String, &'static str> = HashMap::new();
        let own = owner
            .var_blocks
            .iter()
            .flat_map(|b| b.decls.iter().flat_map(move |d| d.names.iter().map(move |n| (b, d, n))));
        let all: Vec<(&VarBlock, &VarDecl, &Ident)> = self.inherited_vars(&okey).into_iter().chain(own).collect();
        for (b, d, n) in all {
            let k = upper(&n.text);
            let why = match b.kind {
                VarKind::Temp => Some("a VAR_TEMP"),
                VarKind::InOut => Some("a VAR_IN_OUT"),
                _ if d.edge.is_some() => Some("an R_EDGE or F_EDGE input"),
                _ => None,
            };
            match why {
                Some(w) => {
                    hidden.insert(k, w);
                }
                None => {
                    let v = self.var_of(b, d, &k);
                    seen.insert(k, v);
                }
            }
        }
        for m in &owner.methods {
            if m.name.text.is_empty()
                || !self.pous.get(&crate::method_key(&okey, &m.name.text)).is_some_and(|p| std::ptr::eq(*p, m))
            {
                continue;
            }
            self.locals = seen.clone();
            self.own_vars = seen.keys().cloned().collect();
            self.hidden = hidden.clone();
            self.this_type = Some(okey.clone());
            self.check_method_specs(owner, &okey, m);
            self.check_pou_in(m, crate::method_key(&okey, &m.name.text));
        }
        self.own_vars.clear();
        self.hidden.clear();
    }

    /// A method's OVERRIDE, FINAL and ABSTRACT against its type and the
    /// method it overrides (Ed.3 6.6.5.5.3, 6.6.5.8.3; OO.md D6, D12).
    fn check_method_specs(&mut self, owner: &Pou, okey: &str, m: &'a Pou) {
        let name = &m.name.text;
        let base = self.base_method(okey, name);
        match (m.specs.override_, &base) {
            (Some(o), None) => self.error(
                Code::BadObject,
                o,
                format!(
                    "OVERRIDE, but {} has no base type with a method '{name}' to override (Ed.3 6.6.5.5.5, error 4)",
                    owner.name.text
                ),
            ),
            (None, Some((b, _))) => self.error(
                Code::BadObject,
                m.name.span,
                format!(
                    "{b} has a method '{name}' already: write METHOD OVERRIDE {name} to replace it (Ed.3 6.6.5.5.3; OO.md D6)"
                ),
            ),
            (Some(_), Some((b, bm))) => {
                let bkey = crate::method_key(b, name);
                let mkey = crate::method_key(okey, name);
                if bm.specs.final_.is_some() {
                    self.error(
                        Code::BadObject,
                        m.name.span,
                        format!("{b}.{name} is FINAL: it is not overridden (Ed.3 6.6.5.5.4)"),
                    );
                }
                let access = |p: &Pou| p.specs.access.map_or(Access::Protected, |(a, _)| a);
                if access(bm) != access(m) {
                    self.error(
                        Code::BadObject,
                        m.name.span,
                        format!(
                            "{b}.{name} is {}, so its override is too (Ed.3 6.6.5.5.3 rule 2)",
                            access(bm).name()
                        ),
                    );
                }
                let ret = |c: &mut Self, p: &Pou| p.return_type.as_ref().map(|t| c.resolve_spec(t, false));
                let (rb, rm) = (ret(self, bm), ret(self, m));
                let sig = |ps: Vec<Param>| ps.into_iter().map(|p| (p.name, p.dir, p.ty)).collect::<Vec<_>>();
                if rb != rm || sig(self.params(&bkey)) != sig(self.params(&mkey)) {
                    self.error(
                        Code::BadObject,
                        m.name.span,
                        format!(
                            "an override has its base method's signature: the result and each input, output and in-out of {b}.{name}, by name, type and order (Ed.3 6.6.5.5.3 rule 1)"
                        ),
                    );
                }
            }
            (None, None) => {}
        }
        if let Some(a) = m.specs.abstract_ {
            if owner.specs.abstract_.is_none() {
                self.error(
                    Code::BadObject,
                    a,
                    format!(
                        "an ABSTRACT method is for an ABSTRACT type; {} is not one (Ed.3 6.6.5.8.3)",
                        owner.name.text
                    ),
                );
            }
            if m.specs.override_.is_some() {
                self.error(Code::BadObject, a, "ABSTRACT and OVERRIDE together (Ed.3 6.6.5.8.3)");
            }
            if let Some(s) = m.body.first() {
                self.error(
                    Code::BadObject,
                    s.span,
                    "an ABSTRACT method has no statements: the types derived from its type implement it (OO.md D12)",
                );
            }
        }
    }

    /// An ABSTRACT type has an abstract method to implement; any other
    /// implements every one it inherits (Ed.3 6.6.5.8.2).
    fn check_type_specs(&mut self, pou: &Pou) {
        let key = upper(&pou.name.text);
        let mut open = Vec::new();
        let mut names = std::collections::BTreeSet::new();
        for k in std::iter::once(key.clone()).chain(self.ancestors(&key)) {
            for m in &self.pous[&k].methods {
                names.insert(upper(&m.name.text));
            }
        }
        for n in names {
            // One of its own is reported where it is (check_method_specs).
            if let Some((owner, m)) = self.find_method(&key, &n)
                && m.specs.abstract_.is_some()
                && (owner != key || pou.specs.abstract_.is_some())
            {
                open.push(format!("{owner}.{}", m.name.text));
            }
        }
        open.extend(self.check_implements(pou, &key));
        match (pou.specs.abstract_, open.is_empty()) {
            (Some(a), true) => self.error(
                Code::BadObject,
                a,
                format!(
                    "{} is ABSTRACT, so it holds an ABSTRACT method (Ed.3 6.6.5.8.2): without one, leave ABSTRACT out",
                    pou.name.text
                ),
            ),
            (None, false) => self.error(
                Code::BadObject,
                pou.name.span,
                format!(
                    "{} implements every ABSTRACT method it inherits and every method of its interfaces, or is ABSTRACT itself (Ed.3 6.6.5.8.2, 6.6.6.4.2): not {}",
                    pou.name.text,
                    open.join(", ")
                ),
            ),
            _ => {}
        }
    }

    /// FINAL and ABSTRACT, where they stand (Ed.3 Tables 48 and 53).
    fn check_specs(&mut self, pou: &Pou) {
        if let (Some(f), Some(_)) = (pou.specs.final_, pou.specs.abstract_) {
            self.error(
                Code::BadObject,
                f,
                "FINAL and ABSTRACT together: an abstract type or method is there to be derived from",
            );
        }
    }

    /// A section where it may stand: a CLASS has only VAR and VAR_EXTERNAL
    /// (Ed.3 6.6.5.2), and only a CLASS's or FUNCTION_BLOCK's VAR has an
    /// access specifier (6.6.5.10, 6.6.7.2.6).
    fn check_section(&mut self, pou: &Pou, b: &VarBlock) {
        // VAR_GLOBAL stands in a CONFIGURATION, a RESOURCE or a PROGRAM
        // (Ed.3 6.5.2.2, Table 47): the grammar of the others has none.
        if b.kind == VarKind::Global && matches!(pou.kind, PouKind::FunctionBlock | PouKind::Function | PouKind::Method)
        {
            let what = match pou.kind {
                PouKind::FunctionBlock => "FUNCTION_BLOCK",
                PouKind::Function => "FUNCTION",
                _ => "METHOD",
            };
            self.error(
                Code::UnexpectedToken,
                b.span,
                format!(
                    "a {what} declares no VAR_GLOBAL: a CONFIGURATION, a RESOURCE or a PROGRAM does (Ed.3 6.5.2.2), \
                     and a {what} reaches it by VAR_EXTERNAL; a variable of its own is a VAR"
                ),
            );
        }
        // A FUNCTION_BLOCK's input or output is a parameter, given at
        // each call: its declaration has no address (Ed.3 Annex A,
        // FB_Input_Decl, FB_Output_Decl).
        if pou.kind == PouKind::FunctionBlock && matches!(b.kind, VarKind::Input | VarKind::Output | VarKind::InOut) {
            // A `*` address there has its own message (`check_location`).
            for d in b.decls.iter().filter(|d| d.location.as_ref().is_some_and(|l| !l.is_partly())) {
                let what = d.names.first().map_or("this one", |n| n.text.as_str());
                self.error(
                    Code::BadLocation,
                    d.span,
                    format!(
                        "'{what}' is an input or output of a FUNCTION_BLOCK, which has no address (Ed.3 Annex A, \
                         FB_Input_Decl): declare the address in a VAR of its own, or in the PROGRAM, and pass it"
                    ),
                );
            }
        }
        if pou.kind == PouKind::Class && !matches!(b.kind, VarKind::Var | VarKind::External) {
            self.error(
                Code::BadObject,
                b.span,
                "a CLASS declares only VAR and VAR_EXTERNAL (Ed.3 6.6.5.2): its methods have the inputs and outputs",
            );
        }
        if let Some((a, at)) = b.access
            && !(matches!(pou.kind, PouKind::Class | PouKind::FunctionBlock) && b.kind == VarKind::Var)
        {
            self.error(
                Code::BadObject,
                at,
                format!(
                    "{} is for a CLASS's or FUNCTION_BLOCK's VAR section (Ed.3 6.6.5.10, 6.6.7.2.6): {}",
                    a.name(),
                    match b.kind {
                        VarKind::Input | VarKind::Output => "inputs and outputs are always PUBLIC",
                        VarKind::External => "a VAR_EXTERNAL is always PROTECTED",
                        _ if pou.kind == PouKind::Method => "a method's variables live only in its call",
                        _ => "here it has no meaning",
                    }
                ),
            );
        }
    }

    /// Each EXTENDS: a base that is there, of a kind that may be one, not
    /// FINAL, and no type its own base (Ed.3 6.6.5.5.5 error 3). Only
    /// the bases that pass are kept, so a walk up them ends.
    fn check_bases(&mut self) {
        let mut keys: Vec<String> = self.pous.keys().cloned().collect();
        keys.sort();
        for key in &keys {
            let p = self.pous[key];
            let Some(b) = &p.extends else { continue };
            if !matches!(p.kind, PouKind::FunctionBlock | PouKind::Class) {
                continue;
            }
            let bkey = upper(&b.text);
            let Some(base) = self.pous.get(&bkey).copied() else {
                let msg = if std_fb(&bkey).is_some() {
                    format!(
                        "'{}' is a standard function block: a type EXTENDS a CLASS or FUNCTION_BLOCK of the project",
                        b.text
                    )
                } else {
                    format!("there is no CLASS or FUNCTION_BLOCK '{}' to extend", b.text)
                };
                self.error(Code::UnknownType, b.span, msg);
                continue;
            };
            let why = match (p.kind, base.kind) {
                (PouKind::Class, PouKind::FunctionBlock) => {
                    Some("a CLASS extends a CLASS, not a FUNCTION_BLOCK (Ed.3 Table 48 row 6)".to_string())
                }
                (_, PouKind::Class | PouKind::FunctionBlock) => None,
                _ => Some(format!("'{}' is not a CLASS or FUNCTION_BLOCK", b.text)),
            };
            if let Some(why) = why {
                self.error(Code::BadObject, b.span, why);
                continue;
            }
            if base.specs.final_.is_some() {
                self.error(
                    Code::BadObject,
                    b.span,
                    format!("{} is FINAL: it is not a base type (Ed.3 6.6.5.5.4)", base.name.text),
                );
                continue;
            }
            self.bases.insert(key.clone(), bkey);
        }
        for key in &keys {
            let mut at = key.clone();
            let mut seen = std::collections::HashSet::new();
            while let Some(b) = self.bases.get(&at).cloned() {
                if !seen.insert(at.clone()) {
                    break;
                }
                if b == *key {
                    let p = self.pous[key];
                    let span = p.extends.as_ref().map_or(p.name.span, |e| e.span);
                    self.error(
                        Code::BadObject,
                        span,
                        format!("{} is its own base, through EXTENDS (Ed.3 6.6.5.5.5 error 3)", p.name.text),
                    );
                    self.bases.remove(key);
                    break;
                }
                at = b;
            }
        }
        self.out.bases = self.bases.clone();
    }

    /// Each INTERFACE's EXTENDS and each CLASS's and FUNCTION_BLOCK's
    /// IMPLEMENTS: interfaces that are there, and no interface its own
    /// base (Ed.3 6.6.6.3, 6.6.6.4). Only the names that pass are kept.
    fn check_interface_names(&mut self) {
        let mut keys: Vec<String> = self.pous.keys().cloned().collect();
        keys.sort();
        for key in &keys {
            let p = self.pous[key];
            if !matches!(p.kind, PouKind::Interface | PouKind::FunctionBlock | PouKind::Class) {
                continue;
            }
            let mut kept: Vec<String> = Vec::new();
            for i in &p.interfaces {
                let ikey = upper(&i.text);
                match self.pous.get(&ikey).map(|q| q.kind) {
                    Some(PouKind::Interface) => {}
                    Some(k) => {
                        let what = if p.kind == PouKind::Interface { "EXTENDS" } else { "IMPLEMENTS" };
                        self.error(
                            Code::BadObject,
                            i.span,
                            format!(
                                "'{}' is a {}: {what} names interfaces (Ed.3 6.6.6.3, 6.6.6.4)",
                                i.text,
                                k.keyword()
                            ),
                        );
                        continue;
                    }
                    None => {
                        self.error(Code::UnknownType, i.span, format!("there is no INTERFACE '{}'", i.text));
                        continue;
                    }
                }
                if kept.contains(&ikey) {
                    self.error(Code::Duplicate, i.span, format!("'{}' is named twice", i.text));
                    continue;
                }
                kept.push(ikey);
            }
            if p.kind == PouKind::Interface {
                self.out.itf_bases.insert(key.clone(), kept);
            } else if !kept.is_empty() {
                self.out.implements.insert(key.clone(), kept);
            }
        }
        for key in &keys {
            let p = self.pous[key];
            if p.kind != PouKind::Interface {
                continue;
            }
            let bases = self.out.itf_bases.get(key).cloned().unwrap_or_default();
            if bases.iter().any(|b| self.out.itf_line(b).contains(key)) {
                self.error(
                    Code::BadObject,
                    p.interfaces.first().map_or(p.name.span, |i| i.span),
                    format!("{} is its own base, through EXTENDS (Ed.3 6.6.6.3)", p.name.text),
                );
                self.out.itf_bases.insert(key.clone(), Vec::new());
            }
        }
    }

    /// The prototype `name` (upper-cased) of the interface `i`: its own
    /// or a base's, and the interface that declares it.
    fn itf_method(&self, i: &str, name: &str) -> Option<(String, &'a Pou)> {
        self.out.itf_line(i).into_iter().find_map(|k| self.pous.get(&crate::method_key(&k, name)).map(|m| (k, *m)))
    }

    /// An INTERFACE: no variables, prototypes only (a name, a result, its
    /// inputs, outputs and in-outs; no body, always PUBLIC), none a base
    /// already has (Ed.3 6.6.6.1 to 6.6.6.3).
    fn check_interface(&mut self, pou: &'a Pou) {
        let key = upper(&pou.name.text);
        for b in &pou.var_blocks {
            self.error(
                Code::BadObject,
                b.span,
                "an INTERFACE declares no variables: only its methods' prototypes (Ed.3 6.6.6.1)",
            );
        }
        let bases: Vec<String> = self.out.itf_line(&key).into_iter().skip(1).collect();
        // Two bases with a prototype of one name each.
        let mut from: HashMap<String, String> = HashMap::new();
        for b in &bases {
            for m in self.pous[b].methods.iter().filter(|m| !m.name.text.is_empty()) {
                let n = upper(&m.name.text);
                match from.get(&n) {
                    Some(other)
                        if other != b
                            && !self.out.itf_line(other).contains(b)
                            && !self.out.itf_line(b).contains(other) =>
                    {
                        self.error(
                            Code::Duplicate,
                            pou.name.span,
                            format!(
                                "{other} and {b} both declare a method '{}': {} would have two (Ed.3 6.6.6.3)",
                                m.name.text, pou.name.text
                            ),
                        );
                    }
                    Some(_) => {}
                    None => {
                        from.insert(n, b.clone());
                    }
                }
            }
        }
        for m in &pou.methods {
            if m.name.text.is_empty()
                || !self.pous.get(&crate::method_key(&key, &m.name.text)).is_some_and(|p| std::ptr::eq(*p, m))
            {
                continue;
            }
            if let Some(b) = from.get(&upper(&m.name.text)) {
                self.error(
                    Code::Duplicate,
                    m.name.span,
                    format!(
                        "{b} declares the method '{}' already: an interface does not declare it again (Ed.3 6.6.6.3)",
                        m.name.text
                    ),
                );
            }
            let written = [m.specs.access.map(|a| a.1), m.specs.final_, m.specs.abstract_, m.specs.override_];
            if let Some(at) = written.into_iter().flatten().next() {
                self.error(
                    Code::BadObject,
                    at,
                    "a method of an INTERFACE is a prototype, always PUBLIC: no access specifier, FINAL, ABSTRACT or OVERRIDE (Ed.3 6.6.6.1)",
                );
            }
            if let Some(st) = m.body.first() {
                self.error(
                    Code::BadObject,
                    st.span,
                    "a method of an INTERFACE has no statements: the types that implement it do (Ed.3 6.6.6.1)",
                );
                continue;
            }
            for b in
                m.var_blocks.iter().filter(|b| !matches!(b.kind, VarKind::Input | VarKind::Output | VarKind::InOut))
            {
                self.error(
                    Code::BadObject,
                    b.span,
                    "a prototype declares its inputs, outputs and in-outs only (Ed.3 6.6.6.1)",
                );
            }
            self.locals.clear();
            self.own_vars.clear();
            self.hidden.clear();
            self.this_type = None;
            let mkey = crate::method_key(&key, &m.name.text);
            self.check_pou_in(m, mkey);
        }
    }

    /// Each method of each interface `key` implements (Ed.3 6.6.6.4.2):
    /// its own or inherited, with the prototype's signature, PUBLIC or
    /// INTERNAL (OO.md D10). The prototypes it leaves out, for
    /// check_type_specs: it is ABSTRACT, or reported.
    fn check_implements(&mut self, pou: &Pou, key: &str) -> Vec<String> {
        let mut open = Vec::new();
        for i in self.out.implemented(key) {
            let protos: Vec<&'a Pou> = self.pous[&i].methods.iter().filter(|m| !m.name.text.is_empty()).collect();
            for proto in protos {
                let n = upper(&proto.name.text);
                let Some((owner, m)) = self.find_method(key, &n) else {
                    open.push(format!("{i}.{}", proto.name.text));
                    continue;
                };
                // A base that implements the interface was checked as it.
                if owner != key && self.out.implemented(&owner).contains(&i) {
                    continue;
                }
                let at = if owner == key { m.name.span } else { pou.name.span };
                let access = m.specs.access.map_or(Access::Protected, |(a, _)| a);
                if !matches!(access, Access::Public | Access::Internal) {
                    self.error(
                        Code::BadObject,
                        at,
                        format!(
                            "{owner}.{} implements {i}.{}, so it is PUBLIC or INTERNAL, not {} (Ed.3 6.6.6.4.2; OO.md D10): write METHOD PUBLIC {}",
                            m.name.text,
                            proto.name.text,
                            access.name(),
                            m.name.text
                        ),
                    );
                }
                let ret = |c: &mut Self, p: &Pou| p.return_type.as_ref().map(|t| c.resolve_spec(t, false));
                let (rp, rm) = (ret(self, proto), ret(self, m));
                let sig = |ps: Vec<Param>| ps.into_iter().map(|p| (p.name, p.dir, p.ty)).collect::<Vec<_>>();
                let pk = crate::method_key(&i, &n);
                let mk = crate::method_key(&owner, &n);
                if rp != rm || sig(self.params(&pk)) != sig(self.params(&mk)) {
                    self.error(
                        Code::BadObject,
                        at,
                        format!(
                            "{owner}.{} implements {i}.{} with its signature: the result and each input, output and in-out, by name, type and order (Ed.3 6.6.6.4.2)",
                            m.name.text, proto.name.text
                        ),
                    );
                }
            }
        }
        open
    }

    /// `a ?= b` (Ed.3 6.6.6.7): an interface from an interface, a
    /// reference to a type from an interface, or a reference to a type
    /// from a reference to its base or a type derived from it; `a` gets
    /// NULL when what `b` references is not of a type it can hold.
    fn assign_attempt(&mut self, target: &Expr, value: &Expr, span: Span) {
        let want = self.place(target);
        let got = self.expr(value);
        let block = |c: &Self, t: &Ty| match t {
            Ty::Ref(to) => match &**to {
                Ty::Fb(k)
                    if c.pous.get(k).is_some_and(|p| matches!(p.kind, PouKind::Class | PouKind::FunctionBlock)) =>
                {
                    Some(k.clone())
                }
                _ => None,
            },
            _ => None,
        };
        let ok = match (&want, &got) {
            (Ty::Unknown, _) | (_, Ty::Unknown) => true,
            (Ty::Itf(_) | Ty::Ref(_), Ty::Itf(_) | Ty::Null)
                if !matches!(want, Ty::Ref(_)) || block(self, &want).is_some() =>
            {
                true
            }
            (Ty::Ref(_), Ty::Ref(_)) => match (block(self, &want), block(self, &got)) {
                (Some(d), Some(b)) => self.derives(&d, &b) || self.derives(&b, &d),
                _ => false,
            },
            _ => false,
        };
        if !ok {
            self.error(
                Code::BadObject,
                span,
                format!(
                    "?= takes an interface, or a reference to a CLASS or FUNCTION_BLOCK, from an interface, or from a reference to one of its line (Ed.3 6.6.6.7): not {got} into {want}"
                ),
            );
        }
    }

    /// `key`'s bases, the nearest first.
    fn ancestors(&self, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut at = key;
        while let Some(b) = self.bases.get(at) {
            out.push(b.clone());
            at = b;
        }
        out
    }

    /// Whether `derived` is `base` or derives from it.
    fn derives(&self, derived: &str, base: &str) -> bool {
        derived == base || self.ancestors(derived).iter().any(|a| a == base)
    }

    /// Whether some type extends `key`.
    fn has_derived(&self, key: &str) -> bool {
        self.bases.values().any(|b| b == key)
    }

    /// The method `name` of `ty`: its own, or the nearest base's, and the
    /// type that declares it.
    fn find_method(&self, ty: &str, name: &str) -> Option<(String, &'a Pou)> {
        std::iter::once(ty.to_string())
            .chain(self.ancestors(ty))
            .find_map(|k| self.pous.get(&crate::method_key(&k, name)).map(|m| (k, *m)))
    }

    /// The method `name` that `owner` inherits: its nearest base's, but
    /// not a PRIVATE one (Ed.3 6.6.5.5.2 rule 1).
    fn base_method(&self, owner: &str, name: &str) -> Option<(String, &'a Pou)> {
        let b = self.bases.get(owner)?;
        self.find_method(b, name).filter(|(_, m)| !matches!(m.specs.access, Some((Access::Private, _))))
    }

    fn is_abstract(&self, key: &str) -> bool {
        self.pous.get(key).is_some_and(|p| p.specs.abstract_.is_some())
    }

    /// The variables `key` inherits from its bases (Ed.3 6.6.5.5.2 rule
    /// 2): all but PRIVATE ones and VAR_TEMPs, which are a body's.
    fn inherited_vars(&self, key: &str) -> Vec<(&'a VarBlock, &'a VarDecl, &'a Ident)> {
        let mut out = Vec::new();
        for a in self.ancestors(key) {
            let p = self.pous[&a];
            for b in &p.var_blocks {
                if b.kind == VarKind::Temp || matches!(b.access, Some((Access::Private, _))) {
                    continue;
                }
                for d in &b.decls {
                    for n in &d.names {
                        out.push((b, d, n));
                    }
                }
            }
        }
        out
    }

    /// Whether instances of `key` are objects: a CLASS, a block with
    /// methods, a base or a derived type. One is not copied, nor an
    /// input, output or a variable of a FUNCTION or method (OO.md D11).
    fn is_object(&self, key: &str) -> bool {
        self.pous.get(key).is_some_and(|p| p.kind == PouKind::Class || !p.methods.is_empty())
            || self.bases.contains_key(key)
            || self.has_derived(key)
            || !self.out.implemented(key).is_empty()
    }

    /// The object (`is_object`) a value of `ty` is or holds, in an array
    /// or a structure as well: copied with it, so refused as it is (OO.md
    /// D11). `true` when it is held, not `ty` itself.
    fn object_in(&self, ty: &Ty) -> Option<(String, bool)> {
        fn find(c: &Checker, ty: &Ty, depth: u32) -> Option<String> {
            match ty {
                _ if depth > 64 => None,
                Ty::Fb(t) if c.is_object(t) => Some(t.clone()),
                Ty::Array { elem, .. } => find(c, elem, depth + 1),
                Ty::Struct(s) => c.struct_fields.get(s)?.values().find_map(|f| find(c, f, depth + 1)),
                _ => None,
            }
        }
        find(self, ty, 0).map(|t| (t, !matches!(ty, Ty::Fb(_))))
    }

    /// Whether a derived instance of `derived` may stand where a `base`
    /// is wanted, through an in-out or a reference: it derives from it
    /// and adds no in-out, which the base's call would leave unbound
    /// (OO.md D7). The error said at `span`.
    fn may_stand_for(&mut self, derived: &str, base: &str, span: Span) -> bool {
        if !self.derives(derived, base) {
            return false;
        }
        if derived != base {
            let own: Vec<String> =
                self.params(base).into_iter().filter(|p| p.dir == Dir::InOut).map(|p| p.name).collect();
            if let Some(p) = self.params(derived).into_iter().find(|p| p.dir == Dir::InOut && !own.contains(&p.name)) {
                self.error(
                    Code::BadObject,
                    span,
                    format!(
                        "{derived} adds the in-out {}, which a call through a {base} would leave unbound (Ed.3 6.6.8.3; OO.md D7)",
                        p.name
                    ),
                );
            }
        }
        true
    }

    /// The access of `key`'s variable `member`, when it is one of its VAR
    /// sections': PROTECTED when none is written.
    /// The type that declares it too: `key` or a base.
    fn var_access(&self, key: &str, member: &str) -> Option<(Access, bool, String)> {
        std::iter::once(key.to_string()).chain(self.ancestors(key)).find_map(|k| {
            let p = self.pous.get(&k)?;
            p.var_blocks.iter().find_map(|b| {
                (b.kind == VarKind::Var && b.decls.iter().any(|d| d.names.iter().any(|n| n.is(member))))
                    .then(|| b.access.map_or((Access::Protected, false), |(a, _)| (a, true)))
                    .map(|(a, w)| (a, w, k.clone()))
            })
        })
    }

    /// The namespace part of a POU's or type's key: `A__B` of `A__B__C`.
    fn namespace_of(key: &str) -> &str {
        key.rsplit_once("__").map_or("", |(ns, _)| ns)
    }

    /// The namespace the code being checked is in.
    fn here_ns(&self) -> String {
        let k = self.this_type.clone().unwrap_or_else(|| self.pou_key.clone());
        Self::namespace_of(&k).to_string()
    }

    /// Whether the code being checked may use what `access` keeps in
    /// `owner`: in `owner` itself anything; PROTECTED in the types
    /// derived from it too (Ed.3 6.6.5.9).
    fn may_use(&self, owner: &str, access: Access) -> bool {
        self.this_type.as_deref() == Some(owner)
            || match access {
                Access::Public => true,
                Access::Internal => self.here_ns() == Self::namespace_of(owner),
                Access::Protected => self.this_type.as_deref().is_some_and(|t| self.derives(t, owner)),
                Access::Private => false,
            }
    }

    fn declare_block(&mut self, b: &VarBlock, pou: Option<PouKind>) {
        self.check_block_form(b, pou);
        for d in &b.decls {
            self.star_ok = matches!(
                (pou, b.kind),
                (Some(PouKind::Function), VarKind::Input | VarKind::Output | VarKind::InOut)
                    | (Some(PouKind::FunctionBlock), VarKind::InOut)
            );
            let ty = self.resolve_spec(&d.ty, true);
            self.out.decls.insert(d.span, ty.clone());
            self.check_location(b, d, &ty, pou);
            self.check_reference_kept(b, d, &ty);
            if d.edge.is_some() && !(b.kind == VarKind::Input && ty.is_bool()) {
                self.error(Code::BadCall, d.span, "R_EDGE and F_EDGE are for BOOL inputs (Ed.3 6.5.1)");
            } else if d.edge.is_some() && matches!(pou, Some(PouKind::Function | PouKind::Method)) {
                self.error(
                    Code::BadCall,
                    d.span,
                    "R_EDGE and F_EDGE are for a FUNCTION_BLOCK's or a PROGRAM's inputs: their implicit R_TRIG or F_TRIG lives in the instance (Ed.3 6.6.3.2 rule 13), and a FUNCTION or a method keeps nothing from one call to the next",
                );
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
                if self.own_vars.contains(&key) && pou == Some(PouKind::Function) {
                    self.error(
                        Code::Duplicate,
                        n.span,
                        format!(
                            "'{}' is a variable of the method's type: a method's names differ from its type's (Ed.3 6.6.5.4.3 rule 10)",
                            n.text
                        ),
                    );
                    continue;
                }
                if self.inherited.contains(&key) {
                    self.error(
                        Code::Duplicate,
                        n.span,
                        format!(
                            "'{}' is a variable of the base type already: a derived type declares its own names (Ed.3 6.6.5.5.5 error 1)",
                            n.text
                        ),
                    );
                    continue;
                }
                if self.locals.contains_key(&key) {
                    self.error(Code::Duplicate, n.span, format!("'{}' is declared twice in this POU", n.text));
                    continue;
                }
                if let Ty::Itf(i) = &ty {
                    let why = if b.kind == VarKind::InOut {
                        Some(format!(
                            "{i} is an INTERFACE, not an in-out (Ed.3 6.6.6.2; OO.md D9): declare it VAR_INPUT, it references the instance given"
                        ))
                    } else {
                        d.location.as_ref().map(|l| {
                            format!(
                                "{i} is an INTERFACE, a reference to an instance: it is not at {} in the process image",
                                l.text
                            )
                        })
                    };
                    if let Some(why) = why {
                        self.error(Code::BadObject, d.ty.span(), why);
                        continue;
                    }
                }
                if let Ty::Fb(t) = &ty
                    && self.is_abstract(t)
                    && b.kind != VarKind::InOut
                {
                    self.error(
                        Code::BadObject,
                        d.ty.span(),
                        format!(
                            "{t} is ABSTRACT: it has no instances (Ed.3 6.6.5.8.2); a VAR_IN_OUT or a REF_TO {t} takes an instance of a type derived from it"
                        ),
                    );
                    continue;
                }
                if let Some((t, held)) = self.object_in(&ty) {
                    let why = match b.kind {
                        VarKind::Input | VarKind::Output => Some("an input or output, a copy"),
                        _ if pou == Some(PouKind::Function)
                            && b.kind != VarKind::InOut
                            && b.kind != VarKind::External =>
                        {
                            Some("a variable of a FUNCTION or method, gone when it returns")
                        }
                        _ => None,
                    };
                    if let Some(why) = why {
                        let what = if held {
                            format!("this type holds an instance of {t}, a class or a block with methods, so it")
                        } else {
                            format!("an instance of {t}, a class or a block with methods,")
                        };
                        self.error(
                            Code::BadObject,
                            d.ty.span(),
                            format!("{what} is not {why} (OO.md D11): pass it as a VAR_IN_OUT"),
                        );
                    }
                }
                let mut input = at_input(d);
                // Which global it is depends on where the POU runs: checked
                // once every POU is (`check_externals`).
                if b.kind == VarKind::External {
                    input = self.globals.get(&key).is_some_and(|(g, _)| at_input(g));
                }
                let temp = b.kind == VarKind::Temp || (pou == Some(PouKind::Function) && b.kind != VarKind::External);
                let flex_in = b.kind == VarKind::Input && is_flex(&ty);
                let inst_in = b.kind == VarKind::Input && self.holds_instance(&ty);
                self.locals.insert(key, Var { temp, input, flex_in, inst_in, ..Var::of(ty.clone(), b.constant) });
            }
        }
    }

    /// A `VAR` of a PROGRAM or FUNCTION_BLOCK is one of three forms, each
    /// its own production (Ed.3 Annex A; Ed.2 B.1.4.3 the same): plain
    /// (`Var_Decls`), addressed in full (`Loc_Var_Decls`), or with `*`
    /// addresses (`Loc_Partly_Var_Decl`). A block that mixes them is
    /// refused at its first declaration of another form.
    fn check_block_form(&mut self, b: &VarBlock, pou: Option<PouKind>) {
        if b.kind != VarKind::Var || !matches!(pou, Some(PouKind::Program | PouKind::FunctionBlock | PouKind::Class)) {
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
    /// element, a field, or a variable of a function block instance. An
    /// interface is one (Ed.3 6.6.6.5.1: "a reference to an instance").
    fn holds_ref(&mut self, ty: &Ty, depth: usize) -> bool {
        self.holds_kind(ty, depth, false)
    }

    /// `holds_ref` for a reference (`itf_only` false: a REF_TO, a
    /// REFERENCE TO or an interface) or for an interface alone.
    fn holds_kind(&mut self, ty: &Ty, depth: usize, itf_only: bool) -> bool {
        if depth > 32 {
            return false;
        }
        match ty {
            Ty::Itf(_) => true,
            Ty::Ref(_) | Ty::Reference(_) => !itf_only,
            Ty::Array { elem, .. } => self.holds_kind(elem, depth + 1, itf_only),
            Ty::Struct(k) => {
                let fields: Vec<&'a VarDecl> = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().collect(),
                    _ => Vec::new(),
                };
                fields.into_iter().any(|f| {
                    let t = self.resolve_spec(&f.ty, false);
                    self.holds_kind(&t, depth + 1, itf_only)
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
                    self.holds_kind(&t, depth + 1, itf_only)
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
        // An interface at an address is E0031's (OO.md).
        if !retained && matches!(ty, Ty::Itf(_)) {
            return;
        }
        let what = if matches!(ty, Ty::Ref(_) | Ty::Reference(_)) {
            "a reference".to_string()
        } else if let Ty::Itf(i) = ty {
            format!("an interface ({i} references an instance)")
        } else {
            format!("{ty}, which holds a reference,")
        };
        // RETAIN: a reference is kept as it is and warned of, as CODESYS
        // and TwinCAT do a pointer in retained memory (decided
        // 2026-11-13, florin); an interface, which holds the address of
        // code, is refused (decided 2026-11-11).
        if retained && !self.holds_kind(ty, 0, true) {
            let what = if matches!(ty, Ty::Ref(_) | Ty::Reference(_)) {
                "a reference".to_string()
            } else {
                format!("{ty}, which holds a reference,")
            };
            self.error(
                Code::KeptReference,
                d.span,
                format!(
                    "{what} is RETAIN and kept as it is: it is valid after a restart only while what it references \
                     is where it was; retain what it references and take the reference again at start"
                ),
            );
            return;
        }
        let message = if retained {
            format!("{what} is not RETAIN: after a restart it would reference nothing; retain what it references")
        } else {
            format!("{what} has no place at an address: the process image holds values")
        };
        self.error(Code::ReferenceNotKept, d.span, message);
    }

    /// `REF(x)` (Ed.3 6.4.4.10.3): a reference to the variable `x`.
    /// `LOWER_BOUND(A, d)` or `UPPER_BOUND(A, d)` (Ed.3 6.5.3, Table
    /// 15): a bound of dimension `d` of the array `A`, a DINT (PLCnext).
    fn bound_call(&mut self, name: &str, args: &[Arg], span: Span) -> Ty {
        let (arr, dim) = match args {
            [Arg::Positional(a), Arg::Positional(d)] => (a, d),
            [Arg::Input(n, a), Arg::Input(m, d)] if n.is("ARR") && m.is("DIM") => (a, d),
            [Arg::Input(m, d), Arg::Input(n, a)] if n.is("ARR") && m.is("DIM") => (a, d),
            _ => {
                self.error(
                    Code::BadCall,
                    span,
                    format!("{name} takes an array and a dimension: {name}(A, 1), or {name}(ARR := A, DIM := 1)"),
                );
                self.args_only(args);
                return Ty::Elem(Elem::Dint);
            }
        };
        let t = self.expr(arr);
        let dt = self.expr(dim);
        if !dt.is_unknown() && !dt.is_int() {
            self.error(Code::OperandTypes, dim.span, format!("an array's dimension is an integer, not {dt}"));
        }
        match &t {
            Ty::Array { dims, .. } => {
                let n = dims.len() as i128;
                if let Some(v) = self.const_int(dim)
                    && !(1..=n).contains(&v)
                {
                    self.error(
                        Code::OperandTypes,
                        dim.span,
                        format!(
                            "this array has {n} dimension(s): its dimensions are 1 to {n}, not {v} (Ed.3 Table 15)"
                        ),
                    );
                }
            }
            Ty::Unknown => {}
            other => self.error(Code::OperandTypes, arr.span, format!("{name} takes an array, not {other}")),
        }
        self.out.std_calls.insert(
            span,
            crate::StdCall {
                name: name.to_string(),
                called: name.to_string(),
                inputs: vec![("ARR".into(), Some(arr.span)), ("DIM".into(), Some(dim.span))],
                outputs: Vec::new(),
                common: t,
                typed_k: None,
            },
        );
        Ty::Elem(Elem::Dint)
    }

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
            ExprKind::Index(..) if self.is_string_char(x) => {
                Err("REF() of one character of a string: a reference is to a whole variable".into())
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
                Some(_) => Some(Elem::Int),
                None => match self.inplace.get(k).map(|d| &d.0) {
                    Some(Some(b)) => Elem::from_name(&b.text),
                    Some(None) => Some(Elem::Int),
                    None => None,
                },
            },
            // Its first part is not its first component (Ed.3 6.4.4.7).
            Ty::Struct(k) if self.layout_kind(k).is_some() => None,
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
        let reachable =
            matches!(pou, Some(PouKind::Program | PouKind::FunctionBlock | PouKind::Class)) && b.kind == VarKind::Var;
        if let Some(loc) = &d.location {
            if loc.is_relative() {
                self.error(
                    Code::BadLocation,
                    loc.span,
                    format!(
                        "{} is a relative location, a component's place in its structure (Ed.3 6.4.4.7); \
                         a variable's address names its area: %IB2, %QX3.0, %MW4",
                        loc.text
                    ),
                );
            } else if pou == Some(PouKind::Function) {
                self.error(
                    Code::BadLocation,
                    loc.span,
                    "a FUNCTION has no located variables: it has no memory between calls (Ed.2 Table E.1)",
                );
            } else if matches!(ty, Ty::Fb(_)) {
                // (An interface there is E0031, below.)
                self.error(
                    Code::BadLocation,
                    loc.span,
                    format!(
                        "{ty} at {}: a function block instance has no address; AT takes an elementary type, an array, a structure or a string (Ed.3 6.5.5, Loc_Var_Spec_Init)",
                        loc.text
                    ),
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
            } else if self.has_partly(ty) {
                // Its `*` components are each where VAR_CONFIG says.
                self.error(
                    Code::BadLocation,
                    loc.span,
                    format!(
                        "{ty} at {}: its components at `*` addresses are each where VAR_CONFIG puts them, \
                         so it has no address of its own (Ed.3 6.4.4.8)",
                        loc.text
                    ),
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
        let decls: Vec<&'a VarDecl> = if self.pous.contains_key(key) {
            // Its own and what it inherits (Ed.3 6.6.5.5.2): an instance
            // of a derived type has its base's variables too.
            std::iter::once(key.to_string())
                .chain(self.ancestors(key))
                .filter_map(|k| self.pous.get(&k).copied())
                .flat_map(|pou| pou.var_blocks.iter().filter(|b| b.kind == VarKind::Var).flat_map(|b| &b.decls))
                .collect()
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
            // The configuration's and the resource's globals: what TASK
            // parameters and program connections name.
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
            let resource = r.name.as_ref().map(|n| upper(&n.text));
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
                        "PRIORITY" => {
                            // Ed.3 6.8.2, Task_Init: an Unsigned_Int.
                            if !matches!(&value.kind, ExprKind::Literal(l) if l.kind == LiteralKind::Int && !l.negative)
                            {
                                self.error(
                                    Code::BadCall,
                                    value.span,
                                    "a TASK's PRIORITY is an unsigned integer literal (Ed.3 6.8.2, Task_Init)",
                                );
                                continue;
                            }
                            Ty::Elem(Elem::Uint)
                        }
                        _ => {
                            self.error(
                                Code::BadCall,
                                name.span,
                                format!("a TASK has no parameter '{}': SINGLE, INTERVAL, PRIORITY", name.text),
                            );
                            continue;
                        }
                    };
                    // SINGLE and INTERVAL are a Data_Source: a constant, a
                    // global or one field of one, a program's output, an
                    // address.
                    let got = match self.program_output(value, r) {
                        Some(t) => t,
                        None => {
                            if want != Ty::Elem(Elem::Uint) && !self.connection_place(value, true, resource.as_deref())
                            {
                                self.error(
                                    Code::BadCall,
                                    value.span,
                                    "a TASK's SINGLE or INTERVAL is a constant, a global variable or one field of one, a program's output (P.OUT), or an address (Ed.3 6.8.2, Data_Source)",
                                );
                                continue;
                            }
                            // `R.g`, this resource's own `g`.
                            let own = {
                                let shadowed = |n: &str| self.locals.contains_key(n);
                                crate::own_resource_access(value, resource.as_deref(), &shadowed)
                            };
                            self.expr(own.as_ref().unwrap_or(value))
                        }
                    };
                    self.coerce(&got, &want, value.span);
                }
            }
            // A program's connections (Ed.3 6.8.2, Figure 15): its inputs
            // given and its outputs taken by name, from and to the
            // configuration's and the resource's globals, addresses and
            // literals - checked as a function block call's.
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
                // What a connection may name (Ed.3 6.8.2, Prog_Cnxn): a
                // constant, an enumerated value, a global or one field of
                // one, or an address; an output only the last two.
                // `fb1 WITH t2` (Ed.3 6.8.2 e, g): the block run by a task
                // of its own, apart from its program's evaluation, its
                // inputs and outputs kept consistent between the two.
                // Not built, by decision (2026-11-13, florin): no vendor
                // has it (docs/dependencies.md).
                for (fb, task) in &p.fb_tasks {
                    self.error(
                        Code::NotYetSupported,
                        fb.span,
                        format!(
                            "'{} WITH {}' gives a function block instance a task of its own (Ed.3 6.8.2), which iecf \
                             does not compile: the block runs where its program calls it; to run it at another rate, \
                             make it a PROGRAM on that task",
                            fb.text, task.text
                        ),
                    );
                }
                let resource = r.name.as_ref().map(|n| upper(&n.text));
                let mut placed = true;
                for a in &p.args {
                    let (e, source) = match a {
                        Arg::Input(_, e) => (e, true),
                        Arg::Output { target, .. } => (target, false),
                        Arg::Positional(_) => continue,
                    };
                    if !self.connection_place(e, source, resource.as_deref()) {
                        placed = false;
                        // `R2.g` written in R1.
                        let mut root = e;
                        while let ExprKind::Member(b, _) = &root.kind {
                            root = b;
                        }
                        if let ExprKind::Name(n) = &root.kind
                            && !self.locals.contains_key(&upper(&n.text))
                            && c.resources
                                .iter()
                                .any(|o| o.name.as_ref().is_some_and(|x| x.text.eq_ignore_ascii_case(&n.text)))
                        {
                            self.error(
                                Code::BadCall,
                                e.span,
                                format!(
                                    "'{}' is another resource: a connection names the globals of its own resource and of \
                                     the configuration, as a global's scope is the resource that declares it (Ed.3 6.8.1)",
                                    n.text
                                ),
                            );
                            continue;
                        }
                        let what = if source {
                            "a program's input is connected to a constant, an enumerated value, a global variable or one field of one, or an address (Ed.3 6.8.2, Prog_Data_Source)"
                        } else {
                            "a program's output is connected to a global variable or one field of one, or an address (Ed.3 6.8.2, Data_Sink)"
                        };
                        self.error(Code::BadCall, e.span, what);
                    }
                }
                // A program instance declared RETAIN keeps its variables
                // of sections neither RETAIN nor NON_RETAIN (Ed.3
                // 6.5.6.2). A reference among them is kept as it is, and
                // warned of (W0006), as CODESYS and TwinCAT do a pointer
                // in retained memory (decided 2026-11-13, florin); an
                // interface is set again at every start.
                if p.retain == Some(Retain::Retain)
                    && let Some(pou) = self.pous.get(&upper(&p.program_type.text)).copied()
                    && pou.kind == PouKind::Program
                {
                    for b in pou.var_blocks.iter().filter(|b| {
                        b.retain.is_none()
                            && !b.constant
                            && matches!(b.kind, VarKind::Var | VarKind::Input | VarKind::Output)
                    }) {
                        for d in &b.decls {
                            let ty = self.resolve_spec(&d.ty, false);
                            if matches!(ty, Ty::Fb(_)) || !self.holds_ref(&ty, 0) {
                                continue;
                            }
                            let name = d.names.first().map_or("", |n| n.text.as_str());
                            let message = if matches!(ty, Ty::Itf(_)) {
                                format!(
                                    "program instance '{}' is RETAIN, but '{name}' of PROGRAM {} is an interface, which is \
                                     not kept: it is set to its initial value at every start",
                                    p.name.text, p.program_type.text
                                )
                            } else {
                                format!(
                                    "program instance '{}' is RETAIN and keeps '{name}' of PROGRAM {}, which holds a \
                                     reference: it is valid after a restart only while what it references is where it \
                                     was; declare it VAR NON_RETAIN to have it set again",
                                    p.name.text, p.program_type.text
                                )
                            };
                            self.error(Code::KeptReference, p.name.span, message);
                        }
                    }
                }
                match self.pous.get(&upper(&p.program_type.text)) {
                    // A connection that names no place was reported.
                    Some(pou) if pou.kind == PouKind::Program && !placed => {}
                    Some(pou) if pou.kind == PouKind::Program => {
                        // `R.g` names this resource's own `g` (Ed.3
                        // 6.8.2, Global_Var_Access).
                        let args: Vec<Arg> = {
                            let shadowed = |n: &str| self.locals.contains_key(n);
                            let own = |e: &Expr| {
                                crate::own_resource_access(e, resource.as_deref(), &shadowed)
                                    .unwrap_or_else(|| e.clone())
                            };
                            p.args
                                .iter()
                                .map(|a| match a {
                                    Arg::Input(n, e) => Arg::Input(n.clone(), own(e)),
                                    Arg::Output { not, name, target } => {
                                        Arg::Output { not: *not, name: name.clone(), target: own(target) }
                                    }
                                    Arg::Positional(e) => Arg::Positional(e.clone()),
                                })
                                .collect()
                        };
                        self.call_args(&upper(&p.program_type.text), Callee::Program, &args, p.span);
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
        self.check_accesses(&c.accesses, Some(c), None);
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
        let mut target: Option<(Option<&'a VarDecl>, Ty, Option<VarKind>, bool)> = None;
        for (i, id) in rest.iter().enumerate() {
            // A POU's own variables, then those it inherits (Ed.3 6.6.5.5.2).
            let in_pou = std::iter::once(container.clone()).chain(self.ancestors(&container)).find_map(|k| {
                self.pous.get(&k).copied()?.var_blocks.iter().find_map(|b| {
                    b.decls
                        .iter()
                        .find(|d| d.names.iter().any(|n| n.is(&id.text)))
                        .map(|d| (d, Some(b.kind), b.constant))
                })
            });
            let found = if self.pous.contains_key(&container) {
                in_pou.map(|(d, k, c)| (Some(d), self.resolve_spec(&d.ty, false), k, c))
            } else if let Some(TypeDef::Struct { fields, .. }) = self.types.get(&container).map(|t| &t.def) {
                let d = fields.iter().find(|d| d.names.iter().any(|n| n.is(&id.text)));
                d.map(|d| (Some(d), self.resolve_spec(&d.ty, false), None, false))
            } else if let Some(params) = std_fb(&container) {
                // A standard block's input or output (Ed.3 Tables 43-46).
                params.into_iter().find(|p| id.is(p.name)).map(|p| {
                    let kind = match p.dir {
                        Dir::Input => VarKind::Input,
                        Dir::Output => VarKind::Output,
                        Dir::InOut => VarKind::InOut,
                    };
                    (None, p.ty, Some(kind), false)
                })
            } else {
                None
            };
            let Some(found) = found else {
                if std_fb(&container).is_some() {
                    let message = format!("{container} has no input or output '{}'", id.text);
                    self.error(Code::UnknownName, id.span, message);
                    return false;
                }
                if !self.pous.contains_key(&container) && !self.types.contains_key(&container) {
                    return false; // reported where the type is declared or used
                }
                self.error(Code::UnknownName, id.span, format!("{container} has no variable '{}'", id.text));
                return false;
            };
            if i + 1 < rest.len() {
                match found.1.clone() {
                    Ty::Fb(k) | Ty::Struct(k) => container = upper(&k),
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
        let Some((decl, declared, kind, constant)) = target else { return false };
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
        let location = decl.and_then(|d| d.location.as_ref());
        let completes = e.location.is_some() && location.is_some_and(|d| d.is_partly());
        if let Some(loc) = &e.location {
            match location {
                _ if loc.is_partly() || loc.is_relative() => self.error(
                    Code::BadConfigEntry,
                    loc.span,
                    format!("VAR_CONFIG gives the complete address, as %IX0.0, not {}", loc.text),
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
                if let Some((t, held)) = self.object_in(&want) {
                    let what = if held {
                        format!("this value holds an instance of {t}, a class or a block with methods, so it")
                    } else {
                        format!("an instance of {t}, a class or a block with methods,")
                    };
                    self.error(Code::BadObject, s.span, format!("{what} is not copied (OO.md D11)"));
                    return;
                }
                if got == Ty::Void {
                    self.error(Code::BadCall, value.span, "this call gives no value to assign");
                } else if want == Ty::Void {
                    // Reported by place(): nothing to assign to.
                } else {
                    self.coerce(&got, &want, value.span);
                }
            }
            StmtKind::Call(e) => {
                self.expr(e);
            }
            StmtKind::AssignAttempt { target, value } => self.assign_attempt(target, value, s.span),
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
                // Ed.3 7.3.3.3.3: the selector is "of elementary data
                // type", or an enumeration, the labels constants it
                // "shall be able to be compared with".
                let ok = sel.is_unknown() || sel.is_int() || matches!(sel, Ty::Enum(_) | Ty::Elem(_) | Ty::Str { .. });
                if !ok {
                    self.error(
                        Code::OperandTypes,
                        selector.span,
                        format!("CASE selects on a value of an elementary type or an enumeration, not {sel}"),
                    );
                }
                for arm in arms {
                    for label in &arm.labels {
                        match label {
                            CaseLabel::Value(e) => self.case_label(e, &sel, ok),
                            CaseLabel::Range(lo, hi) => {
                                for e in [lo, hi] {
                                    self.case_label(e, &sel, ok);
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
    /// `P1.OUT`, an output of a program instance of the resource `r`
    /// (Ed.3 6.8.2, Prog_Output_Access): its type. `None` for anything else.
    fn program_output(&mut self, e: &Expr, r: &Resource) -> Option<Ty> {
        let ExprKind::Member(b, m) = &e.kind else { return None };
        let ExprKind::Name(p) = &b.kind else { return None };
        if self.locals.contains_key(&upper(&p.text)) {
            return None;
        }
        let pc = r.programs.iter().find(|pc| pc.name.text.eq_ignore_ascii_case(&p.text))?;
        let pou = self.pous.get(&upper(&pc.program_type.text)).copied()?;
        let d = pou
            .var_blocks
            .iter()
            .filter(|b| b.kind == VarKind::Output)
            .flat_map(|b| &b.decls)
            .find(|d| d.names.iter().any(|n| n.text.eq_ignore_ascii_case(&m.text)))?;
        let t = self.resolve_spec(&d.ty, false);
        self.out.exprs.insert(e.span, t.clone());
        Some(t)
    }

    /// Whether `e` is what a program connection may name (Ed.3 6.8.2):
    /// Global_Var_Access - `(Resource_Name '.')? Global_Var_Name ('.'
    /// Struct_Elem_Name)?` - or a Direct_Variable; for an input
    /// (`source`) also a Constant (NULL among them) or an Enum_Value.
    fn connection_place(&self, e: &Expr, source: bool, resource: Option<&str>) -> bool {
        let global = |n: &Ident| self.locals.contains_key(&upper(&n.text));
        let is_resource = |x: &Expr| matches!(&x.kind, ExprKind::Name(n) if Some(upper(&n.text).as_str()) == resource);
        match &e.kind {
            ExprKind::DirectAddress(_) => true,
            ExprKind::Literal(_) | ExprKind::EnumValue { .. } => source,
            ExprKind::Unary(UnaryOp::Neg, x) => source && matches!(x.kind, ExprKind::Literal(_)),
            ExprKind::Name(n) if n.is("NULL") => source,
            ExprKind::Name(n) => {
                global(n) || (source && !self.enum_values.get(&upper(&n.text)).is_none_or(Vec::is_empty))
            }
            // R.g, g.x, R.g.x
            ExprKind::Member(b, m) => match &b.kind {
                ExprKind::Name(g) if global(g) => true,
                _ if is_resource(b) => self.locals.contains_key(&upper(&m.text)),
                ExprKind::Member(r, g) => is_resource(r) && global(g),
                _ => false,
            },
            _ => false,
        }
    }

    /// E0013 unless `t`, a variable given to the in-out `name` of type
    /// `want`, is of that type exactly: a STRING of its length, an array
    /// of its bounds (docs/behaviour/strings.md).
    fn in_out_type(&mut self, name: &str, want: &Ty, t: &Ty, span: Span) {
        // A STRING declared without a length is a STRING[80] (docs/dependencies.md).
        fn norm(t: &Ty) -> Ty {
            match t {
                Ty::Str { wide, len } => Ty::Str { wide: *wide, len: Some(len.unwrap_or(80)) },
                Ty::Array { elem, dims } => Ty::Array { elem: Box::new(norm(elem)), dims: dims.clone() },
                t => t.clone(),
            }
        }
        if t.is_unknown()
            || want.is_unknown()
            || norm(t) == norm(want)
            // Named values: a variable of the base type.
            || matches!(want, Ty::Enum(k) if self.named_values(k).map(Ty::Elem).as_ref() == Some(t))
        {
            return;
        }
        self.error(Code::TypeMismatch, span, format!("in-out '{name}' is {want}: pass a {want} variable, not {t}"));
    }

    fn coerce(&mut self, from: &Ty, to: &Ty, span: Span) -> bool {
        // A type with named values takes what its base type takes (Ed.3
        // 6.4.4.3: "other constants can be assigned").
        let as_base = |c: &Self, t: &Ty| match t {
            Ty::Enum(k) => c.named_values(k).map(Ty::Elem),
            _ => None,
        };
        let (fb, tb) = (as_base(self, from), as_base(self, to));
        let (from, to) = (fb.as_ref().unwrap_or(from), tb.as_ref().unwrap_or(to));
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
            // 'a' where a CHAR is wanted, "a" where a WCHAR is (Ed.3 Table
            // 6, 1b and 2b).
            (Ty::Str { wide, .. }, Ty::Elem(c @ (Elem::Char | Elem::Wchar)))
                if self.one_char.get(&span) == Some(wide) && *wide == (*c == Elem::Wchar) =>
            {
                self.out.exprs.insert(span, Ty::Elem(*c));
                true
            }
            (Ty::Elem(Elem::Char), Ty::Str { wide: false, .. })
            | (Ty::Elem(Elem::Wchar), Ty::Str { wide: true, .. }) => true,
            (Ty::Str { wide: a, .. }, Ty::Str { wide: b, .. }) if a == b => true,
            (Ty::Null, Ty::Ref(_) | Ty::Itf(_)) => true,
            // An interface takes an instance of a type that implements it,
            // or an interface that extends it (Ed.3 6.6.6.5.1).
            (Ty::Fb(t), Ty::Itf(i)) => {
                if self.out.implemented(t).contains(i) {
                    return true;
                }
                self.error(
                    Code::TypeMismatch,
                    span,
                    format!("{t} does not implement {i}: an interface takes an instance of a type that IMPLEMENTS it (Ed.3 6.6.6.5.1)"),
                );
                false
            }
            (Ty::Itf(j), Ty::Itf(i)) => {
                if self.out.itf_line(j).contains(i) {
                    return true;
                }
                self.error(
                    Code::TypeMismatch,
                    span,
                    format!(
                        "{j} does not extend {i}: to take one whose instance may implement {i}, write ?= (Ed.3 6.6.6.7)"
                    ),
                );
                false
            }
            // A reference to a derived instance where one to its base is
            // wanted (Ed.3 6.6.8; OO.md D7).
            (Ty::Ref(a), Ty::Ref(b)) if matches!((&**a, &**b), (Ty::Fb(d), Ty::Fb(base)) if self.derives(d, base)) => {
                if let (Ty::Fb(d), Ty::Fb(base)) = (&**a, &**b) {
                    let (d, base) = (d.clone(), base.clone());
                    self.may_stand_for(&d, &base, span);
                }
                true
            }
            (Ty::Ref(a), Ty::Ref(b)) if a.is_unknown() || b.is_unknown() => true,
            // A bound not known (`*`, or already reported) meets any.
            // An array to a variable-length one of as many dimensions
            // (Ed.3 6.5.3); not the other way: its bounds are not known.
            (Ty::Array { elem: a, dims: da }, Ty::Array { elem: b, dims: db })
                if a == b && da.len() == db.len() && da.iter().zip(db).all(|(x, y)| y.is_none() || x == y) =>
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
        if let Ty::Struct(k) = ty
            && self.layout_kind(k) == Some(true)
        {
            self.error(
                Code::BadLayout,
                init_span(init),
                format!(
                    "{ty} is a STRUCT OVERLAP: an overlapped structure is not initialised explicitly (Ed.3 6.4.4.7.2); \
                     its components start at their types' initial values, in the order declared"
                ),
            );
            return;
        }
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
                // An initial value is a constant expression (Ed.3 Figure 6):
                // one naming a CONSTANT, worked out for the generator.
                if ty.is_int()
                    && let Some(v) = self.const_int(e)
                {
                    self.out.consts.insert(e.span, v);
                }
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
                        if let Some(v) = self.const_int(r) {
                            self.out.consts.insert(r.span, v);
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
                        // Its inputs, outputs and PUBLIC variables (Ed.3
                        // 6.6.3.3, 6.6.5.3; INTERNAL ones in its namespace).
                        Ty::Fb(fb) => {
                            let fb = fb.clone();
                            let key = upper(&name.text);
                            let public = match self.var_access(&fb, &key) {
                                Some((a @ (Access::Public | Access::Internal), true, owner)) => self.may_use(&owner, a),
                                _ => false,
                            };
                            match self.params(&fb).into_iter().find(|p| p.name == key) {
                                Some(p) if matches!(p.dir, Dir::Input | Dir::Output) => p.ty,
                                None if public => self.var_type(&fb, &key).unwrap_or(Ty::Unknown),
                                _ => {
                                    self.error(
                                        Code::UnknownName,
                                        name.span,
                                        format!(
                                            "{ty} has no input, output or PUBLIC variable '{}' to give an initial value (Ed.3 6.6.3.3)",
                                            name.text
                                        ),
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
        self.place_depth += 1;
        let t = self.place_of(e);
        self.place_depth -= 1;
        // Checked once, for the whole of what is written.
        if self.place_depth == 0
            && let Some(n) = self.input_instance(e)
        {
            self.error(
                Code::NotAssignable,
                e.span,
                format!(
                    "'{n}' is given as an input: a function block instance in it is read, never written \
                     (Ed.3 6.6.3.4 rule 6; a VAR_IN_OUT is written and called)"
                ),
            );
        }
        t
    }

    fn place_of(&mut self, e: &Expr) -> Ty {
        match &e.kind {
            ExprKind::Name(id) => {
                let t = self.expr(e);
                if let Some(v) = self.locals.get(&upper(&id.text)) {
                    if v.ty == Ty::Void && upper(&id.text) == self.pou_key {
                        self.error(
                            Code::NotAssignable,
                            e.span,
                            format!(
                                "{} has no result to assign: a FUNCTION declared without a type gives none (Ed.3 6.6.2.2)",
                                id.text
                            ),
                        );
                    } else if v.constant {
                        self.error(Code::NotAssignable, e.span, format!("'{}' is CONSTANT", id.text));
                    } else if v.flex_in {
                        self.error(
                            Code::NotAssignable,
                            e.span,
                            format!(
                                "'{}' is an ARRAY[*] input, the caller's array itself and not a copy: it is read,                                  never written (a VAR_IN_OUT is written)",
                                id.text
                            ),
                        );
                    }
                } else if !t.is_unknown() {
                    self.error(Code::NotAssignable, e.span, format!("'{}' is not a variable", id.text));
                }
                t
            }
            // THIS.x is x (Ed.3 6.6.5.7.2), assigned as x is.
            ExprKind::Member(base, m) if matches!(&base.kind, ExprKind::Name(t) if t.is("THIS")) => {
                if self.this_type.is_some() && self.own_vars.contains(&upper(&m.text)) {
                    self.out.this_members.insert(e.span);
                    return self.place(&Expr { kind: ExprKind::Name(m.clone()), span: e.span });
                }
                self.expr(e)
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

    /// `s[i]`, one character of a string, counted from 1 (Ed.3 6.4.2): a
    /// CHAR of a STRING, a WCHAR of a WSTRING. A constant position outside `1..n` is an error
    /// here; one computed is MID's or REPLACE's at run time.
    fn string_char(&mut self, e: &Expr, wide: bool, len: Option<u64>, index: &[Expr]) -> Ty {
        let mut ok = true;
        for i in index {
            let it = self.expr(i);
            if !it.is_unknown() && !it.is_int() {
                self.error(Code::OperandTypes, i.span, format!("a character's position is an integer, not {it}"));
                ok = false;
            }
        }
        let s = Ty::Str { wide, len };
        if index.len() != 1 {
            self.error(Code::OperandTypes, e.span, format!("{s} takes one position in [ ], not {}", index.len()));
            return Ty::Unknown;
        }
        if let Some(p) = self.const_int(&index[0]) {
            let past = len.is_some_and(|n| p > n as i128);
            if p < 1 || past {
                let range = match len {
                    Some(n) => format!("1..{n}"),
                    None => "from 1".to_string(),
                };
                self.error(Code::OperandTypes, index[0].span, format!("{s} has characters {range}, not {p}"));
                ok = false;
            }
        }
        match (ok, wide) {
            (false, _) => Ty::Unknown,
            (true, false) => Ty::Elem(Elem::Char),
            (true, true) => Ty::Elem(Elem::Wchar),
        }
    }

    /// Whether `e` is `s[i]`, one character of a string: not a variable,
    /// so no REF() of it and no in-out given it.
    fn is_string_char(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Paren(x) => self.is_string_char(x),
            ExprKind::Index(base, _) => matches!(self.out.exprs.get(&base.span), Some(Ty::Str { .. })),
            _ => false,
        }
    }

    /// A CASE label `e` for a selector of type `sel` (`ok`: one CASE
    /// selects on). For an integer it is a constant expression (Ed.3
    /// 7.3.3.3.3, Case_List_Elem: "Constant_Expr"), worked out for
    /// the generator; a variable there is E0032.
    /// Whether `e` is written of constants alone: literals, enumerated
    /// values, CONSTANT variables, and operators on them.
    fn constant_expr(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Literal(_) | ExprKind::EnumValue { .. } => true,
            ExprKind::Name(n) => {
                let k = upper(&n.text);
                match self.locals.get(&k) {
                    Some(v) => v.constant,
                    None => !self.enum_values.get(&k).is_none_or(Vec::is_empty),
                }
            }
            ExprKind::Paren(x) | ExprKind::Unary(_, x) => self.constant_expr(x),
            ExprKind::Binary(_, a, b) => self.constant_expr(a) && self.constant_expr(b),
            _ => false,
        }
    }

    fn case_label(&mut self, e: &Expr, sel: &Ty, ok: bool) {
        let t = self.expr(e);
        if !ok || t.is_unknown() {
            return;
        }
        if !(sel.is_int() && t.is_int()) {
            self.coerce(&t, sel, e.span);
            if !matches!(sel, Ty::Enum(_)) && !self.constant_expr(e) {
                self.error(
                    Code::NotConstant,
                    e.span,
                    "a CASE label is a constant expression: literals, CONSTANT variables and operators on them \
                     (Ed.3 7.3.3.3.3)",
                );
            }
            return;
        }
        // Compared by value, whatever integer type its constants are: a
        // value the selector's type cannot hold is refused as a literal is.
        match self.const_int(e) {
            Some(v) => {
                self.coerce(&Ty::IntLit(Some(v)), sel, e.span);
                self.out.consts.insert(e.span, v);
            }
            None => self.error(
                Code::NotConstant,
                e.span,
                "a CASE label is a constant integer expression: literals, CONSTANT variables and + - * / MOD (Ed.3 7.3.3.3.3)",
            ),
        }
    }

    /// The value of a constant integer expression, when it is one.
    fn const_int(&mut self, e: &Expr) -> Option<i128> {
        match &e.kind {
            ExprKind::Literal(l) if matches!(l.kind, LiteralKind::Int | LiteralKind::Based) => literal_int(l),
            ExprKind::Paren(x) => self.const_int(x),
            ExprKind::Name(id) => self.named_const(&upper(&id.text)),
            ExprKind::Unary(UnaryOp::Neg, x) => self.const_int(x).map(|v| -v),
            ExprKind::Unary(UnaryOp::Plus, x) => self.const_int(x),
            ExprKind::Binary(op, a, b) => {
                let (a, b) = (self.const_int(a)?, self.const_int(b)?);
                match op {
                    BinaryOp::Add => a.checked_add(b),
                    BinaryOp::Sub => a.checked_sub(b),
                    BinaryOp::Mul => a.checked_mul(b),
                    BinaryOp::Div if b != 0 => Some(a / b),
                    BinaryOp::Mod if b != 0 => Some(a % b),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// The value of the constant `key` names, an integer (Ed.3 6.6.1.2.4):
    /// the POU's own CONSTANT, or a VAR_GLOBAL CONSTANT that no name of
    /// the POU hides. A type's declaration sees only the globals.
    fn named_const(&mut self, key: &str) -> Option<i128> {
        if self.const_depth > 16 {
            return None;
        }
        let local = if self.resolving.is_empty() { self.consts.get(key).copied() } else { None };
        let (d, e, global) = match local {
            Some(Konst::Value(d, e)) => (d, e, false),
            Some(Konst::Not) => return None,
            Some(Konst::Global) | None => match self.globals.get(key).copied() {
                Some((d, true)) => match &d.init {
                    Some(Init::Expr(e)) => (d, e, true),
                    _ => return None,
                },
                // A named value (Ed.3 6.4.4.3), when one type alone has it.
                None if local.is_none() => {
                    let types = self.enum_values.get(key)?.clone();
                    let [t] = types.as_slice() else { return None };
                    self.named_values(t)?;
                    self.const_depth += 1;
                    let v = self.named_value(t, key);
                    self.const_depth -= 1;
                    return v;
                }
                _ => return None,
            },
        };
        if !int_decl(d) {
            return None;
        }
        // A global's value names only globals.
        let saved = if global { std::mem::take(&mut self.consts) } else { HashMap::new() };
        self.const_depth += 1;
        let v = self.const_int(e);
        self.const_depth -= 1;
        if global {
            self.consts = saved;
        }
        v
    }

    /// The names `pou` sees as its own, for [`Self::named_const`]: its
    /// type's, inherited ones first, for a method or a block.
    fn pou_consts(&mut self, pou: &'a Pou) {
        self.consts.clear();
        let mut chain: Vec<&'a Pou> = Vec::new();
        if let Some(t) = self.this_type.clone() {
            let mut keys = self.ancestors(&t);
            keys.reverse();
            keys.push(t);
            chain.extend(keys.iter().filter_map(|k| self.pous.get(k).copied()));
        }
        if !chain.iter().any(|p| std::ptr::eq(*p, pou)) {
            chain.push(pou);
        }
        for p in chain {
            for b in &p.var_blocks {
                for d in &b.decls {
                    let k = match (&b.kind, b.constant, &d.init) {
                        (VarKind::External, ..) => Konst::Global,
                        (_, true, Some(Init::Expr(e))) => Konst::Value(d, e),
                        _ => Konst::Not,
                    };
                    for n in &d.names {
                        self.consts.insert(upper(&n.text), k);
                    }
                }
            }
        }
    }

    /// `e`, which the standard says is a constant expression of an
    /// integer (`what`): its value, recorded for the code generator.
    fn const_bound(&mut self, e: &Expr, report: bool, what: &str) -> Option<i128> {
        let v = self.const_int(e);
        match v {
            Some(v) => {
                self.out.consts.insert(e.span, v);
            }
            None if report => self.error(
                Code::NotConstant,
                e.span,
                format!(
                    "{what} is a constant integer expression: literals, CONSTANT variables and + - * / MOD (Ed.3 6.6.1.2.4)"
                ),
            ),
            None => {}
        }
        v
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
        // A type with named values is used as its base type is (Ed.3
        // 6.4.4.3); the generator is told which type's value a name is.
        if let Ty::Enum(k) = &t
            && let Some(b) = self.named_values(k)
        {
            return Ty::Elem(b);
        }
        t
    }

    /// The base type of `key` when it is a type with named values (Ed.3
    /// 6.4.4.3): an enumeration declared with a base type or with a value
    /// given. INT when it names none, as CODESYS and TwinCAT have it
    /// (docs/dependencies.md).
    fn named_values(&self, key: &str) -> Option<Elem> {
        let rc;
        let (base, values) = match self.types.get(key).map(|d| &d.def) {
            Some(TypeDef::Spec(TypeSpec::Enum { base, values, .. })) => (base, values),
            Some(_) => return None,
            None => {
                rc = self.inplace.get(key)?.clone();
                (&rc.0, &rc.1)
            }
        };
        if base.is_none() && values.iter().all(|v| v.value.is_none()) {
            return None;
        }
        match base {
            Some(b) => Elem::from_name(&b.text).filter(|e| e.int_range().is_some() && *e != Elem::Bool),
            None => Some(Elem::Int),
        }
    }

    /// The number of the named value `name` of the type `key`: the one
    /// given, or one more than the value before it, from 0, as CODESYS
    /// and TwinCAT count.
    fn named_value(&mut self, key: &str, name: &str) -> Option<i128> {
        let rc;
        // One declared in place sees its POU's constants, as its other
        // declarations do; a TYPE's the global ones.
        let (values, own) = match self.types.get(key).map(|d| &d.def) {
            Some(TypeDef::Spec(TypeSpec::Enum { values, .. })) => (values, false),
            Some(_) => return None,
            None => {
                rc = self.inplace.get(key)?.clone();
                (&rc.1, true)
            }
        };
        let mut next = 0i128;
        for v in values {
            let n = match &v.value {
                Some(e) if own => {
                    self.const_depth += 1;
                    let n = self.const_int(e);
                    self.const_depth -= 1;
                    n?
                }
                Some(e) => self.type_const(e)?,
                None => next,
            };
            if v.name.is(name) {
                return Some(n);
            }
            next = n.checked_add(1)?;
        }
        None
    }

    /// A constant expression in a TYPE declaration: it sees the global
    /// constants only.
    fn type_const(&mut self, e: &Expr) -> Option<i128> {
        let saved = std::mem::take(&mut self.consts);
        self.const_depth += 1;
        let v = self.const_int(e);
        self.const_depth -= 1;
        self.consts = saved;
        v
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
        let derived = match (&got, to) {
            (Ty::Fb(d), Ty::Fb(b)) => {
                let (d, b) = (d.clone(), b.clone());
                self.may_stand_for(&d, &b, x.span)
            }
            _ => false,
        };
        if got != *to && !derived {
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
            ExprKind::Member(base, m) if matches!(&base.kind, ExprKind::Name(t) if t.is("THIS")) => {
                let key = upper(&m.text);
                if self.this_type.is_some() && self.own_vars.contains(&key) {
                    // THIS.x is the instance's x (Ed.3 6.6.5.7.2), as `x`.
                    self.out.this_members.insert(e.span);
                    return self.name(m);
                }
                let bt = self.expr(base);
                if let (Ty::Fb(t), true) = (&bt, self.hidden.contains_key(&key)) {
                    let why = self.hidden[&key];
                    self.error(
                        Code::UnknownName,
                        m.span,
                        format!("'{}' is {why} of {t}: a method cannot reach it (Ed.3 6.6.7.2.3)", m.text),
                    );
                    return Ty::Unknown;
                }
                self.member(&bt, m)
            }
            ExprKind::Member(base, m) => {
                let bt = self.expr(base);
                self.member(&bt, m)
            }
            ExprKind::BitAccess(base, bit) => {
                let bt = self.expr(base);
                // Ed.3 6.6.1.3, Table 17: a bit, byte, word or double word
                // of a BYTE, WORD, DWORD or LWORD, numbered from the least
                // significant; not of a direct variable.
                if matches!(base.kind, ExprKind::DirectAddress(_)) {
                    self.error(
                        Code::OperandTypes,
                        e.span,
                        "a partial access is not taken of a direct variable (Ed.3 6.6.1.3): %IX0.3, not %IB0.3",
                    );
                    return Ty::Unknown;
                }
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
                            format!("a part is taken of BYTE, WORD, DWORD or LWORD, not {bt}"),
                        );
                        return Ty::Unknown;
                    }
                };
                let Some((size, n)) = bit.part() else { return Ty::Unknown };
                let (part, name) = match size {
                    PartSize::Bit => (Elem::Bool, "bits"),
                    PartSize::Byte => (Elem::Byte, "bytes"),
                    PartSize::Word => (Elem::Word, "words"),
                    PartSize::Dword => (Elem::Dword, "double words"),
                    PartSize::Lword => (Elem::Lword, "long words"),
                };
                let count = width / u64::from(size.bits());
                if size.bits() as u64 >= width && size != PartSize::Bit {
                    self.error(
                        Code::OperandTypes,
                        bit.span,
                        format!(
                            "{bt} has no {} part: a part is smaller than its variable (Ed.3 Table 17)",
                            part.name()
                        ),
                    );
                } else if n >= count {
                    self.error(Code::OperandTypes, bit.span, format!("{bt} has {name} 0..{}", count - 1));
                }
                Ty::Elem(part)
            }
            ExprKind::Index(base, index) => {
                let bt = self.expr(base);
                if let Ty::Str { wide, len } = bt {
                    return self.string_char(e, wide, len, index);
                }
                for i in index {
                    let it = self.expr(i);
                    if !it.is_unknown() && !it.is_int() {
                        self.error(Code::OperandTypes, i.span, format!("an array index is an integer, not {it}"));
                    }
                }
                match bt {
                    Ty::Array { elem, dims } => {
                        if dims.len() < index.len() {
                            self.error(
                                Code::OperandTypes,
                                e.span,
                                format!("this array has {} dimension(s), indexed with {}", dims.len(), index.len()),
                            );
                            // Reported once: what it would be is not known.
                            return Ty::Unknown;
                        }
                        // Indexed in its first dimensions only, it is the
                        // array of the others: A2[2] of Ed.3 6.5.3's
                        // Example 2 (docs/dependencies.md).
                        if index.len() < dims.len() {
                            return Ty::Array { elem, dims: dims[index.len()..].to_vec() };
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
                // 'a' against a CHAR is a CHAR, "a" against a WCHAR a WCHAR
                // (Ed.3 Table 6, 1b and 2b).
                let mut ta = ta;
                let mut tb = tb;
                for c in [Elem::Char, Elem::Wchar] {
                    let char = Ty::Elem(c);
                    if tb == char && self.char_literal(a, c) {
                        ta = char.clone();
                    }
                    if ta == char && self.char_literal(b, c) {
                        tb = char;
                    }
                }
                self.binary(*op, &ta, &tb, e.span)
            }
            ExprKind::Paren(x) => self.expr(x),
            ExprKind::Error => Ty::Unknown,
        }
    }

    /// Whether `e` is an untyped one-character literal of `c`'s width, a
    /// STRING one for a CHAR, a WSTRING one for a WCHAR, which it then
    /// records as a `c` for the generator.
    fn char_literal(&mut self, e: &Expr, c: Elem) -> bool {
        let inner = match &e.kind {
            ExprKind::Paren(x) => return self.char_literal(x, c),
            ExprKind::Literal(l) => l.span,
            _ => return false,
        };
        let wide = c == Elem::Wchar;
        if self.one_char.get(&inner) != Some(&wide) && self.one_char.get(&e.span) != Some(&wide) {
            return false;
        }
        self.out.exprs.insert(e.span, Ty::Elem(c));
        self.out.exprs.insert(inner, Ty::Elem(c));
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
                    // `WCHAR#16#263A`: the UTF-16 unit of that code.
                    Some(Ty::Elem(Elem::Wchar)) => {
                        if let Some(v) = v
                            && !(0..=0xFFFF).contains(&v)
                        {
                            self.error(Code::LiteralRange, l.span, format!("{v} is outside WCHAR (0..65535)"));
                        }
                        Ty::Elem(Elem::Wchar)
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
                let double = l.kind == LiteralKind::WString;
                let prefix = prefixed(self);
                // A double-byte literal: in double quotes, or typed WSTRING#
                // or WCHAR# in either (Ed.3 Table 6, 4c and 4d).
                let wide = match &prefix {
                    Some(Ty::Str { wide, .. }) => *wide,
                    Some(Ty::Elem(Elem::Wchar)) => true,
                    _ => double,
                };
                if !wide && double {
                    self.error(Code::LiteralRange, l.span, "a single-byte literal is written in single quotes: 'X'");
                }
                let len = if wide {
                    match crate::strings::wstring_units(&l.text) {
                        Ok(u) => u.len(),
                        Err(ch) => {
                            self.error(Code::LiteralRange, l.span, format!("{ch} is not a character"));
                            0
                        }
                    }
                } else {
                    match crate::strings::string_bytes(&l.text) {
                        Ok(b) => b.len(),
                        Err(ch) => {
                            self.error(
                                Code::LiteralRange,
                                l.span,
                                format!(
                                    "{ch} is not in Latin-1, the character set of a STRING: use a WSTRING (docs/dependencies.md)"
                                ),
                            );
                            0
                        }
                    }
                };
                match prefix {
                    // `CHAR#'X'`: one character (Ed.3 Table 6, 3b).
                    Some(Ty::Elem(Elem::Char)) if len != 1 => {
                        self.error(Code::LiteralRange, l.span, "a CHAR literal holds one character: CHAR#'X'");
                        Ty::Elem(Elem::Char)
                    }
                    // `WCHAR#"X"`: one UTF-16 unit (Ed.3 Table 6, 4b and 4d).
                    Some(Ty::Elem(Elem::Wchar)) if len != 1 => {
                        self.error(
                            Code::LiteralRange,
                            l.span,
                            "a WCHAR literal holds one character, up to U+FFFF: WCHAR#\"X\" (docs/dependencies.md)",
                        );
                        Ty::Elem(Elem::Wchar)
                    }
                    Some(t) => t,
                    None => {
                        if len == 1 {
                            self.one_char.insert(l.span, wide);
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
        // Outside a POU - a TYPE's initial value, a VAR_GLOBAL's - a
        // VAR_GLOBAL CONSTANT is seen directly, as in a TYPE's bounds
        // (docs/dependencies.md).
        if self.pou_kind.is_none()
            && let Some((decl, true)) = self.globals.get(&key).copied()
        {
            return self.resolve_spec(&decl.ty, false);
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
                return Ty::Enum(enums[0].to_string());
            }
            // Types with named values that all give it one number: it is
            // that number whichever is meant (Ed.3 6.4.4.3: used as the
            // base type), and one declared in place has no name to write.
            let numbers: Vec<Option<i128>> = enums
                .iter()
                .map(|t| if self.named_values(t).is_some() { self.named_value(t, &key) } else { None })
                .collect();
            if numbers[0].is_some() && numbers.iter().all(|n| *n == numbers[0]) {
                let bases: Vec<Option<Elem>> = enums.iter().map(|t| self.named_values(t)).collect();
                if bases.iter().all(|b| *b == bases[0]) {
                    return Ty::Enum(enums[0].to_string());
                }
            }
            self.error(
                Code::UnknownName,
                id.span,
                format!("'{}' is a value of {} enumerations: write Type#{}", id.text, enums.len(), id.text),
            );
            return Ty::Unknown;
        }
        if key == "SUPER" {
            self.error(
                Code::BadObject,
                id.span,
                "SUPER calls the base type's method, SUPER.m(...), or its body, SUPER() (Ed.3 6.6.5.7.3, 6.6.7.2.9)",
            );
            return Ty::Unknown;
        }
        if key == "THIS" {
            if let Some(t) = &self.this_type {
                return Ty::Fb(t.clone());
            }
            self.error(
                Code::BadObject,
                id.span,
                "THIS is the own instance: in a method, or in a function block's body (Ed.3 6.6.5.7.2)",
            );
            return Ty::Unknown;
        }
        if let Some(why) = self.hidden.get(&key) {
            let msg = format!("'{}' is {why} of the method's type: a method cannot reach it (Ed.3 6.6.7.2.3)", id.text);
            self.error(Code::UnknownName, id.span, msg);
            return Ty::Unknown;
        }
        let message = if self.globals.contains_key(&key) && self.pou_kind.is_none() {
            format!("'{}' is a global variable: only a VAR_GLOBAL CONSTANT is named here", id.text)
        } else if self.globals.contains_key(&key) {
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
        self.out.implicit_externals.entry(self.pou_key.clone()).or_default().push((key.to_string(), constant));
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
                if self.find_method(&fb, &key).is_some() {
                    self.error(
                        Code::BadCall,
                        m.span,
                        format!(
                            "'{}' is a method of {fb}: call it, {}(...); its result and outputs are only its call's (Ed.3 6.6.5.4.7)",
                            m.text, m.text
                        ),
                    );
                    return Ty::Unknown;
                }
                if let Some((access, written, owner)) = self.var_access(&fb, &key) {
                    if self.may_use(&owner, access) {
                        if let Some(t) = self.var_type(&fb, &key) {
                            return t;
                        }
                    } else if written {
                        self.error(
                            Code::AccessDenied,
                            m.span,
                            format!(
                                "'{}' is {} in {owner}: not reached from here (Ed.3 6.6.5.10)",
                                m.text,
                                access.name()
                            ),
                        );
                        return Ty::Unknown;
                    }
                }
                if self.is_internal_member(&fb, &key) {
                    self.error(
                        Code::UnknownName,
                        m.span,
                        if self.pous.get(&fb).is_some_and(|p| p.kind == PouKind::Class) {
                            format!(
                                "'{}' is PROTECTED in the CLASS {fb} (no specifier: Ed.3 6.6.5.10): declare it in VAR PUBLIC to reach it from outside",
                                m.text
                            )
                        } else {
                            format!("'{}' is internal to {fb}: only its inputs and outputs are seen outside", m.text)
                        },
                    );
                } else if self
                    .pous
                    .get(&fb)
                    .is_some_and(|p| p.sfc.iter().any(|el| matches!(el, SfcElement::Action(a) if a.name.is(&m.text))))
                {
                    // `fb.act()`, CODESYS's call of an action: in IEC a
                    // step runs an action, through its association.
                    self.error(
                        Code::BadCall,
                        m.span,
                        format!(
                            "'{}' is an ACTION of {fb}: a step runs it, through its action association (Ed.3 6.7.4), and \
                             nothing calls it from outside; write it as a METHOD to call it (the action-call extension \
                             is proposed, docs/extensions.md)",
                            m.text
                        ),
                    );
                } else {
                    self.error(Code::UnknownName, m.span, format!("{fb} has no input or output '{}'", m.text));
                }
                Ty::Unknown
            }
            Ty::Itf(i) => {
                let msg = if self.itf_method(i, &key).is_some() {
                    format!("'{}' is a method of {i}: call it, {}(...)", m.text, m.text)
                } else {
                    format!("{i} is an INTERFACE: it has methods only, and none '{}' (Ed.3 6.6.6.1)", m.text)
                };
                self.error(Code::UnknownName, m.span, msg);
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
            // Interfaces: = and <> only, equal when they reference one
            // instance or are both NULL (Ed.3 6.6.6.5.1).
            Eq | Ne | Lt | Le | Gt | Ge
                if matches!((a, b), (Ty::Itf(_), Ty::Itf(_) | Ty::Null) | (Ty::Null, Ty::Itf(_))) =>
            {
                if !matches!(op, Eq | Ne) {
                    return fail(self, "compares interfaces with = and <> only");
                }
                if let (Ty::Itf(x), Ty::Itf(y)) = (a, b)
                    && !self.out.itf_line(x).contains(y)
                    && !self.out.itf_line(y).contains(x)
                {
                    fail(self, "compares two interfaces one of which extends the other, or one with NULL");
                }
                Ty::Elem(Elem::Bool)
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
            // Two instances have no order and no equality the standard
            // or a vendor states (docs/dependencies.md).
            // Nor are arrays and structures: Ed.3 Table 33 compares
            // ANY_ELEMENTARY, as CODESYS, TwinCAT and PLCnext do.
            Eq | Ne | Lt | Le | Gt | Ge
                if matches!(a, Ty::Array { .. } | Ty::Struct(_)) || matches!(b, Ty::Array { .. } | Ty::Struct(_)) =>
            {
                self.error(
                    Code::OperandTypes,
                    span,
                    format!(
                        "{sym} compares elementary values (Ed.3 Table 33), not {a} and {b}: compare their elements or fields"
                    ),
                );
                Ty::Elem(Elem::Bool)
            }
            Eq | Ne | Lt | Le | Gt | Ge if matches!(a, Ty::Fb(_)) || matches!(b, Ty::Fb(_)) => {
                self.error(
                    Code::OperandTypes,
                    span,
                    format!("{sym} does not compare function block instances ({a} and {b}): compare their variables"),
                );
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
            if key == "SUPER" {
                return self.super_body(args, span);
            }
            if !self.locals.contains_key(&key) {
                if let Some(pou) = self.pous.get(&key).copied() {
                    return match pou.kind {
                        PouKind::Function => {
                            let ret = match &pou.return_type {
                                Some(rt) => self.resolve_spec(rt, false),
                                None => Ty::Void,
                            };
                            self.call_args(&key, Callee::Function, args, span);
                            self.record_call(CallTo::Exactly(key.clone()), span);
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
                        PouKind::Class | PouKind::Method => {
                            self.error(
                                Code::BadCall,
                                id.span,
                                format!("'{}' is a CLASS: declare an instance and call its methods", id.text),
                            );
                            self.args_only(args);
                            Ty::Unknown
                        }
                        PouKind::Program => {
                            self.error(Code::BadCall, id.span, format!("'{}' is a PROGRAM: a task runs it", id.text));
                            self.args_only(args);
                            Ty::Unknown
                        }
                        PouKind::Interface => {
                            self.error(
                                Code::BadCall,
                                id.span,
                                format!("'{}' is an INTERFACE: declare a variable of it and call its methods", id.text),
                            );
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
                if key == "LOWER_BOUND" || key == "UPPER_BOUND" {
                    return self.bound_call(&key, args, span);
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
        let t = match &callee.kind {
            ExprKind::Member(base, m) => {
                if matches!(&base.kind, ExprKind::Name(s) if s.is("SUPER")) {
                    return self.super_method(m, args, span);
                }
                let bt = self.expr(base);
                if let Ty::Fb(owner) = &bt
                    && let Some((def, method)) = self.find_method(owner, &m.text)
                {
                    return self.method_call(&def, method, m, args, span);
                }
                // Through an interface: bound at run time to the method of
                // the instance it references (Ed.3 6.6.6.5.2).
                if let Ty::Itf(i) = &bt
                    && let Some((def, proto)) = self.itf_method(i, &upper(&m.text))
                {
                    let key = crate::method_key(&def, &m.text);
                    let ret = match &proto.return_type {
                        Some(rt) => self.resolve_spec(rt, false),
                        None => Ty::Void,
                    };
                    self.call_args(&key, Callee::Function, args, span);
                    self.out.method_calls.insert(span, key);
                    self.record_call(CallTo::Itf(i.clone(), upper(&m.text)), span);
                    return ret;
                }
                let t = self.member(&bt, m);
                self.out.exprs.insert(callee.span, t.clone());
                t
            }
            _ => self.expr(callee),
        };
        match t {
            Ty::Fb(fb) if self.pous.get(&fb).is_some_and(|p| p.kind == PouKind::Class) => {
                self.error(
                    Code::BadCall,
                    callee.span,
                    format!("an instance of the CLASS {fb} is not called: call one of its methods (Ed.3 6.6.5.2)"),
                );
                self.args_only(args);
                Ty::Unknown
            }
            Ty::Fb(fb) => {
                if let Some(n) = self.input_instance(callee) {
                    self.error(
                        Code::BadCall,
                        callee.span,
                        format!(
                            "'{n}' is a function block instance given as an input: it is read, not called (Ed.3 6.6.3.4 rule 7: one given as a VAR_IN_OUT is called)"
                        ),
                    );
                }
                self.call_args(&fb, Callee::FunctionBlock, args, span);
                self.record_call(CallTo::Body(fb.clone()), span);
                Ty::Void
            }
            Ty::Unknown => {
                self.args_only(args);
                Ty::Unknown
            }
            // `F(n - 1)` in F's own body: F there is its result.
            other
                if matches!(&callee.kind, ExprKind::Name(id)
                if upper(&id.text) == self.pou_key
                    || self.pou_key.ends_with(&format!("__{}", upper(&id.text)))) =>
            {
                self.error(
                    Code::BadCall,
                    callee.span,
                    format!(
                        "{} here is the function's result, a {other}: a FUNCTION or method does not call itself - recursion, which Ed.3 6.6.1 leaves to the implementer, iecf does not allow (docs/extensions.md)",
                        callee_name(callee)
                    ),
                );
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

    /// The base type SUPER names here: in a method or body of a derived
    /// type (Ed.3 6.6.5.7.3), else an error at `at`.
    fn super_type(&mut self, at: Span, what: &str) -> Option<String> {
        let base = self.this_type.as_ref().and_then(|t| self.bases.get(t)).cloned();
        if base.is_none() {
            self.error(
                Code::BadObject,
                at,
                format!("SUPER is the base type's: {what} in a type that EXTENDS one (Ed.3 6.6.5.7.3)"),
            );
        }
        base
    }

    /// `SUPER.m(...)`: the base's method, bound statically (Ed.3
    /// 6.6.5.7.3), its access checked as the base's.
    fn super_method(&mut self, m: &Ident, args: &[Arg], span: Span) -> Ty {
        let Some(base) = self.super_type(span, "SUPER.m() calls a method") else {
            self.args_only(args);
            return Ty::Unknown;
        };
        let Some((def, method)) = self.find_method(&base, &m.text) else {
            self.error(Code::UnknownName, m.span, format!("{base} has no method '{}' for SUPER to call", m.text));
            self.args_only(args);
            return Ty::Unknown;
        };
        if method.specs.abstract_.is_some() {
            self.error(
                Code::BadObject,
                m.span,
                format!("{def}.{} is ABSTRACT: it has no statements for SUPER to run (Ed.3 6.6.5.8.3)", m.text),
            );
        }
        self.static_call = true;
        self.method_call(&def, method, m, args, span)
    }

    /// `SUPER()` in a derived block's body: the base's body, once and
    /// not in a loop, never in a method (Ed.3 6.6.7.2.9).
    fn super_body(&mut self, args: &[Arg], span: Span) -> Ty {
        self.args_only(args);
        let Some(base) = self.super_type(span, "SUPER() runs the base's body") else { return Ty::Void };
        let why = if self.pou_kind != Some(PouKind::FunctionBlock) {
            Some("SUPER() is for a function block's body, not a method (Ed.3 6.6.7.2.9 rule 5)".to_string())
        } else if self.pous.get(&base).is_some_and(|p| p.kind == PouKind::Class) {
            Some(format!("{base} is a CLASS: it has no body for SUPER() to run (Ed.3 6.6.5.2)"))
        } else if !args.is_empty() {
            Some(
                "SUPER() has no parameters: the base's body runs on the instance's own inputs (Ed.3 6.6.7.2.9 rule 2)"
                    .to_string(),
            )
        } else if self.loop_depth > 0 {
            Some("SUPER() is not in a loop (Ed.3 6.6.7.2.9)".to_string())
        } else if self.super_body.is_some() {
            Some("SUPER() is called once in a body (Ed.3 6.6.7.2.9)".to_string())
        } else {
            None
        };
        match why {
            Some(w) => self.error(Code::BadObject, span, w),
            None => self.super_body = Some(span),
        }
        self.record_call(CallTo::Exactly(base), span);
        Ty::Void
    }

    /// `inst.m(...)` or `THIS.m(...)`: the method's access checked, its
    /// arguments as a FUNCTION's (Ed.3 6.6.5.4.5, Table 50).
    fn method_call(&mut self, owner: &str, method: &Pou, m: &Ident, args: &[Arg], span: Span) -> Ty {
        let (access, written) = method.specs.access.map_or((Access::Protected, false), |(a, _)| (a, true));
        if !self.may_use(owner, access) {
            let none = if written { "" } else { " (none written: Ed.3 6.6.5.9)" };
            let who = match access {
                Access::Private => format!("only {owner} itself calls it"),
                Access::Protected => format!("only {owner} and the types derived from it call it"),
                _ => format!("only {owner}'s namespace calls it"),
            };
            self.error(
                Code::AccessDenied,
                m.span,
                format!(
                    "{owner}.{} is {}{none}: {who}; write METHOD PUBLIC {} to call it from here",
                    m.text,
                    access.name(),
                    m.text
                ),
            );
        }
        let key = crate::method_key(owner, &m.text);
        let ret = match &method.return_type {
            Some(rt) => self.resolve_spec(rt, false),
            None => Ty::Void,
        };
        self.call_args(&key, Callee::Function, args, span);
        let to = if std::mem::take(&mut self.static_call) {
            CallTo::Exactly(key.clone())
        } else {
            CallTo::Method(owner.to_string(), upper(&m.text))
        };
        self.record_call(to, span);
        self.out.method_calls.insert(span, key);
        ret
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

    /// An enumeration's values are numbered from 0 in an INT
    /// (docs/dependencies.md): the first it cannot number is an error.
    fn enum_fits(&mut self, values: &[EnumValue]) {
        if let Some(v) = values.get(32768) {
            self.error(
                Code::LiteralRange,
                v.name.span,
                format!(
                    "'{}' is this enumeration's 32769th value: an enumeration is held in an INT, which numbers 32768",
                    v.name.text
                ),
            );
        }
    }

    /// Whether a value of `ty` is a function block instance or holds
    /// one, in an array or a structure.
    fn holds_instance(&self, ty: &Ty) -> bool {
        fn find(c: &Checker, ty: &Ty, depth: u32) -> bool {
            match ty {
                _ if depth > 64 => false,
                Ty::Fb(_) => true,
                Ty::Array { elem, .. } => find(c, elem, depth + 1),
                Ty::Struct(s) => c.struct_fields.get(s).is_some_and(|fs| fs.values().any(|f| find(c, f, depth + 1))),
                _ => false,
            }
        }
        find(self, ty, 0)
    }

    /// The input's name when `e`, already checked, is a function block
    /// instance in an input (`Var::inst_in`), a part of one, or a part of
    /// the input that holds one.
    fn input_instance(&self, e: &Expr) -> Option<String> {
        let mut inside = self.out.exprs.get(&e.span).is_some_and(|t| self.holds_instance(t));
        let mut at = e;
        loop {
            inside |= matches!(self.out.exprs.get(&at.span), Some(Ty::Fb(_)));
            match &at.kind {
                ExprKind::Name(id) => {
                    let v = self.locals.get(&upper(&id.text))?;
                    return (v.inst_in && inside).then(|| id.text.clone());
                }
                ExprKind::Paren(x) | ExprKind::Index(x, _) | ExprKind::Member(x, _) => at = x,
                _ => return None,
            }
        }
    }

    /// An `ARRAY[*]` input or output is the caller's array, so a call
    /// gives one: there is none to stand in for it (Siemens, where an
    /// FC's parameters are all supplied; docs/dependencies.md).
    fn flex_given(&mut self, key: &str, params: &[Param], given: impl Fn(&Param) -> bool, span: Span) {
        for p in params.iter().filter(|p| p.dir != Dir::InOut && is_flex(&p.ty)) {
            if !given(p) {
                let (what, how) = if p.dir == Dir::Input { ("input", ":=") } else { ("output", "=>") };
                self.error(
                    Code::BadCall,
                    span,
                    format!(
                        "ARRAY[*] {what} '{}' of {key} must be given an array ({} {how} ...): it is the caller's array, not a copy",
                        p.name, p.name
                    ),
                );
            }
        }
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
            // A FUNCTION's and a function block's call may give its
            // arguments in order (Ed.3 6.6.1.4.2, 6.6.3.4.1: "including
            // the formal and the non-formal call"); a program's
            // connections are named (6.8.2, Prog_Cnxn).
            if kind == Callee::Program {
                self.error(Code::BadCall, span, "a program's connections are given by name: IN := ..., OUT => ...");
                self.args_only(args);
                return;
            }
            let ins: Vec<&Param> = params.iter().filter(|p| p.dir != Dir::Output).collect();
            if args.len() < ins.len() || args.len() > params.len() {
                self.error(
                    Code::BadCall,
                    span,
                    format!(
                        "{key} takes {} argument(s) in order{}, not {} (Ed.3 6.6.1.4)",
                        ins.len(),
                        match params.len() - ins.len() {
                            0 => String::new(),
                            n => format!(", then its {n} output(s)"),
                        },
                        args.len()
                    ),
                );
            }
            let ordered: Vec<&Param> = params
                .iter()
                .filter(|p| p.dir != Dir::Output)
                .chain(params.iter().filter(|p| p.dir == Dir::Output))
                .collect();
            let got: Vec<String> = ordered.iter().take(args.len()).map(|p| p.name.clone()).collect();
            self.flex_given(key, &params, |p| got.contains(&p.name), span);
            for (a, p) in args.iter().zip(ordered) {
                let Arg::Positional(e) = a else { continue };
                if p.dir == Dir::Input {
                    let t = self.expr(e);
                    if !flex_takes(&p.ty, &t) {
                        self.coerce(&t, &p.ty, e.span);
                    }
                } else {
                    let t = self.place(e);
                    if let (Dir::InOut, Ty::Fb(d), Ty::Fb(b)) = (p.dir, &t, &p.ty)
                        && self.may_stand_for(d, b, e.span)
                    {
                        continue;
                    }
                    if p.dir == Dir::InOut && self.is_string_char(e) {
                        self.error(
                            Code::NotAssignable,
                            e.span,
                            format!(
                                "in-out '{}' takes a variable; one character of a string is not one (Ed.3 6.4.2: a write of it is a REPLACE)",
                                p.name
                            ),
                        );
                    }
                    if flex_takes(&p.ty, &t) {
                        continue;
                    }
                    // An ARRAY[*] is not given to an in-out of fixed bounds.
                    if p.dir == Dir::InOut && matches!(&t, Ty::Array { dims, .. } if dims.iter().any(Option::is_none)) {
                        self.coerce(&t, &p.ty, e.span);
                        continue;
                    }
                    // An in-out is given a variable of its type exactly, a
                    // STRING of its length (by name, below, the same).
                    if p.dir == Dir::InOut {
                        self.in_out_type(&p.name, &p.ty, &t, e.span);
                        continue;
                    }
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
                        if !flex_takes(&p.ty, &t) {
                            self.coerce(&t, &p.ty, e.span);
                        }
                    }
                    Dir::InOut => {
                        let t = self.place(e);
                        if self.is_string_char(e) {
                            self.error(
                                Code::NotAssignable,
                                e.span,
                                format!(
                                    "in-out '{}' takes a variable; one character of a string is not one (Ed.3 6.4.2: a write of it is a REPLACE)",
                                    name.text
                                ),
                            );
                        } else if let (Ty::Fb(d), Ty::Fb(b)) = (&t, &p.ty)
                            && self.may_stand_for(d, b, e.span)
                        {
                        } else if flex_takes(&p.ty, &t) {
                        } else {
                            self.in_out_type(&name.text, &p.ty, &t, e.span);
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
                    if !flex_takes(&p.ty, &t) {
                        self.coerce(&p.ty, &t, target.span);
                    }
                }
                Arg::Positional(_) => {}
            }
        }
        self.flex_given(key, &params, |p| seen.contains_key(&p.name), span);
        // Every in-out "properly mapped" (Ed.3 6.6.1.4 rule 2: "of any
        // call within a POU"): a function's and a function block's at
        // each call - a block does not keep the variable an earlier call
        // gave it - and a program's in its connections.
        for p in params.iter().filter(|p| p.dir == Dir::InOut) {
            if !seen.contains_key(&p.name) {
                let how = match kind {
                    Callee::Program => "connected to a global variable".to_string(),
                    Callee::FunctionBlock => {
                        format!("passed at each call ({} := a variable)", p.name.to_ascii_lowercase())
                    }
                    Callee::Function => "passed".to_string(),
                };
                self.error(Code::BadCall, span, format!("in-out '{}' of {key} must be {how}", p.name));
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

/// `byte 3`, `bytes 3 to 6`: where `n` bytes from `from` are.
fn span_of_bytes(from: u64, n: u64) -> String {
    if n <= 1 { format!("byte {from}") } else { format!("bytes {from} to {}", from + n - 1) }
}

fn init_span(i: &Init) -> Span {
    match i {
        Init::Expr(e) => e.span,
        Init::Array { span, .. } | Init::Struct { span, .. } => *span,
    }
}

/// Where an access path has got to.
enum Reached {
    /// A resource of the configuration, by its index.
    Resource(usize),
    /// A program instance: its PROGRAM.
    Program(String),
    /// A variable: its type, the section it is declared in, CONSTANT.
    Variable(Ty, Option<VarKind>, bool),
}

impl<'a> Checker<'a> {
    /// VAR_ACCESS (Ed.3 6.8.1): each entry's path leads to a variable an
    /// access path may name, of the type written; a CONSTANT is
    /// READ_ONLY. `c` for a CONFIGURATION's entries, `program` for a
    /// PROGRAM's own.
    pub(crate) fn check_accesses(&mut self, entries: &[AccessDecl], c: Option<&Configuration>, program: Option<&str>) {
        let mut seen: HashMap<String, Span> = HashMap::new();
        for a in entries {
            if seen.insert(upper(&a.name.text), a.name.span).is_some() {
                self.error(Code::Duplicate, a.name.span, format!("access name '{}' is declared twice", a.name.text));
            }
            let written = self.resolve_spec(&a.ty, true);
            // A directly represented variable: the type written is its
            // type, of the address's size.
            if let ExprKind::DirectAddress(addr) = &a.path.kind {
                if let Some(r) = &a.resource {
                    let known =
                        c.is_some_and(|c| c.resources.iter().any(|x| x.name.as_ref().is_some_and(|n| n.is(&r.text))));
                    if !known {
                        self.error(Code::UnknownName, r.span, format!("no resource '{}'", r.text));
                    }
                }
                if addr.is_partly() || addr.is_relative() {
                    self.error(
                        Code::BadAccessPath,
                        addr.span,
                        format!("an access path names a complete address, as %IX0.0, not {}", addr.text),
                    );
                    continue;
                }
                self.check_location_size(addr, &written);
                self.out.accesses.push(crate::AccessPath {
                    config: c.map(|c| upper(&c.name.text)),
                    program: program.map(str::to_string),
                    name: upper(&a.name.text),
                    path: addr.text.to_ascii_uppercase(),
                    ty: written.to_string(),
                    read_write: a.read_write,
                });
                continue;
            }
            let mut path = String::new();
            let Some(reached) = self.access_walk(&a.path, c, program, &mut path) else { continue };
            let (ty, kind, constant) = match reached {
                Reached::Variable(t, k, c) => (t, k, c),
                Reached::Resource(_) | Reached::Program(_) => {
                    let what =
                        if matches!(reached, Reached::Resource(_)) { "a resource" } else { "a program instance" };
                    self.error(
                        Code::BadAccessPath,
                        a.path.span,
                        format!("'{path}' is {what}: an access path names a variable in it"),
                    );
                    continue;
                }
            };
            let refused = match kind {
                Some(VarKind::Temp) => Some("VAR_TEMP"),
                Some(VarKind::External) => Some("VAR_EXTERNAL"),
                Some(VarKind::InOut) => Some("VAR_IN_OUT"),
                _ => None,
            };
            if let Some(what) = refused {
                self.error(
                    Code::BadAccessPath,
                    a.path.span,
                    format!("'{path}' is a {what} variable: an access path leads to none (Ed.3 6.8.1)"),
                );
                continue;
            }
            if !ty.is_unknown() && !written.is_unknown() && ty != written {
                self.error(
                    Code::TypeMismatch,
                    a.ty.span(),
                    format!("'{path}' is declared {ty}: VAR_ACCESS repeats its type, not {written}"),
                );
            }
            if constant && a.read_write {
                self.error(
                    Code::BadAccessPath,
                    a.span,
                    format!("'{path}' is CONSTANT: its access path is READ_ONLY (Ed.3 6.8.1)"),
                );
            }
            self.out.accesses.push(crate::AccessPath {
                config: c.map(|c| upper(&c.name.text)),
                program: program.map(str::to_string),
                name: upper(&a.name.text),
                path,
                ty: ty.to_string(),
                read_write: a.read_write,
            });
        }
    }

    /// The variable `name` of the POU `key`, or of a base of it; of a
    /// standard block, its input or output.
    fn access_member(&mut self, key: &str, name: &Ident) -> Option<(Ty, Option<VarKind>, bool)> {
        let chain: Vec<String> = std::iter::once(key.to_string()).chain(self.ancestors(key)).collect();
        for k in chain {
            let Some(pou) = self.pous.get(&k).copied() else { continue };
            for b in &pou.var_blocks {
                if let Some(d) = b.decls.iter().find(|d| d.names.iter().any(|n| n.is(&name.text))) {
                    return Some((self.resolve_spec(&d.ty, false), Some(b.kind), b.constant));
                }
            }
        }
        std_fb(key)?.into_iter().find(|p| name.is(p.name)).map(|p| {
            let kind = match p.dir {
                Dir::Input => VarKind::Input,
                Dir::Output => VarKind::Output,
                Dir::InOut => VarKind::InOut,
            };
            (p.ty, Some(kind), false)
        })
    }

    /// A global of `blocks` by name.
    fn access_global(&mut self, blocks: &[VarBlock], name: &Ident) -> Option<Reached> {
        for b in blocks {
            if let Some(d) = b.decls.iter().find(|d| d.names.iter().any(|n| n.is(&name.text))) {
                return Some(Reached::Variable(self.resolve_spec(&d.ty, false), None, b.constant));
            }
        }
        None
    }

    /// Follows `e`, an access path, writing it into `path` as the symbol
    /// table names it. None when it leads nowhere, reported.
    fn access_walk(
        &mut self,
        e: &Expr,
        c: Option<&Configuration>,
        program: Option<&str>,
        path: &mut String,
    ) -> Option<Reached> {
        match &e.kind {
            ExprKind::Paren(x) => self.access_walk(x, c, program, path),
            ExprKind::Name(id) => {
                path.push_str(&upper(&id.text));
                // A PROGRAM's own entry starts at one of its variables.
                if let Some(p) = program {
                    return match self.access_member(p, id) {
                        Some((t, k, k2)) => Some(Reached::Variable(t, k, k2)),
                        None => {
                            self.error(Code::UnknownName, id.span, format!("{p} has no variable '{}'", id.text));
                            None
                        }
                    };
                }
                let c = c?;
                if let Some(v) = self.access_global(&c.globals, id) {
                    return Some(v);
                }
                if let Some(i) = c.resources.iter().position(|r| r.name.as_ref().is_some_and(|n| n.is(&id.text))) {
                    return Some(Reached::Resource(i));
                }
                // No resource named: the one there is.
                if let [r] = c.resources.as_slice()
                    && r.name.is_none()
                    && let Some(p) = r.programs.iter().find(|p| p.name.is(&id.text))
                {
                    return Some(Reached::Program(upper(&p.program_type.text)));
                }
                self.error(
                    Code::UnknownName,
                    id.span,
                    format!(
                        "no global variable, resource or program instance '{}' in CONFIGURATION {}",
                        id.text, c.name.text
                    ),
                );
                None
            }
            ExprKind::Member(b, id) => {
                let at = self.access_walk(b, c, program, path)?;
                let before = path.clone();
                path.push('.');
                path.push_str(&upper(&id.text));
                match at {
                    Reached::Resource(i) => {
                        let r = &c?.resources[i];
                        if let Some(v) = self.access_global(&r.globals, id) {
                            return Some(v);
                        }
                        if let Some(p) = r.programs.iter().find(|p| p.name.is(&id.text)) {
                            return Some(Reached::Program(upper(&p.program_type.text)));
                        }
                        self.error(
                            Code::UnknownName,
                            id.span,
                            format!("resource {before} has no global variable or program instance '{}'", id.text),
                        );
                        None
                    }
                    Reached::Program(p) => match self.access_member(&p, id) {
                        Some((t, k, k2)) => Some(Reached::Variable(t, k, k2)),
                        None => {
                            self.error(Code::UnknownName, id.span, format!("{p} has no variable '{}'", id.text));
                            None
                        }
                    },
                    Reached::Variable(Ty::Fb(k), kind, _) => {
                        // What holds it says whether a path may pass.
                        if matches!(kind, Some(VarKind::Temp | VarKind::External | VarKind::InOut)) {
                            return Some(Reached::Variable(Ty::Unknown, kind, false));
                        }
                        match self.access_member(&upper(&k), id) {
                            Some((t, k, k2)) => Some(Reached::Variable(t, k, k2)),
                            None => {
                                self.error(Code::UnknownName, id.span, format!("{k} has no variable '{}'", id.text));
                                None
                            }
                        }
                    }
                    Reached::Variable(Ty::Struct(k), kind, constant) => {
                        let field = match self.types.get(&upper(&k)).map(|t| &t.def) {
                            Some(TypeDef::Struct { fields, .. }) => {
                                fields.iter().find(|d| d.names.iter().any(|n| n.is(&id.text)))
                            }
                            _ => None,
                        };
                        match field {
                            Some(d) => Some(Reached::Variable(self.resolve_spec(&d.ty, false), kind, constant)),
                            None => {
                                self.error(Code::UnknownName, id.span, format!("{k} has no field '{}'", id.text));
                                None
                            }
                        }
                    }
                    Reached::Variable(Ty::Unknown, ..) => None,
                    Reached::Variable(t, ..) => {
                        self.error(Code::UnknownName, id.span, format!("'{before}' is {t}: it has no parts to name"));
                        None
                    }
                }
            }
            ExprKind::Index(b, idx) => {
                let at = self.access_walk(b, c, program, path)?;
                let Reached::Variable(Ty::Array { elem, dims }, kind, constant) = at else {
                    self.error(Code::OperandTypes, e.span, format!("'{path}' is not an array: it has no elements"));
                    return None;
                };
                if idx.len() != dims.len() {
                    self.error(
                        Code::OperandTypes,
                        e.span,
                        format!("'{path}' has {} dimension(s), not {}", dims.len(), idx.len()),
                    );
                    return None;
                }
                let mut written = Vec::new();
                for (x, d) in idx.iter().zip(&dims) {
                    let Some(v) = self.const_int(x) else {
                        self.error(
                            Code::NotConstant,
                            x.span,
                            "an access path names one element: its index is a constant integer expression",
                        );
                        return None;
                    };
                    if let Some((lo, hi)) = d
                        && (v < *lo || v > *hi)
                    {
                        self.error(Code::LiteralRange, x.span, format!("the index {v} is outside {lo}..{hi}"));
                        return None;
                    }
                    written.push(v.to_string());
                }
                path.push_str(&format!("[{}]", written.join(",")));
                Some(Reached::Variable(*elem, kind, constant))
            }
            ExprKind::BitAccess(b, lit) => {
                let at = self.access_walk(b, c, program, path)?;
                let Reached::Variable(t, kind, constant) = at else {
                    self.error(Code::OperandTypes, e.span, format!("'{path}' has no bits to name"));
                    return None;
                };
                let (size, n) = lit.part()?;
                let part = match size {
                    PartSize::Bit => (Elem::Bool, 'X'),
                    PartSize::Byte => (Elem::Byte, 'B'),
                    PartSize::Word => (Elem::Word, 'W'),
                    PartSize::Dword => (Elem::Dword, 'D'),
                    PartSize::Lword => (Elem::Lword, 'L'),
                };
                if !(t.is_bit() || t.is_int()) || t.is_bool() {
                    self.error(
                        Code::OperandTypes,
                        e.span,
                        format!("'{path}' is {t}: a part is taken of a bit string or an integer (Ed.3 Table 17)"),
                    );
                    return None;
                }
                path.push_str(&format!(".%{}{n}", part.1));
                Some(Reached::Variable(Ty::Elem(part.0), kind, constant))
            }
            _ => {
                self.error(
                    Code::BadAccessPath,
                    e.span,
                    "an access path is a variable's name: through a resource, a program instance and instances, to a \
                     variable, a field, an element or a part of one (Ed.3 6.8.1)",
                );
                None
            }
        }
    }
}

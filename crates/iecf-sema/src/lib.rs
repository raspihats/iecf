//! iecf's checker: names and types across a project's files.
//!
//! The rules are Ed.3's where Ed.4 is not yet bought (PROPOSAL.md §3a):
//! implicit conversions by Figure 11, names by 6.1.2 and 6.1.3, calls by
//! 6.6.1.4, globals by 6.5.2.2.

mod check;
pub mod datetime;
pub mod namespaces;
pub mod stdlib;
pub mod strings;
#[cfg(test)]
mod tests;
pub mod types;

use iecf_diagnostics::{Diagnostic, Span};
use iecf_syntax::ast::SourceUnit;
use std::collections::HashMap;
use types::Ty;

/// An implementer extension (Ed.3 clause 5 c): something the standard
/// prohibits or does not specify, which iecf accepts only when asked.
/// Each is off by default and described in `docs/extensions.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Extension {
    /// A POU uses a global variable without declaring it VAR_EXTERNAL.
    ImplicitExternals,
    /// VAR_CONFIG at the top of a file, each PROGRAM its own instance.
    VarConfigList,
    /// TIME and LTIME converted to and from integers and bit strings,
    /// counting milliseconds (nanoseconds for LTIME).
    TimeIntegerConversions,
    /// BYTE, WORD, DWORD and LWORD to BOOL: TRUE for any value but 0.
    BitStringToBool,
    /// CODESYS's REFERENCE TO, set with REF= and tested with __ISVALIDREF.
    ReferenceTo,
    /// `_.` names the global namespace, as Siemens and PLCnext write it.
    GlobalNamespace,
}

impl Extension {
    pub const ALL: &'static [Extension] = &[
        Extension::ImplicitExternals,
        Extension::VarConfigList,
        Extension::TimeIntegerConversions,
        Extension::BitStringToBool,
        Extension::ReferenceTo,
        Extension::GlobalNamespace,
    ];

    /// As the command line and `docs/extensions.md` spell it.
    pub fn name(self) -> &'static str {
        match self {
            Extension::ImplicitExternals => "implicit-externals",
            Extension::VarConfigList => "var-config-list",
            Extension::TimeIntegerConversions => "time-integer-conversions",
            Extension::BitStringToBool => "bit-string-to-bool",
            Extension::ReferenceTo => "reference-to",
            Extension::GlobalNamespace => "global-namespace",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Extension::ImplicitExternals => "a POU uses a global variable without declaring it VAR_EXTERNAL",
            Extension::VarConfigList => "VAR_CONFIG outside a CONFIGURATION, paths from the PROGRAM's name",
            Extension::TimeIntegerConversions => "TIME and LTIME to and from integers, in ms and ns",
            Extension::BitStringToBool => "BYTE, WORD, DWORD and LWORD to BOOL: TRUE for any value but 0",
            Extension::ReferenceTo => "CODESYS's REFERENCE TO, bound with REF=, tested with __ISVALIDREF",
            Extension::GlobalNamespace => "`_.` names the global namespace, as Siemens and PLCnext write it",
        }
    }

    pub fn from_name(name: &str) -> Option<Extension> {
        Extension::ALL.iter().copied().find(|e| e.name().eq_ignore_ascii_case(name))
    }
}

/// How to check. The default is the standard as written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    extensions: Vec<Extension>,
}

impl Options {
    pub fn with(mut self, e: Extension) -> Self {
        if !self.extensions.contains(&e) {
            self.extensions.push(e);
        }
        self
    }

    pub fn has(&self, e: Extension) -> bool {
        self.extensions.contains(&e)
    }
}

/// Check a project by the standard as written: every file's tree,
/// parsed. Names declared in one file are seen in the others.
pub fn check(units: &[&SourceUnit]) -> Vec<Diagnostic> {
    check_with(units, &Options::default())
}

/// Check a project with the extensions `options` switches on.
pub fn check_with(units: &[&SourceUnit], options: &Options) -> Vec<Diagnostic> {
    analyze(units, options).diagnostics
}

/// What the checker found and what it worked out: the diagnostics, and
/// the types the code generator needs.
pub struct Analysis {
    pub diagnostics: Vec<Diagnostic>,
    pub types: Types,
}

/// Check a project and keep what the check worked out.
pub fn analyze(units: &[&SourceUnit], options: &Options) -> Analysis {
    let mut c = check::Checker::new(options.clone());
    c.collect(units);
    c.check_all(units);
    let (mut diagnostics, types) = c.finish();
    diagnostics.sort_by_key(|d| (d.span.file, d.span.start, d.span.end));
    Analysis { diagnostics, types }
}

/// Check a project as the build does: its namespaces taken out first
/// (namespaces.rs), which rewrites `units`, then [`analyze`].
pub fn analyze_project(units: &mut [SourceUnit], options: &Options) -> Analysis {
    let mut diagnostics = namespaces::flatten(units, options);
    let refs: Vec<&SourceUnit> = units.iter().collect();
    let mut analysis = analyze(&refs, options);
    diagnostics.append(&mut analysis.diagnostics);
    diagnostics.sort_by_key(|d| (d.span.file, d.span.start, d.span.end));
    analysis.diagnostics = diagnostics;
    analysis
}

/// The types the check worked out, by the span of what they belong to.
/// An expression's type is its own, before its context converts it: an
/// untyped literal is [`Ty::IntLit`] or [`Ty::RealLit`].
#[derive(Debug, Default)]
pub struct Types {
    exprs: HashMap<Span, Ty>,
    decls: HashMap<Span, Ty>,
    std_calls: HashMap<Span, StdCall>,
    implicit_eno: std::collections::HashSet<String>,
    refs: HashMap<Span, RefUse>,
    implicit_externals: HashMap<String, Vec<(String, bool)>>,
    /// Each method call, by the call's span: the method's key (`method_key`).
    method_calls: HashMap<Span, String>,
    /// Each `THIS.x` that is the instance's own `x`, by its span.
    this_members: std::collections::HashSet<Span>,
    /// Each CLASS's and FUNCTION_BLOCK's base type, by upper-cased key.
    pub(crate) bases: HashMap<String, String>,
    /// Each INTERFACE's bases (EXTENDS), and each CLASS's and
    /// FUNCTION_BLOCK's interfaces (IMPLEMENTS), by upper-cased key.
    pub(crate) itf_bases: HashMap<String, Vec<String>>,
    pub(crate) implements: HashMap<String, Vec<String>>,
    /// The value of each constant expression the standard requires one
    /// for (array bounds, a STRING's length, a subrange's limits), by
    /// its span: one that names a CONSTANT too.
    pub(crate) consts: HashMap<Span, i128>,
    /// Which globals each POU's VAR_EXTERNALs reach where it runs, by
    /// configuration and POU (a method's under its type), then name:
    /// those its code reaches itself, and those the instances it creates
    /// as it runs reach (Ed.3 6.5.2.2).
    pub(crate) bindings:
        HashMap<(String, String), std::collections::BTreeMap<String, std::collections::BTreeSet<Binding>>>,
    /// The VAR_ACCESS entries, checked (Ed.3 6.8.1).
    pub(crate) accesses: Vec<AccessPath>,
}

/// A VAR_ACCESS entry: the access name, the variable's path as the
/// symbol table names it (`R.P.FB.X`, `G[2].F`, `%IX1.0`), its type and
/// direction. A CONFIGURATION's is whole; a PROGRAM's starts at the
/// program's variable, each instance's path to go before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPath {
    /// The configuration, upper-cased; none for a PROGRAM's.
    pub config: Option<String>,
    /// The PROGRAM, upper-cased; none for a CONFIGURATION's.
    pub program: Option<String>,
    pub name: String,
    pub path: String,
    pub ty: String,
    pub read_write: bool,
}

/// The global a VAR_EXTERNAL reaches in one place its POU runs (Ed.3
/// 6.5.2.2), by upper-cased names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Binding {
    /// A VAR_GLOBAL at the top of a file, a global variable list.
    Top(String),
    /// The configuration's own.
    Config(String),
    /// A resource's: the resource (empty when the configuration names
    /// none), and the global.
    Resource(String, String),
    /// A PROGRAM's own VAR_GLOBAL, which each instance of it has: the
    /// resource, the program instance, and the global.
    Program(String, String, String),
}

impl Types {
    /// The VAR_ACCESS entries, in the order they are written.
    pub fn accesses(&self) -> &[AccessPath] {
        &self.accesses
    }

    /// The globals the VAR_EXTERNAL `name` of the POU `key` (a method's:
    /// its type's) reaches in the CONFIGURATION `config`, where it runs
    /// there: one, or more when its instances run in places that name
    /// different ones.
    pub fn bindings(&self, config: &str, key: &str, name: &str) -> Option<&std::collections::BTreeSet<Binding>> {
        self.bindings.get(&(config.to_ascii_uppercase(), key.to_ascii_uppercase()))?.get(&name.to_ascii_uppercase())
    }

    /// The names whose bindings are known for `key` in `config`.
    pub fn bound_names(&self, config: &str, key: &str) -> Vec<String> {
        self.bindings
            .get(&(config.to_ascii_uppercase(), key.to_ascii_uppercase()))
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// `e` without the name of its own resource: `R.g` as `g`, `R.g.x` as
/// `g.x` (Ed.3 6.8.2, Global_Var_Access), where `R` is `resource`, the
/// one the connection or TASK is written in, and `shadowed` does not
/// say a global variable is named so - then `R.x` is its field. The
/// name keeps the whole access's span. None when `e` is not such an
/// access.
pub fn own_resource_access(
    e: &iecf_syntax::ast::Expr,
    resource: Option<&str>,
    shadowed: &dyn Fn(&str) -> bool,
) -> Option<iecf_syntax::ast::Expr> {
    use iecf_syntax::ast::{Expr, ExprKind};
    let resource = resource?;
    let named = |x: &Expr| matches!(&x.kind, ExprKind::Name(n) if n.text.eq_ignore_ascii_case(resource) && !shadowed(&n.text.to_ascii_uppercase()));
    let ExprKind::Member(b, m) = &e.kind else { return None };
    if named(b) {
        return Some(Expr {
            kind: ExprKind::Name(iecf_syntax::ast::Ident { span: e.span, ..m.clone() }),
            span: e.span,
        });
    }
    if let ExprKind::Member(r, g) = &b.kind
        && named(r)
    {
        let inner = Expr { kind: ExprKind::Name(iecf_syntax::ast::Ident { span: b.span, ..g.clone() }), span: b.span };
        return Some(Expr { kind: ExprKind::Member(Box::new(inner), m.clone()), span: e.span });
    }
    None
}

/// The type a method belongs to, from its key (`method_key`); a POU's
/// own key otherwise. IEC names hold no `__`.
pub fn owner_of(key: &str) -> &str {
    key.split("__").next().unwrap_or(key)
}

/// A method's key, as a FUNCTION's is its name: its type's key and its
/// own upper-cased, joined by `__`, which no IEC name holds.
pub fn method_key(owner: &str, method: &str) -> String {
    format!("{}__{}", owner.to_ascii_uppercase(), method.to_ascii_uppercase())
}

/// What the generator does with an expression where a REFERENCE TO (the
/// reference-to extension) is used: the checker decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefUse {
    /// A REFERENCE TO standing for what it references: as `r^`.
    Deref,
    /// A variable a REFERENCE TO is bound to, by REF= or a call: as `REF(x)`.
    Bind,
    /// `r REF= 0`: unbound, as `NULL`.
    Unbind,
}

impl Types {
    /// Whether the `THIS.x` with this span is the instance's own `x`,
    /// generated as `x` is.
    pub fn this_member(&self, span: Span) -> bool {
        self.this_members.contains(&span)
    }

    /// Whether anything is generated differently from how it is written:
    /// a REFERENCE TO used, or a `THIS.x` (codegen's reference.rs).
    pub fn rewrites(&self) -> bool {
        !self.refs.is_empty() || !self.this_members.is_empty()
    }

    /// The base type `key` EXTENDS, upper-cased.
    pub fn base(&self, key: &str) -> Option<&str> {
        self.bases.get(key).map(String::as_str)
    }

    /// `key`'s bases, the nearest first.
    pub fn ancestors(&self, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut at = key;
        while let Some(b) = self.bases.get(at) {
            out.push(b.clone());
            at = b;
        }
        out
    }

    /// Whether `derived` is `base` or derives from it.
    pub fn derives(&self, derived: &str, base: &str) -> bool {
        derived == base || self.ancestors(derived).iter().any(|a| a == base)
    }

    /// The types that extend `key`, directly or not, in key order.
    pub fn descendants(&self, key: &str) -> Vec<String> {
        let mut out: Vec<String> =
            self.bases.keys().filter(|k| k.as_str() != key && self.derives(k, key)).cloned().collect();
        out.sort();
        out
    }

    /// The interface `key` and every one it extends, directly or not:
    /// `key` first, each once.
    pub fn itf_line(&self, key: &str) -> Vec<String> {
        let mut out = vec![key.to_string()];
        let mut i = 0;
        while i < out.len() {
            for b in self.itf_bases.get(&out[i]).into_iter().flatten() {
                if !out.contains(b) {
                    out.push(b.clone());
                }
            }
            i += 1;
        }
        out
    }

    /// The interfaces the CLASS or FUNCTION_BLOCK `key` implements: its
    /// own, its bases', and those they extend, in key order.
    pub fn implemented(&self, key: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for t in std::iter::once(key.to_string()).chain(self.ancestors(key)) {
            for i in self.implements.get(&t).into_iter().flatten() {
                for j in self.itf_line(i) {
                    if !out.contains(&j) {
                        out.push(j);
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// The method the call with this span calls, by its key.
    pub fn method_call(&self, span: Span) -> Option<&str> {
        self.method_calls.get(&span).map(String::as_str)
    }

    /// The type of the expression with this span.
    pub fn expr(&self, span: Span) -> Option<&Ty> {
        self.exprs.get(&span)
    }

    /// The type of the variable declaration with this span (`VarDecl::span`),
    /// or of a FUNCTION's result (its `TypeSpec::span`).
    pub fn decl(&self, span: Span) -> Option<&Ty> {
        self.decls.get(&span)
    }

    /// Whether the FUNCTION or FUNCTION_BLOCK `pou` (upper-cased) uses
    /// ENO without declaring it (Ed.3 6.6.1.5).
    pub fn implicit_eno(&self, pou: &str) -> bool {
        self.implicit_eno.contains(pou)
    }

    /// The globals the POU `pou` (upper-cased) uses without declaring
    /// them (the implicit-externals extension), upper-cased, each with
    /// whether it is CONSTANT: as if it declared them VAR_EXTERNAL.
    pub fn implicit_externals(&self, pou: &str) -> &[(String, bool)] {
        self.implicit_externals.get(pou).map_or(&[], Vec::as_slice)
    }

    /// What a REFERENCE TO's use at this span is (reference-to extension).
    pub fn ref_use(&self, span: Span) -> Option<RefUse> {
        self.refs.get(&span).copied()
    }

    /// Whether the project uses a REFERENCE TO anywhere.
    pub fn has_ref_uses(&self) -> bool {
        !self.refs.is_empty()
    }

    /// The call of a standard function with this span, as bound.
    /// The value of the constant expression at `span`, where the
    /// standard requires one (`consts`).
    pub fn const_value(&self, span: Span) -> Option<i128> {
        self.consts.get(&span).copied()
    }

    pub fn std_call(&self, span: Span) -> Option<&StdCall> {
        self.std_calls.get(&span)
    }
}

/// A call of a standard function, its arguments bound to its inputs and
/// outputs (Ed.3 6.6.1.4) and its overloading resolved.
#[derive(Clone, Debug)]
pub struct StdCall {
    /// The function, upper-cased, a typed form's types left off (6.6.2.4):
    /// `ADD` for `ADD_INT`, `MUX` for `MUX_UINT_REAL`; `MAX`,
    /// `INT_TO_DINT`, `ADD_TOD_TIME` as called.
    pub name: String,
    /// As called, upper-cased: `ADD_INT`.
    pub called: String,
    /// Each input in order (`IN1`, `IN2`, ... or `IN`, `N`), and the span
    /// of the argument given to it.
    pub inputs: Vec<(String, Option<Span>)>,
    pub outputs: Vec<(String, Option<Span>)>,
    /// The type the common inputs share, or the typed form's type.
    pub common: Ty,
    /// MUX's K, when the typed form names it.
    pub typed_k: Option<Ty>,
}

/// The name of an enumeration declared in place, `x : (a, b, c)`: its
/// values' names. One with named values (Ed.3 6.4.4.3: a base type, or a
/// number given) is its declaration's own, marked by where it is. The
/// checker's types and the generator both know it by this.
pub fn inplace_enum_name(
    base: Option<&iecf_syntax::ast::Ident>,
    values: &[iecf_syntax::ast::EnumValue],
    span: iecf_diagnostics::Span,
) -> String {
    let names = values.iter().map(|v| v.name.text.as_str()).collect::<Vec<_>>().join(", ");
    if base.is_none() && values.iter().all(|v| v.value.is_none()) {
        format!("({names})")
    } else {
        format!("({names})#{}", span.start)
    }
}

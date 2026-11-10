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

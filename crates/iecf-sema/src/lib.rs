//! iecf's checker: names and types across a project's files.
//!
//! The rules are Ed.3's where Ed.4 is not yet bought (PROPOSAL.md §3a):
//! implicit conversions by Figure 11, names by 6.1.2 and 6.1.3, calls by
//! 6.6.1.4, globals by 6.5.2.2.

mod check;
pub mod stdlib;
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
}

impl Extension {
    pub const ALL: &'static [Extension] =
        &[Extension::ImplicitExternals, Extension::VarConfigList, Extension::TimeIntegerConversions];

    /// As the command line and `docs/extensions.md` spell it.
    pub fn name(self) -> &'static str {
        match self {
            Extension::ImplicitExternals => "implicit-externals",
            Extension::VarConfigList => "var-config-list",
            Extension::TimeIntegerConversions => "time-integer-conversions",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Extension::ImplicitExternals => "a POU uses a global variable without declaring it VAR_EXTERNAL",
            Extension::VarConfigList => "VAR_CONFIG outside a CONFIGURATION, paths from the PROGRAM's name",
            Extension::TimeIntegerConversions => "TIME and LTIME to and from integers, in ms and ns",
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

/// The types the check worked out, by the span of what they belong to.
/// An expression's type is its own, before its context converts it: an
/// untyped literal is [`Ty::IntLit`] or [`Ty::RealLit`].
#[derive(Debug, Default)]
pub struct Types {
    exprs: HashMap<Span, Ty>,
    decls: HashMap<Span, Ty>,
    std_calls: HashMap<Span, StdCall>,
}

impl Types {
    /// The type of the expression with this span.
    pub fn expr(&self, span: Span) -> Option<&Ty> {
        self.exprs.get(&span)
    }

    /// The type of the variable declaration with this span (`VarDecl::span`).
    pub fn decl(&self, span: Span) -> Option<&Ty> {
        self.decls.get(&span)
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
    /// As called, upper-cased: `ADD_INT`, `MAX`, `INT_TO_DINT`.
    pub name: String,
    /// Each input in order (`IN1`, `IN2`, ... or `IN`, `N`), and the span
    /// of the argument given to it.
    pub inputs: Vec<(String, Option<Span>)>,
    pub outputs: Vec<(String, Option<Span>)>,
    /// The type the common inputs share, or the typed form's type.
    pub common: Ty,
    /// MUX's K, when the typed form names it.
    pub typed_k: Option<Ty>,
}

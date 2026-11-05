//! iecf's checker: names and types across a project's files.
//!
//! The rules are Ed.3's where Ed.4 is not yet bought (PROPOSAL.md §3a):
//! implicit conversions by Figure 11, names by 6.1.2 and 6.1.3, calls by
//! 6.6.1.4, globals by 6.5.2.2.

mod check;
pub mod stdlib;
pub mod types;

use iecf_diagnostics::Diagnostic;
use iecf_syntax::ast::SourceUnit;

/// Check a project: every file's tree, parsed. Names declared in one
/// file are seen in the others.
pub fn check(units: &[&SourceUnit]) -> Vec<Diagnostic> {
    let mut c = check::Checker::new();
    c.collect(units);
    c.check_all(units);
    let mut diags = c.finish();
    diags.sort_by_key(|d| (d.span.file, d.span.start, d.span.end));
    diags
}

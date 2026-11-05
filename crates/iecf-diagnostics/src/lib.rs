//! Source positions and diagnostics for iecf.
//!
//! Every diagnostic carries a [`Code`]. Each code has one markdown file in
//! `docs/errors/`, compiled in here: the file is the code's explanation
//! (`iecf explain`), and the set of files is the document of detectable
//! errors that IEC 61131-3 Ed.3 clause 5 e asks an implementer for. A
//! code without its file does not compile.

mod code;
mod render;
mod source;

pub use code::Code;
pub use render::{render_json, render_short};
pub use source::{FileId, LineIndex, SourceFile, SourceMap, Span};

/// How bad a diagnostic is. Ordered: `Warning < Error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// One finding: what, where, how bad, and which code explains it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    /// A diagnostic at the code's default severity.
    pub fn new(code: Code, span: Span, message: impl Into<String>) -> Self {
        Diagnostic { code, severity: code.default_severity(), message: message.into(), span }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

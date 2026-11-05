//! A set of IEC 61131-3 source files in, every diagnostic out: what
//! `iecf check` does, without the files being files. The command line
//! reads them from disk, the browser hands them over from its editor
//! (PROPOSAL.md §2a), and both run this, so both say the same.
//!
//! No I/O here, as in the whole core: it builds for wasm32.

use iecf_diagnostics::{Code, Diagnostic, SourceMap, Span, render_json, render_short};
pub use iecf_sema::{Extension, Options};

/// What a check found, with the files it was found in.
pub struct Report {
    pub map: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// The JSON array `iecf check --format json` prints.
    pub fn to_json(&self) -> String {
        render_json(&self.map, &self.diagnostics)
    }

    /// `file:line:col: error[E0005]: ...`, one line each.
    pub fn to_short(&self) -> String {
        render_short(&self.map, &self.diagnostics)
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Check a project: `files` are `(name, text)`, in any order; a name
/// declared in one is seen in the others (§2b). A `.il` file is
/// Instruction List, which Ed.4 removed: E0008, and not read further.
pub fn check<N, T>(files: impl IntoIterator<Item = (N, T)>, options: &Options) -> Report
where
    N: Into<String>,
    T: Into<String>,
{
    let mut map = SourceMap::new();
    let mut diagnostics = Vec::new();
    let mut units = Vec::new();
    for (name, text) in files {
        let name: String = name.into();
        let is_il = name.to_ascii_lowercase().ends_with(".il");
        let id = map.add(name, text);
        if is_il {
            diagnostics.push(Diagnostic::new(
                Code::InstructionList,
                Span::new(id, 0, 0),
                "this is an Instruction List file; IL was removed from IEC 61131-3 in Ed.4: rewrite it in ST",
            ));
            continue;
        }
        let parsed = iecf_syntax::parse(id, &map.get(id).text);
        diagnostics.extend(parsed.diagnostics);
        units.push(parsed.unit);
    }
    let refs: Vec<&iecf_syntax::ast::SourceUnit> = units.iter().collect();
    diagnostics.extend(iecf_sema::check_with(&refs, options));
    Report { map, diagnostics }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_cross_files_and_json_names_the_file() {
        let files = [
            ("gvl.st", "VAR_GLOBAL g : INT; END_VAR\n"),
            ("main.st", "PROGRAM main\nVAR_EXTERNAL g : INT; END_VAR\ng := nope;\nEND_PROGRAM\n"),
        ];
        let report = check(files, &Options::default());
        assert_eq!(report.to_short(), "main.st:3:6: error[E0010]: 'nope' is not declared\n");
        assert!(report.to_json().contains("\"file\":\"main.st\""), "{}", report.to_json());
        assert!(report.has_errors());
    }

    #[test]
    fn instruction_list_is_refused_by_name() {
        let report = check([("old.IL", "LD x\n")], &Options::default());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, Code::InstructionList);
    }
}

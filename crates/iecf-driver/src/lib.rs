//! A set of IEC 61131-3 source files in, every diagnostic out: what
//! `iecf check` does, without the files being files. The command line
//! reads them from disk, the browser hands them over from its editor
//! (PROPOSAL.md §2a), and both run this, so both say the same.
//!
//! No I/O here, as in the whole core: it builds for wasm32.

use iecf_diagnostics::{Code, Diagnostic, SourceMap, Span, render_json, render_short};
pub use iecf_sema::{Extension, GvlFile, Library, Options, Pack};

/// iecf's version and the commit it was built from, as semantic
/// versioning's build metadata: `0.0.1+3f2a9c1d0b7e`, with `.dirty.` and a
/// hash of the uncommitted changes when there are some (build.rs). What
/// `iecf --version` prints and the browser's module says: a platform that
/// runs both compares them, and two builds of one tree say the same.
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("IECF_COMMIT"));

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
///
/// What only the generator finds - a constant expression's value that
/// its type does not hold, a part of the language not compiled yet - is
/// said too, the C written in memory and dropped: `iecf check` and the
/// browser's check say what `iecf build` would. Only for a project with
/// a CONFIGURATION, which says what is generated; files without one, a
/// library's, are checked, not generated.
pub fn check<N, T>(files: impl IntoIterator<Item = (N, T)>, options: &Options) -> Report
where
    N: Into<String>,
    T: Into<String>,
{
    let (mut report, units, types) = front(files, options);
    let configured = units.iter().flat_map(|u| &u.items).any(|i| matches!(i, iecf_syntax::ast::Item::Configuration(_)));
    if configured && !report.has_errors() {
        let _ = generate(&mut report, &units, &types, options);
    }
    report
}

/// What `iecf build` makes: the C files (docs/c-output.md), with the
/// warnings met on the way.
pub struct Built {
    pub files: Vec<(String, String)>,
    pub report: Report,
}

/// Check a project and, if it has no error, write its C: `(name, text)`
/// for `<config>.h`, `<config>.c`, `iecf.h` and `<config>.symbols.json`. A part of the language the
/// generator does not cover yet is E0009 where it stands.
pub fn build<N, T>(files: impl IntoIterator<Item = (N, T)>, options: &Options) -> Result<Built, Report>
where
    N: Into<String>,
    T: Into<String>,
{
    let (mut report, units, types) = front(files, options);
    if report.has_errors() {
        return Err(report);
    }
    match generate(&mut report, &units, &types, options) {
        Some(files) => Ok(Built { files, report }),
        None => Err(report),
    }
}

/// The C of a project the checker passed, or None with the generator's
/// diagnostics added to `report`.
fn generate(
    report: &mut Report,
    units: &[iecf_syntax::ast::SourceUnit],
    types: &iecf_sema::Types,
    options: &Options,
) -> Option<Vec<(String, String)>> {
    let refs: Vec<&iecf_syntax::ast::SourceUnit> = units.iter().collect();
    match iecf_codegen::generate_with(&refs, types, &report.map, options.debug()) {
        Ok(files) => Some(files),
        Err(mut diags) => {
            shown(&mut diags);
            report.diagnostics.extend(diags);
            report.diagnostics.sort_by_key(|d| (d.span.file, d.span.start, d.span.end));
            None
        }
    }
}

fn front<N, T>(
    files: impl IntoIterator<Item = (N, T)>,
    options: &Options,
) -> (Report, Vec<iecf_syntax::ast::SourceUnit>, iecf_sema::Types)
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
        let id = map.add(name.clone(), text);
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
        units.push((name, id, parsed.unit));
    }
    // Global variable lists (`--gvl`): each file's place, for gvls.rs.
    let lists: Vec<iecf_sema::gvls::Gvl> = options
        .gvls()
        .iter()
        .flat_map(|g| {
            units
                .iter()
                .enumerate()
                .filter(|(_, (name, ..))| under(name, &g.path))
                .map(|(unit, _)| iecf_sema::gvls::Gvl { name: &g.name, qualified: g.qualified, unit })
        })
        .collect();
    let mut units = libraries(units, options);
    iecf_sema::gvls::rewrite(&mut units, &lists);
    let mut analysis = iecf_sema::analyze_project(&mut units, options);
    // Not the parser's: a name it quotes is as written, `a__b` too.
    shown(&mut analysis.diagnostics);
    diagnostics.extend(analysis.diagnostics);
    (Report { map, diagnostics }, units, analysis.types)
}

/// Whether `file` is `path`, or under the directory `path`.
fn under(file: &str, path: &str) -> bool {
    let path = path.trim_end_matches('/');
    file == path || file.starts_with(&format!("{path}/"))
}

/// Libraries (`--library NAME=PATH`), as CODESYS and TwinCAT have them:
/// each file under a library's path has its types and POUs (PROGRAMs
/// too, as two libraries may each have a MAIN) put in a NAMESPACE of the
/// library's name (Ed.3 6.9) - its global variables and configurations
/// stay where they are - and every file of the project uses that namespace,
/// as CODESYS lets a library's elements be named without it, unless the
/// library is qualified-only (docs/behaviour/namespaces.md).
fn libraries(
    units: Vec<(String, iecf_diagnostics::FileId, iecf_syntax::ast::SourceUnit)>,
    options: &Options,
) -> Vec<iecf_syntax::ast::SourceUnit> {
    use iecf_syntax::ast::{Ident, Item, Namespace, Using};
    let libs = options.libraries();
    if libs.is_empty() {
        return units.into_iter().map(|(_, _, u)| u).collect();
    }
    let mut used: Vec<&Library> = Vec::new();
    let mut out = Vec::new();
    let mut ids = Vec::new();
    for (name, id, mut unit) in units {
        if let Some(lib) = libs.iter().find(|l| under(&name, &l.path)) {
            let at = Span::new(id, 0, 0);
            let (inner, outer): (Vec<Item>, Vec<Item>) = std::mem::take(&mut unit.items)
                .into_iter()
                .partition(|i| matches!(i, Item::Types(_) | Item::Namespace(_) | Item::Pou(_)));
            unit.items = outer;
            if !inner.is_empty() {
                unit.items.push(Item::Namespace(Namespace {
                    name: lib.name.split('.').map(|n| Ident { text: n.to_string(), span: at }).collect(),
                    internal: false,
                    using: Vec::new(),
                    items: inner,
                    span: at,
                }));
                if !used.iter().any(|l| l.name == lib.name) {
                    used.push(lib);
                }
            }
        }
        ids.push(id);
        out.push(unit);
    }
    // Each file uses each library that has elements, but the
    // qualified-only ones.
    for (unit, id) in out.iter_mut().zip(ids) {
        let at = Span::new(id, 0, 0);
        for lib in used.iter().filter(|l| !l.qualified) {
            let path = lib.name.split('.').map(|n| Ident { text: n.to_string(), span: at }).collect();
            unit.items.insert(0, Item::Using(Using { paths: vec![path], span: at }));
        }
    }
    out
}

/// A namespace's element is named by its whole path joined by `__` once
/// its namespaces are taken out (iecf-sema namespaces.rs); a message
/// gives it as written, `Standard.Timers.TON`.
fn shown(diagnostics: &mut [Diagnostic]) {
    for d in diagnostics {
        d.message = iecf_sema::namespaces::display(&d.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_is_a_namespace_used_everywhere() {
        let files = [
            ("core/obj.st", "FUNCTION_BLOCK Obj\nVAR_OUTPUT q : INT; END_VAR\nq := 1;\nEND_FUNCTION_BLOCK\n"),
            (
                "app/main.st",
                "FUNCTION_BLOCK M EXTENDS Core.Obj\nVAR_OUTPUT r : INT; END_VAR\nr := q;\nEND_FUNCTION_BLOCK\n\
                 PROGRAM main\nVAR mm : M; o : Obj; END_VAR\nmm();\nEND_PROGRAM\n",
            ),
        ];
        let errors = |o: &Options| check(files, o).diagnostics.iter().filter(|d| d.is_error()).count();
        // Without it Core is nothing; with it both names; qualified-only,
        // the short one is not.
        assert!(errors(&Options::default()) > 0);
        assert_eq!(errors(&Options::default().with_library("Core", "core", false)), 0);
        assert_eq!(errors(&Options::default().with_library("Core", "core/", true)), 1);
    }

    #[test]
    fn a_library_s_programs_are_in_its_namespace() {
        let main = "PROGRAM MAIN\nVAR n : INT; END_VAR\nn := 1;\nEND_PROGRAM\n";
        let files = [("a/main.st", main), ("b/main.st", main), ("main.st", main)];
        let errors = |o: &Options| check(files, o).diagnostics.iter().filter(|d| d.is_error()).count();
        // Three MAINs in one namespace; then A's and B's each in its own.
        assert_eq!(errors(&Options::default()), 2);
        assert_eq!(errors(&Options::default().with_library("A", "a", false).with_library("B", "b", true)), 0);
    }

    #[test]
    fn a_gvl_is_named_by_the_command_line() {
        let files = [
            ("gvl/io.st", "VAR_GLOBAL a : INT; END_VAR\n"),
            ("main.st", "PROGRAM main\nVAR x : INT; END_VAR\nx := IO.a;\nx := a;\nEND_PROGRAM\n"),
        ];
        let o = Options::default().with(Extension::ImplicitExternals);
        let errors = |o: &Options| check(files, o).diagnostics.iter().filter(|d| d.is_error()).count();
        // Without it IO is nothing; with it both names; qualified-only,
        // the plain one is not.
        assert_eq!(errors(&o), 1);
        assert_eq!(errors(&o.clone().with_gvl("IO", "gvl/io.st", false)), 0);
        assert_eq!(errors(&o.clone().with_gvl("IO", "gvl", true)), 1);
    }

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
    fn a_check_says_what_the_generator_finds_where_a_configuration_says_what_runs() {
        let main = "PROGRAM main\nVAR CONSTANT K : INT := 300; END_VAR\nVAR x : INT := K * 200; END_VAR\nx := x;\nEND_PROGRAM\n";
        let config = "CONFIGURATION c RESOURCE r ON x TASK t(INTERVAL := T#10ms, PRIORITY := 1); \
                      PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION\n";
        let report = check([("main.st", main), ("c.st", config)], &Options::default());
        assert_eq!(
            report.to_short(),
            "main.st:3:16: error[E0020]: this constant expression is 60000, which INT does not hold\n"
        );
        // Without one there is nothing to generate: checked only.
        assert!(check([("main.st", main)], &Options::default()).diagnostics.is_empty());
    }

    #[test]
    fn instruction_list_is_refused_by_name() {
        let report = check([("old.IL", "LD x\n")], &Options::default());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].code, Code::InstructionList);
    }
}

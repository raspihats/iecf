//! `iecf`: the command line.
//!
//!     iecf check [--format short|json] [--extension NAME]... FILE...
//!     iecf parse FILE                            print the syntax tree
//!     iecf explain CODE                          explain a diagnostic code

use iecf_diagnostics::{Code, Diagnostic, SourceMap, Span, render_json, render_short};
use std::process::ExitCode;

const USAGE: &str = "\
iecf - the IEC Forge, an IEC 61131-3 compiler (front end only, for now)

usage:
    iecf check [--format short|json] [--extension NAME]... FILE...
                                               report every diagnostic
    iecf parse FILE                            print the syntax tree
    iecf explain CODE                          explain a code, e.g. E0005
    iecf --version

extensions, off by default (docs/extensions.md):
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..]),
        Some("parse") => parse(&args[1..]),
        Some("explain") => explain(&args[1..]),
        Some("--version" | "-V") => {
            println!("iecf {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help" | "-h" | "help") => {
            print!("{USAGE}");
            print!("{}", extension_list());
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Read a file into the map; `None` after saying why it could not be.
fn load(map: &mut SourceMap, path: &str) -> Option<iecf_diagnostics::FileId> {
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Some(map.add(path, text)),
            Err(_) => {
                eprintln!("iecf: {path}: not UTF-8 text");
                None
            }
        },
        Err(e) => {
            eprintln!("iecf: {path}: {e}");
            None
        }
    }
}

fn extension_list() -> String {
    iecf_sema::Extension::ALL.iter().map(|e| format!("    {:<20} {}\n", e.name(), e.summary())).collect()
}

fn check(args: &[String]) -> ExitCode {
    let mut json = false;
    let mut options = iecf_sema::Options::default();
    let mut files = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--format" => match it.next().map(String::as_str) {
                Some("json") => json = true,
                Some("short") => json = false,
                other => {
                    eprintln!("iecf: --format takes short or json, not {}", other.unwrap_or("nothing"));
                    return ExitCode::from(2);
                }
            },
            "--extension" => match it.next().map(|n| (n, iecf_sema::Extension::from_name(n))) {
                Some((_, Some(e))) => options = options.with(e),
                other => {
                    eprintln!(
                        "iecf: --extension takes one of these, not {}:\n{}",
                        other.map_or("nothing", |(n, _)| n.as_str()),
                        extension_list()
                    );
                    return ExitCode::from(2);
                }
            },
            _ => files.push(a.clone()),
        }
    }
    if files.is_empty() {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut map = SourceMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut units = Vec::new();
    let mut unreadable = false;
    for path in &files {
        let Some(id) = load(&mut map, path) else {
            unreadable = true;
            continue;
        };
        if path.to_ascii_lowercase().ends_with(".il") {
            diagnostics.push(Diagnostic::new(
                Code::InstructionList,
                Span::new(id, 0, 0),
                "this is an Instruction List file; IL was removed from IEC 61131-3 in Ed.4: rewrite it in ST",
            ));
            continue;
        }
        let text = map.get(id).text.clone();
        let parsed = iecf_syntax::parse(id, &text);
        diagnostics.extend(parsed.diagnostics);
        units.push(parsed.unit);
    }
    // Names and types across all the files: a name declared in one is
    // seen in the others.
    let refs: Vec<&iecf_syntax::ast::SourceUnit> = units.iter().collect();
    diagnostics.extend(iecf_sema::check_with(&refs, &options));
    if json {
        println!("{}", render_json(&map, &diagnostics));
    } else {
        print!("{}", render_short(&map, &diagnostics));
    }
    if unreadable || diagnostics.iter().any(Diagnostic::is_error) { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

fn parse(args: &[String]) -> ExitCode {
    let [path] = args else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut map = SourceMap::new();
    let Some(id) = load(&mut map, path) else { return ExitCode::FAILURE };
    let text = map.get(id).text.clone();
    let parsed = iecf_syntax::parse(id, &text);
    println!("{:#?}", parsed.unit);
    eprint!("{}", render_short(&map, &parsed.diagnostics));
    if parsed.diagnostics.iter().any(Diagnostic::is_error) { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

fn explain(args: &[String]) -> ExitCode {
    let [id] = args else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    match Code::from_id(id) {
        Some(code) => {
            print!("{}", code.explanation());
            ExitCode::SUCCESS
        }
        None => {
            eprintln!(
                "iecf: no code {id}; codes are {}",
                Code::ALL.iter().map(|c| c.id()).collect::<Vec<_>>().join(", ")
            );
            ExitCode::FAILURE
        }
    }
}

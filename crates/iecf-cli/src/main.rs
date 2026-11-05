//! `iecf`: the command line.
//!
//!     iecf check [--format short|json] [--extension NAME]... FILE...
//!     iecf parse FILE                            print the syntax tree
//!     iecf explain CODE                          explain a diagnostic code

use iecf_diagnostics::{Code, Diagnostic, SourceMap, render_short};
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

/// A file's text; `None` after saying why it could not be read.
fn read(path: &str) -> Option<String> {
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => Some(text),
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
    iecf_driver::Extension::ALL.iter().map(|e| format!("    {:<20} {}\n", e.name(), e.summary())).collect()
}

fn check(args: &[String]) -> ExitCode {
    let mut json = false;
    let mut options = iecf_driver::Options::default();
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
            "--extension" => match it.next().map(|n| (n, iecf_driver::Extension::from_name(n))) {
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
    let mut sources = Vec::new();
    let mut unreadable = false;
    for path in &files {
        match read(path) {
            Some(text) => sources.push((path.clone(), text)),
            None => unreadable = true,
        }
    }
    // The same entry point as the browser's (PROPOSAL.md §2a).
    let report = iecf_driver::check(sources, &options);
    if json {
        println!("{}", report.to_json());
    } else {
        print!("{}", report.to_short());
    }
    if unreadable || report.has_errors() { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

fn parse(args: &[String]) -> ExitCode {
    let [path] = args else {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    };
    let Some(text) = read(path) else { return ExitCode::FAILURE };
    let mut map = SourceMap::new();
    let id = map.add(path.as_str(), text.as_str());
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

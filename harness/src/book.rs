//! `iecf-harness book`: the behaviour reference's examples, checked
//! (docs/behaviour/overview.md). A table the line `<!-- checked -->` comes
//! before has the columns
//!
//! ```text
//! | Expression | Type | Result | Flag | ...anything else, not read |
//! ```
//!
//! and each row becomes a program of its own:
//!
//! ```text
//! PROGRAM main VAR ok AT %MX0.0 : BOOL; END_VAR VAR r, z : <Type>; END_VAR
//! r := <Expression>;                                            (line 2)
//! ok := <r is Result>;                                          (line 3)
//! END_PROGRAM
//! ```
//!
//! built with iecf, compiled as strict C99 and run one tick under the
//! harness's host. The row holds when `ok` is set and the flag or fault is
//! the one the row names, raised on line 2.
//!
//! Result is a literal of the type, or `NaN`, `+inf`, `-inf`, or
//! `unchanged` (the statement was left undone: `r` keeps its initial
//! value, which `z` shares), or `—` for a fault (nothing to compare).
//! Flag is `—`, `flag NAME` or `fault NAME`, NAME an `IECF_FAULT_*` of
//! runtime/iecf.h without the prefix.

use crate::{Built, Case, iecf_trace};
use std::fs;
use std::path::{Path, PathBuf};

const CODES: [(&str, u32); 10] = [
    ("DIVISION_BY_ZERO", 1),
    ("OVERFLOW", 2),
    ("INDEX", 3),
    ("CONVERSION", 4),
    ("MUX", 5),
    ("NEGATIVE", 6),
    ("STRING_LENGTH", 7),
    ("IMAGE", 8),
    ("NAN", 9),
    ("STRING_POSITION", 10),
];

struct Row {
    /// `file:line`, for the report.
    at: String,
    expr: String,
    ty: String,
    result: String,
    /// `None`, or ("flag" | "fault", code).
    flag: Option<(String, u32)>,
}

/// Every checked table's rows in the Markdown files under `dir`.
fn rows(dir: &Path) -> Result<Vec<Row>, String> {
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let text = fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            if lines[i].trim() != "<!-- checked -->" {
                i += 1;
                continue;
            }
            i += 1;
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }
            let head = cells(lines.get(i).copied().unwrap_or_default());
            let want = ["Expression", "Type", "Result", "Flag"];
            if head.len() < 4 || head[..4] != want {
                return Err(format!("{}:{}: a checked table starts | {} |", f.display(), i + 1, want.join(" | ")));
            }
            i += 2; // the header and the |---| line
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                let c = cells(lines[i]);
                let at = format!("{}:{}", f.display(), i + 1);
                if c.len() < 4 {
                    return Err(format!("{at}: four columns at least"));
                }
                let flag = match c[3].as_str() {
                    "—" | "-" | "" => None,
                    s => {
                        let (kind, name) =
                            s.split_once(' ').ok_or(format!("{at}: Flag is —, flag NAME or fault NAME"))?;
                        let code =
                            CODES.iter().find(|(n, _)| *n == name).ok_or(format!("{at}: no IECF_FAULT_{name}"))?.1;
                        if kind != "flag" && kind != "fault" {
                            return Err(format!("{at}: Flag is —, flag NAME or fault NAME"));
                        }
                        Some((kind.to_string(), code))
                    }
                };
                out.push(Row { at, expr: c[0].clone(), ty: c[1].clone(), result: c[2].clone(), flag });
                i += 1;
            }
        }
    }
    Ok(out)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let p = e.map_err(|e| e.to_string())?.path();
        if p.is_dir() {
            collect(&p, out)?;
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
    Ok(())
}

/// A table row's cells, trimmed, code marks taken off. `\|` is a `|`.
fn cells(line: &str) -> Vec<String> {
    let line = line.trim().trim_start_matches('|').trim_end_matches('|').replace("\\|", "\u{1}");
    line.split('|').map(|c| c.trim().trim_matches('`').replace('\u{1}', "|")).collect()
}

/// The ST that sets `ok` when `r` is what the row says.
fn check(row: &Row) -> String {
    match row.result.as_str() {
        "—" | "-" => "ok := TRUE".into(),
        "unchanged" => "ok := r = z".into(),
        "NaN" => "ok := r <> r".into(),
        // r - r is NaN for an infinity, 0 for any number.
        "+inf" => "ok := r > 0.0 AND r - r <> r - r".into(),
        "-inf" => "ok := r < 0.0 AND r - r <> r - r".into(),
        v => format!("ok := r = {v}"),
    }
}

fn program(row: &Row) -> String {
    format!(
        "PROGRAM main VAR ok AT %MX0.0 : BOOL; END_VAR VAR r, z : {}; END_VAR\nr := {};\n{};\nEND_PROGRAM\n\
         CONFIGURATION plant RESOURCE res ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 0);\n\
         PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION\n",
        row.ty,
        row.expr,
        check(row)
    )
}

/// What the row's run should print for its tick.
fn expected(row: &Row) -> String {
    let ok = if matches!(&row.flag, Some((k, _)) if k == "fault") { "00" } else { "01" };
    let mut s = format!("0 Q M {ok}");
    if let Some((kind, code)) = &row.flag {
        s.push_str(&format!(" {kind} {code} MAIN 2"));
    }
    s
}

/// Check every row under `docs`; print each that fails. True when all hold.
pub(crate) fn run(root: &Path, docs: &Path) -> Result<bool, String> {
    let rows = rows(docs)?;
    if rows.is_empty() {
        return Err(format!("no checked table under {}", docs.display()));
    }
    let work = root.join("../target/harness/book");
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let failures: Vec<String> = std::thread::scope(|s| {
        let chunks: Vec<_> = rows.chunks(rows.len().div_ceil(threads)).enumerate().collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|(t, chunk)| {
                let work = work.join(format!("w{t}"));
                s.spawn(move || {
                    let mut bad = Vec::new();
                    for row in chunk {
                        if let Err(e) = one(root, &work, row) {
                            bad.push(format!("{}: `{}` as {}: {e}", row.at, row.expr, row.ty));
                        }
                    }
                    bad
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_else(|_| vec!["a thread panicked".into()])).collect()
    });
    for f in &failures {
        println!("FAILED {f}");
    }
    println!("{} of {} examples hold", rows.len() - failures.len(), rows.len());
    Ok(failures.is_empty())
}

fn one(root: &Path, work: &Path, row: &Row) -> Result<(), String> {
    fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let src = work.join("src");
    fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    fs::write(src.join("row.st"), program(row)).map_err(|e| e.to_string())?;
    let case = Case { name: "row".into(), dir: src, script: "ticks 1\n".into() };
    let got = match iecf_trace(root, &work.join("build"), &case)? {
        Built::Trace(t) => t,
        Built::NotYet(why) => return Err(format!("not generated: {why}")),
    };
    let line = got.lines().nth(1).unwrap_or_default();
    let want = expected(row);
    if line != want {
        return Err(format!("ran `{line}`, the table says `{want}`"));
    }
    Ok(())
}

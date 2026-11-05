//! Two ways to print diagnostics: one line each for a terminal, and JSON
//! for the Workbench.

use crate::{Diagnostic, SourceMap};
use std::fmt::Write;

/// `file:line:col: error[E0005]: message`, one line per diagnostic, the
/// format editors and CI logs already understand.
pub fn render_short(map: &SourceMap, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for d in diagnostics {
        let file = map.get(d.span.file);
        let (line, col) = map.line_col(d.span.file, d.span.start);
        let _ =
            writeln!(out, "{}:{}:{}: {}[{}]: {}", file.name, line, col, d.severity.as_str(), d.code.id(), d.message);
    }
    out
}

/// A JSON array, one object per diagnostic, with both byte offsets and
/// 1-based line/column for start and end.
pub fn render_json(map: &SourceMap, diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("[");
    for (i, d) in diagnostics.iter().enumerate() {
        let file = map.get(d.span.file);
        let (line, col) = map.line_col(d.span.file, d.span.start);
        let (end_line, end_col) = map.line_col(d.span.file, d.span.end);
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"code\":\"{}\",\"severity\":\"{}\",\"message\":{},\"file\":{},\
             \"start\":{},\"end\":{},\"line\":{},\"column\":{},\"end_line\":{},\"end_column\":{}}}",
            d.code.id(),
            d.severity.as_str(),
            json_string(&d.message),
            json_string(&file.name),
            d.span.start,
            d.span.end,
            line,
            col,
            end_line,
            end_col
        );
    }
    out.push(']');
    out
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Code, Span};

    #[test]
    fn short_and_json() {
        let mut map = SourceMap::new();
        let f = map.add("a.st", "x\n  y \"q\"");
        let d = vec![Diagnostic::new(Code::UnexpectedToken, Span::new(f, 4, 5), "say \"hi\"")];
        assert_eq!(render_short(&map, &d), "a.st:2:3: error[E0005]: say \"hi\"\n");
        assert_eq!(
            render_json(&map, &d),
            "[{\"code\":\"E0005\",\"severity\":\"error\",\"message\":\"say \\\"hi\\\"\",\
             \"file\":\"a.st\",\"start\":4,\"end\":5,\"line\":2,\"column\":3,\"end_line\":2,\"end_column\":4}]"
        );
    }
}

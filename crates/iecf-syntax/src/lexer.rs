//! Text to tokens, hand-written so it has no dependencies and builds for
//! wasm. Every byte of the input lands in exactly one token, trivia
//! included, so the token spans tile the file.

use crate::token::{Kw, Token, TokenKind};
use iecf_diagnostics::{Code, Diagnostic, FileId, Span};

/// The tokens of `text`, ending with one `Eof`, and the lexical errors.
pub fn lex(file: FileId, text: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lx = Lexer { file, src: text.as_bytes(), pos: 0, tokens: Vec::new(), diags: Vec::new() };
    lx.run();
    (lx.tokens, lx.diags)
}

struct Lexer<'a> {
    file: FileId,
    src: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
}

/// Prefixes of the literals whose text after `#` is not a number: a
/// duration (`T#1h2m`) or a date and time (`DT#2026-11-05-12:00:00`).
const DURATION_PREFIXES: &[&str] = &["T", "TIME", "LT", "LTIME"];
const DATE_PREFIXES: &[&str] = &[
    "D",
    "DATE",
    "LD",
    "LDATE",
    "TOD",
    "TIME_OF_DAY",
    "LTOD",
    "LTIME_OF_DAY",
    "DT",
    "DATE_AND_TIME",
    "LDT",
    "LDATE_AND_TIME",
];

impl Lexer<'_> {
    fn peek(&self, ahead: usize) -> u8 {
        self.src.get(self.pos + ahead).copied().unwrap_or(0)
    }

    fn span(&self, start: usize) -> Span {
        Span::new(self.file, start as u32, self.pos as u32)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        let span = self.span(start);
        self.tokens.push(Token { kind, span });
    }

    fn error(&mut self, code: Code, start: usize, end: usize, message: impl Into<String>) {
        let span = Span::new(self.file, start as u32, end as u32);
        self.diags.push(Diagnostic::new(code, span, message));
    }

    fn text(&self, start: usize) -> &str {
        // Token boundaries are always on ASCII bytes, so this cannot split
        // a UTF-8 sequence.
        std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("")
    }

    fn run(&mut self) {
        while self.pos < self.src.len() {
            let start = self.pos;
            let kind = self.next_kind(start);
            self.push(kind, start);
        }
        let end = self.src.len();
        self.tokens.push(Token { kind: TokenKind::Eof, span: Span::new(self.file, end as u32, end as u32) });
    }

    fn next_kind(&mut self, start: usize) -> TokenKind {
        let c = self.peek(0);
        match c {
            b' ' | b'\t' | b'\r' | b'\n' | 0x0c => {
                while matches!(self.peek(0), b' ' | b'\t' | b'\r' | b'\n' | 0x0c) {
                    self.pos += 1;
                }
                TokenKind::Whitespace
            }
            b'(' if self.peek(1) == b'*' => self.block_comment(start, b'(', b')'),
            b'/' if self.peek(1) == b'*' => self.block_comment(start, b'/', b'/'),
            b'/' if self.peek(1) == b'/' => {
                while !matches!(self.peek(0), b'\n' | 0) {
                    self.pos += 1;
                }
                TokenKind::Comment
            }
            b'{' => self.pragma(start),
            b'\'' => self.string(start, b'\'', TokenKind::String),
            b'"' => self.string(start, b'"', TokenKind::WString),
            b'%' => self.direct_address(start),
            b'0'..=b'9' => self.number(start),
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => self.word(start),
            _ => self.punct(start),
        }
    }

    /// `(* ... *)` or `/* ... */`, nesting (Ed.3 Table 3, 3a and 3b).
    fn block_comment(&mut self, start: usize, open: u8, close: u8) -> TokenKind {
        self.pos += 2;
        let mut depth = 1;
        while self.pos < self.src.len() {
            if self.peek(0) == open && self.peek(1) == b'*' {
                depth += 1;
                self.pos += 2;
            } else if self.peek(0) == b'*' && self.peek(1) == close {
                depth -= 1;
                self.pos += 2;
                if depth == 0 {
                    return TokenKind::Comment;
                }
            } else {
                self.pos += 1;
            }
        }
        // Report once, at the opener, and let the comment run to the end:
        // guessing where it was meant to stop would invent errors.
        let opener = if open == b'(' { "(*" } else { "/*" };
        self.error(
            Code::UnterminatedComment,
            start,
            start + 2,
            format!("this comment is never closed: '{opener}' without its '*{}'", close as char),
        );
        TokenKind::Comment
    }

    fn pragma(&mut self, start: usize) -> TokenKind {
        let mut depth = 0;
        while self.pos < self.src.len() {
            match self.peek(0) {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += 1;
                        return TokenKind::Pragma;
                    }
                }
                _ => {}
            }
            self.pos += 1;
        }
        self.error(Code::UnterminatedComment, start, start + 1, "this pragma is never closed: '{' without its '}'");
        TokenKind::Pragma
    }

    /// A string literal; it ends on its own line. Escapes are checked
    /// here (Ed.3 Table 7, and Ed.4's `${...}`), decoded later.
    fn string(&mut self, start: usize, quote: u8, kind: TokenKind) -> TokenKind {
        self.pos += 1;
        let hex_len = if quote == b'"' { 4 } else { 2 };
        loop {
            if self.pos >= self.src.len() || matches!(self.peek(0), b'\n' | b'\r') {
                let which = if quote == b'"' { "wide string" } else { "string" };
                self.error(
                    Code::UnterminatedString,
                    start,
                    self.pos,
                    format!("this {which} is never closed: a {which} ends on the line it starts"),
                );
                return kind;
            }
            match self.peek(0) {
                b'$' => {
                    let esc = self.pos;
                    self.pos += 1;
                    let e = self.peek(0);
                    match e {
                        b'$' | b'\'' | b'"' | b'L' | b'l' | b'N' | b'n' | b'P' | b'p' | b'R' | b'r' | b'T' | b't' => {
                            self.pos += 1
                        }
                        b'{' => {
                            let mut n = 0;
                            self.pos += 1;
                            while self.peek(0).is_ascii_hexdigit() {
                                self.pos += 1;
                                n += 1;
                            }
                            if self.peek(0) == b'}' && (1..=6).contains(&n) {
                                self.pos += 1;
                            } else {
                                self.error(
                                    Code::InvalidEscape,
                                    esc,
                                    self.pos,
                                    "'${' takes one to six hex digits and a closing '}', e.g. '${1F579}'",
                                );
                            }
                        }
                        _ if (0..hex_len).all(|i| self.peek(i).is_ascii_hexdigit()) => self.pos += hex_len,
                        _ => {
                            let end = if e == 0 || e == b'\n' { self.pos } else { self.pos + 1 };
                            self.error(
                                Code::InvalidEscape,
                                esc,
                                end,
                                format!(
                                    "'$' starts an escape: $$, $', $\", $L, $N, $P, $R, $T, {hex_len} hex digits or ${{code}}"
                                ),
                            );
                        }
                    }
                }
                c if c == quote => {
                    self.pos += 1;
                    return kind;
                }
                _ => self.pos += 1,
            }
        }
    }

    /// `%` area size? number (`.` number)*, or `%I*` (Ed.3 6.5.5, Table 16).
    fn direct_address(&mut self, start: usize) -> TokenKind {
        self.pos += 1;
        while matches!(self.peek(0), b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'*') {
            // Stop before `..`, so `%IW0..` never eats a range.
            if self.peek(0) == b'.' && !self.peek(1).is_ascii_digit() {
                break;
            }
            self.pos += 1;
        }
        let text = self.text(start).to_ascii_uppercase();
        if !valid_direct_address(&text) {
            self.error(
                Code::InvalidLiteral,
                start,
                self.pos,
                if text.ends_with('*') {
                    format!(
                        "'{}' is not a direct address: '*' takes the place of the size and the numbers, as in %I*",
                        self.text(start)
                    )
                } else {
                    format!(
                        "'{}' is not a direct address: '%', then I, Q or M, then X, B, W, D or L, then numbers separated by '.'",
                        self.text(start)
                    )
                },
            );
        }
        TokenKind::DirectAddress
    }

    fn digits(&mut self) {
        while matches!(self.peek(0), b'0'..=b'9' | b'_') {
            self.pos += 1;
        }
    }

    fn number(&mut self, start: usize) -> TokenKind {
        self.digits();
        if self.peek(0) == b'#' {
            return self.based(start);
        }
        if self.peek(0) == b'.' && self.peek(1).is_ascii_digit() {
            self.pos += 1;
            self.digits();
            if matches!(self.peek(0), b'e' | b'E') {
                let sign = usize::from(matches!(self.peek(1), b'+' | b'-'));
                if self.peek(1 + sign).is_ascii_digit() {
                    self.pos += 1 + sign;
                    self.digits();
                }
            }
            return TokenKind::Real;
        }
        TokenKind::Int
    }

    /// `16#FF`, `2#1010`, `8#17` (Ed.3 Table 5).
    fn based(&mut self, start: usize) -> TokenKind {
        let base_text = self.text(start).replace('_', "");
        self.pos += 1;
        let digits_start = self.pos;
        while self.peek(0).is_ascii_alphanumeric() || self.peek(0) == b'_' {
            self.pos += 1;
        }
        let digits = self.text(digits_start).replace('_', "");
        let base: Option<u32> = match base_text.as_str() {
            "2" => Some(2),
            "8" => Some(8),
            "16" => Some(16),
            _ => None,
        };
        match base {
            None => self.error(
                Code::InvalidLiteral,
                start,
                self.pos,
                format!("base {base_text} does not exist: a based literal is 2#, 8# or 16#"),
            ),
            Some(_) if digits.is_empty() => {
                self.error(Code::InvalidLiteral, start, self.pos, format!("'{base_text}#' has no digits after it"))
            }
            Some(b) if !digits.chars().all(|c| c.is_digit(b)) => self.error(
                Code::InvalidLiteral,
                start,
                self.pos,
                format!("'{}' has a digit that base {b} does not have", self.text(start)),
            ),
            Some(8) => self.error(
                Code::OctalLiteral,
                start,
                self.pos,
                format!("'{}' is an octal literal, removed in Ed.4: write it in decimal or 16#", self.text(start)),
            ),
            Some(_) => {}
        }
        TokenKind::Based
    }

    fn word(&mut self, start: usize) -> TokenKind {
        while matches!(self.peek(0), b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_') {
            self.pos += 1;
        }
        let word = self.text(start).to_ascii_uppercase();
        if self.peek(0) == b'#' {
            if DURATION_PREFIXES.contains(&word.as_str()) {
                return self.time_literal(start, true);
            }
            if DATE_PREFIXES.contains(&word.as_str()) {
                return self.time_literal(start, false);
            }
        }
        match Kw::from_text(&word) {
            Some(kw) => TokenKind::Kw(kw),
            None => TokenKind::Ident,
        }
    }

    /// The body of a duration (`1h2m3s4ms`, `-5s`, `1.5s`) or of a date or
    /// time of day (`2026-11-05`, `12:00:00.5`, `2026-11-05-12:00`). Its
    /// shape is checked here; its value is computed later.
    fn time_literal(&mut self, start: usize, duration: bool) -> TokenKind {
        self.pos += 1;
        let body_start = self.pos;
        if duration {
            if matches!(self.peek(0), b'-' | b'+') {
                self.pos += 1;
            }
            while matches!(self.peek(0), b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'.') {
                self.pos += 1;
            }
        } else {
            while matches!(self.peek(0), b'0'..=b'9' | b'_' | b'-' | b':' | b'.') {
                self.pos += 1;
            }
        }
        let body = self.text(body_start).to_string();
        let ok = if duration { valid_duration(&body) } else { !body.is_empty() };
        if !ok {
            let what = if duration {
                "a duration such as T#1h2m3s or T#250ms"
            } else {
                "a date or time such as D#2026-11-05 or TOD#12:00:00"
            };
            self.error(Code::InvalidLiteral, start, self.pos, format!("'{}' is not {what}", self.text(start)));
        }
        TokenKind::Time
    }

    fn punct(&mut self, start: usize) -> TokenKind {
        let (kind, len) = match (self.peek(0), self.peek(1)) {
            (b':', b'=') => (TokenKind::Assign, 2),
            (b'=', b'>') => (TokenKind::Arrow, 2),
            (b'.', b'.') => (TokenKind::DotDot, 2),
            (b'*', b'*') => (TokenKind::Power, 2),
            (b'<', b'>') => (TokenKind::Ne, 2),
            (b'<', b'=') => (TokenKind::Le, 2),
            (b'>', b'=') => (TokenKind::Ge, 2),
            (b':', _) => (TokenKind::Colon, 1),
            (b';', _) => (TokenKind::Semi, 1),
            (b',', _) => (TokenKind::Comma, 1),
            (b'.', _) => (TokenKind::Dot, 1),
            (b'(', _) => (TokenKind::LParen, 1),
            (b')', _) => (TokenKind::RParen, 1),
            (b'[', _) => (TokenKind::LBracket, 1),
            (b']', _) => (TokenKind::RBracket, 1),
            (b'^', _) => (TokenKind::Caret, 1),
            (b'#', _) => (TokenKind::Hash, 1),
            (b'+', _) => (TokenKind::Plus, 1),
            (b'-', _) => (TokenKind::Minus, 1),
            (b'*', _) => (TokenKind::Star, 1),
            (b'/', _) => (TokenKind::Slash, 1),
            (b'=', _) => (TokenKind::Eq, 1),
            (b'<', _) => (TokenKind::Lt, 1),
            (b'>', _) => (TokenKind::Gt, 1),
            (b'&', _) => (TokenKind::Amp, 1),
            _ => {
                // One whole character, however many bytes it takes.
                let rest = std::str::from_utf8(&self.src[start..]).ok();
                let ch = rest.and_then(|s| s.chars().next()).unwrap_or('\u{fffd}');
                let len = ch.len_utf8().max(1);
                self.pos += len;
                self.error(
                    Code::UnexpectedCharacter,
                    start,
                    self.pos,
                    format!("'{ch}' has no place here; outside comments and strings, IEC 61131-3 text is ASCII"),
                );
                return TokenKind::Unknown;
            }
        };
        self.pos += len;
        kind
    }
}

fn valid_direct_address(upper: &str) -> bool {
    let Some(rest) = upper.strip_prefix('%') else { return false };
    let mut chars = rest.chars().peekable();
    if !matches!(chars.next(), Some('I' | 'Q' | 'M')) {
        return false;
    }
    let sized = matches!(chars.peek(), Some('X' | 'B' | 'W' | 'D' | 'L'));
    if sized {
        chars.next();
    }
    let tail: String = chars.collect();
    if tail == "*" {
        // The asterisk stands for the size and the numbers both (Ed.3 6.5.5.4).
        return !sized;
    }
    !tail.is_empty() && tail.split('.').all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

/// `d h m s ms us ns` parts in that order, each a number, the last
/// possibly with a fraction (Ed.3 Table 8).
fn valid_duration(body: &str) -> bool {
    let body = body.trim_start_matches(['-', '+']).to_ascii_lowercase().replace('_', "");
    if body.is_empty() {
        return false;
    }
    const UNITS: [&str; 7] = ["d", "h", "m", "s", "ms", "us", "ns"];
    let mut rest = body.as_str();
    let mut last_unit: Option<usize> = None;
    while !rest.is_empty() {
        let n = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
        if n == 0 {
            return false;
        }
        rest = &rest[n..];
        let u = rest.find(|c: char| c.is_ascii_digit()).unwrap_or(rest.len());
        let unit = &rest[..u];
        let Some(i) = UNITS.iter().position(|x| *x == unit) else { return false };
        if last_unit.is_some_and(|l| i <= l) {
            return false;
        }
        last_unit = Some(i);
        rest = &rest[u..];
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<TokenKind> {
        let (tokens, diags) = lex(FileId(0), text);
        assert!(diags.is_empty(), "{diags:?}");
        tokens.into_iter().map(|t| t.kind).filter(|k| !k.is_trivia()).collect()
    }

    fn codes(text: &str) -> Vec<&'static str> {
        lex(FileId(0), text).1.iter().map(|d| d.code.id()).collect()
    }

    #[test]
    fn spans_tile_the_text() {
        let text = "PROGRAM p (* c *) VAR x : INT := 16#FF; END_VAR x := 'a$'b'; é END_PROGRAM";
        let (tokens, _) = lex(FileId(0), text);
        let mut at = 0;
        for t in &tokens {
            assert_eq!(t.span.start, at);
            at = t.span.end;
        }
        assert_eq!(at as usize, text.len());
    }

    #[test]
    fn keywords_names_and_operators() {
        use TokenKind::*;
        assert_eq!(
            kinds("if X >= 1 THEN y := a ** 2; end_if"),
            vec![
                Kw(crate::Kw::If),
                Ident,
                Ge,
                Int,
                Kw(crate::Kw::Then),
                Ident,
                Assign,
                Ident,
                Power,
                Int,
                Semi,
                Kw(crate::Kw::EndIf),
                Eof
            ]
        );
    }

    #[test]
    fn literals() {
        use TokenKind::*;
        assert_eq!(
            kinds(
                "1_000 3.14 1.0E-3 16#FF 2#1010 T#1h2m3s4ms TIME#-5s t#1.5s D#2026-11-05 TOD#12:00:00.5 DT#2026-11-05-12:00"
            ),
            vec![Int, Real, Real, Based, Based, Time, Time, Time, Time, Time, Time, Eof]
        );
        assert_eq!(kinds("'it$'s' \"wide$0041\" '${1F579}'"), vec![String, WString, String, Eof]);
        assert_eq!(
            kinds("%IX0.0 %QW2 %MD4 %I*"),
            vec![DirectAddress, DirectAddress, DirectAddress, DirectAddress, Eof]
        );
    }

    #[test]
    fn a_range_is_not_a_real() {
        use TokenKind::*;
        assert_eq!(kinds("1..10"), vec![Int, DotDot, Int, Eof]);
    }

    #[test]
    fn typed_literals_are_name_hash_value() {
        use TokenKind::*;
        assert_eq!(kinds("INT#-5 Color#Red"), vec![Ident, Hash, Minus, Int, Ident, Hash, Ident, Eof]);
    }

    #[test]
    fn comments_nest() {
        assert_eq!(kinds("(* a (* b *) c *) x /* d /* e */ f */ // g"), vec![TokenKind::Ident, TokenKind::Eof]);
    }

    #[test]
    fn errors_are_reported_once() {
        assert_eq!(codes("(* (* *) x := 1;"), vec!["E0002"]);
        assert_eq!(codes("s := 'abc\nx := 1;"), vec!["E0003"]);
        assert_eq!(codes("x := a @ b;"), vec!["E0001"]);
        assert_eq!(codes("x := 'a$q';"), vec!["E0007"]);
        assert_eq!(codes("x := 3#12;"), vec!["E0004"]);
        assert_eq!(codes("x := 2#102;"), vec!["E0004"]);
        assert_eq!(codes("x := 16#;"), vec!["E0004"]);
        assert_eq!(codes("t := T#5x;"), vec!["E0004"]);
        assert_eq!(codes("t := T#5s1m;"), vec!["E0004"]);
        assert_eq!(codes("a AT %IY0 : BOOL;"), vec!["E0004"]);
        assert_eq!(codes("x := 8#17;"), vec!["W0001"]);
    }
}

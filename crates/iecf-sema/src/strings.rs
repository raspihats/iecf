//! String literals decoded (Ed.3 6.3.3, Table 7; Ed.4's `${...}`).
//!
//! A STRING holds one byte per character, read as Latin-1 (decided
//! 2026-11-05, docs/dependencies.md), so a character in a STRING literal
//! is its code point if that is below 256. `$N`, the newline, is a line
//! feed, as the reference compiler and C have it.

/// The code points of a string literal `text`, quotes included: `$hh`
/// in a single-quoted one, `$hhhh` in a double-quoted one (Ed.3 Table 7),
/// `${...}` in either (Ed.4), each with whether it was written as an
/// escape. An escape that is not one is the error.
fn code_points(text: &str) -> Result<Vec<(u32, bool)>, String> {
    let wide = text.starts_with('"');
    let q = if wide { '"' } else { '\'' };
    let body = text.strip_prefix(q).and_then(|t| t.strip_suffix(q)).unwrap_or(text);
    let mut out = Vec::new();
    let mut it = body.chars().peekable();
    while let Some(c) = it.next() {
        let code = if c != '$' {
            c as u32
        } else {
            match it.next() {
                Some('$') => '$' as u32,
                Some('\'') => '\'' as u32,
                Some('"') => '"' as u32,
                Some('L' | 'l' | 'N' | 'n') => 0x0A,
                Some('P' | 'p') => 0x0C,
                Some('R' | 'r') => 0x0D,
                Some('T' | 't') => 0x09,
                Some('{') => {
                    let hex: String = it.by_ref().take_while(|c| *c != '}').collect();
                    u32::from_str_radix(&hex, 16).map_err(|_| format!("${{{hex}}}"))?
                }
                Some(h1) => {
                    let mut hex = h1.to_string();
                    for _ in 0..if wide { 3 } else { 1 } {
                        hex.push(it.next().unwrap_or('0'));
                    }
                    u32::from_str_radix(&hex, 16).map_err(|_| format!("${hex}"))?
                }
                None => return Err("$".into()),
            }
        };
        out.push((code, c == '$'));
    }
    Ok(out)
}

/// The bytes of a STRING literal `text`, quotes included, or the first
/// character a STRING cannot hold.
pub fn string_bytes(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for (code, escaped) in code_points(text)? {
        match u8::try_from(code) {
            Ok(b) => out.push(b),
            Err(_) => {
                return Err(match char::from_u32(code) {
                    Some(ch) if !escaped => format!("'{ch}'"),
                    _ => format!("U+{code:04X}"),
                });
            }
        }
    }
    Ok(out)
}

/// The UTF-16 units of a WSTRING literal `text`, quotes included (Ed.3
/// Table 27: ISO/IEC 10646 as UTF-16): a character above U+FFFF is two,
/// a surrogate pair (decided, florin, 2026-11-10, docs/dependencies.md).
/// The error is an escape that is not one, or a code that is no
/// character.
pub fn wstring_units(text: &str) -> Result<Vec<u16>, String> {
    let mut out = Vec::new();
    for (code, _) in code_points(text)? {
        match char::from_u32(code) {
            Some(ch) => out.extend_from_slice(ch.encode_utf16(&mut [0u16; 2])),
            None => return Err(format!("U+{code:04X}")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{string_bytes, wstring_units};

    #[test]
    fn decodes_escapes_and_latin_1() {
        assert_eq!(string_bytes("'a$'b$$'").unwrap(), b"a'b$");
        assert_eq!(string_bytes("'$L$N$R$T$P'").unwrap(), [10, 10, 13, 9, 12]);
        assert_eq!(string_bytes("'$41$e9${FF}'").unwrap(), [0x41, 0xE9, 0xFF]);
        assert_eq!(string_bytes("'caf\u{e9}'").unwrap(), b"caf\xE9");
        assert_eq!(string_bytes("''").unwrap(), b"");
        assert_eq!(string_bytes("'\u{20ac}'"), Err("'\u{20ac}'".into()));
        assert_eq!(string_bytes("'${1F579}'"), Err("U+1F579".into()));
    }

    #[test]
    fn wide_literals_are_utf16() {
        assert_eq!(wstring_units("\"ab\"").unwrap(), [0x61, 0x62]);
        assert_eq!(wstring_units("\"$0041$263A'\"").unwrap(), [0x41, 0x263A, 0x27]);
        assert_eq!(wstring_units("\"$\"$$\"").unwrap(), [0x22, 0x24]);
        assert_eq!(wstring_units("\"\u{20ac}\"").unwrap(), [0x20AC]);
        // Above U+FFFF: a surrogate pair.
        assert_eq!(wstring_units("\"${1F579}\"").unwrap(), [0xD83D, 0xDD79]);
        assert_eq!(wstring_units("\"\u{1F579}\"").unwrap(), [0xD83D, 0xDD79]);
        // WSTRING#'..': single quotes, two hex digits.
        assert_eq!(wstring_units("'$41b'").unwrap(), [0x41, 0x62]);
        assert_eq!(wstring_units("\"${D800}\""), Err("U+D800".into()));
        // STRING#"..": four hex digits.
        assert_eq!(string_bytes("\"$0041\"").unwrap(), b"A");
    }
}

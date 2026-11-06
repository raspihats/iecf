//! String literals decoded (Ed.3 6.3.3, Table 7; Ed.4's `${...}`).
//!
//! A STRING holds one byte per character, read as Latin-1 (decided
//! 2026-11-05, docs/dependencies.md), so a character in a STRING literal
//! is its code point if that is below 256. `$N`, the newline, is a line
//! feed, as matiec and C have it.

/// The bytes of a STRING literal `text`, quotes included, or the first
/// character a STRING cannot hold.
pub fn string_bytes(text: &str) -> Result<Vec<u8>, String> {
    let body = text.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')).unwrap_or(text);
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
                    let h2 = it.next().unwrap_or('0');
                    u32::from_str_radix(&format!("{h1}{h2}"), 16).map_err(|_| format!("${h1}{h2}"))?
                }
                None => return Err("$".into()),
            }
        };
        match u8::try_from(code) {
            Ok(b) => out.push(b),
            Err(_) => {
                return Err(match char::from_u32(code) {
                    Some(ch) if c != '$' => format!("'{ch}'"),
                    _ => format!("U+{code:04X}"),
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::string_bytes;

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
}

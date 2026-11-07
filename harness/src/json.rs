//! Just enough JSON to read `<p>.symbols.json` (docs/c-output.md §6):
//! the workspace takes no dependencies.

#[derive(Clone, Debug)]
pub(crate) enum J {
    Null,
    Bool,
    Num(i64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub(crate) fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub(crate) fn str(&self) -> Option<&str> {
        if let J::Str(s) = self { Some(s) } else { None }
    }
    pub(crate) fn num(&self) -> Option<i64> {
        if let J::Num(n) = self { Some(*n) } else { None }
    }
    pub(crate) fn arr(&self) -> &[J] {
        if let J::Arr(a) = self { a } else { &[] }
    }
}

pub(crate) fn parse(text: &str) -> Result<J, String> {
    let b = text.as_bytes();
    let mut i = 0;
    let v = value(b, &mut i)?;
    skip(b, &mut i);
    if i != b.len() {
        return Err(format!("JSON: text after the value at {i}"));
    }
    Ok(v)
}

fn skip(b: &[u8], i: &mut usize) {
    while *i < b.len() && b[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn value(b: &[u8], i: &mut usize) -> Result<J, String> {
    skip(b, i);
    let err = |i: usize| format!("JSON: unexpected text at {i}");
    match b.get(*i) {
        Some(b'{') => {
            *i += 1;
            let mut m = Vec::new();
            loop {
                skip(b, i);
                if b.get(*i) == Some(&b'}') {
                    *i += 1;
                    return Ok(J::Obj(m));
                }
                let J::Str(k) = value(b, i)? else { return Err(err(*i)) };
                skip(b, i);
                if b.get(*i) != Some(&b':') {
                    return Err(err(*i));
                }
                *i += 1;
                m.push((k, value(b, i)?));
                skip(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b'}') => {}
                    _ => return Err(err(*i)),
                }
            }
        }
        Some(b'[') => {
            *i += 1;
            let mut a = Vec::new();
            loop {
                skip(b, i);
                if b.get(*i) == Some(&b']') {
                    *i += 1;
                    return Ok(J::Arr(a));
                }
                a.push(value(b, i)?);
                skip(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b']') => {}
                    _ => return Err(err(*i)),
                }
            }
        }
        Some(b'"') => {
            *i += 1;
            let mut s = String::new();
            while let Some(&c) = b.get(*i) {
                *i += 1;
                match c {
                    b'"' => return Ok(J::Str(s)),
                    b'\\' => {
                        let e = *b.get(*i).ok_or_else(|| err(*i))?;
                        *i += 1;
                        match e {
                            b'u' => {
                                let h = std::str::from_utf8(b.get(*i..*i + 4).ok_or_else(|| err(*i))?)
                                    .map_err(|_| err(*i))?;
                                let n = u32::from_str_radix(h, 16).map_err(|_| err(*i))?;
                                s.push(char::from_u32(n).unwrap_or('?'));
                                *i += 4;
                            }
                            b'n' => s.push('\n'),
                            b't' => s.push('\t'),
                            other => s.push(other as char),
                        }
                    }
                    _ => s.push(c as char),
                }
            }
            Err(err(*i))
        }
        Some(b't') if b[*i..].starts_with(b"true") => {
            *i += 4;
            Ok(J::Bool)
        }
        Some(b'f') if b[*i..].starts_with(b"false") => {
            *i += 5;
            Ok(J::Bool)
        }
        Some(b'n') if b[*i..].starts_with(b"null") => {
            *i += 4;
            Ok(J::Null)
        }
        Some(c) if *c == b'-' || c.is_ascii_digit() => {
            let start = *i;
            *i += 1;
            while b.get(*i).is_some_and(|c| c.is_ascii_digit()) {
                *i += 1;
            }
            let t = std::str::from_utf8(&b[start..*i]).map_err(|_| err(start))?;
            t.parse().map(J::Num).map_err(|_| err(start))
        }
        _ => Err(err(*i)),
    }
}

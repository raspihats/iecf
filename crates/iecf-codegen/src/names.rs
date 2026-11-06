//! Where a variable lives in the C, and how it is read and written.

use iecf_sema::types::Elem;
use std::collections::HashMap;

/// A variable, as C reaches it.
#[derive(Clone, Debug)]
pub(crate) enum Place {
    /// A field: `self->V_X` in an instance, `ctx->V_G` for a global.
    Field(String, Elem),
    /// In the process image, read and written at each use
    /// (docs/c-output.md §4): area `I`/`Q`/`M`, size `X`/`B`/`W`/`D`/`L`.
    Located { area: char, size: char, byte: u32, bit: u8, ty: Elem },
}

/// The names a body sees, upper-cased.
#[derive(Default)]
pub(crate) struct Scope {
    pub(crate) vars: HashMap<String, Place>,
}

fn image(area: char) -> &'static str {
    match area {
        'I' => "ctx->image.i",
        'Q' => "ctx->image.q",
        _ => "ctx->image.m",
    }
}

impl Place {
    pub(crate) fn ty(&self) -> Elem {
        match self {
            Place::Field(_, t) => *t,
            Place::Located { ty, .. } => *ty,
        }
    }

    /// An lvalue, for FOR's control variable; none in the image.
    pub(crate) fn lvalue(&self) -> Option<&str> {
        match self {
            Place::Field(f, _) => Some(f),
            Place::Located { .. } => None,
        }
    }

    /// Its value, as its own type.
    pub(crate) fn load(&self) -> String {
        match self {
            Place::Field(f, _) => f.clone(),
            Place::Located { area, size, byte, bit, ty } => {
                let p = image(*area);
                let raw = match size {
                    'X' => return format!("iecf_get_x({p}, {byte}u, {bit}u)"),
                    'B' => format!("iecf_get_b({p}, {byte}u)"),
                    'W' => format!("iecf_get_w({p}, {byte}u)"),
                    'D' => format!("iecf_get_d({p}, {byte}u)"),
                    _ => format!("iecf_get_l({p}, {byte}u)"),
                };
                match ty {
                    Elem::Sint => format!("iecf_to_i8({raw})"),
                    Elem::Int => format!("iecf_to_i16({raw})"),
                    Elem::Dint => format!("iecf_to_i32({raw})"),
                    Elem::Lint => format!("iecf_to_i64({raw})"),
                    _ => raw,
                }
            }
        }
    }

    /// A statement storing `value`, a C expression of its type.
    pub(crate) fn store(&self, value: &str) -> String {
        match self {
            Place::Field(f, _) => format!("{f} = {value};"),
            Place::Located { area, size, byte, bit, ty } => {
                let p = image(*area);
                match size {
                    'X' => format!("iecf_set_x({p}, {byte}u, {bit}u, {value});"),
                    'B' => format!("iecf_set_b({p}, {byte}u, (uint8_t)({value}));"),
                    'W' => format!("iecf_set_w({p}, {byte}u, (uint16_t)({value}));"),
                    'D' => format!("iecf_set_d({p}, {byte}u, (uint32_t)({value}));"),
                    _ => {
                        let _ = ty;
                        format!("iecf_set_l({p}, {byte}u, (uint64_t)({value}));")
                    }
                }
            }
        }
    }
}

/// `%QW2` → (Q, W, 2, 0); `%IX1.3` → (I, X, 1, 3). A missing size is X
/// (Ed.3 6.5.5.2). One level only, and not `*` (docs/c-output.md §4).
pub(crate) fn parse_address(text: &str) -> Option<(char, char, u32, u8)> {
    let rest = text.strip_prefix('%')?;
    let mut chars = rest.chars();
    let area = chars.next()?.to_ascii_uppercase();
    if !matches!(area, 'I' | 'Q' | 'M') {
        return None;
    }
    let rest = chars.as_str();
    let (size, num) = match rest.chars().next()?.to_ascii_uppercase() {
        c @ ('X' | 'B' | 'W' | 'D' | 'L') => (c, &rest[1..]),
        c if c.is_ascii_digit() => ('X', rest),
        _ => return None,
    };
    let parts: Vec<&str> = num.split('.').collect();
    match (size, parts.as_slice()) {
        ('X', [byte, bit]) => {
            let bit: u8 = bit.parse().ok().filter(|b| *b < 8)?;
            Some((area, 'X', byte.parse().ok()?, bit))
        }
        ('X', _) => None,
        (_, [byte]) => Some((area, size, byte.parse().ok()?, 0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(parse_address("%QW2"), Some(('Q', 'W', 2, 0)));
        assert_eq!(parse_address("%ix1.3"), Some(('I', 'X', 1, 3)));
        assert_eq!(parse_address("%I1.3"), Some(('I', 'X', 1, 3)));
        assert_eq!(parse_address("%ML30"), Some(('M', 'L', 30, 0)));
        assert_eq!(parse_address("%IX1.8"), None);
        assert_eq!(parse_address("%IX1.2.3"), None, "hierarchical");
        assert_eq!(parse_address("%QW1.2"), None);
        assert_eq!(parse_address("%I*"), None);
    }
}

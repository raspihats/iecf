//! Where a variable lives in the C, and how it is read and written.

use crate::derived::DTy;
use iecf_sema::stdlib::{Dir, std_fb};
use iecf_sema::types::{Elem, Ty};
use std::collections::HashMap;

/// A variable, as C reaches it.
#[derive(Clone, Debug)]
pub(crate) enum Place {
    /// An lvalue: `self->V_X` in an instance, `ctx->V_G` for a global,
    /// `V_X` for a FUNCTION's input or local.
    Field(String, Elem),
    /// In the process image, read and written at each use
    /// (docs/c-output.md §4): area `I`/`Q`/`M`, size `X`/`B`/`W`/`D`/`L`.
    /// `size` the width it is read and written in (lib.rs `located`).
    Located { area: char, size: char, byte: u32, bit: u8, ty: Elem },
    /// An instance of a standard function block, `self->V_T1`.
    Fb(String, StdFb),
    /// An instance of the user's FUNCTION_BLOCK, by its upper-cased name.
    Block(String, String),
    /// A variable of a derived type (derived.rs), an lvalue: an
    /// enumerated or subrange one is held as its base type.
    Data(String, DTy),
}

/// A standard function block (Ed.3 Tables 43-46) as runtime/iecf.h has
/// it: `iecf_<stem>_t`, `iecf_<stem>_init`, `iecf_<stem>`.
#[derive(Clone, Debug)]
pub(crate) struct StdFb {
    pub(crate) stem: String,
    /// A timer: its function takes the task's time.
    pub(crate) timer: bool,
    /// Inputs and outputs, by their standard names, the second spellings
    /// of SR and RS (Table 43 1b, 2b) left out.
    pub(crate) params: Vec<(String, Elem, Dir)>,
}

impl StdFb {
    /// `TON`, `CTU_DINT`, ... as the checker names the type.
    pub(crate) fn named(name: &str) -> Option<StdFb> {
        let upper = name.to_ascii_uppercase();
        let (stem, timer) = match upper.as_str() {
            "SR" | "RS" | "R_TRIG" | "F_TRIG" => (upper.clone(), false),
            _ => {
                let (base, rest) = ["CTUD", "CTU", "CTD", "TON", "TOF", "TP"]
                    .iter()
                    .find_map(|b| upper.strip_prefix(b).map(|r| (*b, r)))?;
                let timer = matches!(base, "TON" | "TOF" | "TP");
                let suffix = match rest {
                    "" | "_INT" if !timer => "",
                    "" | "_TIME" if timer => "",
                    "_LTIME" if timer => "_LTIME",
                    "_DINT" | "_LINT" | "_UDINT" | "_ULINT" if !timer => rest,
                    _ => return None,
                };
                (format!("{base}{suffix}"), timer)
            }
        };
        let params = std_fb(&upper)?
            .into_iter()
            .filter(|p| !matches!(p.name, "SET1" | "RESET" | "SET" | "RESET1"))
            .filter_map(|p| match p.ty {
                Ty::Elem(e) => Some((p.name.to_string(), e, p.dir)),
                _ => None,
            })
            .collect();
        Some(StdFb { stem, timer, params })
    }

    /// The field of a parameter named `name`, either spelling.
    pub(crate) fn param(&self, name: &str) -> Option<&(String, Elem, Dir)> {
        let upper = name.to_ascii_uppercase();
        let canon = match (self.stem.as_str(), upper.as_str()) {
            ("SR", "SET1") => "S1",
            ("SR", "RESET") => "R",
            ("RS", "SET") => "S",
            ("RS", "RESET1") => "R1",
            (_, n) => n,
        };
        self.params.iter().find(|(n, _, _)| n == canon)
    }
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
    /// Its elementary type; `None` for a function block instance.
    pub(crate) fn elem(&self) -> Option<Elem> {
        match self {
            Place::Field(_, t) => Some(*t),
            Place::Located { ty, .. } => Some(*ty),
            Place::Data(_, t) => t.scalar(),
            Place::Fb(..) | Place::Block(..) => None,
        }
    }

    /// An lvalue, for FOR's control variable; none in the image.
    pub(crate) fn lvalue(&self) -> Option<&str> {
        match self {
            Place::Field(f, _) | Place::Data(f, _) => Some(f),
            _ => None,
        }
    }

    /// Its value, as its own type.
    pub(crate) fn load(&self) -> String {
        match self {
            Place::Field(f, _) | Place::Fb(f, _) | Place::Block(f, _) | Place::Data(f, _) => f.clone(),
            Place::Located { area, size, byte, bit, ty } => {
                let p = image(*area);
                match (ty, size) {
                    (Elem::Real, _) => return format!("iecf_get_real({p}, {byte}u)"),
                    (Elem::Lreal, _) => return format!("iecf_get_lreal({p}, {byte}u)"),
                    // A BOOL in a whole byte: any value but 0 is TRUE.
                    (Elem::Bool, 'B') => return format!("((IEC_BOOL)(iecf_get_b({p}, {byte}u) != 0u))"),
                    _ => {}
                }
                let raw = match size {
                    'X' => return format!("iecf_get_x({p}, {byte}u, {bit}u)"),
                    'B' => format!("iecf_get_b({p}, {byte}u)"),
                    'W' => format!("iecf_get_w({p}, {byte}u)"),
                    'D' => format!("iecf_get_d({p}, {byte}u)"),
                    _ => format!("iecf_get_l({p}, {byte}u)"),
                };
                // The signed ones, TIME and the long dates included, from
                // their two's complement bits.
                match ty {
                    Elem::Sint => format!("iecf_to_i8({raw})"),
                    Elem::Int => format!("iecf_to_i16({raw})"),
                    Elem::Dint | Elem::Time => format!("iecf_to_i32({raw})"),
                    Elem::Lint | Elem::Ltime | Elem::Ldate | Elem::Ltod | Elem::Ldt => format!("iecf_to_i64({raw})"),
                    _ => raw,
                }
            }
        }
    }

    /// A statement storing `value`, a C expression of its type.
    pub(crate) fn store(&self, value: &str) -> String {
        match self {
            Place::Field(f, _) | Place::Fb(f, _) | Place::Block(f, _) | Place::Data(f, _) => format!("{f} = {value};"),
            Place::Located { area, size, byte, bit, ty } => {
                let p = image(*area);
                match (ty, size) {
                    (Elem::Real, _) => return format!("iecf_set_real({p}, {byte}u, {value});"),
                    (Elem::Lreal, _) => return format!("iecf_set_lreal({p}, {byte}u, {value});"),
                    (Elem::Bool, 'B') => return format!("iecf_set_b({p}, {byte}u, (uint8_t)(({value}) ? 1u : 0u));"),
                    _ => {}
                }
                match size {
                    'X' => format!("iecf_set_x({p}, {byte}u, {bit}u, {value});"),
                    'B' => format!("iecf_set_b({p}, {byte}u, (uint8_t)({value}));"),
                    'W' => format!("iecf_set_w({p}, {byte}u, (uint16_t)({value}));"),
                    'D' => format!("iecf_set_d({p}, {byte}u, (uint32_t)({value}));"),
                    _ => format!("iecf_set_l({p}, {byte}u, (uint64_t)({value}));"),
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

    #[test]
    fn standard_blocks() {
        let ton = StdFb::named("ton_time").unwrap();
        assert_eq!((ton.stem.as_str(), ton.timer), ("TON", true));
        assert_eq!(StdFb::named("TP_LTIME").unwrap().stem, "TP_LTIME");
        assert_eq!(StdFb::named("CTU_INT").unwrap().stem, "CTU");
        assert_eq!(StdFb::named("CTUD_ULINT").unwrap().stem, "CTUD_ULINT");
        assert!(StdFb::named("CTU_SINT").is_none());
        let sr = StdFb::named("SR").unwrap();
        assert_eq!(sr.params.len(), 3, "S1, R, Q1");
        assert_eq!(sr.param("set1").map(|p| p.0.as_str()), Some("S1"));
        assert!(StdFb::named("AVERAGE").is_none());
    }
}

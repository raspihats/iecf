//! The standard's function blocks (Ed.3 6.6.3.5, Tables 43-46) and what
//! the checker needs to know of its functions (6.6.2.5).

use crate::types::{Elem, Ty};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Input,
    Output,
    InOut,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: &'static str,
    pub dir: Dir,
    pub ty: Ty,
}

fn p(name: &'static str, dir: Dir, ty: Ty) -> Param {
    Param { name, dir, ty }
}

fn bool_() -> Ty {
    Ty::Elem(Elem::Bool)
}

/// The parameters of a standard function block, or `None` if `name` is
/// not one. SR and RS take both spellings of their inputs (Table 43,
/// features 1a/1b and 2a/2b).
pub fn std_fb(name: &str) -> Option<Vec<Param>> {
    use Dir::*;
    let upper = name.to_ascii_uppercase();
    let counter = |suffix: &str| -> Option<Elem> {
        Some(match suffix {
            "" | "_INT" => Elem::Int,
            "_DINT" => Elem::Dint,
            "_LINT" => Elem::Lint,
            "_UDINT" => Elem::Udint,
            "_ULINT" => Elem::Ulint,
            _ => return None,
        })
    };
    let timer = |suffix: &str| -> Option<Elem> {
        Some(match suffix {
            "" | "_TIME" => Elem::Time,
            "_LTIME" => Elem::Ltime,
            _ => return None,
        })
    };
    let params = match upper.as_str() {
        "SR" => vec![
            p("S1", Input, bool_()),
            p("SET1", Input, bool_()),
            p("R", Input, bool_()),
            p("RESET", Input, bool_()),
            p("Q1", Output, bool_()),
        ],
        "RS" => vec![
            p("S", Input, bool_()),
            p("SET", Input, bool_()),
            p("R1", Input, bool_()),
            p("RESET1", Input, bool_()),
            p("Q1", Output, bool_()),
        ],
        "R_TRIG" | "F_TRIG" => vec![p("CLK", Input, bool_()), p("Q", Output, bool_())],
        _ => {
            if let Some(rest) = upper.strip_prefix("CTUD") {
                let t = Ty::Elem(counter(rest)?);
                vec![
                    p("CU", Input, bool_()),
                    p("CD", Input, bool_()),
                    p("R", Input, bool_()),
                    p("LD", Input, bool_()),
                    p("PV", Input, t.clone()),
                    p("QU", Output, bool_()),
                    p("QD", Output, bool_()),
                    p("CV", Output, t),
                ]
            } else if let Some(rest) = upper.strip_prefix("CTU") {
                let t = Ty::Elem(counter(rest)?);
                vec![
                    p("CU", Input, bool_()),
                    p("R", Input, bool_()),
                    p("PV", Input, t.clone()),
                    p("Q", Output, bool_()),
                    p("CV", Output, t),
                ]
            } else if let Some(rest) = upper.strip_prefix("CTD") {
                let t = Ty::Elem(counter(rest)?);
                vec![
                    p("CD", Input, bool_()),
                    p("LD", Input, bool_()),
                    p("PV", Input, t.clone()),
                    p("Q", Output, bool_()),
                    p("CV", Output, t),
                ]
            } else {
                let rest = upper.strip_prefix("TON").or(upper.strip_prefix("TOF")).or(upper.strip_prefix("TP"))?;
                let t = Ty::Elem(timer(rest)?);
                vec![p("IN", Input, bool_()), p("PT", Input, t.clone()), p("Q", Output, bool_()), p("ET", Output, t)]
            }
        }
    };
    Some(params)
}

/// What a standard function returns, given its arguments' types; `None`
/// if `name` is no standard function. `Some(Ty::Unknown)` when it is one
/// but the result is not worked out yet: no error, no guess.
pub fn std_function(name: &str, args: &[Ty]) -> Option<Ty> {
    let upper = name.to_ascii_uppercase();
    let first = || args.first().cloned().unwrap_or(Ty::Unknown);
    let same = |from: usize| unify_all(&args[from.min(args.len())..]);
    // Conversions: X_TO_Y, TO_Y, TRUNC_Y, X_TRUNC_Y (Ed.3 6.6.2.5.2-5).
    if let Some(target) = conversion_target(&upper) {
        return Some(target);
    }
    let ty = match upper.as_str() {
        // Numerical (Table 28).
        "ABS" | "SQRT" | "LN" | "LOG" | "EXP" | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ATAN" | "ATAN2" => {
            match first() {
                Ty::IntLit(_) | Ty::RealLit => Ty::Unknown,
                t => t,
            }
        }
        // Arithmetic (Table 29).
        "ADD" | "MUL" | "SUB" | "DIV" | "MOD" | "MOVE" => same(0),
        "EXPT" => first(),
        // Bit shift and bitwise (Tables 30, 31).
        "SHL" | "SHR" | "ROL" | "ROR" => first(),
        "AND" | "OR" | "XOR" | "NOT" => same(0),
        // Selection and comparison (Tables 32, 33).
        "SEL" => same(1),
        "MAX" | "MIN" | "LIMIT" => same(0),
        "MUX" => same(1),
        "GT" | "GE" | "EQ" | "LE" | "LT" | "NE" => Ty::Elem(Elem::Bool),
        // Strings (Table 34).
        "LEN" | "FIND" => Ty::Elem(Elem::Int),
        "LEFT" | "RIGHT" | "MID" | "CONCAT" | "INSERT" | "DELETE" | "REPLACE" => match first() {
            Ty::Str { wide, .. } => Ty::Str { wide, len: None },
            _ => Ty::Unknown,
        },
        // Time and date (Tables 35, 36).
        "ADD_TIME" | "SUB_TIME" | "MUL_TIME" | "DIV_TIME" | "SUB_DATE_DATE" | "SUB_TOD_TOD" | "SUB_DT_DT" => {
            Ty::Elem(Elem::Time)
        }
        "ADD_LTIME" | "SUB_LTIME" | "MUL_LTIME" | "DIV_LTIME" | "SUB_LDATE_LDATE" | "SUB_LTOD_LTOD" | "SUB_LDT_LDT" => {
            Ty::Elem(Elem::Ltime)
        }
        "ADD_TOD_TIME" | "SUB_TOD_TIME" => Ty::Elem(Elem::Tod),
        "ADD_DT_TIME" | "SUB_DT_TIME" | "CONCAT_DATE_TOD" => Ty::Elem(Elem::Dt),
        "ADD_LTOD_LTIME" | "SUB_LTOD_LTIME" => Ty::Elem(Elem::Ltod),
        "ADD_LDT_LTIME" | "SUB_LDT_LTIME" | "CONCAT_DATE_LTOD" => Ty::Elem(Elem::Ldt),
        "CONCAT_DATE" => Ty::Elem(Elem::Date),
        "CONCAT_TOD" => Ty::Elem(Elem::Tod),
        "CONCAT_LTOD" => Ty::Elem(Elem::Ltod),
        "CONCAT_DT" => Ty::Elem(Elem::Dt),
        "CONCAT_LDT" => Ty::Elem(Elem::Ldt),
        "SPLIT_DATE" | "SPLIT_TOD" | "SPLIT_LTOD" | "SPLIT_DT" | "SPLIT_LDT" => Ty::Void,
        "DAY_OF_WEEK" => Ty::Elem(Elem::Usint),
        // References (6.4.4.10).
        "REF" => Ty::Ref(Box::new(first())),
        _ => return None,
    };
    Some(ty)
}

/// The result type of a conversion function's name, or `None` if the
/// name is not one.
fn conversion_target(upper: &str) -> Option<Ty> {
    let target_of = |name: &str| -> Option<Ty> {
        match name {
            "STRING" => Some(Ty::Str { wide: false, len: None }),
            "WSTRING" => Some(Ty::Str { wide: true, len: None }),
            _ => Elem::from_name(name).map(Ty::Elem),
        }
    };
    let is_type = |name: &str| target_of(name).is_some() || name == "BCD";
    if let Some(t) = upper.strip_prefix("TO_").or(upper.strip_prefix("TRUNC_")) {
        return target_of(t);
    }
    // X_TO_Y and X_TRUNC_Y; X or Y may themselves contain '_'
    // (TIME_OF_DAY_TO_INT, WORD_BCD_TO_INT, INT_TO_BCD_WORD).
    for sep in ["_TO_", "_TRUNC_"] {
        let mut from = 0;
        while let Some(i) = upper[from..].find(sep) {
            let (src, dst) = (&upper[..from + i], &upper[from + i + sep.len()..]);
            let src_ok = is_type(src) || src.strip_suffix("_BCD").is_some_and(is_type);
            if src_ok {
                if let Some(t) = target_of(dst) {
                    return Some(t);
                }
                if let Some(t) = dst.strip_prefix("BCD_").and_then(target_of) {
                    return Some(t);
                }
            }
            from += i + 1;
        }
    }
    None
}

/// The common type of several operands, by the implicit conversions; an
/// untyped literal takes the other side's type.
pub fn unify_all(tys: &[Ty]) -> Ty {
    let mut acc: Option<Ty> = None;
    for t in tys {
        acc = Some(match acc {
            None => t.clone(),
            Some(a) => unify(&a, t).unwrap_or(Ty::Unknown),
        });
    }
    acc.unwrap_or(Ty::Unknown)
}

/// The type both `a` and `b` convert to implicitly, if there is one.
pub fn unify(a: &Ty, b: &Ty) -> Option<Ty> {
    use crate::types::Conversion::{Implicit, Same};
    match (a, b) {
        (Ty::Unknown, _) | (_, Ty::Unknown) => Some(Ty::Unknown),
        (Ty::IntLit(_), Ty::IntLit(_)) => Some(a.clone()),
        (Ty::IntLit(_) | Ty::RealLit, Ty::IntLit(_) | Ty::RealLit) => Some(Ty::RealLit),
        (Ty::IntLit(_), t) | (t, Ty::IntLit(_)) if t.is_num() || t.is_bit() => Some(t.clone()),
        (Ty::RealLit, t) | (t, Ty::RealLit) if t.elem().is_some_and(Elem::is_real) => Some(t.clone()),
        (Ty::Elem(x), Ty::Elem(y)) => match (x.conversion_to(*y), y.conversion_to(*x)) {
            (Same, _) => Some(a.clone()),
            (Implicit, _) => Some(b.clone()),
            (_, Implicit) => Some(a.clone()),
            _ => None,
        },
        (Ty::Str { wide: w1, .. }, Ty::Str { wide: w2, .. }) if w1 == w2 => Some(Ty::Str { wide: *w1, len: None }),
        _ if a == b => Some(a.clone()),
        _ => None,
    }
}

/// Every name the standard reserves as a data type, function or function
/// block (Ed.2 Table C.2, "Data type names", "Function names", "Function
/// Block names"; Ed.3 6.1.3) that a variable may not take. Checked by
/// name shape, not by list.
pub fn reserved_name(name: &str) -> Option<&'static str> {
    let upper = name.to_ascii_uppercase();
    if Elem::from_name(&upper).is_some() || matches!(upper.as_str(), "STRING" | "WSTRING") {
        return Some("a data type");
    }
    if std_fb(&upper).is_some() {
        return Some("a standard function block");
    }
    if std_function(&upper, &[]).is_some() {
        return Some("a standard function");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_by_name() {
        assert_eq!(conversion_target("INT_TO_REAL"), Some(Ty::Elem(Elem::Real)));
        assert_eq!(conversion_target("TIME_OF_DAY_TO_STRING"), Some(Ty::Str { wide: false, len: None }));
        assert_eq!(conversion_target("TO_LREAL"), Some(Ty::Elem(Elem::Lreal)));
        assert_eq!(conversion_target("LREAL_TRUNC_DINT"), Some(Ty::Elem(Elem::Dint)));
        assert_eq!(conversion_target("WORD_BCD_TO_INT"), Some(Ty::Elem(Elem::Int)));
        assert_eq!(conversion_target("INT_TO_BCD_WORD"), Some(Ty::Elem(Elem::Word)));
        assert_eq!(conversion_target("MY_TO_THING"), None);
        assert_eq!(conversion_target("INTERVAL"), None);
    }

    #[test]
    fn standard_blocks() {
        assert_eq!(std_fb("ton").map(|p| p.len()), Some(4));
        assert_eq!(std_fb("TON_LTIME").unwrap()[1].ty, Ty::Elem(Elem::Ltime));
        assert_eq!(std_fb("CTUD_DINT").unwrap()[4].ty, Ty::Elem(Elem::Dint));
        assert!(std_fb("CTU_REAL").is_none());
        assert!(std_fb("TONIGHT").is_none());
    }

    #[test]
    fn unification_widens() {
        let int = Ty::Elem(Elem::Int);
        let dint = Ty::Elem(Elem::Dint);
        let real = Ty::Elem(Elem::Real);
        assert_eq!(unify(&int, &dint), Some(dint.clone()));
        assert_eq!(unify(&int, &real), Some(real.clone()));
        assert_eq!(unify(&dint, &real), None, "DINT to REAL is explicit");
        assert_eq!(unify(&Ty::IntLit(Some(3)), &real), Some(real));
        assert_eq!(unify(&Ty::RealLit, &int), None);
    }

    #[test]
    fn reserved() {
        assert_eq!(reserved_name("dt"), Some("a data type"));
        assert_eq!(reserved_name("Ton"), Some("a standard function block"));
        assert_eq!(reserved_name("LT"), Some("a standard function"));
        assert_eq!(reserved_name("INT_TO_REAL"), Some("a standard function"));
        assert_eq!(reserved_name("lamp"), None);
    }
}

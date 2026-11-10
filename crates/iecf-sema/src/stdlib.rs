//! The standard's function blocks (Ed.3 6.6.3.5, Tables 43-46) and what
//! the checker needs to know of its functions (6.6.2.5).

use crate::types::{Elem, Ty};
use iecf_syntax::ast::BinaryOp;

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

/// A class of types a standard function's input takes (Ed.3 6.4.3,
/// Figure 5, the generic data types).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Any,
    AnyElementary,
    AnyMagnitude,
    AnyNum,
    AnyReal,
    AnyInt,
    AnyBit,
    AnyString,
    AnyChars,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Any => "ANY",
            Class::AnyElementary => "ANY_ELEMENTARY",
            Class::AnyMagnitude => "ANY_MAGNITUDE",
            Class::AnyNum => "ANY_NUM",
            Class::AnyReal => "ANY_REAL",
            Class::AnyInt => "ANY_INT",
            Class::AnyBit => "ANY_BIT",
            Class::AnyString => "ANY_STRING",
            Class::AnyChars => "ANY_CHARS",
        }
    }

    /// Whether a value of type `t` belongs to the class. An untyped
    /// integer literal is ANY_INT, ANY_BIT (`16#FF`) and ANY_REAL
    /// (`SQRT(2)`, as CODESYS documents `SQRT(16)`): Ed.3 6.6.1.6 rule 8
    /// leaves untyped literals to the implementer, docs/dependencies.md.
    pub fn has(self, t: &Ty) -> bool {
        let e = t.elem();
        match self {
            Class::Any => !matches!(t, Ty::Void),
            Class::AnyElementary => {
                matches!(t, Ty::Elem(_) | Ty::Str { .. } | Ty::IntLit(_) | Ty::RealLit | Ty::Enum(_))
            }
            Class::AnyMagnitude => t.is_num() || e.is_some_and(Elem::is_duration),
            Class::AnyNum => t.is_num(),
            Class::AnyReal => matches!(t, Ty::RealLit | Ty::IntLit(_)) || e.is_some_and(Elem::is_real),
            Class::AnyInt => t.is_int(),
            Class::AnyBit => t.is_bit() || matches!(t, Ty::IntLit(_)),
            Class::AnyString => matches!(t, Ty::Str { .. }),
            Class::AnyChars => matches!(t, Ty::Str { .. }) || matches!(e, Some(Elem::Char | Elem::Wchar)),
        }
    }
}

/// What one input or output of a standard function takes.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// This type, or one that converts to it implicitly.
    Ty(Ty),
    /// Any type of the class.
    Class(Class),
    /// The call's common type, of the class: every input marked so has
    /// it, and so may the result. The type in a typed call (`ADD_INT`).
    Common(Class),
}

/// How the result of a standard function follows from its inputs.
#[derive(Clone, Debug, PartialEq)]
pub enum Res {
    /// The common type.
    Common,
    Ty(Ty),
    /// The operator applied left to right, with its rules for time and
    /// date (Table 35); the class is what a typed form may name.
    Op(BinaryOp, Class),
    /// A conversion to this type; Figure 11 must define one from the
    /// input's type.
    Convert(Ty),
    /// `REF(x)`: a reference to the input's type (6.4.4.10.3).
    Ref,
}

/// More inputs of one kind: `IN1, IN2, ...` (`IN0, ...` for MUX).
#[derive(Clone, Debug, PartialEq)]
pub struct Ext {
    pub first: u32,
    pub kind: Kind,
    /// How many a call passes at least, in order.
    pub min: usize,
}

/// A standard function's interface (Ed.3 6.6.2.5, Tables 22-37).
#[derive(Clone, Debug, PartialEq)]
pub struct Sig {
    pub inputs: Vec<(&'static str, Kind)>,
    /// Inputs `IN<n>` after `inputs`.
    pub ext: Option<Ext>,
    pub outputs: Vec<(&'static str, Kind)>,
    pub result: Res,
    /// The type a typed form names (`ADD_INT`: INT, 6.6.2.4).
    pub typed: Option<Ty>,
    /// `MUX_K_T`: the type of K (Table 32, footnote c).
    pub typed_k: Option<Ty>,
    /// A typed form's function without its types: `ADD` for `ADD_INT`.
    pub base: Option<String>,
}

fn sig(inputs: Vec<(&'static str, Kind)>, result: Res) -> Sig {
    Sig { inputs, ext: None, outputs: Vec::new(), result, typed: None, typed_k: None, base: None }
}

fn ext(inputs: Vec<(&'static str, Kind)>, first: u32, kind: Kind, min: usize, result: Res) -> Sig {
    Sig { ext: Some(Ext { first, kind, min }), ..sig(inputs, result) }
}

fn ty(e: Elem) -> Ty {
    Ty::Elem(e)
}

/// The type a name denotes in a function's name: an elementary type,
/// STRING or WSTRING.
fn type_named(name: &str) -> Option<Ty> {
    match name {
        "STRING" => Some(Ty::Str { wide: false, len: None }),
        "WSTRING" => Some(Ty::Str { wide: true, len: None }),
        _ => Elem::from_name(name).map(Ty::Elem),
    }
}

/// The standard functions by their own names, before typed forms and
/// conversions.
fn base_sig(upper: &str) -> Option<Sig> {
    use Class::*;
    use Kind::{Class as C, Common as T, Ty as Exact};
    let one = |k: Kind, r: Res| sig(vec![("IN", k)], r);
    let pair = |k1: Kind, k2: Kind, r: Res| sig(vec![("IN1", k1), ("IN2", k2)], r);
    let time2 = |a: Elem, b: Kind, r: Elem| pair(Exact(ty(a)), b, Res::Ty(ty(r)));
    let ints = |names: &[&'static str], r: Elem| sig(names.iter().map(|n| (*n, C(AnyInt))).collect(), Res::Ty(ty(r)));
    let split = |from: Elem, names: &[&'static str]| Sig {
        outputs: names.iter().map(|n| (*n, C(AnyInt))).collect(),
        ..one(Exact(ty(from)), Res::Ty(Ty::Void))
    };
    const DATE: [&str; 3] = ["YEAR", "MONTH", "DAY"];
    const TOD: [&str; 4] = ["HOUR", "MINUTE", "SECOND", "MILLISECOND"];
    const DT: [&str; 7] = ["YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND", "MILLISECOND"];
    let s = match upper {
        // Table 28: numerical functions of one variable; ATAN2.
        "ABS" => one(T(AnyNum), Res::Common),
        "SQRT" | "LN" | "LOG" | "EXP" | "SIN" | "COS" | "TAN" | "ASIN" | "ACOS" | "ATAN" => {
            one(T(AnyReal), Res::Common)
        }
        "ATAN2" => sig(vec![("Y", T(AnyReal)), ("X", T(AnyReal))], Res::Common),
        // Table 29: arithmetic. ADD and MUL are extensible.
        "ADD" => ext(vec![], 1, C(Any), 2, Res::Op(BinaryOp::Add, AnyMagnitude)),
        "MUL" => ext(vec![], 1, C(Any), 2, Res::Op(BinaryOp::Mul, AnyMagnitude)),
        "SUB" => pair(C(Any), C(Any), Res::Op(BinaryOp::Sub, AnyMagnitude)),
        "DIV" => pair(C(Any), C(Any), Res::Op(BinaryOp::Div, AnyMagnitude)),
        "MOD" => pair(T(AnyInt), T(AnyInt), Res::Common),
        "EXPT" => pair(T(AnyReal), C(AnyNum), Res::Common),
        "MOVE" => one(T(Any), Res::Common),
        // Tables 30, 31: bit strings.
        "SHL" | "SHR" | "ROL" | "ROR" => sig(vec![("IN", T(AnyBit)), ("N", C(AnyInt))], Res::Common),
        "AND" | "OR" | "XOR" => ext(vec![], 1, T(AnyBit), 2, Res::Common),
        "NOT" => one(T(AnyBit), Res::Common),
        // Table 32: selection.
        "SEL" => sig(vec![("G", Exact(ty(Elem::Bool))), ("IN0", T(Any)), ("IN1", T(Any))], Res::Common),
        "MAX" | "MIN" => ext(vec![], 1, T(AnyElementary), 2, Res::Common),
        "LIMIT" => sig(vec![("MN", T(AnyElementary)), ("IN", T(AnyElementary)), ("MX", T(AnyElementary))], Res::Common),
        "MUX" => ext(vec![("K", C(AnyInt))], 0, T(AnyElementary), 1, Res::Common),
        // Table 33: comparison; NE is not extensible.
        "GT" | "GE" | "EQ" | "LE" | "LT" => ext(vec![], 1, T(AnyElementary), 2, Res::Ty(ty(Elem::Bool))),
        "NE" => pair(T(AnyElementary), T(AnyElementary), Res::Ty(ty(Elem::Bool))),
        // Table 34: strings. LEN and FIND give INT: the standard leaves
        // the ANY_INT to the implementer.
        "LEN" => one(C(AnyString), Res::Ty(ty(Elem::Int))),
        "LEFT" | "RIGHT" => sig(vec![("IN", T(AnyString)), ("L", C(AnyInt))], Res::Common),
        "MID" => sig(vec![("IN", T(AnyString)), ("L", C(AnyInt)), ("P", C(AnyInt))], Res::Common),
        "CONCAT" => ext(vec![], 1, T(AnyChars), 2, Res::Common),
        "INSERT" => sig(vec![("IN1", T(AnyString)), ("IN2", T(AnyChars)), ("P", C(AnyInt))], Res::Common),
        "DELETE" => sig(vec![("IN", T(AnyString)), ("L", C(AnyInt)), ("P", C(AnyInt))], Res::Common),
        "REPLACE" => {
            sig(vec![("IN1", T(AnyString)), ("IN2", T(AnyChars)), ("L", C(AnyInt)), ("P", C(AnyInt))], Res::Common)
        }
        "FIND" => pair(T(AnyString), T(AnyChars), Res::Ty(ty(Elem::Int))),
        // Table 35: time and duration, typed.
        "ADD_TIME" => time2(Elem::Time, Exact(ty(Elem::Time)), Elem::Time),
        "ADD_LTIME" => time2(Elem::Ltime, Exact(ty(Elem::Ltime)), Elem::Ltime),
        "ADD_TOD_TIME" => time2(Elem::Tod, Exact(ty(Elem::Time)), Elem::Tod),
        "ADD_LTOD_LTIME" => time2(Elem::Ltod, Exact(ty(Elem::Ltime)), Elem::Ltod),
        "ADD_DT_TIME" => time2(Elem::Dt, Exact(ty(Elem::Time)), Elem::Dt),
        "ADD_LDT_LTIME" => time2(Elem::Ldt, Exact(ty(Elem::Ltime)), Elem::Ldt),
        "SUB_TIME" => time2(Elem::Time, Exact(ty(Elem::Time)), Elem::Time),
        "SUB_LTIME" => time2(Elem::Ltime, Exact(ty(Elem::Ltime)), Elem::Ltime),
        "SUB_DATE_DATE" => time2(Elem::Date, Exact(ty(Elem::Date)), Elem::Time),
        "SUB_LDATE_LDATE" => time2(Elem::Ldate, Exact(ty(Elem::Ldate)), Elem::Ltime),
        "SUB_TOD_TIME" => time2(Elem::Tod, Exact(ty(Elem::Time)), Elem::Tod),
        "SUB_LTOD_LTIME" => time2(Elem::Ltod, Exact(ty(Elem::Ltime)), Elem::Ltod),
        "SUB_TOD_TOD" => time2(Elem::Tod, Exact(ty(Elem::Tod)), Elem::Time),
        "SUB_LTOD_LTOD" => time2(Elem::Ltod, Exact(ty(Elem::Ltod)), Elem::Ltime),
        "SUB_DT_TIME" => time2(Elem::Dt, Exact(ty(Elem::Time)), Elem::Dt),
        "SUB_LDT_LTIME" => time2(Elem::Ldt, Exact(ty(Elem::Ltime)), Elem::Ldt),
        "SUB_DT_DT" => time2(Elem::Dt, Exact(ty(Elem::Dt)), Elem::Time),
        "SUB_LDT_LDT" => time2(Elem::Ldt, Exact(ty(Elem::Ldt)), Elem::Ltime),
        "MUL_TIME" => time2(Elem::Time, C(AnyNum), Elem::Time),
        "MUL_LTIME" => time2(Elem::Ltime, C(AnyNum), Elem::Ltime),
        "DIV_TIME" => time2(Elem::Time, C(AnyNum), Elem::Time),
        "DIV_LTIME" => time2(Elem::Ltime, C(AnyNum), Elem::Ltime),
        // Table 36: concatenate and split dates and times.
        "CONCAT_DATE_TOD" => {
            sig(vec![("DATE", Exact(ty(Elem::Date))), ("TOD", Exact(ty(Elem::Tod)))], Res::Ty(ty(Elem::Dt)))
        }
        "CONCAT_DATE_LTOD" => {
            sig(vec![("DATE", Exact(ty(Elem::Date))), ("LTOD", Exact(ty(Elem::Ltod)))], Res::Ty(ty(Elem::Ldt)))
        }
        "CONCAT_DATE" => ints(&DATE, Elem::Date),
        "CONCAT_TOD" => ints(&TOD, Elem::Tod),
        "CONCAT_LTOD" => ints(&TOD, Elem::Ltod),
        "CONCAT_DT" => ints(&DT, Elem::Dt),
        "CONCAT_LDT" => ints(&DT, Elem::Ldt),
        "SPLIT_DATE" => split(Elem::Date, &DATE),
        "SPLIT_TOD" => split(Elem::Tod, &TOD),
        "SPLIT_LTOD" => split(Elem::Ltod, &TOD),
        "SPLIT_DT" => split(Elem::Dt, &DT),
        "SPLIT_LDT" => split(Elem::Ldt, &DT),
        // DAY_OF_WEEK gives USINT: the standard leaves the ANY_INT to
        // the implementer.
        "DAY_OF_WEEK" => one(Exact(ty(Elem::Date)), Res::Ty(ty(Elem::Usint))),
        // Table 37: endianness.
        "TO_BIG_ENDIAN" | "TO_LITTLE_ENDIAN" | "FROM_BIG_ENDIAN" | "FROM_LITTLE_ENDIAN" => one(T(Any), Res::Common),
        // References (6.4.4.10.3).
        "REF" => one(C(Any), Res::Ref),
        _ => return None,
    };
    Some(s)
}

/// A conversion function (Table 22): `X_TO_Y`, `TO_Y`, `X_TRUNC_Y`,
/// `TRUNC_Y`, and the BCD forms `X_BCD_TO_Y`, `BCD_TO_Y`, `X_TO_BCD_Y`,
/// `TO_BCD_Y`. And `TRUNC` alone, deprecated (2a, W0003): DINT, as
/// CODESYS, TwinCAT and Siemens give (docs/dependencies.md).
fn conversion_sig(upper: &str) -> Option<Sig> {
    use Class::*;
    let conv = |input: Kind, to: Ty| Some(sig(vec![("IN", input)], Res::Convert(to)));
    if upper == "TRUNC" {
        return conv(Kind::Class(AnyReal), Ty::Elem(Elem::Dint));
    }
    let int_target = |name: &str| type_named(name).filter(|t| t.is_int());
    if let Some(to) = upper.strip_prefix("TO_BCD_") {
        return conv(Kind::Class(AnyInt), type_named(to).filter(Ty::is_bit)?);
    }
    if let Some(to) = upper.strip_prefix("BCD_TO_") {
        return conv(Kind::Class(AnyBit), int_target(to)?);
    }
    if let Some(to) = upper.strip_prefix("TO_") {
        return conv(Kind::Class(AnyElementary), type_named(to)?);
    }
    if let Some(to) = upper.strip_prefix("TRUNC_") {
        return conv(Kind::Class(AnyReal), int_target(to)?);
    }
    // X_TO_Y and X_TRUNC_Y; X or Y may themselves contain '_'
    // (TIME_OF_DAY_TO_INT, WORD_BCD_TO_INT, INT_TO_BCD_WORD).
    for sep in ["_TO_", "_TRUNC_"] {
        let mut from = 0;
        while let Some(i) = upper[from..].find(sep) {
            let (src, dst) = (&upper[..from + i], &upper[from + i + sep.len()..]);
            from += i + 1;
            if sep == "_TRUNC_" {
                if let (Some(s), Some(d)) = (type_named(src).filter(|t| Class::AnyReal.has(t)), int_target(dst)) {
                    return conv(Kind::Ty(s), d);
                }
                continue;
            }
            if let (Some(s), Some(d)) = (src.strip_suffix("_BCD").and_then(type_named), int_target(dst)) {
                if s.is_bit() {
                    return conv(Kind::Ty(s), d);
                }
            }
            if let (Some(s), Some(d)) = (type_named(src), dst.strip_prefix("BCD_").and_then(type_named)) {
                if s.is_int() && d.is_bit() {
                    return conv(Kind::Ty(s), d);
                }
            }
            if let (Some(s), Some(d)) = (type_named(src), type_named(dst)) {
                return conv(Kind::Ty(s), d);
            }
        }
    }
    None
}

/// Whether the overloaded `sig` may be typed with `t` (6.4.4: the type
/// must be one the overloaded form takes).
fn typable(sig: &Sig, t: &Ty) -> bool {
    let commons = sig.inputs.iter().map(|(_, k)| k).chain(sig.ext.iter().map(|e| &e.kind));
    match &sig.result {
        Res::Op(_, class) => class.has(t),
        _ => {
            let mut any = false;
            for k in commons {
                if let Kind::Common(c) = k {
                    any = true;
                    if !c.has(t) {
                        return false;
                    }
                }
            }
            any
        }
    }
}

/// The interface of the standard function `name`, if it is one: by its
/// own name, a conversion, or a typed form (`ADD_INT`, `MUX_INT_REAL`,
/// Ed.3 6.6.2.4).
pub fn std_sig(name: &str) -> Option<Sig> {
    let upper = name.to_ascii_uppercase();
    if let Some(s) = base_sig(&upper) {
        return Some(s);
    }
    if let Some(s) = conversion_sig(&upper) {
        return Some(s);
    }
    for (i, _) in upper.match_indices('_') {
        let (base, t) = (&upper[..i], &upper[i + 1..]);
        let Some(mut s) = base_sig(base) else { continue };
        if let Some(t) = type_named(t).filter(|t| typable(&s, t)) {
            s.typed = Some(t);
            s.base = Some(base.to_string());
            return Some(s);
        }
        // MUX_K_T: K's type, then the inputs' type.
        if base == "MUX" {
            for (j, _) in t.match_indices('_') {
                let (k, t) = (type_named(&t[..j]), type_named(&t[j + 1..]));
                if let (Some(k), Some(t)) = (k.filter(Ty::is_int), t.filter(|t| typable(&s, t))) {
                    s.typed = Some(t);
                    s.typed_k = Some(k);
                    s.base = Some("MUX".to_string());
                    return Some(s);
                }
            }
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

/// The type both `a` and `b` convert to implicitly, if there is one: the
/// wider of the two; or, for two integers neither of which widens to the
/// other (INT and UINT), the narrowest signed integer both widen to, as
/// Siemens SCL gives "the next larger signed integer type"
/// (docs/dependencies.md): SINT and USINT an INT, INT and UINT a DINT,
/// DINT and UDINT a LINT; ULINT and LINT none.
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
            _ if x.is_int() && y.is_int() => [Elem::Int, Elem::Dint, Elem::Lint]
                .into_iter()
                .find(|c| x.conversion_to(*c) == Implicit && y.conversion_to(*c) == Implicit)
                .map(Ty::Elem),
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
    if std_sig(&upper).is_some() {
        return Some("a standard function");
    }
    if matches!(upper.as_str(), "THIS" | "SUPER") {
        return Some("the own instance or its base (Ed.3 6.6.5.7)");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(name: &str) -> Option<Res> {
        std_sig(name).map(|s| s.result)
    }

    #[test]
    fn conversions_by_name() {
        let to = |t: Ty| Some(Res::Convert(t));
        assert_eq!(result("INT_TO_REAL"), to(Ty::Elem(Elem::Real)));
        assert_eq!(result("TIME_OF_DAY_TO_STRING"), to(Ty::Str { wide: false, len: None }));
        assert_eq!(result("TO_LREAL"), to(Ty::Elem(Elem::Lreal)));
        assert_eq!(result("LREAL_TRUNC_DINT"), to(Ty::Elem(Elem::Dint)));
        assert_eq!(result("TRUNC_INT"), to(Ty::Elem(Elem::Int)));
        assert_eq!(result("TRUNC_REAL"), None, "truncation gives an integer");
        assert_eq!(result("TRUNC"), to(Ty::Elem(Elem::Dint)), "deprecated TRUNC: DINT, as the vendors");
        assert_eq!(result("WORD_BCD_TO_INT"), to(Ty::Elem(Elem::Int)));
        assert_eq!(result("INT_TO_BCD_WORD"), to(Ty::Elem(Elem::Word)));
        assert_eq!(result("BCD_TO_UINT"), to(Ty::Elem(Elem::Uint)));
        assert_eq!(result("TO_BCD_DWORD"), to(Ty::Elem(Elem::Dword)));
        assert_eq!(result("MY_TO_THING"), None);
        assert_eq!(result("INTERVAL"), None);
    }

    #[test]
    fn typed_forms() {
        let s = std_sig("add_int").unwrap();
        assert_eq!(s.typed, Some(Ty::Elem(Elem::Int)));
        assert_eq!(std_sig("ADD_TIME").unwrap().result, Res::Ty(Ty::Elem(Elem::Time)), "Table 35's own");
        assert_eq!(std_sig("MAX_TIME_OF_DAY").unwrap().typed, Some(Ty::Elem(Elem::Tod)));
        assert!(std_sig("ADD_STRING").is_none(), "ADD takes ANY_MAGNITUDE");
        assert!(std_sig("SQRT_INT").is_none(), "SQRT takes ANY_REAL");
        assert!(std_sig("LEN_STRING").is_none(), "LEN is not overloaded on a common type");
        let mux = std_sig("MUX_UINT_REAL").unwrap();
        assert_eq!((mux.typed_k, mux.typed), (Some(Ty::Elem(Elem::Uint)), Some(Ty::Elem(Elem::Real))));
        assert_eq!(std_sig("MUX_REAL").unwrap().typed_k, None);
        assert!(std_sig("MUX_REAL_INT").is_none(), "K is an integer");
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
        assert_eq!(reserved_name("Add_Int"), Some("a standard function"));
        assert_eq!(reserved_name("lamp"), None);
    }
}

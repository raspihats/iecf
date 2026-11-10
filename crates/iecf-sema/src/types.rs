//! Types, and the standard's conversion rules between the elementary ones.

use std::fmt;

/// The elementary data types (Ed.3 6.4.2, Table 10), in the order of the
/// rows and columns of Ed.3 Figure 11.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Elem {
    Lreal,
    Real,
    Lint,
    Dint,
    Int,
    Sint,
    Ulint,
    Udint,
    Uint,
    Usint,
    Lword,
    Dword,
    Word,
    Byte,
    Bool,
    Ltime,
    Time,
    Ldt,
    Dt,
    Ldate,
    Date,
    Ltod,
    Tod,
    /// Strings take part in Figure 11 but carry a length, so a string type
    /// is [`Ty::Str`]; these two are only the matrix's rows and columns.
    Wstring,
    String,
    Wchar,
    Char,
}

use Elem::*;

const ALL: [Elem; 27] = [
    Lreal, Real, Lint, Dint, Int, Sint, Ulint, Udint, Uint, Usint, Lword, Dword, Word, Byte, Bool, Ltime, Time, Ldt,
    Dt, Ldate, Date, Ltod, Tod, Wstring, String, Wchar, Char,
];

/// Ed.3 Figure 11, "Data type conversion rules - implicit and/or explicit":
/// row = source, column = target, in the order of [`ALL`]. `i` implicit,
/// `e` explicit (a conversion function), `-` none defined, `.` the type
/// itself. Transcribed from the 2013 text; Ed.4 is said to make STRING to
/// WSTRING and CHAR to WCHAR implicit (PROPOSAL.md §3a) - to be applied
/// once the Ed.4 text is bought.
const FIGURE_11: [&str; 27] = [
    ". e e e e e e e e e e - - - - - - - - - - - - - - - -", // LREAL
    "i . e e e e e e e e - e - - - - - - - - - - - - - - -", // REAL
    "e e . e e e e e e e e e e e - - - - - - - - - - - - -", // LINT
    "i e i . e e e e e e e e e e - - - - - - - - - - - - -", // DINT
    "i i i i . e e e e e e e e e - - - - - - - - - - - - -", // INT
    "i i i i i . e e e e e e e e - - - - - - - - - - - - -", // SINT
    "e e e e e e . e e e e e e e - - - - - - - - - - - - -", // ULINT
    "i e i e e e i . e e e e e e - - - - - - - - - - - - -", // UDINT
    "i i i i e e i i . e e e e e - - - - - - - - - - - - -", // UINT
    "i i i i i e i i i . e e e e - - - - - - - - - - - - -", // USINT
    "e - e e e e e e e e . e e e - - - - - - - - - - - - -", // LWORD
    "- e e e e e e e e e i . e e - - - - - - - - - - - - -", // DWORD
    "- - e e e e e e e e i i . e - - - - - - - - - - - e -", // WORD
    "- - e e e e e e e e i i i . - - - - - - - - - - - - e", // BYTE
    "- - e e e e e e e e i i i i . - - - - - - - - - - - -", // BOOL
    "- - - - - - - - - - - - - - - . e - - - - - - - - - -", // LTIME
    "- - - - - - - - - - - - - - - i . - - - - - - - - - -", // TIME
    "- - - - - - - - - - - - - - - - - . e e e e e - - - -", // LDT
    "- - - - - - - - - - - - - - - - - i . e e e e - - - -", // DT
    "- - - - - - - - - - - - - - - - - - - . e - - - - - -", // LDATE
    "- - - - - - - - - - - - - - - - - - - i . - - - - - -", // DATE
    "- - - - - - - - - - - - - - - - - - - - - . e - - - -", // LTOD
    "- - - - - - - - - - - - - - - - - - - - - i . - - - -", // TOD
    "- - - - - - - - - - - - - - - - - - - - - - - . e - -", // WSTRING
    "- - - - - - - - - - - - - - - - - - - - - - - e . - e", // STRING
    "- - - - - - - - - - e e e - - - - - - - - - - i - . e", // WCHAR
    "- - - - - - - - - e e e e e - - - - - - - - - - i e .", // CHAR
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conversion {
    Same,
    Implicit,
    Explicit,
    None,
}

impl Elem {
    fn index(self) -> usize {
        ALL.iter().position(|e| *e == self).unwrap_or(0)
    }

    /// What Ed.3 Figure 11 allows from `self` to `to`.
    pub fn conversion_to(self, to: Elem) -> Conversion {
        let row = FIGURE_11[self.index()];
        match row.split(' ').nth(to.index()) {
            Some(".") => Conversion::Same,
            Some("i") => Conversion::Implicit,
            Some("e") => Conversion::Explicit,
            _ => Conversion::None,
        }
    }

    /// Its size in the process image, in bits (docs/dependencies.md):
    /// the widths iecf holds the types in; none for the strings.
    pub fn bits(self) -> Option<u32> {
        match self {
            Bool => Some(1),
            Byte | Sint | Usint | Char => Some(8),
            Word | Int | Uint | Wchar => Some(16),
            Dword | Dint | Udint | Real | Time | Date | Tod | Dt => Some(32),
            Lword | Lint | Ulint | Lreal | Ltime | Ldate | Ltod | Ldt => Some(64),
            String | Wstring => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lreal => "LREAL",
            Real => "REAL",
            Lint => "LINT",
            Dint => "DINT",
            Int => "INT",
            Sint => "SINT",
            Ulint => "ULINT",
            Udint => "UDINT",
            Uint => "UINT",
            Usint => "USINT",
            Lword => "LWORD",
            Dword => "DWORD",
            Word => "WORD",
            Byte => "BYTE",
            Bool => "BOOL",
            Ltime => "LTIME",
            Time => "TIME",
            Ldt => "LDT",
            Dt => "DT",
            Ldate => "LDATE",
            Date => "DATE",
            Ltod => "LTOD",
            Tod => "TOD",
            Wstring => "WSTRING",
            String => "STRING",
            Wchar => "WCHAR",
            Char => "CHAR",
        }
    }

    /// The elementary type a name denotes, long forms included
    /// (`TIME_OF_DAY` is `TOD`), ignoring case. Strings are not here: they
    /// are [`Ty::Str`].
    pub fn from_name(name: &str) -> Option<Elem> {
        let upper = name.to_ascii_uppercase();
        let e = match upper.as_str() {
            "TIME_OF_DAY" => Tod,
            "LTIME_OF_DAY" => Ltod,
            "DATE_AND_TIME" => Dt,
            "LDATE_AND_TIME" => Ldt,
            "STRING" | "WSTRING" => return None,
            _ => *ALL.iter().find(|e| e.name() == upper)?,
        };
        Some(e)
    }

    pub fn is_signed_int(self) -> bool {
        matches!(self, Lint | Dint | Int | Sint)
    }

    pub fn is_unsigned_int(self) -> bool {
        matches!(self, Ulint | Udint | Uint | Usint)
    }

    /// ANY_INT
    pub fn is_int(self) -> bool {
        self.is_signed_int() || self.is_unsigned_int()
    }

    /// ANY_REAL
    pub fn is_real(self) -> bool {
        matches!(self, Real | Lreal)
    }

    /// ANY_NUM
    pub fn is_num(self) -> bool {
        self.is_int() || self.is_real()
    }

    /// ANY_BIT
    pub fn is_bit(self) -> bool {
        matches!(self, Bool | Byte | Word | Dword | Lword)
    }

    /// ANY_DURATION
    pub fn is_duration(self) -> bool {
        matches!(self, Time | Ltime)
    }

    /// ANY_DATE
    pub fn is_date(self) -> bool {
        matches!(self, Date | Ldate | Tod | Ltod | Dt | Ldt)
    }

    /// The range of an integer type, for literals.
    pub fn int_range(self) -> Option<(i128, i128)> {
        Some(match self {
            Sint => (i8::MIN.into(), i8::MAX.into()),
            Int => (i16::MIN.into(), i16::MAX.into()),
            Dint => (i32::MIN.into(), i32::MAX.into()),
            Lint => (i64::MIN.into(), i64::MAX.into()),
            Usint | Byte => (0, u8::MAX.into()),
            Uint | Word => (0, u16::MAX.into()),
            Udint | Dword => (0, u32::MAX.into()),
            Ulint | Lword => (0, u64::MAX.into()),
            Bool => (0, 1),
            _ => return None,
        })
    }
}

/// A type as the checker sees it. Named types are kept by their name in
/// upper case, which is how IEC compares names.
#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Elem(Elem),
    /// `STRING[n]` or `WSTRING[n]`; `len` `None` is the default length.
    Str {
        wide: bool,
        len: Option<u64>,
    },
    /// Each dimension's bounds, `None` where not a constant (`*`, or
    /// already reported): two arrays are one type only with the same
    /// bounds.
    Array {
        elem: Box<Ty>,
        dims: Vec<Option<(i128, i128)>>,
    },
    Struct(std::string::String),
    Enum(std::string::String),
    /// A function block type, standard or the project's.
    Fb(std::string::String),
    /// An INTERFACE (Ed.3 6.6.6.5): a reference to an instance of a type
    /// that implements it, or NULL.
    Itf(std::string::String),
    Ref(Box<Ty>),
    /// CODESYS's `REFERENCE TO` (the reference-to extension): a reference
    /// a value of it stands for what it references; the checker gives an
    /// expression of it that type, but where REF= or a call binds it.
    Reference(Box<Ty>),
    /// An integer literal without a type: it takes the type it meets,
    /// if its value fits. The value, if it could be computed.
    IntLit(Option<i128>),
    /// A real literal without a type.
    RealLit,
    /// `NULL`
    Null,
    /// What a function block call or a function without a result gives.
    Void,
    /// Unknown because an error was already reported, or not knowable
    /// yet. Checks against it pass silently: one error, not a cascade.
    Unknown,
}

impl Ty {
    pub fn elem(&self) -> Option<Elem> {
        match self {
            Ty::Elem(e) => Some(*e),
            _ => None,
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Ty::Unknown)
    }

    /// ANY_INT, an untyped integer literal included.
    pub fn is_int(&self) -> bool {
        matches!(self, Ty::IntLit(_)) || self.elem().is_some_and(Elem::is_int)
    }

    pub fn is_num(&self) -> bool {
        matches!(self, Ty::IntLit(_) | Ty::RealLit) || self.elem().is_some_and(Elem::is_num)
    }

    pub fn is_bit(&self) -> bool {
        self.elem().is_some_and(Elem::is_bit)
    }

    pub fn is_bool(&self) -> bool {
        self.elem() == Some(Bool)
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Elem(e) => f.write_str(e.name()),
            Ty::Str { wide, len } => {
                f.write_str(if *wide { "WSTRING" } else { "STRING" })?;
                match len {
                    Some(n) => write!(f, "[{n}]"),
                    None => Ok(()),
                }
            }
            Ty::Array { elem, dims } => {
                f.write_str("ARRAY[")?;
                for (i, d) in dims.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    match d {
                        Some((lo, hi)) => write!(f, "{lo}..{hi}")?,
                        None => f.write_str("*")?,
                    }
                }
                write!(f, "] OF {elem}")
            }
            Ty::Struct(n) | Ty::Enum(n) | Ty::Fb(n) | Ty::Itf(n) => f.write_str(n),
            Ty::Ref(t) => write!(f, "REF_TO {t}"),
            Ty::Reference(t) => write!(f, "REFERENCE TO {t}"),
            Ty::IntLit(_) => f.write_str("an integer literal"),
            Ty::RealLit => f.write_str("a real literal"),
            Ty::Null => f.write_str("NULL"),
            Ty::Void => f.write_str("no value"),
            Ty::Unknown => f.write_str("?"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_matrix_is_square_with_its_diagonal() {
        for (i, row) in FIGURE_11.iter().enumerate() {
            let cells: Vec<&str> = row.split(' ').collect();
            assert_eq!(cells.len(), 27, "row {}", ALL[i].name());
            for (j, c) in cells.iter().enumerate() {
                assert!(["i", "e", "-", "."].contains(c), "row {} col {j}: {c}", ALL[i].name());
                assert_eq!(*c == ".", i == j, "row {} col {j}", ALL[i].name());
            }
        }
    }

    #[test]
    fn implicit_conversions_only_widen() {
        // Every implicit conversion keeps the value (Ed.3 6.6.1.6, rule 1):
        // spot-check the ones the figure is about, and that no narrowing
        // slipped into the transcription.
        assert_eq!(Int.conversion_to(Dint), Conversion::Implicit);
        assert_eq!(Int.conversion_to(Real), Conversion::Implicit);
        assert_eq!(Dint.conversion_to(Real), Conversion::Explicit, "a DINT does not fit a REAL's mantissa");
        assert_eq!(Dint.conversion_to(Int), Conversion::Explicit);
        assert_eq!(Usint.conversion_to(Int), Conversion::Implicit);
        assert_eq!(Usint.conversion_to(Sint), Conversion::Explicit);
        assert_eq!(Bool.conversion_to(Byte), Conversion::Implicit);
        assert_eq!(Byte.conversion_to(Bool), Conversion::None, "bit-string-to-bool is an extension");
        assert_eq!(Time.conversion_to(Ltime), Conversion::Implicit);
        assert_eq!(Time.conversion_to(Dint), Conversion::None, "no integer-TIME conversion in Figure 11");
        assert_eq!(Char.conversion_to(String), Conversion::Implicit);
        assert_eq!(String.conversion_to(Wstring), Conversion::Explicit);
        for a in ALL {
            for b in ALL {
                if a.conversion_to(b) == Conversion::Implicit {
                    if let (Some((alo, ahi)), Some((blo, bhi))) = (a.int_range(), b.int_range()) {
                        assert!(blo <= alo && ahi <= bhi, "{} to {} is implicit but narrows", a.name(), b.name());
                    }
                }
            }
        }
    }

    #[test]
    fn names() {
        assert_eq!(Elem::from_name("time_of_day"), Some(Tod));
        assert_eq!(Elem::from_name("Dint"), Some(Dint));
        assert_eq!(Elem::from_name("STRING"), None);
        assert_eq!(Elem::from_name("TON"), None);
    }
}

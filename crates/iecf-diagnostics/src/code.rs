use crate::Severity;

macro_rules! codes {
    ($( $variant:ident = $id:literal, $severity:ident, $title:literal; )*) => {
        /// Every diagnostic iecf can give. The explanation of each is
        /// `docs/errors/<id>.md`, compiled in: no file, no build.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Code {
            $( $variant, )*
        }

        impl Code {
            pub const ALL: &'static [Code] = &[$( Code::$variant, )*];

            /// `E0005`, `W0001`: stable, printed with every diagnostic.
            pub fn id(self) -> &'static str {
                match self { $( Code::$variant => $id, )* }
            }

            pub fn default_severity(self) -> Severity {
                match self { $( Code::$variant => Severity::$severity, )* }
            }

            /// One line: what the code is about.
            pub fn title(self) -> &'static str {
                match self { $( Code::$variant => $title, )* }
            }

            /// The markdown explanation: the rule, an example, the fix.
            pub fn explanation(self) -> &'static str {
                match self {
                    $( Code::$variant => include_str!(concat!("../../../docs/errors/", $id, ".md")), )*
                }
            }

            /// The code with this id, ignoring case.
            pub fn from_id(id: &str) -> Option<Code> {
                Code::ALL.iter().copied().find(|c| c.id().eq_ignore_ascii_case(id))
            }
        }
    };
}

codes! {
    UnexpectedCharacter = "E0001", Error, "a character that has no place in IEC 61131-3 text";
    UnterminatedComment = "E0002", Error, "a comment that is never closed";
    UnterminatedString = "E0003", Error, "a string literal that is never closed";
    InvalidLiteral = "E0004", Error, "a literal that is not well formed";
    UnexpectedToken = "E0005", Error, "the text does not fit the grammar here";
    UnclosedBlock = "E0006", Error, "a block whose END keyword never comes";
    InvalidEscape = "E0007", Error, "a `$` escape that a string literal does not have";
    InstructionList = "E0008", Error, "Instruction List, removed from IEC 61131-3 in Ed.4";
    NotYetSupported = "E0009", Error, "a part of IEC 61131-3 that iecf does not compile yet";
    UnknownName = "E0010", Error, "a name that nothing declares";
    UnknownType = "E0011", Error, "a type that nothing declares";
    Duplicate = "E0012", Error, "a name declared twice";
    TypeMismatch = "E0013", Error, "a value whose type does not convert implicitly";
    OperandTypes = "E0014", Error, "operands an operator or index does not take";
    NotAssignable = "E0015", Error, "an assignment to what cannot be assigned";
    BadCall = "E0016", Error, "a call that does not fit what it calls";
    ExternalWithoutGlobal = "E0017", Error, "a VAR_EXTERNAL that matches no VAR_GLOBAL";
    ReservedName = "E0018", Error, "a name the standard reserves for a type, function or block";
    NotBool = "E0019", Error, "a condition that is not BOOL";
    LiteralRange = "E0020", Error, "a literal outside its type's range";
    LoopControlOutsideLoop = "E0021", Error, "EXIT or CONTINUE outside a loop";
    BadLocation = "E0022", Error, "an address where the standard does not allow one";
    UnconfiguredLocation = "E0023", Error, "a `*` address that VAR_CONFIG does not complete";
    BadConfigEntry = "E0024", Error, "a VAR_CONFIG entry that does not fit the variable it names";
    ConfigOutsideConfiguration = "E0025", Error, "a VAR_CONFIG outside a CONFIGURATION";
    BadReference = "E0026", Error, "REF() of what cannot be referenced";
    ReferenceNotKept = "E0027", Error, "a reference where it cannot be";
    NamespaceAccess = "E0028", Error, "a name a namespace does not have, or does not let this code see";
    AmbiguousName = "E0029", Error, "a short name that two USINGs both bring in";
    AccessDenied = "E0030", Error, "a method or variable its access specifier keeps from here";
    BadObject = "E0031", Error, "a class, method, interface, THIS or specifier where Ed.3 does not allow it";
    NotConstant = "E0032", Error, "a value the standard requires constant that is not";
    VariableLengthArray = "E0033", Error, "an ARRAY[*] where Ed.3 does not allow one";
    BadLayout = "E0034", Error, "a structure layout Ed.3 does not allow";
    BadAccessPath = "E0035", Error, "a VAR_ACCESS entry that does not fit the variable it names";
    OctalLiteral = "W0001", Warning, "an octal literal, removed from IEC 61131-3 in Ed.4";
    DirectAddressInBody = "W0002", Warning, "a direct address in a body, deprecated since Ed.3";
    TruncDeprecated = "W0003", Warning, "TRUNC without a type, deprecated since Ed.3";
    LocationSize = "W0004", Warning, "an address whose size prefix is not its variable's size";
    ReferenceToInput = "W0005", Warning, "REF() of a variable at an input address";
    KeptReference = "W0006", Warning, "a reference kept across a warm restart";
    IndicatorDeprecated = "W0007", Warning, "an SFC indicator variable, deprecated since Ed.3";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ids_are_unique_and_well_formed() {
        let mut seen = BTreeSet::new();
        for c in Code::ALL {
            let id = c.id();
            assert!(seen.insert(id), "{id} twice");
            let (letter, number) = id.split_at(1);
            let expected = match c.default_severity() {
                Severity::Error => "E",
                Severity::Warning => "W",
            };
            assert_eq!(letter, expected, "{id}: the letter says the default severity");
            assert!(number.len() == 4 && number.bytes().all(|b| b.is_ascii_digit()), "{id}");
        }
    }

    #[test]
    fn every_explanation_starts_with_its_id_and_title() {
        for c in Code::ALL {
            let first = c.explanation().lines().next().unwrap_or_default();
            assert_eq!(first, format!("# {}: {}", c.id(), c.title()), "docs/errors/{}.md", c.id());
        }
    }

    #[test]
    fn every_explanation_file_has_a_code() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/errors");
        for entry in std::fs::read_dir(dir).expect("docs/errors") {
            let name = entry.unwrap().file_name().into_string().unwrap();
            let id = name.strip_suffix(".md").expect("only .md files in docs/errors");
            assert!(Code::from_id(id).is_some(), "docs/errors/{name} has no Code");
        }
    }
}

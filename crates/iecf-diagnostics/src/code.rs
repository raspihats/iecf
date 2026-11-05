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
    OctalLiteral = "W0001", Warning, "an octal literal, removed from IEC 61131-3 in Ed.4";
    DirectAddressInBody = "W0002", Warning, "a direct address in a body, deprecated since Ed.3";
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

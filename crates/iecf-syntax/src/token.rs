//! Tokens. Trivia - whitespace, comments, pragmas - are tokens too, so a
//! formatter or the editor can have them; the parser skips them.

use iecf_diagnostics::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Ident,
    Kw(Kw),
    /// `42`, `1_000`
    Int,
    /// `3.14`, `1.0E-3`
    Real,
    /// `16#FF`, `2#1010`, `8#17`
    Based,
    /// `T#1s`, `TIME#-5ms`, `D#2026-11-05`, `TOD#12:00:00`, `DT#...`
    Time,
    /// `'text'`
    String,
    /// `"text"`
    WString,
    /// `%IX0.0`, `%QW2`, `%MD4`, `%I*`
    DirectAddress,
    /// `{ ... }`
    Pragma,
    /// `:=`
    Assign,
    /// `?=`, an assignment attempt (Ed.3 6.6.6.7)
    AssignAttempt,
    /// `=>`
    Arrow,
    Colon,
    Semi,
    Comma,
    Dot,
    /// `..`
    DotDot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Caret,
    Hash,
    Plus,
    Minus,
    Star,
    /// `**`
    Power,
    Slash,
    Eq,
    /// `<>`
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Amp,
    Whitespace,
    Comment,
    /// A character E0001 was reported for.
    Unknown,
    Eof,
}

impl TokenKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, TokenKind::Whitespace | TokenKind::Comment | TokenKind::Pragma)
    }

    /// How the kind reads in "expected X, found Y".
    pub fn describe(self) -> &'static str {
        match self {
            TokenKind::Ident => "a name",
            TokenKind::Kw(kw) => kw.as_str(),
            TokenKind::Int => "an integer",
            TokenKind::Real => "a real number",
            TokenKind::Based => "a based number",
            TokenKind::Time => "a time or date literal",
            TokenKind::String => "a string",
            TokenKind::WString => "a wide string",
            TokenKind::DirectAddress => "a direct address",
            TokenKind::Pragma => "a pragma",
            TokenKind::Assign => "':='",
            TokenKind::AssignAttempt => "'?='",
            TokenKind::Arrow => "'=>'",
            TokenKind::Colon => "':'",
            TokenKind::Semi => "';'",
            TokenKind::Comma => "','",
            TokenKind::Dot => "'.'",
            TokenKind::DotDot => "'..'",
            TokenKind::LParen => "'('",
            TokenKind::RParen => "')'",
            TokenKind::LBracket => "'['",
            TokenKind::RBracket => "']'",
            TokenKind::Caret => "'^'",
            TokenKind::Hash => "'#'",
            TokenKind::Plus => "'+'",
            TokenKind::Minus => "'-'",
            TokenKind::Star => "'*'",
            TokenKind::Power => "'**'",
            TokenKind::Slash => "'/'",
            TokenKind::Eq => "'='",
            TokenKind::Ne => "'<>'",
            TokenKind::Lt => "'<'",
            TokenKind::Le => "'<='",
            TokenKind::Gt => "'>'",
            TokenKind::Ge => "'>='",
            TokenKind::Amp => "'&'",
            TokenKind::Whitespace => "white space",
            TokenKind::Comment => "a comment",
            TokenKind::Unknown => "an unknown character",
            TokenKind::Eof => "the end of the file",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

macro_rules! keywords {
    ($( $variant:ident = $text:literal, )*) => {
        /// The keywords iecf's grammar uses. Data type names (`INT`,
        /// `STRING`) and standard function names are keywords in the
        /// standard too (Ed.2 Table C.2), but they are read as names and
        /// resolved later, which gives better messages.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Kw {
            $( $variant, )*
        }

        impl Kw {
            pub fn as_str(self) -> &'static str {
                match self { $( Kw::$variant => $text, )* }
            }

            /// The keyword spelled `text`, ignoring case (Ed.3 6.1.3).
            pub fn from_text(text: &str) -> Option<Kw> {
                let mut buf = [0u8; 24];
                if text.len() > buf.len() {
                    return None;
                }
                let upper = &mut buf[..text.len()];
                upper.copy_from_slice(text.as_bytes());
                upper.make_ascii_uppercase();
                match &*upper {
                    $( x if x == $text.as_bytes() => Some(Kw::$variant), )*
                    _ => None,
                }
            }
        }
    };
}

keywords! {
    Program = "PROGRAM",
    EndProgram = "END_PROGRAM",
    Function = "FUNCTION",
    EndFunction = "END_FUNCTION",
    FunctionBlock = "FUNCTION_BLOCK",
    EndFunctionBlock = "END_FUNCTION_BLOCK",
    Class = "CLASS",
    EndClass = "END_CLASS",
    Interface = "INTERFACE",
    EndInterface = "END_INTERFACE",
    Method = "METHOD",
    EndMethod = "END_METHOD",
    PropertyGet = "PROPERTY_GET",
    PropertySet = "PROPERTY_SET",
    EndProperty = "END_PROPERTY",
    Namespace = "NAMESPACE",
    EndNamespace = "END_NAMESPACE",
    Using = "USING",
    Extends = "EXTENDS",
    Implements = "IMPLEMENTS",
    Var = "VAR",
    VarInput = "VAR_INPUT",
    VarOutput = "VAR_OUTPUT",
    VarInOut = "VAR_IN_OUT",
    VarTemp = "VAR_TEMP",
    VarExternal = "VAR_EXTERNAL",
    VarGlobal = "VAR_GLOBAL",
    VarAccess = "VAR_ACCESS",
    VarConfig = "VAR_CONFIG",
    EndVar = "END_VAR",
    Constant = "CONSTANT",
    Retain = "RETAIN",
    NonRetain = "NON_RETAIN",
    At = "AT",
    Type = "TYPE",
    EndType = "END_TYPE",
    Struct = "STRUCT",
    EndStruct = "END_STRUCT",
    Overlap = "OVERLAP",
    Array = "ARRAY",
    Of = "OF",
    RefTo = "REF_TO",
    REdge = "R_EDGE",
    FEdge = "F_EDGE",
    If = "IF",
    Then = "THEN",
    Elsif = "ELSIF",
    Else = "ELSE",
    EndIf = "END_IF",
    Case = "CASE",
    EndCase = "END_CASE",
    For = "FOR",
    To = "TO",
    By = "BY",
    Do = "DO",
    EndFor = "END_FOR",
    While = "WHILE",
    EndWhile = "END_WHILE",
    Repeat = "REPEAT",
    Until = "UNTIL",
    EndRepeat = "END_REPEAT",
    Exit = "EXIT",
    Continue = "CONTINUE",
    Return = "RETURN",
    And = "AND",
    Or = "OR",
    Xor = "XOR",
    Not = "NOT",
    Mod = "MOD",
    True = "TRUE",
    False = "FALSE",
    Step = "STEP",
    InitialStep = "INITIAL_STEP",
    EndStep = "END_STEP",
    Transition = "TRANSITION",
    From = "FROM",
    EndTransition = "END_TRANSITION",
    Action = "ACTION",
    EndAction = "END_ACTION",
    Configuration = "CONFIGURATION",
    EndConfiguration = "END_CONFIGURATION",
    Resource = "RESOURCE",
    On = "ON",
    EndResource = "END_RESOURCE",
    Task = "TASK",
    With = "WITH",
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_ignore_case() {
        assert_eq!(Kw::from_text("end_if"), Some(Kw::EndIf));
        assert_eq!(Kw::from_text("End_If"), Some(Kw::EndIf));
        assert_eq!(Kw::from_text("endif"), None);
        assert_eq!(Kw::from_text("INT"), None);
        assert_eq!(Kw::from_text("a_very_long_name_that_is_no_keyword_at_all"), None);
    }
}

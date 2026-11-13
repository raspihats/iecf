//! The syntax tree: what the text says, with a span on every node. Names
//! keep their spelling; IEC compares them ignoring case (Ed.3 6.1.2), and
//! that is the resolver's business, not the tree's.

use iecf_diagnostics::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub text: String,
    pub span: Span,
}

impl Ident {
    /// Same name, by IEC's rule: case does not matter.
    pub fn is(&self, other: &str) -> bool {
        self.text.eq_ignore_ascii_case(other)
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SourceUnit {
    pub items: Vec<Item>,
}

// Most items are POUs, held in a file's list once: boxing each would
// buy nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Pou(Pou),
    Types(Vec<TypeDecl>),
    /// `VAR_GLOBAL` at the top of a file: a global variable list.
    Globals(VarBlock),
    Configuration(Configuration),
    /// `VAR_CONFIG` at the top of a file, outside any CONFIGURATION, as
    /// CODESYS writes it: an implementer extension, checked only when
    /// switched on. The span is the `VAR_CONFIG` keyword.
    ConfigList(Vec<ConfigInit>, Span),
    /// `NAMESPACE [INTERNAL] A.B ... END_NAMESPACE` (Ed.3 6.9). The
    /// checker never sees one: namespaces.rs in iecf-sema takes its
    /// elements out, each named by its whole path.
    Namespace(Namespace),
    /// `USING A.B, C;` at the top of a file (Ed.3 6.9.4): for that file.
    Using(Using),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Namespace {
    /// `Standard.Timers` is two parts.
    pub name: Vec<Ident>,
    pub internal: bool,
    pub using: Vec<Using>,
    pub items: Vec<Item>,
    pub span: Span,
}

/// `USING A.B, C;`: the namespaces, each a path.
#[derive(Clone, Debug, PartialEq)]
pub struct Using {
    pub paths: Vec<Vec<Ident>>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PouKind {
    Program,
    Function,
    FunctionBlock,
    /// `CLASS ... END_CLASS` (Ed.3 6.6.5): variables and methods, no body.
    Class,
    /// A `METHOD` of a CLASS or FUNCTION_BLOCK, in its `methods`; of an
    /// INTERFACE, a prototype: no body (Ed.3 6.6.6.1).
    Method,
    /// `INTERFACE ... END_INTERFACE` (Ed.3 6.6.6): method prototypes only.
    Interface,
}

impl PouKind {
    /// As its declaration's first word spells it.
    pub fn keyword(self) -> &'static str {
        match self {
            PouKind::Program => "PROGRAM",
            PouKind::Function => "FUNCTION",
            PouKind::FunctionBlock => "FUNCTION_BLOCK",
            PouKind::Class => "CLASS",
            PouKind::Method => "METHOD",
            PouKind::Interface => "INTERFACE",
        }
    }
}

/// Who may use a method or a VAR section (Ed.3 6.6.5.9, 6.6.5.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Public,
    Private,
    Protected,
    Internal,
}

impl Access {
    pub fn name(self) -> &'static str {
        match self {
            Access::Public => "PUBLIC",
            Access::Private => "PRIVATE",
            Access::Protected => "PROTECTED",
            Access::Internal => "INTERNAL",
        }
    }
}

/// What may stand before a CLASS's, FUNCTION_BLOCK's or METHOD's name,
/// in any order (OO.md D2), and where each was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Specs {
    pub access: Option<(Access, Span)>,
    pub final_: Option<Span>,
    pub abstract_: Option<Span>,
    pub override_: Option<Span>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pou {
    pub kind: PouKind,
    pub name: Ident,
    /// A FUNCTION's result type, if it has one.
    pub return_type: Option<TypeSpec>,
    pub var_blocks: Vec<VarBlock>,
    /// A PROGRAM's `VAR_ACCESS` entries.
    pub accesses: Vec<AccessDecl>,
    pub body: Vec<Stmt>,
    /// SFC: steps, transitions and actions (Ed.3 6.7).
    pub sfc: Vec<SfcElement>,
    /// `FUNCTION INTERNAL f`: seen only inside its namespace (Ed.3 6.9.2).
    pub internal: bool,
    /// `USING` after its name (Ed.3 6.9.4).
    pub using: Vec<Using>,
    /// FINAL, ABSTRACT; for a method also its access and OVERRIDE.
    pub specs: Specs,
    /// A CLASS's or FUNCTION_BLOCK's methods (Ed.3 6.6.5.4, 6.6.7.2),
    /// each a POU of kind `Method`.
    pub methods: Vec<Pou>,
    /// `EXTENDS base` (Ed.3 6.6.5.5, 6.6.7.2.7): a CLASS's or
    /// FUNCTION_BLOCK's one base type.
    pub extends: Option<Ident>,
    /// A CLASS's or FUNCTION_BLOCK's `IMPLEMENTS i1, i2` (Ed.3 6.6.6.4);
    /// an INTERFACE's `EXTENDS i1, i2`, its bases (6.6.6.3).
    pub interfaces: Vec<Ident>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarKind {
    Var,
    Input,
    Output,
    InOut,
    Temp,
    External,
    Global,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarBlock {
    pub kind: VarKind,
    pub constant: bool,
    pub retain: Option<Retain>,
    /// `VAR PUBLIC` (Ed.3 6.6.5.10): PROTECTED when not written.
    pub access: Option<(Access, Span)>,
    pub decls: Vec<VarDecl>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retain {
    Retain,
    NonRetain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Rising,
    Falling,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarDecl {
    /// Empty for `AT %IX0.0 : BOOL;`, a located variable with no name.
    pub names: Vec<Ident>,
    pub location: Option<DirectAddress>,
    pub ty: TypeSpec,
    pub edge: Option<Edge>,
    pub init: Option<Init>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectAddress {
    /// As written, `%IX0.0`.
    pub text: String,
    pub span: Span,
}

impl DirectAddress {
    /// `%I*`, `%Q*`, `%M*`: a location the configuration completes
    /// (Ed.3 6.5.5.4).
    pub fn is_partly(&self) -> bool {
        self.text.ends_with('*')
    }

    /// `I`, `Q` or `M`; none for a relative location.
    pub fn area(&self) -> Option<char> {
        self.text.chars().nth(1).map(|c| c.to_ascii_uppercase()).filter(|c| matches!(c, 'I' | 'Q' | 'M'))
    }

    /// `%B2`, `%X3.0`: no area, a place relative to the start of the
    /// structure the field is in (Ed.3 6.4.4.7).
    pub fn is_relative(&self) -> bool {
        self.area().is_none()
    }

    /// A relative location's byte, and its bit when it is a bit location
    /// (`X`, or no size prefix, as for a direct address): `%X3.5` is
    /// (3, Some(5)), `%B2` and `%W2` are (2, None). `None` when it is not
    /// one: a bit location has a bit from 0 to 7, a byte location none.
    pub fn relative_place(&self) -> Option<(u32, Option<u8>)> {
        if !self.is_relative() {
            return None;
        }
        let rest = self.text.strip_prefix('%')?;
        let size = self.size()?;
        let rest = rest.strip_prefix(|c: char| c.is_ascii_alphabetic()).unwrap_or(rest);
        let parts: Vec<&str> = rest.split('.').collect();
        match (size, parts.as_slice()) {
            ('X', [byte, bit]) => Some((byte.parse().ok()?, Some(bit.parse().ok().filter(|b: &u8| *b < 8)?))),
            ('X', _) => None,
            (_, [byte]) => Some((byte.parse().ok()?, None)),
            _ => None,
        }
    }

    /// The size prefix (Ed.3 Table 16): `X`, `B`, `W`, `D` or `L`, `X`
    /// when there is none; none for a `*` address.
    pub fn size(&self) -> Option<char> {
        let at = if self.is_relative() { 1 } else { 2 };
        match self.text.chars().nth(at)?.to_ascii_uppercase() {
            c @ ('X' | 'B' | 'W' | 'D' | 'L') => Some(c),
            c if c.is_ascii_digit() => Some('X'),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeSpec {
    /// `INT`, `TON`, `MyStruct`; `Standard.Timers.TON` is one name, its
    /// parts joined by `.` (Ed.3 6.9.3).
    Named(Ident),
    /// `STRING`, `STRING[80]`, `WSTRING(10)`: the name and its length.
    String { name: Ident, len: Option<Box<Expr>>, span: Span },
    /// `ARRAY[1..10, 0..2] OF INT`; `None` bounds are `*` (Ed.3 6.5.3).
    Array { dims: Vec<Option<(Expr, Expr)>>, elem: Box<TypeSpec>, span: Span },
    /// `REF_TO INT`.
    /// `REF_TO T`; `REFERENCE TO T` (CODESYS, the reference-to extension)
    /// is `implicit`: dereferenced wherever it is used.
    Ref { target: Box<TypeSpec>, implicit: bool, span: Span },
    /// `INT (0..100)`.
    Subrange { base: Ident, low: Box<Expr>, high: Box<Expr>, span: Span },
    /// `(Red, Green)` or `INT (Red := 1, Green := 2)`.
    Enum { base: Option<Ident>, values: Vec<EnumValue>, span: Span },
}

impl TypeSpec {
    pub fn span(&self) -> Span {
        match self {
            TypeSpec::Named(id) => id.span,
            TypeSpec::String { span, .. }
            | TypeSpec::Array { span, .. }
            | TypeSpec::Ref { span, .. }
            | TypeSpec::Subrange { span, .. }
            | TypeSpec::Enum { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumValue {
    pub name: Ident,
    pub value: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Init {
    Expr(Expr),
    /// `[1, 2, 3(0)]`
    Array {
        elems: Vec<ArrayInitElem>,
        span: Span,
    },
    /// `(x := 1, y := 2)`
    Struct {
        fields: Vec<(Ident, Init)>,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayInitElem {
    /// `3(0)` repeats `0` three times.
    pub repeat: Option<Expr>,
    pub value: Option<Init>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    pub name: Ident,
    pub def: TypeDef,
    pub init: Option<Init>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeDef {
    Spec(TypeSpec),
    Struct { overlap: bool, fields: Vec<VarDecl>, span: Span },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Assign {
        target: Expr,
        value: Expr,
    },
    /// A call standing as a statement: `timer(IN := x, PT := T#1s);`.
    Call(Expr),
    /// `r REF= x;`: a REFERENCE TO bound to `x` (CODESYS, the
    /// reference-to extension).
    RefAssign {
        target: Expr,
        value: Expr,
    },
    /// `a ?= b;`: `a` gets `b` if what `b` references is of a type `a`
    /// can hold, NULL otherwise (Ed.3 6.6.6.7).
    AssignAttempt {
        target: Expr,
        value: Expr,
    },
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
    },
    Case {
        selector: Expr,
        arms: Vec<CaseArm>,
        else_body: Option<Vec<Stmt>>,
    },
    For {
        var: Ident,
        from: Expr,
        to: Expr,
        by: Option<Expr>,
        body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Repeat {
        body: Vec<Stmt>,
        until: Expr,
    },
    Exit,
    Continue,
    Return,
    /// A lone `;`.
    Empty,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaseArm {
    pub labels: Vec<CaseLabel>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseLabel {
    Value(Expr),
    Range(Expr, Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    Xor,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Power,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Plus,
    Not,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Literal(Literal),
    Name(Ident),
    DirectAddress(DirectAddress),
    /// `Color#Red`: an enumerated value named with its type.
    EnumValue {
        ty: Ident,
        value: Ident,
    },
    /// `a.b`
    Member(Box<Expr>, Ident),
    /// `a.3`, `a.%X3`, `a.%B1`: partial access of a bit string (Ed.3
    /// Table 17), the literal's text as written - [`Literal::part`].
    BitAccess(Box<Expr>, Literal),
    /// `a[i, j]`
    Index(Box<Expr>, Vec<Expr>),
    /// `r^`
    Deref(Box<Expr>),
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Paren(Box<Expr>),
    /// What the parser put where an expression was missing; an error was
    /// already reported for it.
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Positional(Expr),
    /// `IN := x`
    Input(Ident, Expr),
    /// `Q => y`, `NOT Q => y`
    Output {
        not: bool,
        name: Ident,
        target: Expr,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralKind {
    Int,
    Real,
    Based,
    Bool,
    String,
    WString,
    Time,
}

/// A literal as written. Its value is computed when types are known: the
/// same text is a different value as `SINT#` or `LINT#`.
/// The size of a part of a bit string (Ed.3 Table 17).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartSize {
    Bit,
    Byte,
    Word,
    Dword,
    Lword,
}

impl PartSize {
    pub fn bits(self) -> u32 {
        match self {
            PartSize::Bit => 1,
            PartSize::Byte => 8,
            PartSize::Word => 16,
            PartSize::Dword => 32,
            PartSize::Lword => 64,
        }
    }
}

impl Literal {
    /// A partial access's size and index: `3` and `%X3` a bit, `%B1` the
    /// second byte (Ed.3 Table 17).
    pub fn part(&self) -> Option<(PartSize, u64)> {
        let t = self.text.to_ascii_uppercase();
        let (size, n) = match t.strip_prefix('%') {
            None => (PartSize::Bit, t.as_str()),
            Some(r) => match r.as_bytes().first() {
                Some(b'X') => (PartSize::Bit, &r[1..]),
                Some(b'B') => (PartSize::Byte, &r[1..]),
                Some(b'W') => (PartSize::Word, &r[1..]),
                Some(b'D') => (PartSize::Dword, &r[1..]),
                Some(b'L') => (PartSize::Lword, &r[1..]),
                _ => (PartSize::Bit, r),
            },
        };
        Some((size, n.replace('_', "").parse().ok()?))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Literal {
    pub kind: LiteralKind,
    /// `INT` in `INT#5`, `Color` in `Color#Red` is a name, not a literal.
    pub type_prefix: Option<Ident>,
    /// `-` in `INT#-5`.
    pub negative: bool,
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant, reason = "a chart has tens of elements, not millions")]
pub enum SfcElement {
    Step(Step),
    Transition(Transition),
    Action(Action),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub initial: bool,
    pub name: Ident,
    pub actions: Vec<ActionAssociation>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionAssociation {
    pub action: Ident,
    pub qualifier: Option<Ident>,
    pub time: Option<Expr>,
    pub indicators: Vec<Ident>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub name: Option<Ident>,
    pub priority: Option<Expr>,
    pub from: Vec<Ident>,
    pub to: Vec<Ident>,
    pub condition: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub name: Ident,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Configuration {
    pub name: Ident,
    pub globals: Vec<VarBlock>,
    pub resources: Vec<Resource>,
    /// The entries of its `VAR_CONFIG` blocks.
    pub config_inits: Vec<ConfigInit>,
    /// The entries of its `VAR_ACCESS` blocks.
    pub accesses: Vec<AccessDecl>,
    pub span: Span,
}

/// `ALPHA : STATION_2.P1.y1 : BYTE READ_ONLY;` in `VAR_ACCESS`: a name
/// under which a communication service reaches a variable (Ed.3 6.8.1;
/// IEC 61131-5), in a CONFIGURATION or, for its own variables, in a
/// PROGRAM (Table 47, 9).
#[derive(Clone, Debug, PartialEq)]
pub struct AccessDecl {
    pub name: Ident,
    /// `R.` before a directly represented variable, `R.%IX1.0`.
    pub resource: Option<Ident>,
    /// The path: names, `.` members, `[i]` elements, a part of a bit
    /// string, or a direct address.
    pub path: Expr,
    pub ty: TypeSpec,
    /// READ_WRITE; READ_ONLY when none is written.
    pub read_write: bool,
    pub span: Span,
}

/// `STATION_1.P1.FB1.C2 AT %QB25 : BYTE := 0;` in `VAR_CONFIG`: an
/// instance-specific location, initial value, or both (Ed.3 6.8.2).
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigInit {
    /// Resource (if the configuration names its resources), program
    /// instance, function block instances, variable, structure fields.
    pub path: Vec<Ident>,
    pub location: Option<DirectAddress>,
    pub ty: TypeSpec,
    pub init: Option<Init>,
    pub span: Span,
}

/// A RESOURCE, or the one implied when a CONFIGURATION lists its tasks
/// and programs directly (`name` and `on` are then `None`).
#[derive(Clone, Debug, PartialEq)]
pub struct Resource {
    pub name: Option<Ident>,
    pub on: Option<Ident>,
    pub globals: Vec<VarBlock>,
    pub tasks: Vec<Task>,
    pub programs: Vec<ProgramConfig>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub name: Ident,
    /// `INTERVAL := T#10ms, PRIORITY := 0`, `SINGLE := trigger`.
    pub params: Vec<Arg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProgramConfig {
    pub retain: Option<Retain>,
    pub name: Ident,
    pub task: Option<Ident>,
    /// `Plant.Main` is one name, as a type's is.
    pub program_type: Ident,
    pub args: Vec<Arg>,
    /// `fb1 WITH t2` among its elements (Ed.3 Annex A, FB_Task): a
    /// function block instance of the program given a task of its own.
    pub fb_tasks: Vec<(Ident, Ident)>,
    pub span: Span,
}

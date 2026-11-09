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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PouKind {
    Program,
    Function,
    FunctionBlock,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pou {
    pub kind: PouKind,
    pub name: Ident,
    /// A FUNCTION's result type, if it has one.
    pub return_type: Option<TypeSpec>,
    pub var_blocks: Vec<VarBlock>,
    pub body: Vec<Stmt>,
    /// SFC: steps, transitions and actions (Ed.3 6.7).
    pub sfc: Vec<SfcElement>,
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

    /// `I`, `Q` or `M`.
    pub fn area(&self) -> Option<char> {
        self.text.chars().nth(1).map(|c| c.to_ascii_uppercase())
    }

    /// The size prefix (Ed.3 Table 16): `X`, `B`, `W`, `D` or `L`, `X`
    /// when there is none; none for a `*` address.
    pub fn size(&self) -> Option<char> {
        match self.text.chars().nth(2)?.to_ascii_uppercase() {
            c @ ('X' | 'B' | 'W' | 'D' | 'L') => Some(c),
            c if c.is_ascii_digit() => Some('X'),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeSpec {
    /// `INT`, `TON`, `MyStruct`.
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
    /// `a.3`, partial access of a bit (Ed.3 Table 17).
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
    pub program_type: Ident,
    pub args: Vec<Arg>,
    pub span: Span,
}

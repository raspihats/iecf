//! iecf's checker: names and types across a project's files.
//!
//! The rules are Ed.3's where Ed.4 is not yet bought (PROPOSAL.md §3a):
//! implicit conversions by Figure 11, names by 6.1.2 and 6.1.3, calls by
//! 6.6.1.4, globals by 6.5.2.2.

mod check;
mod codesys_oo;
pub mod datetime;
pub mod gvls;
pub mod namespaces;
mod properties;
pub mod stdlib;
pub mod strings;
mod struct_extends;
#[cfg(test)]
mod tests;
pub mod types;
mod var_stat;

use iecf_diagnostics::{Diagnostic, Span};
use iecf_syntax::ast::{Expr, SourceUnit};
use std::collections::{HashMap, HashSet};
use types::Ty;

/// An implementer extension (Ed.3 clause 5 c): something the standard
/// prohibits or does not specify, which iecf accepts only when asked.
/// Each is off by default and described in `docs/extensions.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Extension {
    /// A POU uses a global variable without declaring it VAR_EXTERNAL.
    ImplicitExternals,
    /// VAR_CONFIG at the top of a file, each PROGRAM its own instance.
    VarConfigList,
    /// TIME and LTIME converted to and from integers, bit strings and
    /// reals, counting milliseconds (nanoseconds for LTIME).
    TimeNumberConversions,
    /// BYTE, WORD, DWORD and LWORD to BOOL: TRUE for any value but 0.
    BitStringToBool,
    /// CODESYS's REFERENCE TO, set with REF= and tested with __ISVALIDREF.
    ReferenceTo,
    /// `_.` names the global namespace, as Siemens and PLCnext write it.
    GlobalNamespace,
    /// TO_BIG_ENDIAN and its kin reverse LTIME and the dates and times
    /// of day too, which Ed.3 says are not converted.
    EndianDates,
    /// `STRING(20)` for `STRING[20]`, as CODESYS and TwinCAT write it.
    StringParentheses,
    /// A function block or class instance in VAR_TEMP, or in a
    /// FUNCTION's or METHOD's VAR, set up at each call.
    TempInstances,
    /// CODESYS's VAR_IN_OUT CONSTANT: by reference, not written, given
    /// any value of its type and a STRING of any length.
    InOutConstant,
    /// A variable's initial value any expression: the variables declared
    /// before it, and calls of functions, worked out as it is set up.
    InitialExpressions,
    /// CODESYS's enumerations: a value named `COLOR.yellow`, and the base
    /// type written after the values, `(a, b) DWORD`.
    CodesysEnums,
    /// A structure's `END_STRUCT` with no `;` after it, before END_TYPE
    /// or the next type.
    StructSemicolon,
    /// A real literal with an exponent and no point, `1E37`.
    RealWithoutPoint,
    /// An array's initial values without brackets, `:= 1, 2, 3(0);`.
    BareArrayInit,
    /// CODESYS's VAR_INPUT CONSTANT, read as a plain VAR_INPUT.
    InputConstant,
    /// STEP, ON, R_EDGE and F_EDGE as names, where no keyword fits.
    KeywordNames,
    /// `TIME()`: the running task's clock as a TIME, CODESYS's and
    /// TwinCAT's time since start.
    TimeFunction,
    /// Arithmetic on BYTE, WORD, DWORD and LWORD, each counted as the
    /// unsigned integer of its width; a bit string where a number is
    /// wanted, an array index, and converted to and from a real.
    BitStringArithmetic,
    /// SHL, SHR, ROL and ROR of an integer, SHR keeping a signed one's
    /// sign; a shift's count N a bit string too.
    IntegerShifts,
    /// A bit, byte, word or double word of an integer, `i.3`.
    IntegerBitAccess,
    /// EXPT of an integer base, which widens to a real.
    IntegerExpt,
    /// An integer or bit string where another integer or bit string is
    /// wanted, converted as its `*_TO_*` would, with a warning.
    ImplicitNarrowing,
    /// DATE, TOD, DT and the long forms to and from integers, bit strings
    /// and reals, as the counts iecf holds them in (CODESYS's units).
    DateNumberConversions,
    /// A POU's variable named as a type, a POU, a standard function
    /// block or a standard function: in the POU the name is the variable.
    Shadowing,
    /// DWORD_TO_REAL, REAL_TO_DWORD, LWORD_TO_LREAL and LREAL_TO_LWORD by
    /// value, as CODESYS converts them, where Ed.3 moves the bits.
    CodesysRealConversions,
    /// CODESYS's POINTER TO, ADR and SIZEOF: checked pointers in bytes.
    Pointers,
    /// DATE + TIME and DATE - TIME, the TIME whole days.
    DateTimeArithmetic,
    /// A call's arguments in order first, then by name.
    MixedArguments,
    /// `a := b := 0;`, assigned right to left.
    ChainedAssignment,
    /// `JMP name;` and `name:` in ST.
    JumpLabels,
    /// No `;` after END_IF, END_CASE, END_FOR, END_WHILE, END_REPEAT.
    OptionalSemicolons,
    /// THIS^ and SUPER^, overriding without OVERRIDE, methods PUBLIC by default.
    CodesysOo,
    /// A function block's internal variable read, not written, from outside.
    InternalReads,
    /// CODESYS's PROPERTY with GET and SET.
    Properties,
    /// A method reaches its block's VAR_IN_OUT.
    MethodInOuts,
    /// CODESYS's FB_init: fb : T(args).
    FbInit,
    /// A structure extending another.
    StructExtends,
    /// CODESYS's VAR_STAT: static variables, one for the POU.
    VarStat,
    /// CODESYS's AND_THEN and OR_ELSE.
    ShortCircuit,
    /// CODESYS's TYPE INTERNAL and TYPE PUBLIC.
    TypeAccess,
}

impl Extension {
    pub const ALL: &'static [Extension] = &[
        Extension::ImplicitExternals,
        Extension::VarConfigList,
        Extension::TimeNumberConversions,
        Extension::BitStringToBool,
        Extension::ReferenceTo,
        Extension::GlobalNamespace,
        Extension::EndianDates,
        Extension::StringParentheses,
        Extension::TempInstances,
        Extension::InOutConstant,
        Extension::InitialExpressions,
        Extension::CodesysEnums,
        Extension::StructSemicolon,
        Extension::RealWithoutPoint,
        Extension::BareArrayInit,
        Extension::InputConstant,
        Extension::KeywordNames,
        Extension::TimeFunction,
        Extension::BitStringArithmetic,
        Extension::IntegerShifts,
        Extension::IntegerBitAccess,
        Extension::IntegerExpt,
        Extension::ImplicitNarrowing,
        Extension::DateNumberConversions,
        Extension::Shadowing,
        Extension::CodesysRealConversions,
        Extension::Pointers,
        Extension::DateTimeArithmetic,
        Extension::MixedArguments,
        Extension::ChainedAssignment,
        Extension::JumpLabels,
        Extension::OptionalSemicolons,
        Extension::CodesysOo,
        Extension::InternalReads,
        Extension::Properties,
        Extension::MethodInOuts,
        Extension::FbInit,
        Extension::StructExtends,
        Extension::VarStat,
        Extension::ShortCircuit,
        Extension::TypeAccess,
    ];

    /// As the command line and `docs/extensions.md` spell it.
    pub fn name(self) -> &'static str {
        match self {
            Extension::ImplicitExternals => "implicit-externals",
            Extension::VarConfigList => "var-config-list",
            Extension::TimeNumberConversions => "time-number-conversions",
            Extension::BitStringToBool => "bit-string-to-bool",
            Extension::ReferenceTo => "reference-to",
            Extension::GlobalNamespace => "global-namespace",
            Extension::EndianDates => "endian-dates",
            Extension::StringParentheses => "string-parentheses",
            Extension::TempInstances => "temp-instances",
            Extension::InOutConstant => "in-out-constant",
            Extension::InitialExpressions => "initial-expressions",
            Extension::CodesysEnums => "codesys-enums",
            Extension::StructSemicolon => "struct-semicolon",
            Extension::RealWithoutPoint => "real-without-point",
            Extension::BareArrayInit => "bare-array-init",
            Extension::InputConstant => "input-constant",
            Extension::KeywordNames => "keyword-names",
            Extension::TimeFunction => "time-function",
            Extension::BitStringArithmetic => "bit-string-arithmetic",
            Extension::IntegerShifts => "integer-shifts",
            Extension::IntegerBitAccess => "integer-bit-access",
            Extension::IntegerExpt => "integer-expt",
            Extension::ImplicitNarrowing => "implicit-narrowing",
            Extension::DateNumberConversions => "date-number-conversions",
            Extension::Shadowing => "shadowing",
            Extension::CodesysRealConversions => "codesys-real-conversions",
            Extension::Pointers => "pointers",
            Extension::DateTimeArithmetic => "date-time-arithmetic",
            Extension::MixedArguments => "mixed-arguments",
            Extension::ChainedAssignment => "chained-assignment",
            Extension::JumpLabels => "jump-labels",
            Extension::OptionalSemicolons => "optional-semicolons",
            Extension::CodesysOo => "codesys-oo",
            Extension::InternalReads => "internal-reads",
            Extension::Properties => "properties",
            Extension::MethodInOuts => "method-in-outs",
            Extension::FbInit => "fb-init",
            Extension::StructExtends => "struct-extends",
            Extension::VarStat => "var-stat",
            Extension::ShortCircuit => "short-circuit",
            Extension::TypeAccess => "type-access",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Extension::ImplicitExternals => "a POU uses a global variable without declaring it VAR_EXTERNAL",
            Extension::VarConfigList => "VAR_CONFIG outside a CONFIGURATION, paths from the PROGRAM's name",
            Extension::TimeNumberConversions => "TIME and LTIME to and from integers and reals, in ms and ns",
            Extension::BitStringToBool => "BYTE, WORD, DWORD and LWORD to BOOL: TRUE for any value but 0",
            Extension::ReferenceTo => "CODESYS's REFERENCE TO, bound with REF=, tested with __ISVALIDREF",
            Extension::GlobalNamespace => "`_.` names the global namespace, as Siemens and PLCnext write it",
            Extension::EndianDates => "TO_BIG_ENDIAN and its kin reverse LTIME and the dates too",
            Extension::StringParentheses => "STRING(20) for STRING[20], as CODESYS and TwinCAT write a length",
            Extension::TempInstances => "a function block instance in VAR_TEMP or in a FUNCTION, set up at each call",
            Extension::InOutConstant => "CODESYS's VAR_IN_OUT CONSTANT: given any value, a STRING of any length",
            Extension::InitialExpressions => {
                "a variable's initial value names variables and calls functions, as CODESYS's and TwinCAT's do"
            }
            Extension::CodesysEnums => {
                "CODESYS's enumerations: a value named COLOR.yellow, the base type after the values"
            }
            Extension::StructSemicolon => "END_STRUCT with no `;` after it, as CODESYS and TwinCAT write a structure",
            Extension::RealWithoutPoint => "a real literal with an exponent and no point, 1E37",
            Extension::BareArrayInit => "an array's initial values without brackets, CODESYS 2.3's `:= 1, 2, 3`",
            Extension::InputConstant => "CODESYS's VAR_INPUT CONSTANT, a plain VAR_INPUT outside its CFC editor",
            Extension::KeywordNames => "STEP, ON, R_EDGE and F_EDGE as names, which CODESYS does not reserve",
            Extension::TimeFunction => {
                "TIME() gives the clock in milliseconds, CODESYS's and TwinCAT's time since start"
            }
            Extension::BitStringArithmetic => {
                "arithmetic on BYTE, WORD, DWORD and LWORD, each counted as the unsigned integer of its width"
            }
            Extension::IntegerShifts => "SHL, SHR, ROL and ROR of an integer, SHR keeping its sign; a bit string as N",
            Extension::IntegerBitAccess => "a bit, byte or word of an integer, i.3, of its two's complement",
            Extension::IntegerExpt => "EXPT of an integer base, widened to REAL or LREAL",
            Extension::ImplicitNarrowing => {
                "an integer or bit string where another is wanted, converted as its `*_TO_*` would, with a warning"
            }
            Extension::DateNumberConversions => {
                "DATE, TOD, DT to and from numbers: seconds since 1970, milliseconds since midnight"
            }
            Extension::Shadowing => "a POU's variable named as a type, a POU or a standard block or function",
            Extension::CodesysRealConversions => "DWORD_TO_REAL and its kin by value, as CODESYS converts them",
            Extension::Pointers => "CODESYS's POINTER TO, ADR and SIZEOF, each pointer checked where it is used",
            Extension::DateTimeArithmetic => "DATE + TIME and DATE - TIME, the TIME a whole number of days",
            Extension::MixedArguments => "a call's arguments in order first, then by name, as newer CODESYS takes them",
            Extension::ChainedAssignment => {
                "a := b := 0, assigned right to left, as CODESYS, TwinCAT and Siemens chain them"
            }
            Extension::JumpLabels => {
                "JMP and labels in ST, as CODESYS and TwinCAT write them, never into a loop or branch"
            }
            Extension::OptionalSemicolons => {
                "no ';' after END_IF, END_CASE and the other ends, as CODESYS and TwinCAT take it"
            }
            Extension::CodesysOo => {
                "CODESYS's object orientation: THIS^ and SUPER^, overriding without OVERRIDE, methods PUBLIC unless written"
            }
            Extension::InternalReads => {
                "a block's internal variable read from outside, as CODESYS and TwinCAT allow, warned"
            }
            Extension::Properties => "CODESYS's PROPERTY, a GET and a SET called where it is read and assigned",
            Extension::MethodInOuts => {
                "a method reaches its block's VAR_IN_OUT, as CODESYS lets it with a warning, warned"
            }
            Extension::FbInit => {
                "CODESYS's FB_init, given arguments where an instance is declared, called as it is set up"
            }
            Extension::StructExtends => {
                "TYPE X EXTENDS Base : STRUCT, the base's fields first, as CODESYS extends a structure"
            }
            Extension::VarStat => {
                "CODESYS's VAR_STAT, static variables, one of each for the POU however many instances it has"
            }
            Extension::ShortCircuit => {
                "AND_THEN and OR_ELSE, the right operand evaluated only when the left does not decide"
            }
            Extension::TypeAccess => {
                "TYPE INTERNAL and TYPE PUBLIC, an INTERNAL type seen only in its namespace, as CODESYS's in its library"
            }
        }
    }

    pub fn from_name(name: &str) -> Option<Extension> {
        Extension::ALL.iter().copied().find(|e| e.name().eq_ignore_ascii_case(name))
    }

    /// The extensions `name` switches on: an extension, or a pack of them
    /// (`codesys`).
    pub fn named(name: &str) -> Option<&'static [Extension]> {
        if let Some(e) = Extension::ALL.iter().find(|e| e.name().eq_ignore_ascii_case(name)) {
            return Some(std::slice::from_ref(e));
        }
        Pack::from_name(name).map(Pack::members)
    }
}

/// Several extensions switched on by one name, as `--extension codesys`
/// (docs/extensions.md, "Packs"): what code written for one family of
/// tools uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pack {
    /// Every extension that is CODESYS's and TwinCAT's.
    Codesys,
}

impl Pack {
    pub const ALL: &'static [Pack] = &[Pack::Codesys];

    pub fn name(self) -> &'static str {
        match self {
            Pack::Codesys => "codesys",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Pack::Codesys => "what code written for CODESYS and TwinCAT uses",
        }
    }

    pub fn members(self) -> &'static [Extension] {
        match self {
            Pack::Codesys => &[
                Extension::ImplicitExternals,
                Extension::VarConfigList,
                Extension::TimeNumberConversions,
                Extension::BitStringToBool,
                Extension::ReferenceTo,
                Extension::StringParentheses,
                Extension::TempInstances,
                Extension::InOutConstant,
                Extension::InitialExpressions,
                Extension::CodesysEnums,
                Extension::StructSemicolon,
                Extension::RealWithoutPoint,
                Extension::BareArrayInit,
                Extension::InputConstant,
                Extension::KeywordNames,
                Extension::TimeFunction,
                Extension::BitStringArithmetic,
                Extension::IntegerShifts,
                Extension::IntegerBitAccess,
                Extension::IntegerExpt,
                Extension::ImplicitNarrowing,
                Extension::DateNumberConversions,
                Extension::Shadowing,
                Extension::CodesysRealConversions,
                Extension::Pointers,
                Extension::DateTimeArithmetic,
                Extension::MixedArguments,
                Extension::ChainedAssignment,
                Extension::JumpLabels,
                Extension::OptionalSemicolons,
                Extension::CodesysOo,
                Extension::InternalReads,
                Extension::Properties,
                Extension::MethodInOuts,
                Extension::FbInit,
                Extension::StructExtends,
                Extension::VarStat,
                Extension::ShortCircuit,
                Extension::TypeAccess,
            ],
        }
    }

    pub fn from_name(name: &str) -> Option<Pack> {
        Pack::ALL.iter().copied().find(|p| p.name().eq_ignore_ascii_case(name))
    }
}

/// How to check. The default is the standard as written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    extensions: Vec<Extension>,
    /// A debug build (docs/c-output.md §9): each statement calls the
    /// platform's hook, and the output lists the lines it can stop at.
    debug: bool,
    /// Libraries, as CODESYS and TwinCAT have them: the files whose name
    /// starts with `path` are in the namespace `name`, which every file
    /// uses unless `qualified` (docs/behaviour/namespaces.md).
    libraries: Vec<Library>,
    gvls: Vec<GvlFile>,
}

/// A library: its namespace's name, where its files are, and whether
/// its elements are named only with it (CODESYS's qualified-only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub path: String,
    pub qualified: bool,
}

/// A global variable list (`--gvl NAME=FILE`, gvls.rs): the VAR_GLOBAL
/// sections of the files at `path`, reached as `NAME.x`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GvlFile {
    pub name: String,
    pub path: String,
    pub qualified: bool,
}

impl Options {
    /// With each of `es`: what `Extension::named` gives.
    pub fn with_all(self, es: &[Extension]) -> Self {
        es.iter().fold(self, |o, e| o.with(*e))
    }

    pub fn with(mut self, e: Extension) -> Self {
        if !self.extensions.contains(&e) {
            self.extensions.push(e);
        }
        self
    }

    pub fn has(&self, e: Extension) -> bool {
        self.extensions.contains(&e)
    }

    /// A debug build: statement hooks and the line table.
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    pub fn debug(&self) -> bool {
        self.debug
    }

    /// With the library `name`: the files under `path`.
    pub fn with_library(mut self, name: &str, path: &str, qualified: bool) -> Self {
        self.libraries.push(Library { name: name.to_string(), path: path.to_string(), qualified });
        self
    }

    pub fn libraries(&self) -> &[Library] {
        &self.libraries
    }

    /// With the global variable list `name`, the VAR_GLOBAL sections of
    /// the file (or the files under) `path`; `qualified`: reached only as
    /// `name.x`.
    pub fn with_gvl(mut self, name: &str, path: &str, qualified: bool) -> Self {
        self.gvls.push(GvlFile { name: name.to_string(), path: path.to_string(), qualified });
        self
    }

    pub fn gvls(&self) -> &[GvlFile] {
        &self.gvls
    }
}

/// Check a project by the standard as written: every file's tree,
/// parsed. Names declared in one file are seen in the others.
pub fn check(units: &[&SourceUnit]) -> Vec<Diagnostic> {
    check_with(units, &Options::default())
}

/// Check a project with the extensions `options` switches on.
pub fn check_with(units: &[&SourceUnit], options: &Options) -> Vec<Diagnostic> {
    analyze(units, options).diagnostics
}

/// What the checker found and what it worked out: the diagnostics, and
/// the types the code generator needs.
pub struct Analysis {
    pub diagnostics: Vec<Diagnostic>,
    pub types: Types,
}

/// Check a project and keep what the check worked out.
pub fn analyze(units: &[&SourceUnit], options: &Options) -> Analysis {
    // codesys-oo: CODESYS's spellings rewritten as Ed.3's, on a copy
    // (analyze_project rewrites the build's own units in place).
    let oo = options.has(Extension::CodesysOo) && codesys_oo::needed(units);
    let se = options.has(Extension::StructExtends) && struct_extends::needed(units);
    let vs = options.has(Extension::VarStat) && var_stat::needed(units);
    if oo || se || vs {
        let mut owned: Vec<SourceUnit> = units.iter().map(|u| (*u).clone()).collect();
        let mut found = Vec::new();
        if se {
            struct_extends::rewrite(&mut owned);
        }
        if vs {
            found = var_stat::rewrite(&mut owned);
        }
        if oo {
            codesys_oo::rewrite(&mut owned);
        }
        let refs: Vec<&SourceUnit> = owned.iter().collect();
        let mut analysis = analyze_as_is(&refs, options);
        if !found.is_empty() {
            analysis.diagnostics.append(&mut found);
            analysis.diagnostics.sort_by_key(|d| (d.span.file, d.span.start, d.span.end));
        }
        return analysis;
    }
    analyze_as_is(units, options)
}

/// [`analyze`], the units as they are (rewritten already, or needing
/// nothing rewritten).
fn analyze_as_is(units: &[&SourceUnit], options: &Options) -> Analysis {
    let mut c = check::Checker::new(options.clone());
    c.collect(units);
    c.check_all(units);
    let (mut diagnostics, mut types) = c.finish();
    types.options = options.clone();
    diagnostics.sort_by_key(|d| (d.span.file, d.span.start, d.span.end));
    Analysis { diagnostics, types }
}

/// Check a project as the build does: its namespaces taken out first
/// (namespaces.rs), which rewrites `units`, then [`analyze`].
pub fn analyze_project(units: &mut [SourceUnit], options: &Options) -> Analysis {
    let mut diagnostics = namespaces::flatten(units, options);
    if options.has(Extension::StructExtends) {
        struct_extends::rewrite(units);
    }
    if options.has(Extension::VarStat) {
        diagnostics.append(&mut var_stat::rewrite(units));
    }
    if options.has(Extension::CodesysOo) {
        codesys_oo::rewrite(units);
    }
    let refs: Vec<&SourceUnit> = units.iter().collect();
    let mut analysis = analyze_as_is(&refs, options);
    // properties: each read and assignment the check took for a
    // property's, written as its getter's or setter's call, for the
    // generator.
    if options.has(Extension::Properties) {
        properties::rewrite(units, &analysis.types);
    }
    diagnostics.append(&mut analysis.diagnostics);
    diagnostics.sort_by_key(|d| (d.span.file, d.span.start, d.span.end));
    analysis.diagnostics = diagnostics;
    analysis
}

/// The types the check worked out, by the span of what they belong to.
/// An expression's type is its own, before its context converts it: an
/// untyped literal is [`Ty::IntLit`] or [`Ty::RealLit`].
#[derive(Debug, Default)]
pub struct Types {
    exprs: HashMap<Span, Ty>,
    decls: HashMap<Span, Ty>,
    std_calls: HashMap<Span, StdCall>,
    implicit_eno: std::collections::HashSet<String>,
    refs: HashMap<Span, RefUse>,
    /// `SIZEOF(T)` of a type's name (the pointers extension), by the
    /// argument's span: the type.
    pub(crate) sizeof_types: HashMap<Span, Ty>,
    /// Each JMP's label, by the JMP's span and the label's (jump-labels).
    pub(crate) jumps: HashMap<Span, Span>,
    /// A property read, by the expression's span: its getter called there
    /// (properties).
    pub(crate) prop_reads: HashSet<Span>,
    /// A property assigned, by the statement's span: its setter called.
    pub(crate) prop_writes: HashSet<Span>,
    /// The block's VAR_IN_OUTs each method uses, by the method's key
    /// (method-in-outs).
    pub(crate) method_inouts: HashMap<String, HashSet<String>>,
    /// `__ISVALIDREF(x)` of a block's VAR_IN_OUT in a method, by the
    /// span of `x`: the in-out's name (method-in-outs).
    pub(crate) inout_tests: HashMap<Span, String>,
    /// Each instance's FB_init call, by its declaration's span (fb-init):
    /// `inst.FB_init(bInitRetains := TRUE, bInCopyCode := FALSE, ...)`,
    /// as checked; the generator writes it with `bInitRetains` the cold
    /// start's.
    pub(crate) fb_inits: HashMap<Span, Vec<Expr>>,
    implicit_externals: HashMap<String, Vec<(String, bool)>>,
    /// Each method call, by the call's span: the method's key (`method_key`).
    method_calls: HashMap<Span, String>,
    /// Each `THIS.x` that is the instance's own `x`, by its span.
    this_members: std::collections::HashSet<Span>,
    /// Each CLASS's and FUNCTION_BLOCK's base type, by upper-cased key.
    pub(crate) bases: HashMap<String, String>,
    /// Each INTERFACE's bases (EXTENDS), and each CLASS's and
    /// FUNCTION_BLOCK's interfaces (IMPLEMENTS), by upper-cased key.
    pub(crate) itf_bases: HashMap<String, Vec<String>>,
    pub(crate) implements: HashMap<String, Vec<String>>,
    /// The value of each constant expression the standard requires one
    /// for (array bounds, a STRING's length, a subrange's limits), by
    /// its span: one that names a CONSTANT too.
    pub(crate) consts: HashMap<Span, i128>,
    /// Which globals each POU's VAR_EXTERNALs reach where it runs, by
    /// configuration and POU (a method's under its type), then name:
    /// those its code reaches itself, and those the instances it creates
    /// as it runs reach (Ed.3 6.5.2.2).
    pub(crate) bindings:
        HashMap<(String, String), std::collections::BTreeMap<String, std::collections::BTreeSet<Binding>>>,
    /// The VAR_ACCESS entries, checked (Ed.3 6.8.1).
    pub(crate) accesses: Vec<AccessPath>,
    /// The extensions the check ran with: some change what the
    /// generator writes (endian-dates).
    pub(crate) options: Options,
    /// Each initial value that may be worked out as its variable is set
    /// up rather than by the compiler, by its span: one of a variable,
    /// not a CONSTANT's, a type's or a FUNCTION's parameter's
    /// (initial-expressions).
    pub(crate) init_exprs: std::collections::HashSet<Span>,
}

/// A VAR_ACCESS entry: the access name, the variable's path as the
/// symbol table names it (`R.P.FB.X`, `G[2].F`, `%IX1.0`), its type and
/// direction. A CONFIGURATION's is whole; a PROGRAM's starts at the
/// program's variable, each instance's path to go before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPath {
    /// The configuration, upper-cased; none for a PROGRAM's.
    pub config: Option<String>,
    /// The PROGRAM, upper-cased; none for a CONFIGURATION's.
    pub program: Option<String>,
    pub name: String,
    pub path: String,
    pub ty: String,
    pub read_write: bool,
}

/// The global a VAR_EXTERNAL reaches in one place its POU runs (Ed.3
/// 6.5.2.2), by upper-cased names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Binding {
    /// A VAR_GLOBAL at the top of a file, a global variable list.
    Top(String),
    /// The configuration's own.
    Config(String),
    /// A resource's: the resource (empty when the configuration names
    /// none), and the global.
    Resource(String, String),
    /// A PROGRAM's own VAR_GLOBAL, which each instance of it has: the
    /// resource, the program instance, and the global.
    Program(String, String, String),
}

impl Types {
    /// Whether the check ran with the extension `e`.
    pub fn has(&self, e: Extension) -> bool {
        self.options.has(e)
    }

    /// Whether the initial value with this span is one the extension
    /// initial-expressions works out as its variable is set up, naming
    /// variables and calling functions, when it is on.
    pub fn init_expr(&self, span: Span) -> bool {
        self.init_exprs.contains(&span)
    }

    /// The VAR_ACCESS entries, in the order they are written.
    pub fn accesses(&self) -> &[AccessPath] {
        &self.accesses
    }

    /// The globals the VAR_EXTERNAL `name` of the POU `key` (a method's:
    /// its type's) reaches in the CONFIGURATION `config`, where it runs
    /// there: one, or more when its instances run in places that name
    /// different ones.
    pub fn bindings(&self, config: &str, key: &str, name: &str) -> Option<&std::collections::BTreeSet<Binding>> {
        self.bindings.get(&(config.to_ascii_uppercase(), key.to_ascii_uppercase()))?.get(&name.to_ascii_uppercase())
    }

    /// The names whose bindings are known for `key` in `config`.
    pub fn bound_names(&self, config: &str, key: &str) -> Vec<String> {
        self.bindings
            .get(&(config.to_ascii_uppercase(), key.to_ascii_uppercase()))
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// `e` without the name of its own resource: `R.g` as `g`, `R.g.x` as
/// `g.x` (Ed.3 6.8.2, Global_Var_Access), where `R` is `resource`, the
/// one the connection or TASK is written in, and `shadowed` does not
/// say a global variable is named so - then `R.x` is its field. The
/// name keeps the whole access's span. None when `e` is not such an
/// access.
pub fn own_resource_access(
    e: &iecf_syntax::ast::Expr,
    resource: Option<&str>,
    shadowed: &dyn Fn(&str) -> bool,
) -> Option<iecf_syntax::ast::Expr> {
    use iecf_syntax::ast::{Expr, ExprKind};
    let resource = resource?;
    let named = |x: &Expr| matches!(&x.kind, ExprKind::Name(n) if n.text.eq_ignore_ascii_case(resource) && !shadowed(&n.text.to_ascii_uppercase()));
    let ExprKind::Member(b, m) = &e.kind else { return None };
    if named(b) {
        return Some(Expr {
            kind: ExprKind::Name(iecf_syntax::ast::Ident { span: e.span, ..m.clone() }),
            span: e.span,
        });
    }
    if let ExprKind::Member(r, g) = &b.kind
        && named(r)
    {
        let inner = Expr { kind: ExprKind::Name(iecf_syntax::ast::Ident { span: b.span, ..g.clone() }), span: b.span };
        return Some(Expr { kind: ExprKind::Member(Box::new(inner), m.clone()), span: e.span });
    }
    None
}

/// The type a method belongs to, from its key (`method_key`); a POU's
/// own key otherwise. IEC names hold no `__`.
pub fn owner_of(key: &str) -> &str {
    key.split("__").next().unwrap_or(key)
}

/// A method's key, as a FUNCTION's is its name: its type's key and its
/// own upper-cased, joined by `__`, which no IEC name holds.
pub fn method_key(owner: &str, method: &str) -> String {
    format!("{}__{}", owner.to_ascii_uppercase(), method.to_ascii_uppercase())
}

/// What the generator does with an expression where a REFERENCE TO (the
/// reference-to extension) is used: the checker decides it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefUse {
    /// A REFERENCE TO standing for what it references: as `r^`.
    Deref,
    /// A variable a REFERENCE TO is bound to, by REF= or a call: as `REF(x)`.
    Bind,
    /// `r REF= 0`: unbound, as `NULL`.
    Unbind,
}

impl Types {
    /// Whether the `THIS.x` with this span is the instance's own `x`,
    /// generated as `x` is.
    pub fn this_member(&self, span: Span) -> bool {
        self.this_members.contains(&span)
    }

    /// Whether anything is generated differently from how it is written:
    /// a REFERENCE TO used, or a `THIS.x` (codegen's reference.rs).
    pub fn rewrites(&self) -> bool {
        !self.refs.is_empty() || !self.this_members.is_empty()
    }

    /// The base type `key` EXTENDS, upper-cased.
    pub fn base(&self, key: &str) -> Option<&str> {
        self.bases.get(key).map(String::as_str)
    }

    /// `key`'s bases, the nearest first.
    pub fn ancestors(&self, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut at = key;
        while let Some(b) = self.bases.get(at) {
            out.push(b.clone());
            at = b;
        }
        out
    }

    /// Whether `derived` is `base` or derives from it.
    pub fn derives(&self, derived: &str, base: &str) -> bool {
        derived == base || self.ancestors(derived).iter().any(|a| a == base)
    }

    /// The types that extend `key`, directly or not, in key order.
    pub fn descendants(&self, key: &str) -> Vec<String> {
        let mut out: Vec<String> =
            self.bases.keys().filter(|k| k.as_str() != key && self.derives(k, key)).cloned().collect();
        out.sort();
        out
    }

    /// The interface `key` and every one it extends, directly or not:
    /// `key` first, each once.
    pub fn itf_line(&self, key: &str) -> Vec<String> {
        let mut out = vec![key.to_string()];
        let mut i = 0;
        while i < out.len() {
            for b in self.itf_bases.get(&out[i]).into_iter().flatten() {
                if !out.contains(b) {
                    out.push(b.clone());
                }
            }
            i += 1;
        }
        out
    }

    /// The interfaces the CLASS or FUNCTION_BLOCK `key` implements: its
    /// own, its bases', and those they extend, in key order.
    pub fn implemented(&self, key: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for t in std::iter::once(key.to_string()).chain(self.ancestors(key)) {
            for i in self.implements.get(&t).into_iter().flatten() {
                for j in self.itf_line(i) {
                    if !out.contains(&j) {
                        out.push(j);
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// The method the call with this span calls, by its key.
    pub fn method_call(&self, span: Span) -> Option<&str> {
        self.method_calls.get(&span).map(String::as_str)
    }

    /// The type of the expression with this span.
    pub fn expr(&self, span: Span) -> Option<&Ty> {
        self.exprs.get(&span)
    }

    /// The type of the variable declaration with this span (`VarDecl::span`),
    /// or of a FUNCTION's result (its `TypeSpec::span`).
    pub fn decl(&self, span: Span) -> Option<&Ty> {
        self.decls.get(&span)
    }

    /// Whether the FUNCTION or FUNCTION_BLOCK `pou` (upper-cased) uses
    /// ENO without declaring it (Ed.3 6.6.1.5).
    pub fn implicit_eno(&self, pou: &str) -> bool {
        self.implicit_eno.contains(pou)
    }

    /// The globals the POU `pou` (upper-cased) uses without declaring
    /// them (the implicit-externals extension), upper-cased, each with
    /// whether it is CONSTANT: as if it declared them VAR_EXTERNAL.
    pub fn implicit_externals(&self, pou: &str) -> &[(String, bool)] {
        self.implicit_externals.get(pou).map_or(&[], Vec::as_slice)
    }

    /// What a REFERENCE TO's use at this span is (reference-to extension).
    pub fn ref_use(&self, span: Span) -> Option<RefUse> {
        self.refs.get(&span).copied()
    }

    /// Whether the project uses a REFERENCE TO anywhere.
    pub fn has_ref_uses(&self) -> bool {
        !self.refs.is_empty()
    }

    /// The call of a standard function with this span, as bound.
    /// The value of the constant expression at `span`, where the
    /// standard requires one (`consts`).
    pub fn const_value(&self, span: Span) -> Option<i128> {
        self.consts.get(&span).copied()
    }

    pub fn std_call(&self, span: Span) -> Option<&StdCall> {
        self.std_calls.get(&span)
    }

    /// The FB_init calls of the instance declared at `decl`, its base's
    /// first (fb-init).
    pub fn fb_init(&self, decl: Span) -> Option<&Vec<Expr>> {
        self.fb_inits.get(&decl)
    }

    /// The VAR_IN_OUTs of its block the method `key` uses (method-in-outs).
    pub fn method_inouts(&self, key: &str) -> Option<&HashSet<String>> {
        self.method_inouts.get(key)
    }

    /// The block's VAR_IN_OUT that `__ISVALIDREF(x)` tests in a method,
    /// by the span of `x` (method-in-outs).
    pub fn inout_test(&self, x: Span) -> Option<&String> {
        self.inout_tests.get(&x)
    }

    /// The label a `JMP` goes to, by the JMP statement's span.
    pub fn jump_target(&self, span: Span) -> Option<Span> {
        self.jumps.get(&span).copied()
    }

    /// Whether some JMP goes to the label declared at `span`.
    pub fn jumped_to(&self, label: Span) -> bool {
        self.jumps.values().any(|&s| s == label)
    }

    /// The type `SIZEOF(T)` names, by the argument's span (pointers).
    pub fn sizeof_type(&self, span: Span) -> Option<&Ty> {
        self.sizeof_types.get(&span)
    }
}

/// A call of a standard function, its arguments bound to its inputs and
/// outputs (Ed.3 6.6.1.4) and its overloading resolved.
#[derive(Clone, Debug)]
pub struct StdCall {
    /// The function, upper-cased, a typed form's types left off (6.6.2.4):
    /// `ADD` for `ADD_INT`, `MUX` for `MUX_UINT_REAL`; `MAX`,
    /// `INT_TO_DINT`, `ADD_TOD_TIME` as called.
    pub name: String,
    /// As called, upper-cased: `ADD_INT`.
    pub called: String,
    /// Each input in order (`IN1`, `IN2`, ... or `IN`, `N`), and the span
    /// of the argument given to it.
    pub inputs: Vec<(String, Option<Span>)>,
    pub outputs: Vec<(String, Option<Span>)>,
    /// The type the common inputs share, or the typed form's type.
    pub common: Ty,
    /// MUX's K, when the typed form names it.
    pub typed_k: Option<Ty>,
    /// The default values of the inputs a call by name leaves out (Ed.3
    /// 6.6.1.4.2): literals of the input's type, one per type, which
    /// `inputs` names by their spans (empty spans inside the function's
    /// name, where no argument is).
    pub defaults: Vec<iecf_syntax::ast::Expr>,
}

impl StdCall {
    /// The value an input at `span` (`inputs`) has in this call: its
    /// argument, or the default value of one left out.
    pub fn input<'a>(
        &'a self,
        args: &'a [iecf_syntax::ast::Arg],
        span: Option<Span>,
    ) -> Option<&'a iecf_syntax::ast::Expr> {
        use iecf_syntax::ast::Arg;
        let span = span?;
        args.iter()
            .find_map(|a| match a {
                Arg::Positional(e) | Arg::Input(_, e) if e.span == span => Some(e),
                _ => None,
            })
            .or_else(|| self.defaults.iter().find(|d| d.span == span))
    }
}

/// The name of an enumeration declared in place, `x : (a, b, c)`: its
/// values' names. One with named values (Ed.3 6.4.4.3: a base type, or a
/// number given) is its declaration's own, marked by where it is. The
/// checker's types and the generator both know it by this.
pub fn inplace_enum_name(
    base: Option<&iecf_syntax::ast::Ident>,
    values: &[iecf_syntax::ast::EnumValue],
    span: iecf_diagnostics::Span,
) -> String {
    let names = values.iter().map(|v| v.name.text.as_str()).collect::<Vec<_>>().join(", ");
    if base.is_none() && values.iter().all(|v| v.value.is_none()) {
        format!("({names})")
    } else {
        format!("({names})#{}", span.start)
    }
}

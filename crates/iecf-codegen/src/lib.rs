//! iecf's C: a checked project in, the files of docs/c-output.md out.
//!
//! Built in slices (PROPOSAL.md §5 step 2), each turning on harness cases.
//! What a slice does not cover yet is E0009 at the construct, never C
//! that means something else. Covered: one CONFIGURATION, its tasks and
//! PROGRAM instances; variables of the integer, bit-string, BOOL, real,
//! time and date types, located or not (each its own width from its
//! address; arrays, structures and STRINGs there packed, names.rs `Lv`),
//! references to any of them and to function blocks (refs.rs), and
//! instances of the standard
//! function blocks; every ST statement; the integer operators,
//! conversions, shifts, ABS, MIN, MAX and LIMIT; + and - on durations;
//! the user's FUNCTIONs with inputs and locals, and FUNCTION_BLOCKs with
//! inputs, outputs, in-outs, locals and instances of other blocks.
//!
//! No I/O, like the rest of the core: it builds for wasm32.

mod consts;
mod dates;
mod derived;
mod enable;
mod expr;
mod flex;
mod inst;
mod itf;
mod names;
mod oo;
mod params;
mod reference;
mod refs;
mod sfc;
mod stmt;
mod strings;
mod symbols;

use iecf_diagnostics::{Code, Diagnostic, SourceMap, Span};
use iecf_sema::Types;
use iecf_sema::stdlib::Dir;
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;
use names::{Lv, Place, Scope, StdFb};
use std::collections::HashMap;
use std::fmt::Write as _;

/// The runtime header, the same for every project (docs/c-output.md §2).
pub const IECF_H: &str = include_str!("../../../runtime/iecf.h");

/// C for a checked project: `(file name, text)` for `<name>.h`, `<name>.c`
/// and `iecf.h`; or why not, as E0009 diagnostics.
pub fn generate(
    units: &[&SourceUnit],
    types: &Types,
    map: &SourceMap,
) -> Result<Vec<(String, String)>, Vec<Diagnostic>> {
    let rewritten = reference::rewrite(units, types);
    let own: Vec<&SourceUnit>;
    let units = match &rewritten {
        Some(r) => {
            own = r.iter().collect();
            &own[..]
        }
        None => units,
    };
    let mut g = Gen::new(units, types, map);
    let out = g.project();
    if g.diags.is_empty() { out.ok_or_else(Vec::new) } else { Err(g.diags) }
}

/// Raised when a construct cannot be generated; the diagnostic is
/// already recorded.
#[derive(Debug)]
pub(crate) struct Skip;
pub(crate) type R<T> = Result<T, Skip>;

/// What a declaration declares, as far as the generator goes.
#[derive(Clone, Debug)]
pub(crate) enum Decl {
    Elem(Elem),
    /// A derived type (derived.rs).
    Data(derived::DTy),
    Fb(StdFb),
    /// An instance of the user's FUNCTION_BLOCK, by its upper-cased name.
    Block(String),
    /// A variable of an INTERFACE, by its upper-cased name (itf.rs).
    Itf(String),
}

/// A user's FUNCTION_BLOCK as its callers see it: `B_<NAME>_t`, its
/// inputs, outputs and in-outs (fields `V_<NAME>`, an in-out a pointer).
#[derive(Clone, Debug)]
pub(crate) struct BlockSig {
    pub(crate) params: Vec<(String, Elem, Dir)>,
    /// Its inputs and outputs of an array, structure or STRING type,
    /// copied in and out by a call (stmt.rs).
    pub(crate) data: Vec<(String, derived::DTy, Dir)>,
    /// Its parameters of a subrange type: their bounds.
    pub(crate) ranges: HashMap<String, (i128, i128)>,
    /// Its in-outs of a block type: name and type (a `B_<TYPE>_t *`).
    pub(crate) blocks: Vec<(String, String)>,
    /// The C fields of what it inherits, `base.V_X` (Ed.3 6.6.5.5.2):
    /// any other parameter is `V_<NAME>`.
    pub(crate) paths: HashMap<String, String>,
    /// Its inputs of an INTERFACE: name and interface (an `iecf_itf_t`).
    pub(crate) itfs: Vec<(String, String)>,
    /// Its in-outs of an array, structure or STRING type: a pointer to
    /// the caller's variable, set by each call; an `ARRAY[*]` one's to
    /// an `iecf_flex_t` (flex.rs).
    pub(crate) in_data: Vec<(String, derived::DTy)>,
}

impl BlockSig {
    /// The C field, in an instance, of its parameter `name`.
    pub(crate) fn field(&self, name: &str) -> String {
        self.paths.get(name).cloned().unwrap_or_else(|| format!("V_{name}"))
    }
}

/// A user's FUNCTION as its callers see it: `F_<NAME>(ctx, inputs...)`.
#[derive(Clone, Debug)]
pub(crate) struct FnSig {
    /// Its C name: `F_<NAME>`, or `M_<TYPE>__<METHOD>` for a method.
    pub(crate) c_name: String,
    /// A method's type: its C function takes `B_<TYPE>_t *self` after
    /// ctx, the instance it is called on.
    pub(crate) this: Option<String>,
    /// Its result's type; `None` for a FUNCTION without one (Ed.3
    /// 6.6.2.1), `void` in C, called only as a statement.
    pub(crate) ret: Option<Elem>,
    /// Each input and in-out, in their declared order, as a call gives
    /// them in order (Ed.3 6.6.1.4): name, type, and an input's initial
    /// value as C, for a call that does not give it; `None` for an
    /// in-out, a pointer to the variable the call passes, which every
    /// call must (E0016).
    pub(crate) inputs: Vec<(String, Elem, Option<String>)>,
    /// Its VAR_EXTERNALs: the globals they name.
    pub(crate) externals: Vec<String>,
    /// Each output: name, type, initial value as C. Passed as a pointer
    /// after the inputs, NULL when the call takes no output there; set
    /// from the output's local as the function returns, not when it
    /// faults.
    pub(crate) outputs: Vec<(String, Elem, String)>,
    /// Its body uses ENO without declaring it (Ed.3 6.6.1.5): an
    /// `IEC_BOOL *eno` after ctx, set to 1 as it starts.
    pub(crate) eno: bool,
    /// Its in-outs of a block type, among `inputs`: name and type, a
    /// `B_<TYPE>_t *`.
    pub(crate) in_blocks: HashMap<String, String>,
    /// Its inputs of an INTERFACE, among `inputs`: name and interface, an
    /// `iecf_itf_t`.
    pub(crate) in_itfs: HashMap<String, String>,
    /// Its inputs, outputs and in-outs of a STRING, array or structure
    /// type, among `inputs` and `outputs`, each with its initial value
    /// (params.rs): an input a `const void *P_<NAME>` copied in, NULL when
    /// not given; an in-out a `void *V_<NAME>`; an output a `void
    /// *O_<NAME>` copied to as it returns.
    pub(crate) data: HashMap<String, derived::DTy>,
    /// Its result of such a type: written into the caller's `void *R_`,
    /// a STRING's returned too, so a call is a string expression.
    pub(crate) ret_data: Option<derived::DTy>,
    /// What an enumerated result starts each call as: its type's initial
    /// value, the first of its values unless the type gives one.
    pub(crate) ret_init: Option<String>,
    /// Its inputs, outputs and in-outs of a subrange type, by
    /// upper-cased name: their bounds. A value given to such an input
    /// is clamped and flagged as the function starts, as any store.
    pub(crate) ranges: HashMap<String, (i128, i128)>,
    /// Its result's bounds, of a subrange type.
    pub(crate) ret_range: Option<(i128, i128)>,
}

impl FnSig {
    /// Its C function's result type.
    pub(crate) fn c_ret(&self) -> String {
        match (&self.ret, &self.ret_data) {
            (Some(e), _) => c_type(*e),
            (None, Some(t)) => match t.kind {
                derived::Kind::Str(_, w) => format!("const {} *", crate::strings::unit(w)),
                _ => "void".to_string(),
            },
            (None, None) => "void".to_string(),
        }
    }

    /// Whether its C function returns a value.
    pub(crate) fn gives(&self) -> bool {
        self.ret.is_some() || self.ret_data.as_ref().is_some_and(|t| matches!(t.kind, derived::Kind::Str(..)))
    }
}

pub(crate) struct Gen<'a> {
    units: &'a [&'a SourceUnit],
    pub(crate) types: &'a Types,
    map: &'a SourceMap,
    pub(crate) diags: Vec<Diagnostic>,
    pub(crate) pous: HashMap<String, &'a Pou>,
    /// The project's prefix: the CONFIGURATION's name in lower case.
    pub(crate) prefix: String,
    /// `<p>.symbols.json`: the symbol table as JSON (symbols.rs).
    pub(crate) symbols_json: String,
    /// POU names, in the order of the table `<prefix>_pou_names`.
    pou_names: Vec<String>,
    /// The highest byte addressed plus one, per area I, Q, M.
    pub(crate) extent: [u32; 3],
    /// Set when an expression calls something that can fault.
    pub(crate) may_fault: bool,
    /// Set when it can flag an overflow, a conversion or an infinite or
    /// NaN real (docs/c-output.md §5).
    pub(crate) may_flag: bool,
    /// Whether the expression being written may convert a real to an
    /// integer: only where a statement can be left undone (stmt.rs).
    pub(crate) keep_ok: bool,
    /// The `int` its conversions set when the real does not fit.
    pub(crate) keep: Option<String>,
    /// For unique C temporaries.
    pub(crate) temp: u32,
    /// The user's FUNCTIONs called so far, and those still to write.
    pub(crate) functions: HashMap<String, FnSig>,
    pub(crate) fn_queue: Vec<String>,
    pub(crate) fn_queued: std::collections::HashSet<String>,
    /// The user's FUNCTION_BLOCKs written so far, in the order their
    /// types must appear (a block after those it contains).
    pub(crate) blocks: HashMap<String, BlockSig>,
    pub(crate) block_busy: Vec<String>,
    pub(crate) block_types: String,
    /// `typedef struct B_<NAME>_s B_<NAME>_t;` of every block, first.
    pub(crate) block_fwd: String,
    block_code: String,
    /// The `keep` an instance's `_init` is given where it is an element
    /// or a field (inst.rs); "0" where it is set at a cold start only.
    pub(crate) init_keep: String,
    /// The condition the other parts of such an array or structure are
    /// set under: a RETAIN one's, at a cold start only.
    pub(crate) init_guard: Option<String>,
    /// The configuration's globals, for VAR_EXTERNAL in any POU.
    globals: HashMap<String, Place>,
    /// The project's TYPE declarations, by upper-cased name.
    pub(crate) type_decls: HashMap<String, &'a TypeDecl>,
    /// The enumerations and structures used, worked out once (derived.rs).
    pub(crate) enums: HashMap<String, derived::EnumInfo>,
    pub(crate) structs: HashMap<String, derived::Fields>,
    pub(crate) struct_busy: Vec<String>,
    /// The structures' C types, each after those it contains.
    pub(crate) data_types: String,
    /// For the symbol table: each PROGRAM's and FUNCTION_BLOCK's
    /// variables, and the globals by their paths (symbols.rs).
    pub(crate) pou_members: HashMap<String, Vec<symbols::Member>>,
    pub(crate) block_members: HashMap<String, Vec<symbols::Member>>,
    pub(crate) global_members: Vec<(String, symbols::Member)>,
    /// The PROGRAMs with RETAIN variables.
    pub(crate) pou_retain: std::collections::HashSet<String>,
    /// The FUNCTION_BLOCKs that keep something across a warm start, of
    /// their own or in an instance they contain: an instance of one is
    /// kept whole in `ctx->retain` (docs/c-output.md §7).
    pub(crate) retentive: std::collections::HashSet<String>,
    /// Each PROGRAM's inputs and outputs, for its connections: places as
    /// its own code reaches them, `self->...`.
    pub(crate) program_params: HashMap<String, Vec<(String, Place, Dir)>>,
    /// The math functions called (Ed.3 Table 28): `IECF_SQRT` and `sqrt`.
    pub(crate) math: std::collections::BTreeSet<(String, &'static str)>,
    /// The statement's string buffers, declared before it (strings.rs),
    /// and the calls with EN or ENO written before it (enable.rs).
    pub(crate) str_temps: Vec<String>,
    /// The buffer each call among them wrote its array, structure or
    /// instance into, by the call's span: a part of the result read more
    /// than once is one call (expr.rs `data_path`).
    pub(crate) call_bufs: HashMap<Span, String>,
    /// SEL, MUX or MOVE of whole values in the statement: the pointer to
    /// the input selected (expr.rs `select_data`), by the call's span.
    pub(crate) sel_bufs: HashMap<Span, (String, derived::DTy)>,
    /// Whether an assignment's target is being found, its indices then
    /// able to leave the statement undone (stmt.rs `target`).
    pub(crate) target_keep_ok: bool,
    /// The markers a target's indices set: the assignment stores nothing
    /// when one is set (a real converted to an integer out of range).
    pub(crate) target_keeps: Vec<String>,
    /// The markers of the statement being written whose conversion may
    /// fail: set, the statement is left undone (stmt.rs `stop`).
    pub(crate) undone: Vec<String>,
    /// The CONFIGURATION's name, and its index among the POU names once a
    /// connection that can be flagged has needed one (`config_at`).
    config_key: String,
    config_pou: Option<u16>,
    /// The POU being written, innermost last: whose CONSTANT variables an
    /// initial value may name (consts.rs).
    pub(crate) const_ctx: Vec<String>,
    /// The CONFIGURATION's and its resources' CONSTANT globals.
    pub(crate) global_consts: HashMap<String, &'a VarDecl>,
    /// The constants an initial value is being worked out through: one
    /// that names itself is refused.
    pub(crate) const_busy: Vec<String>,
    /// An expression of literals alone and the type it is computed in
    /// (expr.rs `literal_in`).
    pub(crate) lit_as: Option<(Span, Elem)>,
    /// Why the constant expression being worked out has no value: a
    /// division by zero, a real that is infinite (consts.rs `fold`).
    pub(crate) fold_bad: Option<(iecf_diagnostics::Code, Span, String)>,
    /// The type an untyped real literal is, in the constant expression
    /// being worked out (consts.rs): the one wanted of it.
    pub(crate) fold_want: Option<Elem>,
    /// Whether what was worked out went through the mathematics of Table
    /// 28 or `**`: the target's own gives the value, the compiler's only
    /// says that it is one (consts.rs).
    pub(crate) fold_inexact: bool,
    /// Arguments of an EN/ENO call already computed, by their span.
    pub(crate) subst: HashMap<Span, enable::Sub>,
    /// Whether the expression being written is a statement's, which can
    /// have lines written before it (enable.rs).
    pub(crate) pre_ok: bool,
    /// Inside an EN/ENO call (enable.rs): another in its arguments is
    /// refused, not yet.
    pub(crate) gating: bool,
    /// A gated FUNCTION's output stores, which enable.rs writes after
    /// ENO is decided: a flag storing them is not the call's.
    pub(crate) gated_stores: Vec<String>,
    /// The stores back of the in-outs of the FUNCTION call being written
    /// that were given a copy (expr.rs `in_out_copy`).
    pub(crate) in_out_backs: Vec<String>,
    /// The statement's expression calls a FUNCTION with effects: each of
    /// its operands evaluated on a line of its own, left to right (Ed.3
    /// 7.3.2, rule 3), so a call runs after what is left of it is read
    /// (expr.rs `value`).
    pub(crate) seq: bool,
    /// Whether each FUNCTION has effects beyond its result: an in-out, a
    /// VAR_EXTERNAL not CONSTANT, or a call of one that has.
    pub(crate) effects: HashMap<String, bool>,
    /// Set by an EN/ENO call around the call it gates: a SPLIT may then
    /// write its lines, and leaves them here for inside `if (EN)`.
    pub(crate) split_ok: bool,
    pub(crate) split_lines: Option<Vec<String>>,
    /// The local an EN/ENO call passes as a FUNCTION's `eno`, taken by
    /// the outermost user call (enable.rs), and whether one took it.
    pub(crate) want_eno: Option<String>,
    pub(crate) eno_used: bool,
    /// Each CLASS's and FUNCTION_BLOCK's variables as its body reaches
    /// them, `self->V_X`: its methods reach them so (Ed.3 6.6.5.4.3 rule 11).
    pub(crate) block_scopes: HashMap<String, Scope>,
    /// What each CLASS's and FUNCTION_BLOCK's derived types inherit, as
    /// its body reaches it: all but its PRIVATE variables, VAR_TEMPs, THIS
    /// and ENO (Ed.3 6.6.5.5.2).
    pub(crate) inherit_scopes: HashMap<String, Scope>,
    /// Initial values `REF(x)`, by their marker's number: the variable,
    /// the value, and what it references (refs.rs).
    pub(crate) ref_inits: Vec<(String, Expr, derived::RefTo)>,
    /// An interface's initial value other than NULL, as `ref_inits`: its
    /// C lvalue, the value and the interface (itf.rs).
    pub(crate) itf_inits: Vec<(String, Expr, String)>,
    /// The C types references are dereferenced to, `@` for the name: the
    /// members of `ctx->nowhere` (refs.rs).
    pub(crate) nowhere: std::collections::BTreeSet<String>,
    /// The INTERFACEs the C names: their `ITY_` and `IT_` (itf.rs).
    pub(crate) itfs_used: std::collections::BTreeSet<String>,
}

impl<'a> Gen<'a> {
    fn new(units: &'a [&'a SourceUnit], types: &'a Types, map: &'a SourceMap) -> Self {
        let mut pous = HashMap::new();
        let mut type_decls = HashMap::new();
        for u in units {
            for item in &u.items {
                match item {
                    Item::Pou(p) => {
                        let owner = p.name.text.to_ascii_uppercase();
                        // A method by its key, as a FUNCTION by its name.
                        for m in &p.methods {
                            pous.entry(iecf_sema::method_key(&owner, &m.name.text)).or_insert(m);
                        }
                        pous.insert(owner, p);
                    }
                    Item::Types(ds) => {
                        for d in ds {
                            type_decls.insert(d.name.text.to_ascii_uppercase(), d);
                        }
                    }
                    _ => {}
                }
            }
        }
        Gen {
            units,
            types,
            map,
            diags: Vec::new(),
            pous,
            prefix: String::new(),
            symbols_json: String::new(),
            pou_names: Vec::new(),
            extent: [0; 3],
            may_fault: false,
            may_flag: false,
            keep_ok: false,
            keep: None,
            temp: 0,
            functions: HashMap::new(),
            fn_queue: Vec::new(),
            fn_queued: Default::default(),
            blocks: HashMap::new(),
            block_busy: Vec::new(),
            block_types: String::new(),
            block_fwd: String::new(),
            block_code: String::new(),
            init_keep: "0".into(),
            init_guard: None,
            type_decls,
            enums: HashMap::new(),
            structs: HashMap::new(),
            struct_busy: Vec::new(),
            data_types: String::new(),
            pou_members: HashMap::new(),
            block_members: HashMap::new(),
            global_members: Vec::new(),
            pou_retain: Default::default(),
            retentive: Default::default(),
            program_params: HashMap::new(),
            globals: HashMap::new(),
            math: Default::default(),
            str_temps: Vec::new(),
            call_bufs: HashMap::new(),
            sel_bufs: HashMap::new(),
            target_keep_ok: false,
            target_keeps: Vec::new(),
            undone: Vec::new(),
            config_key: String::new(),
            config_pou: None,
            const_ctx: Vec::new(),
            global_consts: HashMap::new(),
            const_busy: Vec::new(),
            lit_as: None,
            fold_bad: None,
            fold_want: None,
            fold_inexact: false,
            want_eno: None,
            eno_used: false,
            subst: HashMap::new(),
            pre_ok: false,
            gating: false,
            gated_stores: Vec::new(),
            in_out_backs: Vec::new(),
            seq: false,
            effects: HashMap::new(),
            split_ok: false,
            split_lines: None,
            ref_inits: Vec::new(),
            itf_inits: Vec::new(),
            nowhere: Default::default(),
            itfs_used: Default::default(),
            block_scopes: HashMap::new(),
            inherit_scopes: HashMap::new(),
        }
    }

    /// E0009 at `span`.
    pub(crate) fn not_yet<T>(&mut self, span: Span, what: impl Into<String>) -> R<T> {
        self.diags.push(Diagnostic::new(
            Code::NotYetSupported,
            span,
            format!("{} is not compiled to C yet", what.into()),
        ));
        Err(Skip)
    }

    pub(crate) fn line(&self, span: Span) -> u32 {
        self.map.line_col(span.file, span.start).0
    }

    pub(crate) fn fresh(&mut self, stem: &str) -> String {
        self.temp += 1;
        format!("{stem}{}", self.temp)
    }

    fn project(&mut self) -> Option<Vec<(String, String)>> {
        let configs: Vec<&Configuration> = self
            .units
            .iter()
            .flat_map(|u| &u.items)
            .filter_map(|i| if let Item::Configuration(c) = i { Some(c) } else { None })
            .collect();
        let config = match configs.as_slice() {
            [c] => *c,
            [] => {
                let at = self.units.iter().flat_map(|u| &u.items).find_map(|i| match i {
                    Item::Pou(p) => Some(p.span),
                    _ => None,
                });
                let at = at.unwrap_or(Span::new(iecf_diagnostics::FileId(0), 0, 0));
                let _: R<()> = self.not_yet(at, "a project without a CONFIGURATION (nothing says what runs)");
                return None;
            }
            [_, second, ..] => {
                let _: R<()> = self.not_yet(second.name.span, "a second CONFIGURATION");
                return None;
            }
        };
        if !config.config_inits.is_empty() {
            let _: R<()> = self.not_yet(config.config_inits[0].span, "VAR_CONFIG");
        }
        self.prefix = config.name.text.to_ascii_lowercase();
        self.config_key = config.name.text.to_ascii_uppercase();
        self.configuration(config).ok()
    }

    fn configuration(&mut self, config: &'a Configuration) -> R<Vec<(String, String)>> {
        let p = self.prefix.clone();
        let up = p.to_ascii_uppercase();
        for b in config.globals.iter().chain(config.resources.iter().flat_map(|r| &r.globals)).filter(|b| b.constant) {
            for d in &b.decls {
                for n in &d.names {
                    self.global_consts.insert(n.text.to_ascii_uppercase(), d);
                }
            }
        }

        // Globals: the configuration's, then each resource's.
        let mut globals = Scope::default();
        let mut fields = String::new(); // members of <p>_t
        let mut init = String::new(); // body of <p>_init and <p>_warm
        let mut rfields = String::new(); // members of <p>_retain_t
        let mut rinit = String::new(); // a cold start's, after `init`
        let mut links = String::new(); // each instance's pointer into ctx->retain
        // Those whose type names no FUNCTION_BLOCK or CLASS first, then
        // the instances placed, then the rest: a type written for one may
        // name any of them (inst.rs).
        let blocks: Vec<(&VarBlock, String)> = config
            .globals
            .iter()
            .map(|b| (b, String::new()))
            .chain(config.resources.iter().flat_map(|r| {
                let rp = r.name.as_ref().map(|n| format!("{}__", n.text.to_ascii_uppercase())).unwrap_or_default();
                r.globals.iter().map(move |b| (b, rp.clone()))
            }))
            .collect();
        for (b, rp) in &blocks {
            self.global_block(b, rp, false, &mut globals, (&mut fields, &mut init), (&mut rfields, &mut rinit))?;
        }
        let mut placed = globals.clone();
        for (b, rp) in &blocks {
            self.place_global_instances(b, rp, &mut placed);
        }
        self.globals = placed.vars;
        for (b, rp) in &blocks {
            self.global_block(b, rp, true, &mut globals, (&mut fields, &mut init), (&mut rfields, &mut rinit))?;
        }

        self.fill_ref_inits(&mut init, &globals)?;
        self.fill_ref_inits(&mut rinit, &globals)?;
        self.globals = globals.vars.clone();

        // Program instances and the POUs they run.
        let mut pou_code = String::new();
        let mut pou_types = String::new();
        let mut generated: HashMap<String, ()> = HashMap::new();
        struct Instance {
            field: String,
            pou: String,
            task: Option<String>,
            /// `R.P`, as the symbol table names it.
            path: String,
            /// Its connections: inputs set before each run, outputs taken
            /// after it.
            pre: String,
            post: String,
        }
        let mut instances: Vec<Instance> = Vec::new();
        let mut tasks: Vec<(String, i64, i64)> = Vec::new(); // C name, interval ns, priority
        for r in &config.resources {
            let rp = r.name.as_ref().map(|n| format!("{}__", n.text.to_ascii_uppercase())).unwrap_or_default();
            for t in &r.tasks {
                let (interval, priority) = self.task_params(t)?;
                tasks.push((format!("{rp}{}", t.name.text.to_ascii_uppercase()), interval, priority));
            }
            for pc in &r.programs {
                if pc.retain.is_some() {
                    return self.not_yet(pc.span, "RETAIN on a program instance");
                }
                let key = pc.program_type.text.to_ascii_uppercase();
                let Some(pou) = self.pous.get(&key).copied() else {
                    return self.not_yet(pc.program_type.span, "a program that is not in the project");
                };
                if pou.kind != PouKind::Program {
                    return self.not_yet(pc.program_type.span, "an instance of a POU that is not a PROGRAM");
                }
                if generated.insert(key.clone(), ()).is_none() {
                    let index = self.pou_names.len();
                    self.pou_names.push(key.clone());
                    self.const_ctx.push(key.clone());
                    let r = self.program(pou, index as u16, &globals);
                    self.const_ctx.pop();
                    let (ty, code) = r?;
                    pou_types.push_str(&ty);
                    pou_code.push_str(&code);
                }
                let field = format!("V_{rp}{}", pc.name.text.to_ascii_uppercase());
                let _ = writeln!(fields, "    P_{key}_t {field};");
                let _ = writeln!(init, "    P_{key}_init(ctx, &ctx->{field}, warm);");
                let _ = writeln!(init, "    if (ctx->fault.code != IECF_FAULT_NONE) return;");
                if self.pou_retain.contains(&key) {
                    let _ = writeln!(rfields, "    P_{key}_retain_t {field};");
                    let _ = writeln!(links, "    ctx->{field}.retain = &ctx->retain.{field};");
                    let _ = writeln!(rinit, "    P_{key}_init_retain(ctx, &ctx->{field});");
                }
                let task = pc.task.as_ref().map(|t| format!("{rp}{}", t.text.to_ascii_uppercase()));
                let path = format!("{}{}", rp.replace("__", "."), pc.name.text.to_ascii_uppercase());
                let (pre, post) = self.connections(pc, &key, &field, &globals)?;
                instances.push(Instance { field, pou: key, task, path, pre, post });
            }
        }
        if instances.iter().any(|i| i.task.is_none()) {
            tasks.push(("_default".into(), 0, i64::MAX));
        }

        // The user's FUNCTIONs the programs call, and those they call.
        let mut fn_protos = String::new();
        let mut fn_code = String::new();
        let mut done: Vec<String> = Vec::new();
        // The tables may name a method still to write: then again.
        let (oo_head, oo_thunks) = loop {
            while let Some(key) = self.fn_queue.pop() {
                if done.contains(&key) {
                    continue;
                }
                done.push(key.clone());
                self.const_ctx.push(key.clone());
                let r = self.function(&key);
                self.const_ctx.pop();
                let (proto, code) = r?;
                fn_protos.push_str(&proto);
                fn_code.push_str(&code);
            }
            let tables = self.oo_tables()?;
            if self.fn_queue.is_empty() {
                break tables;
            }
        };

        // <p>.h
        let mut h = String::new();
        let guard = format!("{up}_H");
        let _ = writeln!(h, "/* {p}.h - written by iecf from CONFIGURATION {}. */", config.name.text);
        let _ = writeln!(h, "#ifndef {guard}\n#define {guard}\n\n#include \"iecf.h\"\n");
        for (area, n) in ["I", "Q", "M"].iter().zip(self.extent) {
            let _ = writeln!(h, "#define {up}_{area}_BYTES {n}u");
        }
        let _ = writeln!(h, "\n{}{}{}{pou_types}", self.block_fwd, self.data_types, self.block_types);
        // The RETAIN variables, one region the platform keeps across a
        // restart (docs/c-output.md §7).
        let _ = writeln!(h, "typedef struct {{");
        if rfields.is_empty() {
            let _ = writeln!(h, "    char none_; /* no RETAIN variable */");
        }
        h.push_str(&rfields);
        let _ = writeln!(h, "}} {p}_retain_t;\n");
        let _ = writeln!(h, "typedef struct {{");
        let _ = writeln!(h, "    iecf_image_t image; /* the platform's, set before {p}_init */");
        let _ = writeln!(h, "    iecf_fault_t fault;");
        let _ = writeln!(h, "    iecf_fault_t flag;  /* the first infinite or NaN real; the platform clears it */");
        let _ = writeln!(h, "    int64_t now_ns;     /* the running task's time */");
        h.push_str(&fields);
        h.push_str(&self.nowhere_field());
        let _ = writeln!(h, "    {p}_retain_t retain; /* kept across a restart; {p}_warm leaves it */");
        let _ = writeln!(h, "}} {p}_t;\n");
        let _ = writeln!(h, "void {p}_init({p}_t *ctx);");
        let _ = writeln!(h, "void {p}_warm({p}_t *ctx);");
        for (name, _, _) in &tasks {
            let _ = writeln!(h, "void {p}_task_{name}({p}_t *ctx, int64_t now_ns);");
        }
        let _ = writeln!(h, "\n/* The tasks, for the platform's scheduler: interval 0 is cyclic. */");
        let _ = writeln!(h, "typedef struct {{");
        let _ = writeln!(h, "    const char *name;");
        let _ = writeln!(h, "    void (*run)({p}_t *ctx, int64_t now_ns);");
        let _ = writeln!(h, "    int64_t interval_ns;");
        let _ = writeln!(h, "    int64_t priority;");
        let _ = writeln!(h, "}} {p}_task_t;");
        let _ = writeln!(h, "extern const {p}_task_t {p}_tasks[];");
        let _ = writeln!(h, "extern const uint32_t {p}_task_count;");
        let _ = writeln!(h, "extern const char *const {p}_pou_names[];");
        let _ = writeln!(h, "\n/* The symbol table (docs/c-output.md §6). */");
        let _ = writeln!(h, "extern const iecf_symbol_t {p}_symbols[];");
        let _ = writeln!(h, "extern const uint32_t {p}_symbol_count;");
        let _ = writeln!(h, "extern const iecf_type_t {p}_types[];");
        let _ = writeln!(h, "extern const uint32_t {p}_type_count;");
        let _ = writeln!(h, "\n#endif /* {guard} */");

        // <p>.c
        let mut c = String::new();
        let _ = writeln!(c, "/* {p}.c - written by iecf from CONFIGURATION {}. */", config.name.text);
        let _ = writeln!(c, "#include \"{p}.h\"\n");
        // Table 28's functions from <math.h>, unless the platform gives
        // its own (docs/c-output.md §1); none, no <math.h>.
        for (m, f) in &self.math {
            let args = if *f == "pow" { "x, y" } else { "x" };
            let _ = writeln!(c, "#ifndef {m}\n#include <math.h>\n#define {m}({args}) {f}({args})\n#endif");
        }
        if !self.math.is_empty() {
            c.push('\n');
        }
        c.push_str(&fn_protos);
        if !fn_protos.is_empty() {
            c.push('\n');
        }
        c.push_str(&oo_head);
        c.push_str(&self.block_code);
        c.push_str(&oo_thunks);
        c.push_str(&fn_code);
        c.push_str(&pou_code);
        // A warm start sets every variable but the RETAIN ones; a cold
        // start, those too (docs/c-output.md §7).
        let _ = writeln!(c, "static void {p}_start({p}_t *ctx, int warm) {{");
        let _ = writeln!(c, "    ctx->fault.code = IECF_FAULT_NONE;");
        let _ = writeln!(c, "    ctx->fault.pou = 0u;");
        let _ = writeln!(c, "    ctx->fault.line = 0u;");
        let _ = writeln!(c, "    ctx->flag = ctx->fault;");
        let _ = writeln!(c, "    ctx->now_ns = 0;");
        for ((area, n), field) in ["I", "Q", "M"].iter().zip(self.extent).zip(["i", "q", "m"]) {
            if n > 0 {
                let _ = writeln!(
                    c,
                    "    if (ctx->image.{field} == NULL || ctx->image.{field}_size < {up}_{area}_BYTES) {{\n        iecf_fail(&ctx->fault, IECF_FAULT_IMAGE);\n        return;\n    }}"
                );
            }
        }
        c.push_str(&links);
        c.push_str(&init);
        let _ = writeln!(c, "}}\n");
        let _ = writeln!(c, "void {p}_init({p}_t *ctx) {{");
        let _ = writeln!(c, "    {p}_start(ctx, 0);");
        if !rinit.is_empty() {
            let _ = writeln!(c, "    if (ctx->fault.code != IECF_FAULT_NONE) return;");
            c.push_str(&rinit);
        }
        let _ = writeln!(c, "}}\n");
        let _ = writeln!(c, "void {p}_warm({p}_t *ctx) {{\n    {p}_start(ctx, 1);\n}}\n");
        for (name, _, _) in &tasks {
            let _ = writeln!(c, "void {p}_task_{name}({p}_t *ctx, int64_t now_ns) {{");
            let _ = writeln!(c, "    if (ctx->fault.code != IECF_FAULT_NONE) return;");
            let _ = writeln!(c, "    ctx->now_ns = now_ns;");
            let mine = |i: &&Instance| match &i.task {
                Some(t) => t == name,
                None => name == "_default",
            };
            for i in instances.iter().filter(mine) {
                c.push_str(&i.pre);
                let _ = writeln!(c, "    P_{}(ctx, &ctx->{});", i.pou, i.field);
                let _ = writeln!(c, "    if (ctx->fault.code != IECF_FAULT_NONE) return;");
                c.push_str(&i.post);
            }
            let _ = writeln!(c, "}}\n");
        }
        let _ = writeln!(c, "const {p}_task_t {p}_tasks[] = {{");
        for (name, interval, priority) in &tasks {
            let _ = writeln!(c, "    {{\"{name}\", {p}_task_{name}, INT64_C({interval}), INT64_C({priority})}},");
        }
        let _ = writeln!(c, "}};");
        let _ = writeln!(c, "const uint32_t {p}_task_count = {}u;", tasks.len());
        let names: Vec<String> =
            self.pou_names.iter().map(|n| format!("\"{}\"", iecf_sema::namespaces::display(n))).collect();
        let _ = writeln!(c, "const char *const {p}_pou_names[] = {{{}}};\n", names.join(", "));

        // The symbol table: the globals, then each program instance's
        // variables (docs/c-output.md §6).
        let mut syms = Vec::new();
        for (path, m) in &self.global_members {
            if m.flags & symbols::RETAIN != 0 {
                syms.push((path.clone(), m.clone(), format!("{p}_retain_t"), format!("offsetof({p}_t, retain) + ")));
            } else {
                syms.push((path.clone(), m.clone(), format!("{p}_t"), String::new()));
            }
        }
        for i in &instances {
            for m in self.pou_members.get(&i.pou).cloned().unwrap_or_default() {
                let (container, base) = if m.flags & symbols::RETAIN != 0 {
                    (
                        format!("P_{}_retain_t", i.pou),
                        format!("offsetof({p}_t, retain) + offsetof({p}_retain_t, {}) + ", i.field),
                    )
                } else {
                    (format!("P_{}_t", i.pou), format!("offsetof({p}_t, {}) + ", i.field))
                };
                syms.push((format!("{}.{}", i.path, m.name), m, container, base));
            }
        }
        c.push_str(&self.symbol_tables(&syms));
        let c = name_unused(&c, &format!("static void {p}_start({p}_t *ctx, int warm) {{\n"));

        Ok(vec![
            (format!("{p}.h"), h),
            (format!("{p}.c"), c),
            ("iecf.h".into(), IECF_H.into()),
            (format!("{p}.symbols.json"), std::mem::take(&mut self.symbols_json)),
        ])
    }

    /// A program instance's connections (Ed.3 6.8.2): each input set from
    /// a literal, a global or an address before the program runs, each
    /// output written to a global or an address after it.
    fn connections(&mut self, pc: &ProgramConfig, key: &str, field: &str, globals: &Scope) -> R<(String, String)> {
        let params = self.program_params.get(key).cloned().unwrap_or_default();
        let (mut pre, mut post) = (String::new(), String::new());
        let mut bound: Vec<String> = Vec::new();
        let inst = format!("ctx->{field}.");
        for a in &pc.args {
            let (name, dir) = match a {
                Arg::Input(n, _) => (n, Dir::Input),
                Arg::Output { name, .. } => (name, Dir::Output),
                Arg::Positional(e) => return self.not_yet(e.span, "a connection given in order"),
            };
            let Some((key, place, d)) = params.iter().find(|(k, _, _)| name.is(k)).cloned() else {
                return self.not_yet(name.span, format!("the connection {}", name.text));
            };
            // An in-out bound to a global: its pointer set before each run.
            // A located global or an address is in the byte-wise image,
            // which no C pointer of its type may point into.
            if d == Dir::InOut {
                let Arg::Input(_, e) = a else {
                    return self.not_yet(name.span, format!("the connection {}", name.text));
                };
                let (p1, p2) = self.in_out_binding(e, &place, &format!("ctx->{field}.V_{key}"), globals)?;
                pre.push_str(&p1);
                post.push_str(&p2);
                bound.push(key);
                continue;
            }
            // An interface: an input given a global interface or
            // instance, or NULL; an output stored into a global interface.
            if let Place::Itf(plv, i) = &place {
                let lv = plv.replacen("self->", &inst, 1);
                match (a, d == dir) {
                    (Arg::Input(_, e), true) => {
                        let v = self.itf_value(e, i, globals)?;
                        let _ = writeln!(pre, "    {lv} = {v};");
                    }
                    (Arg::Output { target, .. }, true) => {
                        let Some((to, crate::derived::DTy { kind: crate::derived::Kind::Itf(ti), .. })) =
                            self.data_path(target, globals)?
                        else {
                            return self.not_yet(target.span, "a connection to this");
                        };
                        let store = self.itf_store(&to, &ti, &lv, i, target.span)?;
                        let _ = writeln!(post, "    {store}");
                    }
                    _ => return self.not_yet(name.span, format!("the connection {}", name.text)),
                }
                continue;
            }
            // An array, a structure or an instance: copied whole from a
            // global of its type before the program runs, to one after.
            let whole = match &place {
                Place::Data(Lv::C(c), t) if matches!(t.kind, derived::Kind::Array(..) | derived::Kind::Struct(_)) => {
                    Some((c.clone(), t.clone()))
                }
                p @ (Place::Block(..) | Place::Fb(..)) => {
                    inst::whole(p).and_then(|(lv, t)| lv.c().map(|c| (c.to_string(), t)))
                }
                _ => None,
            };
            if let Some((c, t)) = whole {
                let mine = Lv::C(c.replacen("self->", &inst, 1));
                let other = match a {
                    Arg::Input(_, e) => e,
                    Arg::Output { target, .. } => target,
                    Arg::Positional(e) => e,
                };
                self.may_fault = false;
                let found = self.data_path(other, globals)?;
                if self.may_fault {
                    return self.not_yet(other.span, "a connection that can fault or be flagged");
                }
                let Some((theirs, tt)) = found else {
                    return self.not_yet(other.span, "a connection to this");
                };
                // The checker allowed it: one type, so one C type.
                if self.declare(&tt, "x") != self.declare(&t, "x") {
                    return self.not_yet(other.span, "a connection of another type");
                }
                match (a, d == dir) {
                    (Arg::Input(..), true) => {
                        let copy = self.copy_data(&mine, &theirs, &t, other.span)?;
                        let _ = writeln!(pre, "    {copy}");
                    }
                    (Arg::Output { not: false, .. }, true) => {
                        let copy = self.copy_data(&theirs, &mine, &t, other.span)?;
                        let _ = writeln!(post, "    {copy}");
                    }
                    _ => return self.not_yet(name.span, format!("the connection {}", name.text)),
                }
                continue;
            }
            // A STRING: copied, cut to what it is stored in and flagged.
            if let Place::Data(Lv::C(c), derived::DTy { kind: derived::Kind::Str(n, w), .. }) = &place {
                let (n, w) = (*n, *w);
                let mine = c.replacen("self->", &inst, 1);
                let p = crate::strings::infix(w);
                match (a, d == dir) {
                    (Arg::Input(_, e), true) => {
                        let (src, _) = self.string(e, globals)?;
                        let _ = writeln!(pre, "    iecf_{p}scopy(&ctx->flag, {mine}, {n}u, {src});");
                        let (pou, line) = self.config_at(e.span);
                        let _ = writeln!(pre, "    iecf_noted(&ctx->flag, {pou}u, {line}u);");
                    }
                    (Arg::Output { target, .. }, true) => {
                        let Some((to, derived::DTy { kind: derived::Kind::Str(m, _), .. })) =
                            self.data_path(target, globals)?
                        else {
                            return self.not_yet(target.span, "a connection to this");
                        };
                        let copy = format!("iecf_{p}scopy(&ctx->flag, @, {m}u, {mine});");
                        let _ = writeln!(post, "    {}", crate::strings::store_string(&to, m, w, &copy));
                        let (pou, line) = self.config_at(target.span);
                        let _ = writeln!(post, "    iecf_noted(&ctx->flag, {pou}u, {line}u);");
                    }
                    _ => return self.not_yet(name.span, format!("the connection {}", name.text)),
                }
                continue;
            }
            let Some(ty) = place.elem() else {
                return self.not_yet(name.span, "a connection of this variable");
            };
            let range = |p: &Place| match p {
                Place::Data(_, t) => t.range,
                _ => None,
            };
            let lv = match place.lvalue() {
                Some(lv) => lv.replacen("self->", &inst, 1),
                None => return self.not_yet(name.span, "a connection of this variable"),
            };
            match (a, d == dir) {
                (Arg::Input(_, e), true) => {
                    self.may_fault = false;
                    self.may_flag = false;
                    let v = self.value_as(e, ty, globals)?;
                    // Into a subrange input: clamped and flagged.
                    let v = self.clamp(&v, ty, range(&place));
                    if self.may_fault {
                        return self.not_yet(e.span, "a connection that can fault");
                    }
                    let _ = writeln!(pre, "    {lv} = {v};");
                    if self.may_flag {
                        let (pou, line) = self.config_at(e.span);
                        let _ = writeln!(pre, "    iecf_noted(&ctx->flag, {pou}u, {line}u);");
                    }
                }
                (Arg::Output { not, target, .. }, true) => {
                    let v = if *not { format!("((IEC_BOOL)!{lv})") } else { lv.clone() };
                    let to = match &target.kind {
                        ExprKind::Name(id) => globals.vars.get(&id.text.to_ascii_uppercase()).cloned(),
                        ExprKind::DirectAddress(a) => {
                            let t = match self.types.expr(target.span) {
                                Some(Ty::Elem(t)) => *t,
                                _ => return self.not_yet(target.span, "this address"),
                            };
                            Some(self.located(a, t)?)
                        }
                        // A field of a global (Ed.3 6.8.2, Global_Var_Access).
                        _ => self.data_path(target, globals)?.map(|(lv, t)| Place::Data(lv, t)),
                    };
                    let Some(to) = to else { return self.not_yet(target.span, "a connection to this") };
                    let Some(tt) = to.elem() else { return self.not_yet(target.span, "a connection to this") };
                    // Figure 11's implicit conversions widen: a cast, but
                    // not between the time and date types, whose units
                    // differ (TIME in ms, LTIME in ns): their conversions,
                    // exact when widening.
                    self.may_flag = false;
                    let v = if tt == ty {
                        v
                    } else if expr::is_time_or_date(ty) || expr::is_time_or_date(tt) {
                        let v = self.convert(target.span, v, ty, tt)?;
                        if self.may_flag {
                            return self
                                .not_yet(target.span, format!("a connection of {} to {}", ty.name(), tt.name()));
                        }
                        v
                    } else {
                        format!("(({})({v}))", c_type(tt))
                    };
                    // Into a subrange global: clamped and flagged, as any store.
                    let v = self.clamp(&v, tt, range(&to));
                    let _ = writeln!(post, "    {}", to.store(&v));
                    if self.may_flag {
                        let (pou, line) = self.config_at(target.span);
                        let _ = writeln!(post, "    iecf_noted(&ctx->flag, {pou}u, {line}u);");
                    }
                }
                _ => return self.not_yet(name.span, format!("the connection {}", name.text)),
            }
        }
        if let Some((k, ..)) = params.iter().find(|(k, _, d)| *d == Dir::InOut && !bound.contains(k)) {
            return self.not_yet(pc.span, format!("a program whose in-out {k} is not connected"));
        }
        Ok((pre, post))
    }

    /// `AT %QW4 : INT := 5;`, a located variable without a name (Ed.3
    /// Loc_Var_Decl): its address is reserved and given its initial value,
    /// and nothing names it - the body reaches it by its address.
    fn unnamed_located(
        &mut self,
        d: &VarDecl,
        decl: &Decl,
        owner: &str,
        fields: &mut String,
        init: &mut String,
    ) -> R<()> {
        let Some(a) = &d.location else { return self.not_yet(d.span, "a declaration without a name") };
        match decl {
            Decl::Elem(ty) => {
                let place = self.located(a, *ty)?;
                self.init_var(&place, d, "0", init)
            }
            Decl::Data(t) => {
                let k = self.fresh("UNNAMED");
                self.data_var(owner, &k, t, d, fields, init).map(|_| ())
            }
            _ => self.not_yet(d.span, "this located variable without a name"),
        }
    }

    /// An R_EDGE or F_EDGE input `k` of a block or a PROGRAM, its raw value
    /// at `raw` (Ed.3 6.6.3.2 rule 13): an R_TRIG or F_TRIG of its own in
    /// the instance, `E_<k>`, set up with it and called with the input as
    /// the body starts; the body reads its Q. Set from outside, the input
    /// itself is the raw value.
    fn edge_input(
        &mut self,
        k: &str,
        edge: Edge,
        raw: &str,
        (fields, init): (&mut String, &mut String),
        lines: &mut String,
        scope: &mut Scope,
    ) {
        let stem = match edge {
            Edge::Rising => "R_TRIG",
            Edge::Falling => "F_TRIG",
        };
        let _ = writeln!(fields, "    iecf_{stem}_t E_{k};");
        let _ = writeln!(init, "    iecf_{stem}_init(&self->E_{k});");
        let _ = writeln!(lines, "    self->E_{k}.CLK = {raw};\n    iecf_{stem}(&self->E_{k});");
        scope.vars.insert(k.to_string(), Place::Field(format!("self->E_{k}.Q"), Elem::Bool));
    }

    /// Where a flag in a CONFIGURATION's connection code is recorded: the
    /// CONFIGURATION, by its name among the POU names, and the line of
    /// the connection (docs/c-output.md §5).
    fn config_at(&mut self, at: Span) -> (u16, u32) {
        let pou = match self.config_pou {
            Some(p) => p,
            None => {
                let p = self.pou_names.len() as u16;
                self.pou_names.push(self.config_key.clone());
                self.config_pou = Some(p);
                p
            }
        };
        (pou, self.line(at))
    }

    /// The in-out `ptr` of a program instance bound to `e`, a global or
    /// an element or field of one, of its type (the checker saw), as
    /// connection code before and after the run. In C memory it is that
    /// variable's address; at an address, where no C pointer of its type
    /// may point (docs/c-output.md §4), a copy made before the run and
    /// stored back after it - not if the run faults - as a FUNCTION's
    /// in-out given one is.
    fn in_out_binding(&mut self, e: &Expr, place: &Place, ptr: &str, globals: &Scope) -> R<(String, String)> {
        let (mut pre, mut post) = (String::new(), String::new());
        self.may_fault = false;
        self.may_flag = false;
        let theirs = match &e.kind {
            ExprKind::Name(id) => globals.vars.get(&id.text.to_ascii_uppercase()).cloned(),
            _ => match self.data_path(e, globals)? {
                Some((Lv::C(c), t)) => Some(data_place(c, t)),
                Some((lv, t)) => Some(Place::Data(lv, t)),
                None => None,
            },
        };
        if self.may_fault || self.may_flag {
            return self.not_yet(e.span, "an in-out connected through an index that can fault");
        }
        let Some(theirs) = theirs else {
            return self.not_yet(e.span, "an in-out connected to this");
        };
        // A subrange of other bounds than the in-out's: a store through
        // the pointer would not be clamped to the variable's own.
        let range = |p: &Place| match p {
            Place::Data(_, t) => t.range,
            _ => None,
        };
        // A subrange of other bounds than the in-out's: a copy, stored back
        // clamped to the variable's own and flagged (as a FUNCTION's
        // in-out given one, expr.rs).
        if let Some(r) = range(&theirs)
            && range(&theirs) != range(place)
            && let Some(el) = place.elem()
        {
            let c = self.fresh("io");
            let _ = writeln!(pre, "    {} {c} = {};", c_type(el), theirs.load());
            let _ = writeln!(pre, "    {ptr} = &{c};");
            let v = self.clamp(&c, el, Some(r));
            let _ = writeln!(post, "    {}", theirs.store(&v));
            let (pou, line) = self.config_at(e.span);
            let _ = writeln!(post, "    iecf_noted(&ctx->flag, {pou}u, {line}u);");
            return Ok((pre, post));
        }
        match (&theirs, place) {
            // An instance, of its type or a derived one (Ed.3 6.6.8).
            (Place::Block(g, xs), Place::Block(_, k)) => {
                let _ = writeln!(pre, "    {ptr} = &({g}){};", self.up_path(xs, k));
            }
            (Place::Fb(g, _), Place::Fb(..)) | (Place::Field(g, _), _) => {
                let _ = writeln!(pre, "    {ptr} = &({g});");
            }
            (Place::Data(Lv::C(g), _), _) => {
                let _ = writeln!(pre, "    {ptr} = &({g});");
            }
            // At an address: elementary.
            (Place::Located { ty, .. }, _) => {
                let c = self.fresh("io");
                let _ = writeln!(pre, "    {} {c} = {};", c_type(*ty), theirs.load());
                let _ = writeln!(pre, "    {ptr} = &{c};");
                let _ = writeln!(post, "    {}", theirs.store(&c));
            }
            // At an address: an array, a structure, a STRING, or an
            // element or field there.
            (Place::Data(lv, t), _) => {
                let c = self.fresh("io");
                let mine = Lv::C(c.clone());
                let _ = writeln!(pre, "    {};", self.declare(t, &c));
                match t.scalar() {
                    Some(el) => {
                        let _ = writeln!(pre, "    {}", mine.store(el, &lv.load(el)));
                        let _ = writeln!(post, "    {}", lv.store(el, &c));
                    }
                    None => {
                        let fill = self.copy_data(&mine, lv, t, e.span)?;
                        let back = self.copy_data(lv, &mine, t, e.span)?;
                        let _ = writeln!(pre, "    {fill}");
                        let _ = writeln!(post, "    {back}");
                    }
                }
                let _ = writeln!(pre, "    {ptr} = &{c};");
            }
            _ => return self.not_yet(e.span, "an in-out connected to this"),
        }
        Ok((pre, post))
    }

    /// A TASK's interval in ns (0 without one) and priority.
    fn task_params(&mut self, t: &Task) -> R<(i64, i64)> {
        let (mut interval, mut priority) = (0, 0);
        for a in &t.params {
            let Arg::Input(name, value) = a else { return self.not_yet(t.span, "this TASK parameter") };
            match name.text.to_ascii_uppercase().as_str() {
                "INTERVAL" => match &value.kind {
                    ExprKind::Literal(l) if l.kind == LiteralKind::Time => match expr::duration_ns(&l.text) {
                        Some(ns) => interval = ns,
                        None => return self.not_yet(value.span, "this INTERVAL"),
                    },
                    // A VAR_GLOBAL CONSTANT: its value, worked out here.
                    _ => {
                        let unit = match self.types.expr(value.span) {
                            Some(Ty::Elem(Elem::Time)) => 1_000_000,
                            Some(Ty::Elem(Elem::Ltime)) => 1,
                            _ => 0,
                        };
                        self.fold_bad = None;
                        match self.fold(value) {
                            Some(consts::Cv::Int(v)) if unit > 0 => match i64::try_from(v * unit) {
                                Ok(ns) => interval = ns,
                                Err(_) => return self.not_yet(value.span, "this INTERVAL"),
                            },
                            _ => {
                                return self.not_yet(
                                    value.span,
                                    "an INTERVAL that is a variable (the platform would read it)",
                                );
                            }
                        }
                    }
                },
                "PRIORITY" => match expr::const_int(value) {
                    Some(v) => priority = v as i64,
                    None => return self.not_yet(value.span, "a PRIORITY that is not a literal"),
                },
                _ => return self.not_yet(name.span, "a SINGLE-triggered TASK"),
            }
        }
        Ok((interval, priority))
    }

    /// A VAR_GLOBAL block: fields of the context, or of `ctx->retain`
    /// (`retain`, its fields and cold-start initialisation) for RETAIN.
    /// `late`: only the declarations whose type names a FUNCTION_BLOCK or
    /// CLASS (inst.rs `names_block`), else only the others; each late one
    /// known to the POUs as soon as it is placed.
    fn global_block(
        &mut self,
        b: &VarBlock,
        rp: &str,
        late: bool,
        globals: &mut Scope,
        plain: (&mut String, &mut String),
        retain: (&mut String, &mut String),
    ) -> R<()> {
        let is_retain = b.retain == Some(Retain::Retain);
        let (pfields, pinit) = plain;
        let (rfields, rinit) = retain;
        let flags = if b.constant { symbols::CONSTANT } else { 0 } | if is_retain { symbols::RETAIN } else { 0 };
        let mine: Vec<&VarDecl> = b.decls.iter().filter(|d| self.names_block(&d.ty, 0) == late).collect();
        for d in mine {
            let (fields, init, owner) = if is_retain {
                (&mut *rfields, &mut *rinit, "ctx->retain.")
            } else {
                (&mut *pfields, &mut *pinit, "ctx->")
            };
            let decl = self.decl(d)?;
            // An instance, or an array or structure of them: set at every
            // start, as a program's (docs/c-output.md §7), in `ctx->retain`
            // when RETAIN or keeping something of its own.
            let whole = match &decl {
                Decl::Block(key) => Some(is_retain || self.type_keeps(key, 0)),
                Decl::Data(t) if self.has_instance(t) => Some(is_retain || self.keeps_in(t)),
                _ => None,
            };
            if let Some(whole) = whole {
                if let Some(a) = &d.location {
                    return self.not_yet(a.span, "a function block at an address");
                }
                let (f, owner) = if whole { (&mut *rfields, "ctx->retain.") } else { (&mut *pfields, "ctx->") };
                self.init_keep = if is_retain { "warm ? 2 : 0" } else { "warm" }.into();
                self.init_guard = is_retain.then(|| "!warm".into());
                let mut r = Ok(());
                for n in &d.names {
                    let key = n.text.to_ascii_uppercase();
                    let field = format!("V_{rp}{key}");
                    let place = match &decl {
                        Decl::Block(k) => {
                            let _ = writeln!(f, "    B_{k}_t {field};");
                            Place::Block(format!("{owner}{field}"), k.clone())
                        }
                        Decl::Data(t) => {
                            let _ = writeln!(f, "    {};", self.declare(t, &field));
                            Place::Data(Lv::C(format!("{owner}{field}")), t.clone())
                        }
                        _ => unreachable!("an instance's declaration"),
                    };
                    r = match &place {
                        Place::Data(Lv::C(lv), t) => self.init_data(lv, t, d.init.as_ref(), pinit),
                        _ => self.init_var(&place, d, &self.init_keep.clone(), pinit),
                    };
                    if r.is_err() {
                        break;
                    }
                    let flags = flags | if whole { symbols::RETAIN } else { 0 };
                    let m = symbols::member(&key, &field, &place, flags);
                    self.global_members.push((format!("{}{key}", rp.replace("__", ".")), m));
                    globals.vars.insert(key, place);
                }
                self.init_keep = "0".into();
                self.init_guard = None;
                r?;
                self.globals = globals.vars.clone();
                continue;
            }
            let ty = match decl {
                Decl::Elem(ty) => ty,
                Decl::Data(t) => {
                    if let (Some(a), true) = (&d.location, is_retain) {
                        return self.not_yet(a.span, "a RETAIN variable at an address");
                    }
                    for n in &d.names {
                        let key = n.text.to_ascii_uppercase();
                        let place = self.data_var(owner, &format!("{rp}{key}"), &t, d, fields, init)?;
                        let m = symbols::member(&key, &format!("V_{rp}{key}"), &place, flags);
                        self.global_members.push((format!("{}{key}", rp.replace("__", ".")), m));
                        globals.vars.insert(key, place);
                    }
                    self.globals = globals.vars.clone();
                    continue;
                }
                // A standard block's instance: kept whole when RETAIN, as
                // its state is all it has.
                Decl::Fb(fb) => {
                    if let Some(a) = &d.location {
                        return self.not_yet(a.span, "a function block at an address");
                    }
                    for n in &d.names {
                        let key = n.text.to_ascii_uppercase();
                        let field = format!("V_{rp}{key}");
                        let _ = writeln!(fields, "    iecf_{}_t {field};", fb.stem);
                        let place = Place::Fb(format!("{owner}{field}"), fb.clone());
                        self.init_var(&place, d, "0", init)?;
                        let m = symbols::member(&key, &field, &place, flags);
                        self.global_members.push((format!("{}{key}", rp.replace("__", ".")), m));
                        globals.vars.insert(key, place);
                    }
                    self.globals = globals.vars.clone();
                    continue;
                }
                // An interface: a reference, never in the symbol table
                // (docs/c-output.md §6).
                Decl::Itf(i) => {
                    if is_retain || d.location.is_some() {
                        return self.not_yet(d.span, "a RETAIN interface, or one at an address");
                    }
                    for n in &d.names {
                        let key = n.text.to_ascii_uppercase();
                        let field = format!("V_{rp}{key}");
                        let _ = writeln!(fields, "    iecf_itf_t {field};");
                        let place = Place::Itf(format!("{owner}{field}"), i.clone());
                        self.init_var(&place, d, "0", init)?;
                        globals.vars.insert(key, place);
                    }
                    self.globals = globals.vars.clone();
                    continue;
                }
                _ => return self.not_yet(d.ty.span(), "a global of this type"),
            };
            for n in &d.names {
                let key = n.text.to_ascii_uppercase();
                let place = match &d.location {
                    Some(a) if is_retain => return self.not_yet(a.span, "a RETAIN variable at an address"),
                    Some(a) => self.located(a, ty)?,
                    None => {
                        let field = format!("V_{rp}{key}");
                        let _ = writeln!(fields, "    {} {field};", c_type(ty));
                        Place::Field(format!("{owner}{field}"), ty)
                    }
                };
                self.init_var(&place, d, "0", init)?;
                let m = symbols::member(&key, &format!("V_{rp}{key}"), &place, flags);
                self.global_members.push((format!("{}{key}", rp.replace("__", ".")), m));
                globals.vars.insert(key, place);
            }
        }
        Ok(())
    }

    /// Each instance in VAR_GLOBAL `b` by its place, before any type
    /// is written: a type's body may name it (VAR_EXTERNAL), and its
    /// type may be one (inst.rs).
    fn place_global_instances(&mut self, b: &VarBlock, rp: &str, globals: &mut Scope) {
        let is_retain = b.retain == Some(Retain::Retain);
        for d in &b.decls {
            let TypeSpec::Named(id) = &d.ty else { continue };
            let key = id.text.to_ascii_uppercase();
            if !self.pous.get(&key).is_some_and(|p| matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)) {
                continue;
            }
            let owner = if is_retain || self.type_keeps(&key, 0) { "ctx->retain." } else { "ctx->" };
            for n in &d.names {
                let k = n.text.to_ascii_uppercase();
                globals
                    .vars
                    .insert(k, Place::Block(format!("{owner}V_{rp}{}", n.text.to_ascii_uppercase()), key.clone()));
            }
        }
    }

    /// What a declaration declares, if this slice compiles it.
    pub(crate) fn decl(&mut self, d: &VarDecl) -> R<Decl> {
        if let Some(t) = self.derived(&d.ty)? {
            return Ok(Decl::Data(t));
        }
        self.decl_of(d.span, d.ty.span())
    }

    /// A variable of a derived type `t` in an instance, `{owner}V_{k}`:
    /// its field, its place, its initial value.
    fn data_var(
        &mut self,
        owner: &str,
        k: &str,
        t: &derived::DTy,
        d: &VarDecl,
        fields: &mut String,
        init: &mut String,
    ) -> R<Place> {
        if let Some(a) = &d.location {
            let place = self.located_data(a, t)?;
            if let Place::Data(lv, _) = &place {
                self.init_image(lv, t, d.init.as_ref(), init, a.span)?;
            }
            return Ok(place);
        }
        let _ = writeln!(fields, "    {};", self.declare(t, &format!("V_{k}")));
        let lv = format!("{owner}V_{k}");
        self.init_data(&lv, t, d.init.as_ref(), init)?;
        Ok(Place::Data(Lv::C(lv), t.clone()))
    }

    /// `a AT %QB6 : ARRAY[0..9] OF INT`, a structure, a STRING, an
    /// enumeration or a subrange at an address: from that byte, its parts
    /// back to back (docs/dependencies.md).
    fn located_data(&mut self, a: &DirectAddress, t: &derived::DTy) -> R<Place> {
        let Some((area, size, byte, _)) = names::parse_address(&a.text) else {
            return self.not_yet(a.span, format!("the address {}", a.text));
        };
        let Some(n) = self.packed_size(t).filter(|_| size != 'X') else {
            return self.not_yet(a.span, format!("this type at {}", a.text));
        };
        let Some(end) = u64::from(byte).checked_add(n).and_then(|e| u32::try_from(e).ok()) else {
            return self.not_yet(a.span, format!("this type at {}: past the image's 4 GB", a.text));
        };
        let slot = match area {
            'I' => 0,
            'Q' => 1,
            _ => 2,
        };
        self.extent[slot] = self.extent[slot].max(end);
        Ok(Place::Data(Lv::Image { area, off: format!("{byte}u") }, t.clone()))
    }

    /// The initial value of a variable of a derived type at an address,
    /// built in a C object of its type and copied in, when the
    /// declaration or its type gives one; an input's is not written, the
    /// input image being the platform's (docs/c-output.md §4).
    fn init_image(&mut self, lv: &Lv, t: &derived::DTy, given: Option<&Init>, out: &mut String, at: Span) -> R<()> {
        let Lv::Image { area, .. } = lv else { return Ok(()) };
        if *area == 'I' || !(given.is_some() || self.gives_init(t)) {
            return Ok(());
        }
        let tmp = self.fresh("init");
        let _ = writeln!(out, "    {{\n    {};", self.declare(t, &tmp));
        self.init_data(&tmp, t, given, out)?;
        let copy = self.copy_data(lv, &Lv::C(tmp), t, at)?;
        let _ = writeln!(out, "    {copy}\n    }}");
        Ok(())
    }

    fn decl_of(&mut self, key: Span, at: Span) -> R<Decl> {
        match self.types.decl(key) {
            Some(Ty::Elem(e)) if expr::held(*e) => Ok(Decl::Elem(*e)),
            Some(Ty::Fb(name)) => match StdFb::named(name) {
                Some(fb) => Ok(Decl::Fb(fb)),
                None => {
                    let key = name.to_ascii_uppercase();
                    match self.pous.get(&key) {
                        // One being written: only a pointer to it reaches
                        // here - an in-out of its own type - its instance
                        // in itself is E0011.
                        Some(p)
                            if matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)
                                && self.block_busy.contains(&key) =>
                        {
                            Ok(Decl::Block(key))
                        }
                        Some(p) if matches!(p.kind, PouKind::FunctionBlock | PouKind::Class) => {
                            self.block(&key, at)?;
                            Ok(Decl::Block(key))
                        }
                        _ => self.not_yet(at, format!("an instance of {name}")),
                    }
                }
            },
            Some(Ty::Itf(i)) => {
                self.itfs_used.insert(i.clone());
                Ok(Decl::Itf(i.clone()))
            }
            Some(t) => {
                let t = t.to_string();
                self.not_yet(at, format!("a variable of type {t}"))
            }
            None => self.not_yet(at, "this declaration"),
        }
    }

    /// Code setting a variable to its initial value, into `out`.
    /// `keep`, for a FUNCTION_BLOCK instance: the C of its `B_<NAME>_init`'s
    /// last argument (docs/c-output.md §7).
    fn init_var(&mut self, place: &Place, d: &VarDecl, keep: &str, out: &mut String) -> R<()> {
        let ty = match place {
            // Set up, then given what its initial value lists (inst.rs).
            Place::Fb(lv, fb) => return self.init_instance(lv, &derived::Kind::Std(fb.clone()), d.init.as_ref(), out),
            Place::Data(lv, t) => {
                let (lv, t) = (lv.clone(), t.clone());
                return match &lv {
                    Lv::C(c) => self.init_data(c, &t, d.init.as_ref(), out),
                    Lv::Image { .. } => self.init_image(&lv, &t, d.init.as_ref(), out, d.span),
                    Lv::Ref(_) | Lv::Part { .. } => self.not_yet(d.span, "this declaration"),
                };
            }
            Place::Block(lv, key) => {
                let was = std::mem::replace(&mut self.init_keep, keep.to_string());
                let r = self.init_instance(lv, &derived::Kind::Block(key.clone()), d.init.as_ref(), out);
                self.init_keep = was;
                return r;
            }
            Place::Itf(lv, i) => return self.init_itf(lv, i, d.init.as_ref(), out),
            Place::Field(_, t) | Place::Located { ty: t, .. } => *t,
        };
        let value = match &d.init {
            Some(Init::Expr(e)) => Some(self.init_scalar(e, ty)?),
            Some(_) => return self.not_yet(d.span, "this initial value"),
            None => None,
        };
        match place {
            Place::Located { area, .. } => {
                // An input's initial value is not written: the platform owns
                // the input image (docs/c-output.md §4).
                if let Some(v) = value
                    && *area != 'I'
                {
                    let _ = writeln!(out, "    {}", place.store(&v));
                }
            }
            _ => {
                let v = value.unwrap_or_else(|| expr::zero(ty));
                let _ = writeln!(out, "    {}", place.store(&v));
            }
        }
        Ok(())
    }

    /// `name AT %QW2`: where it is, and the extent it adds. The address
    /// is where it starts; it takes its own type's width (Ed.3 6.5.5.3),
    /// a BOOL anywhere but a bit the whole byte. `size` becomes the
    /// width it is read and written in.
    pub(crate) fn located(&mut self, a: &DirectAddress, ty: Elem) -> R<Place> {
        let Some((area, written, byte, bit)) = names::parse_address(&a.text) else {
            return self.not_yet(a.span, format!("the address {}", a.text));
        };
        let held = expr::held(ty);
        let (size, width) = match ty.bits() {
            Some(1) if written == 'X' => ('X', 1),
            Some(1) => ('B', 1),
            _ if written == 'X' => return self.not_yet(a.span, format!("{} at {}", ty.name(), a.text)),
            Some(8) if held => ('B', 1),
            Some(16) if held => ('W', 2),
            Some(32) if held => ('D', 4),
            Some(64) if held => ('L', 8),
            _ => return self.not_yet(a.span, format!("{} at {}", ty.name(), a.text)),
        };
        let slot = match area {
            'I' => 0,
            'Q' => 1,
            _ => 2,
        };
        self.extent[slot] = self.extent[slot].max(byte + width);
        Ok(Place::Located { area, size, byte, bit, ty })
    }

    /// A PROGRAM: its instance type, and its init and body functions.
    fn program(&mut self, pou: &'a Pou, index: u16, globals: &Scope) -> R<(String, String)> {
        let key = pou.name.text.to_ascii_uppercase();
        let mut scope = Scope::default();
        self.implicit_globals(&key, pou, &mut scope)?;
        let mut fields = String::new();
        let mut init = String::new();
        let mut temps = String::new();
        let mut members = Vec::new();
        // RETAIN variables: a structure of their own in ctx->retain, the
        // instance pointing at its part (docs/c-output.md §7).
        let mut rfields = String::new();
        let mut rinit = String::new();
        let mut params: Vec<(String, Place, Dir)> = Vec::new();
        // R_EDGE and F_EDGE inputs: their triggers, in the instance itself
        // even when the input is RETAIN.
        let mut edge_lines = String::new();
        let mut init_edges = String::new();
        let mut edge_fields = String::new();
        for b in &pou.var_blocks {
            let retain = b.retain == Some(Retain::Retain);
            let dir = match b.kind {
                VarKind::Input => Some(Dir::Input),
                VarKind::Output => Some(Dir::Output),
                _ => None,
            };
            let flags = if b.constant { symbols::CONSTANT } else { 0 }
                | if retain { symbols::RETAIN } else { 0 }
                | symbols::dir_flags(dir);
            match b.kind {
                // A program's inputs and outputs are its variables, which a
                // connection in the CONFIGURATION sets and reads.
                VarKind::Var | VarKind::Input | VarKind::Output => {}
                VarKind::Temp => {
                    self.locals(b, "VAR_TEMP", &mut scope, &mut temps)?;
                    continue;
                }
                VarKind::External => {
                    for d in &b.decls {
                        for n in &d.names {
                            let k = n.text.to_ascii_uppercase();
                            match globals.vars.get(&k) {
                                Some(p) => {
                                    scope.vars.insert(k, p.clone());
                                }
                                None => return self.not_yet(n.span, "this VAR_EXTERNAL"),
                            }
                        }
                    }
                    continue;
                }
                // An in-out: a pointer the CONFIGURATION's connection binds
                // to a global before each run (connections), its value read
                // and written through it, as a FUNCTION_BLOCK's.
                VarKind::InOut => {
                    if b.retain.is_some() {
                        return self.not_yet(b.span, "RETAIN or NON_RETAIN on a PROGRAM's in-outs");
                    }
                    for d in &b.decls {
                        if d.init.is_some() || d.location.is_some() {
                            return self.not_yet(d.span, "an initial value or address on a VAR_IN_OUT");
                        }
                        let decl = self.decl(d)?;
                        for n in &d.names {
                            let k = n.text.to_ascii_uppercase();
                            let at = format!("(*self->V_{k})");
                            // Of any type: a pointer to the variable it is
                            // bound to, of that type.
                            let (field, place) = match &decl {
                                Decl::Elem(ty) => (format!("{} *V_{k}", c_type(*ty)), Place::Field(at, *ty)),
                                Decl::Data(t) if !matches!(t.kind, derived::Kind::Flex(..) | derived::Kind::Itf(_)) => {
                                    let t = derived::DTy { init: None, ..t.clone() };
                                    (self.declare(&t, &format!("(*V_{k})")), Place::Data(Lv::C(at), t))
                                }
                                Decl::Block(inner) => {
                                    scope.poly.insert(k.clone());
                                    (format!("B_{inner}_t *V_{k}"), Place::Block(at, inner.clone()))
                                }
                                Decl::Fb(fb) => (format!("iecf_{}_t *V_{k}", fb.stem), Place::Fb(at, fb.clone())),
                                _ => return self.not_yet(d.span, "a PROGRAM in-out of this type"),
                            };
                            let _ = writeln!(fields, "    {field};");
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            params.push((k.clone(), place.clone(), Dir::InOut));
                            scope.vars.insert(k, place);
                        }
                    }
                    continue;
                }
                _ => return self.not_yet(b.span, "this block in a PROGRAM"),
            }
            let owner = if retain { "self->retain->" } else { "self->" };
            for d in &b.decls {
                let decl = self.decl(d)?;
                if d.names.is_empty() {
                    let mut out = std::mem::take(if retain { &mut rinit } else { &mut init });
                    let r = self.unnamed_located(d, &decl, owner, &mut fields, &mut out);
                    *(if retain { &mut rinit } else { &mut init }) = out;
                    r?;
                    continue;
                }
                for n in &d.names {
                    let k = n.text.to_ascii_uppercase();
                    // A FUNCTION_BLOCK instance is kept whole when it is
                    // RETAIN or keeps something of its own, and set at every
                    // start, `keep` saying what it keeps (docs/c-output.md §7).
                    if let (Decl::Block(key), None) = (&decl, &d.location) {
                        let whole = self.retentive.contains(key);
                        let (f, owner) =
                            if retain || whole { (&mut rfields, "self->retain->") } else { (&mut fields, "self->") };
                        let _ = writeln!(f, "    B_{key}_t V_{k};");
                        let place = Place::Block(format!("{owner}V_{k}"), key.clone());
                        let keep = if retain { "warm ? 2 : 0" } else { "warm" };
                        self.init_var(&place, d, keep, &mut init)?;
                        let flags = flags | if whole { symbols::RETAIN } else { 0 };
                        members.push(symbols::member(&k, &format!("V_{k}"), &place, flags));
                        // An input or output, connected as any is.
                        if let Some(dir) = dir {
                            params.push((k.clone(), place.clone(), dir));
                        }
                        scope.vars.insert(k, place);
                        continue;
                    }
                    // An interface: a reference, never kept (E0027), never
                    // a symbol (docs/c-output.md §6). An input or output
                    // is connected as an elementary one is (`connections`).
                    if let Decl::Itf(i) = &decl {
                        if retain || d.location.is_some() {
                            return self.not_yet(d.span, "a RETAIN interface, or one at an address");
                        }
                        let _ = writeln!(fields, "    iecf_itf_t V_{k};");
                        let place = Place::Itf(format!("self->V_{k}"), i.clone());
                        self.init_var(&place, d, "0", &mut init)?;
                        if let Some(dir) = dir {
                            params.push((k.clone(), place.clone(), dir));
                        }
                        scope.vars.insert(k, place);
                        continue;
                    }
                    // An array or structure of instances: as an instance
                    // is, set at every start, kept whole when it keeps
                    // something; its other parts kept when RETAIN (inst.rs).
                    if let (Decl::Data(t), None) = (&decl, &d.location)
                        && self.has_instance(t)
                    {
                        let whole = retain || self.keeps_in(t);
                        let (f, owner) = if whole { (&mut rfields, "self->retain->") } else { (&mut fields, "self->") };
                        let _ = writeln!(f, "    {};", self.declare(t, &format!("V_{k}")));
                        let lv = format!("{owner}V_{k}");
                        self.init_keep = if retain { "warm ? 2 : 0" } else { "warm" }.into();
                        self.init_guard = retain.then(|| "!warm".into());
                        let r = self.init_data(&lv, t, d.init.as_ref(), &mut init);
                        self.init_keep = "0".into();
                        self.init_guard = None;
                        r?;
                        let place = Place::Data(Lv::C(lv), t.clone());
                        let flags = flags | if whole { symbols::RETAIN } else { 0 };
                        members.push(symbols::member(&k, &format!("V_{k}"), &place, flags));
                        // An input or output, connected as any is.
                        if let Some(dir) = dir {
                            params.push((k.clone(), place.clone(), dir));
                        }
                        scope.vars.insert(k, place);
                        continue;
                    }
                    let (fields, init) = if retain { (&mut rfields, &mut rinit) } else { (&mut fields, &mut init) };
                    let place = match (&decl, &d.location) {
                        (_, Some(a)) if retain => {
                            return self.not_yet(a.span, "a RETAIN variable at an address");
                        }
                        (Decl::Data(t), _) => {
                            let p = self.data_var(owner, &k, t, d, fields, init)?;
                            members.push(symbols::member(&k, &format!("V_{k}"), &p, flags));
                            if let Some(dir) = dir {
                                params.push((k.clone(), p.clone(), dir));
                            }
                            scope.vars.insert(k, p);
                            continue;
                        }
                        (Decl::Elem(ty), Some(a)) => self.located(a, *ty)?,
                        (Decl::Elem(ty), None) => {
                            let _ = writeln!(fields, "    {} V_{k};", c_type(*ty));
                            Place::Field(format!("{owner}V_{k}"), *ty)
                        }
                        (Decl::Fb(fb), None) => {
                            let _ = writeln!(fields, "    iecf_{}_t V_{k};", fb.stem);
                            Place::Fb(format!("{owner}V_{k}"), fb.clone())
                        }
                        (Decl::Block(_), None) => unreachable!("a FUNCTION_BLOCK instance is above"),
                        (Decl::Itf(_), _) => unreachable!("an interface is above"),
                        (Decl::Fb(_) | Decl::Block(_), Some(a)) => {
                            return self.not_yet(a.span, "a function block at an address");
                        }
                    };
                    self.init_var(&place, d, "0", init)?;
                    members.push(symbols::member(&k, &format!("V_{k}"), &place, flags));
                    if let Some(dir) = dir {
                        params.push((k.clone(), place.clone(), dir));
                    }
                    if let (Some(edge), Some(raw)) = (d.edge, place.lvalue()) {
                        let raw = raw.to_string();
                        self.edge_input(
                            &k,
                            edge,
                            &raw,
                            (&mut edge_fields, &mut init_edges),
                            &mut edge_lines,
                            &mut scope,
                        );
                        continue;
                    }
                    scope.vars.insert(k, place);
                }
            }
        }
        fields.push_str(&edge_fields);
        self.pou_members.insert(key.clone(), members);
        self.program_params.insert(key.clone(), params);
        let chart = self.chart(pou, "self->", None, &mut fields, &mut init, &mut scope)?;
        let has_retain = !rfields.is_empty();
        if has_retain {
            self.pou_retain.insert(key.clone());
        }
        // Globals the program does not name stay out of its scope: only
        // VAR_EXTERNAL brings one in (Ed.3 6.5.2.2).
        let mut t = String::new();
        if has_retain {
            let _ = writeln!(t, "typedef struct {{\n{rfields}}} P_{key}_retain_t;\n");
            let _ = writeln!(fields, "    P_{key}_retain_t *retain; /* its part of ctx->retain */");
        }
        let _ = writeln!(t, "typedef struct {{");
        if fields.is_empty() {
            let _ = writeln!(t, "    char none_; /* C has no empty structure */");
        }
        t.push_str(&fields);
        let _ = writeln!(t, "}} P_{key}_t;\n");

        self.fill_ref_inits(&mut init, &scope)?;
        self.fill_ref_inits(&mut rinit, &scope)?;
        self.fill_ref_inits(&mut temps, &scope)?;
        let p = self.prefix.clone();
        let mut c = String::new();
        let _ = writeln!(c, "/* PROGRAM {} */", pou.name.text);
        let _ = writeln!(c, "static void P_{key}_init({p}_t *ctx, P_{key}_t *self, int warm) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;\n    (void)warm;");
        c.push_str(&init);
        c.push_str(&init_edges);
        let _ = writeln!(c, "}}\n");
        if has_retain {
            // At a cold start only: a warm one keeps them.
            let _ = writeln!(c, "static void P_{key}_init_retain({p}_t *ctx, P_{key}_t *self) {{");
            let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
            c.push_str(&rinit);
            let _ = writeln!(c, "}}\n");
        }
        let mut body = edge_lines;
        let mut cx = stmt::Body {
            pou: index,
            scope: &scope,
            out: &mut body,
            depth: 1,
            loops: Vec::new(),
            ret: "return;".into(),
            on_return: None,
        };
        match &chart {
            Some(ch) => self.chart_scan(ch, &mut cx)?,
            None => self.stmts(&pou.body, &mut cx)?,
        }
        let _ = writeln!(c, "static void P_{key}({p}_t *ctx, P_{key}_t *self) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
        c.push_str(&temps);
        c.push_str(&body);
        let _ = writeln!(c, "}}\n");
        Ok((t, c))
    }

    /// The variables of `b` as C locals of the body, `V_<NAME>`, set to
    /// their initial values at each call: a FUNCTION's (Ed.3 6.6.2.2) and
    /// VAR_TEMP (Ed.3 6.5.2.2).
    fn locals(&mut self, b: &VarBlock, what: &str, scope: &mut Scope, out: &mut String) -> R<()> {
        for d in &b.decls {
            if d.location.is_some() {
                return self.not_yet(d.span, format!("an address in {what}"));
            }
            let ty = match self.decl(d)? {
                Decl::Elem(ty) => ty,
                // An instance, or what holds one, is set up at each call
                // as any variable here is: it keeps nothing (Ed.3
                // 6.6.2.2 rule 10).
                Decl::Data(t) => {
                    for n in &d.names {
                        let k = n.text.to_ascii_uppercase();
                        let _ = writeln!(out, "    {};\n    (void)V_{k};", self.declare(&t, &format!("V_{k}")));
                        self.init_data(&format!("V_{k}"), &t, d.init.as_ref(), out)?;
                        scope.vars.insert(k.clone(), Place::Data(Lv::C(format!("V_{k}")), t.clone()));
                    }
                    continue;
                }
                Decl::Itf(i) => {
                    for n in &d.names {
                        let k = n.text.to_ascii_uppercase();
                        let place = Place::Itf(format!("V_{k}"), i.clone());
                        let _ = writeln!(out, "    iecf_itf_t V_{k};\n    (void)V_{k};");
                        self.init_var(&place, d, "0", out)?;
                        scope.vars.insert(k, place);
                    }
                    continue;
                }
                decl @ (Decl::Block(_) | Decl::Fb(_)) => {
                    for n in &d.names {
                        let k = n.text.to_ascii_uppercase();
                        let (c, place) = match &decl {
                            Decl::Block(b) => (format!("B_{b}_t"), Place::Block(format!("V_{k}"), b.clone())),
                            Decl::Fb(fb) => (format!("iecf_{}_t", fb.stem), Place::Fb(format!("V_{k}"), fb.clone())),
                            _ => unreachable!("matched above"),
                        };
                        let _ = writeln!(out, "    {c} V_{k};\n    (void)V_{k};");
                        self.init_var(&place, d, "0", out)?;
                        scope.vars.insert(k, place);
                    }
                    continue;
                }
            };
            let value = match &d.init {
                Some(Init::Expr(e)) => self.init_value(e, ty)?,
                Some(_) => return self.not_yet(d.span, "this initial value"),
                None => expr::zero(ty),
            };
            for n in &d.names {
                let k = n.text.to_ascii_uppercase();
                let _ = writeln!(out, "    {} V_{k} = {value};\n    (void)V_{k};", c_type(ty));
                scope.vars.insert(k.clone(), Place::Field(format!("V_{k}"), ty));
            }
        }
        Ok(())
    }

    /// The interface of the user's FUNCTION `key`, worked out once.
    pub(crate) fn fn_sig(&mut self, key: &str, at: Span) -> R<FnSig> {
        self.fn_sig_q(key, at, true)
    }

    /// `queue`: to be written too; an ABSTRACT method's, which a table's
    /// slot takes the type of, is not (oo.rs).
    pub(crate) fn fn_sig_q(&mut self, key: &str, at: Span, queue: bool) -> R<FnSig> {
        if let Some(s) = self.functions.get(key) {
            let s = s.clone();
            if queue && self.fn_queued.insert(key.to_string()) {
                self.fn_queue.push(key.to_string());
            }
            return Ok(s);
        }
        // An input's initial value may name the function's constants.
        self.const_ctx.push(key.to_string());
        let r = self.fn_sig_made(key, at, queue);
        self.const_ctx.pop();
        r
    }

    fn fn_sig_made(&mut self, key: &str, at: Span, queue: bool) -> R<FnSig> {
        let Some(pou) = self.pous.get(key).copied() else {
            return self.not_yet(at, "a call of something that is not a FUNCTION");
        };
        if !matches!(pou.kind, PouKind::Function | PouKind::Method) {
            return self.not_yet(at, "a call of a user's FUNCTION_BLOCK");
        }
        // A method's type, its variables known before the method is written.
        let this = (pou.kind == PouKind::Method).then(|| key.rsplit_once("__").map_or("", |(t, _)| t).to_string());
        // An interface's prototype has no type to write (itf.rs).
        if let Some(t) = &this
            && !self.block_scopes.contains_key(t)
            && self.pous.get(t.as_str()).is_some_and(|p| p.kind != PouKind::Interface)
        {
            self.block(t, at)?;
        }
        let c_name = if this.is_some() { format!("M_{key}") } else { format!("F_{key}") };
        let mut ret_data = None;
        let mut ret_init = None;
        let mut ret_range = None;
        let mut ranges = HashMap::new();
        let ret = match &pou.return_type {
            Some(rt) => match self.derived(rt)? {
                // A reference too, an iecf_ref_t in the caller's buffer.
                Some(t) if self.passes(&t) || matches!(t.kind, derived::Kind::Ref(..)) => {
                    ret_data = Some(t);
                    None
                }
                // An enumeration, as the integer it is held in.
                Some(t) if matches!(t.kind, derived::Kind::Enum(..)) => {
                    ret_init = Some(self.enum_initial(&t, None)?);
                    t.scalar()
                }
                // A subrange, as the integer it is held in: it starts at
                // its type's initial value, else its lower limit.
                Some(t) if matches!(t.kind, derived::Kind::Elem(_)) => {
                    ret_init = Some(self.scalar_initial(&t, None, rt.span())?);
                    ret_range = t.range;
                    t.scalar()
                }
                Some(_) => return self.not_yet(rt.span(), "this result type"),
                None => match self.decl_of(rt.span(), rt.span())? {
                    Decl::Elem(e) => Some(e),
                    // An interface, as params.rs passes a structure.
                    Decl::Itf(i) => {
                        ret_data = Some(derived::DTy::itf(&i));
                        None
                    }
                    // An instance, the function's own, copied to the
                    // caller as a structure is.
                    Decl::Block(k) => {
                        self.block(&k, rt.span())?;
                        ret_data = Some(derived::DTy { kind: derived::Kind::Block(k), init: None, range: None });
                        None
                    }
                    Decl::Fb(fb) => {
                        ret_data = Some(derived::DTy { kind: derived::Kind::Std(fb), init: None, range: None });
                        None
                    }
                    Decl::Data(_) => return self.not_yet(rt.span(), "this result type"),
                },
            },
            None => None,
        };
        let mut data = HashMap::new();
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut externals = Vec::new();
        let mut in_blocks = HashMap::new();
        let mut in_itfs = HashMap::new();
        for b in &pou.var_blocks {
            match b.kind {
                VarKind::Input | VarKind::Output | VarKind::InOut => {}
                VarKind::Var | VarKind::Temp => continue,
                // A global, reached as every POU reaches it (`ctx->`).
                VarKind::External => {
                    for n in b.decls.iter().flat_map(|d| &d.names) {
                        let k = n.text.to_ascii_uppercase();
                        if !self.globals.contains_key(&k) {
                            return self.not_yet(n.span, "this VAR_EXTERNAL");
                        }
                        externals.push(k);
                    }
                    continue;
                }
                _ => return self.not_yet(b.span, "this block of a FUNCTION"),
            }
            for d in &b.decls {
                if d.edge.is_some() {
                    return self.not_yet(d.span, "R_EDGE and F_EDGE");
                }
                let mut initial = None;
                // An instance given as an input is a copy of it, read in
                // the body (Ed.3 6.6.1.4 rule 4; docs/dependencies.md);
                // an output one is the function's own, copied out: each
                // passed as a structure is (params.rs).
                let decl = match self.decl(d)? {
                    Decl::Block(k) if b.kind != VarKind::InOut => {
                        self.block(&k, d.ty.span())?;
                        Decl::Data(derived::DTy { kind: derived::Kind::Block(k), init: None, range: None })
                    }
                    // A standard block's instance, an in-out too: by its
                    // address, as a structure is.
                    Decl::Fb(fb) => Decl::Data(derived::DTy { kind: derived::Kind::Std(fb), init: None, range: None }),
                    decl => decl,
                };
                let instance = matches!(&decl, Decl::Data(t) if matches!(t.kind, derived::Kind::Std(_))
                    || (b.kind != VarKind::InOut && matches!(t.kind, derived::Kind::Block(_))));
                let ty = match decl {
                    Decl::Elem(ty) => ty,
                    // An enumeration, as the integer it is held in: not
                    // given, its type's initial value.
                    Decl::Data(t) if matches!(t.kind, derived::Kind::Enum(..)) => {
                        initial = Some(self.enum_initial(&t, d.init.as_ref())?);
                        match t.scalar() {
                            Some(e) => e,
                            None => return self.not_yet(d.span, "this declaration"),
                        }
                    }
                    // A subrange, as the integer it is held in: not
                    // given, its initial value, else its lower limit.
                    Decl::Data(t) if matches!(t.kind, derived::Kind::Elem(_)) => {
                        if b.kind != VarKind::InOut {
                            initial = Some(self.scalar_initial(&t, d.init.as_ref(), d.span)?);
                        }
                        if let Some(r) = t.range {
                            for n in &d.names {
                                ranges.insert(n.text.to_ascii_uppercase(), r);
                            }
                        }
                        match t.scalar() {
                            Some(e) => e,
                            None => return self.not_yet(d.span, "this declaration"),
                        }
                    }
                    // A STRING, an array or a structure (params.rs), its
                    // initial value with it.
                    Decl::Data(t) if self.passes(&t) || instance => {
                        let t = derived::DTy { init: d.init.clone().or(t.init), ..t };
                        for n in &d.names {
                            let name = n.text.to_ascii_uppercase();
                            data.insert(name.clone(), t.clone());
                            match b.kind {
                                VarKind::Input => inputs.push((name, Elem::Bool, Some("NULL".to_string()))),
                                VarKind::InOut => inputs.push((name, Elem::Bool, None)),
                                _ => outputs.push((name, Elem::Bool, String::new())),
                            }
                        }
                        continue;
                    }
                    // An interface input, NULL when not given (itf.rs).
                    Decl::Itf(i) if b.kind == VarKind::Input && d.init.is_none() => {
                        for n in &d.names {
                            let name = n.text.to_ascii_uppercase();
                            in_itfs.insert(name.clone(), i.clone());
                            inputs.push((name, Elem::Bool, Some("iecf_itf_null()".to_string())));
                        }
                        continue;
                    }
                    // An output, or an input with an initial value: as
                    // params.rs passes a structure.
                    Decl::Itf(i) if b.kind != VarKind::InOut => {
                        let t = derived::DTy { init: d.init.clone(), ..derived::DTy::itf(&i) };
                        for n in &d.names {
                            let name = n.text.to_ascii_uppercase();
                            data.insert(name.clone(), t.clone());
                            match b.kind {
                                VarKind::Input => inputs.push((name, Elem::Bool, Some("NULL".to_string()))),
                                _ => outputs.push((name, Elem::Bool, String::new())),
                            }
                        }
                        continue;
                    }
                    // An instance, passed by its address (OO.md D11).
                    Decl::Block(k) if b.kind == VarKind::InOut => {
                        for n in &d.names {
                            let name = n.text.to_ascii_uppercase();
                            in_blocks.insert(name.clone(), k.clone());
                            inputs.push((name, Elem::Bool, None));
                        }
                        continue;
                    }
                    _ => {
                        return self.not_yet(
                            d.span,
                            "a FUNCTION input, output or in-out of this type (elementary types only, for now)",
                        );
                    }
                };
                let default = match (&d.init, initial) {
                    (_, Some(v)) => v,
                    (Some(Init::Expr(e)), _) => self.init_scalar(e, ty)?,
                    (Some(_), _) => return self.not_yet(d.span, "this initial value"),
                    (None, _) => expr::zero(ty),
                };
                for n in &d.names {
                    let name = n.text.to_ascii_uppercase();
                    match b.kind {
                        VarKind::Input => inputs.push((name, ty, Some(default.clone()))),
                        VarKind::InOut => inputs.push((name, ty, None)),
                        _ => outputs.push((name, ty, default.clone())),
                    }
                }
            }
        }
        let sig = FnSig {
            ret_init,
            c_name,
            this,
            ret,
            inputs,
            outputs,
            externals,
            eno: self.types.implicit_eno(key),
            in_blocks,
            in_itfs,
            data,
            ret_data,
            ranges,
            ret_range,
        };
        self.functions.insert(key.to_string(), sig.clone());
        if queue {
            self.fn_queued.insert(key.to_string());
            self.fn_queue.push(key.to_string());
        }
        Ok(sig)
    }

    /// The globals `pou` uses without declaring them (the
    /// implicit-externals extension, docs/extensions.md), into its scope
    /// as a VAR_EXTERNAL would put them.
    fn implicit_globals(&mut self, key: &str, pou: &Pou, scope: &mut Scope) -> R<()> {
        for (k, _) in self.types.implicit_externals(key) {
            match self.globals.get(k) {
                Some(p) => {
                    scope.vars.insert(k.clone(), p.clone());
                }
                None => return self.not_yet(pou.name.span, format!("the global {k} used without VAR_EXTERNAL here")),
            }
        }
        Ok(())
    }

    /// A user's FUNCTION: its prototype, and its definition. No memory
    /// from call to call (Ed.3 6.6.2.2): its variables are C locals,
    /// initialised at each call.
    fn function(&mut self, key: &str) -> R<(String, String)> {
        let pou = self.pous[key];
        let sig = self.functions[key].clone();
        if !pou.sfc.is_empty() {
            return self.not_yet(pou.name.span, "SFC");
        }
        let index = self.pou_names.len() as u16;
        self.pou_names.push(key.to_string());
        let p = self.prefix.clone();
        let rt = sig.c_ret();
        let mut params: Vec<String> = sig_params(&sig, sig.eno).into_iter().map(|(t, n)| format!("{t}{n}")).collect();
        // A method: its instance first, its type's variables in its scope.
        let mut scope = Scope::default();
        if let Some(t) = &sig.this {
            params.insert(0, format!("B_{t}_t *self"));
            scope = self.block_scopes.get(t).cloned().unwrap_or_default();
            scope.vars.remove("ENO");
        }
        let head = format!(
            "static {rt} {}({p}_t *ctx{}{})",
            sig.c_name,
            if params.is_empty() { "" } else { ", " },
            params.join(", ")
        );

        let mut decls = String::new();
        if let Some(r) = sig.ret {
            let _ = writeln!(decls, "    {rt} V_{key} = {};", sig.ret_init.clone().unwrap_or_else(|| expr::zero(r)));
        }
        let _ = writeln!(decls, "    (void)ctx;");
        if sig.this.is_some() {
            let _ = writeln!(decls, "    (void)self;");
        }
        if sig.eno {
            let _ = writeln!(decls, "    *eno = 1;");
            scope.vars.insert("ENO".into(), Place::Field("(*eno)".into(), Elem::Bool));
        }
        if let Some(r) = sig.ret {
            // The result is its name's: a method's its own short name.
            scope.vars.insert(pou.name.text.to_ascii_uppercase(), ranged(format!("V_{key}"), r, sig.ret_range));
        }
        // A STRING, array or structure result: the caller's buffer, set
        // to its type's initial value first (params.rs).
        if let Some(t) = &sig.ret_data {
            let lv = self.at_ptr(t, "R_");
            self.init_data(&lv, t, None, &mut decls)?;
            scope.vars.insert(pou.name.text.to_ascii_uppercase(), data_place(lv, t.clone()));
        }
        for (n, t, d) in &sig.inputs {
            if let Some(dt) = sig.data.get(n).cloned() {
                // A STRING cut as it is copied in is flagged where the
                // input is declared.
                let line = pou
                    .var_blocks
                    .iter()
                    .flat_map(|b| &b.decls)
                    .find(|d| d.names.iter().any(|x| x.text.eq_ignore_ascii_case(n)))
                    .map_or(0, |d| self.line(d.span));
                let lv = match (&dt.kind, d) {
                    // An ARRAY[*] is the caller's array, an input of it
                    // too (flex.rs): never written here, the checker saw.
                    (derived::Kind::Flex(..), Some(_)) => {
                        let _ = writeln!(decls, "    (void)P_{n};");
                        flex::flex_at(&format!("P_{n}"))
                    }
                    (derived::Kind::Flex(..), None) => flex::flex_at(&format!("V_{n}")),
                    (_, Some(_)) => self.data_input(n, &dt, (index, line), &mut decls)?,
                    (_, None) => self.at_ptr(&dt, &format!("V_{n}")),
                };
                if !matches!((&dt.kind, d), (derived::Kind::Flex(..), Some(_))) {
                    let _ = writeln!(decls, "    (void)V_{n};");
                }
                scope.vars.insert(n.clone(), data_place(lv, dt));
                continue;
            }
            let _ = writeln!(decls, "    (void)V_{n};");
            if let Some(k) = sig.in_blocks.get(n) {
                // An instance, of its type or a derived one (Ed.3 6.6.8).
                scope.vars.insert(n.clone(), Place::Block(format!("(*V_{n})"), k.clone()));
                scope.poly.insert(n.clone());
                continue;
            }
            if let Some(i) = sig.in_itfs.get(n) {
                scope.vars.insert(n.clone(), Place::Itf(format!("V_{n}"), i.clone()));
                continue;
            }
            // An in-out read and written through its pointer.
            let lv = if d.is_some() { format!("V_{n}") } else { format!("(*V_{n})") };
            // An input of a subrange: what it is given is clamped and
            // flagged where the input is declared, as a STRING cut is.
            if let (Some(_), Some(r)) = (d, sig.ranges.get(n).copied()) {
                let line = pou
                    .var_blocks
                    .iter()
                    .flat_map(|b| &b.decls)
                    .find(|d| d.names.iter().any(|x| x.text.eq_ignore_ascii_case(n)))
                    .map_or(0, |d| self.line(d.span));
                let was = self.may_flag;
                let v = self.clamp(&lv, *t, Some(r));
                self.may_flag = was;
                let _ = writeln!(decls, "    {lv} = {v};\n    iecf_noted(&ctx->flag, {index}u, {line}u);");
            }
            scope.vars.insert(n.clone(), ranged(lv, *t, sig.ranges.get(n).copied()));
        }
        for k in &sig.externals {
            scope.vars.insert(k.clone(), self.globals[k].clone());
        }
        self.implicit_globals(key, pou, &mut scope)?;
        // An output is a local, its initial value at each call (Ed.3
        // 6.6.2.2: no memory), given back as the function returns.
        let mut give = String::new();
        for (n, t, init) in &sig.outputs {
            // An ARRAY[*] output is the caller's array (flex.rs), each
            // element set to its initial value as the call starts.
            if let Some(dt @ derived::DTy { kind: derived::Kind::Flex(..), .. }) = sig.data.get(n).cloned() {
                let d = flex::flex_at(&format!("O_{n}"));
                self.flex_init(&d, &dt, &mut decls)?;
                scope.vars.insert(n.clone(), Place::Data(Lv::C(d), dt));
                continue;
            }
            if let Some(dt) = sig.data.get(n).cloned() {
                let _ = writeln!(decls, "    {};", self.declare(&dt, &format!("V_{n}")));
                self.init_data(&format!("V_{n}"), &dt, None, &mut decls)?;
                scope.vars.insert(n.clone(), data_place(format!("V_{n}"), dt));
                let _ = write!(give, "if (O_{n}) iecf_bytes(O_{n}, &V_{n}, sizeof V_{n}); ");
                continue;
            }
            let _ = writeln!(decls, "    {} V_{n} = {init};", c_type(*t));
            scope.vars.insert(n.clone(), ranged(format!("V_{n}"), *t, sig.ranges.get(n).copied()));
            let _ = write!(give, "if (O_{n}) *O_{n} = V_{n}; ");
        }
        for b in pou.var_blocks.iter().filter(|b| matches!(b.kind, VarKind::Var | VarKind::Temp)) {
            self.locals(b, "a FUNCTION", &mut scope, &mut decls)?;
        }
        let mut body = String::new();
        // A fault leaves by `ret`, the outputs not given back: the
        // caller's statement stops before it stores anything.
        let ret = match (&sig.ret, &sig.ret_data) {
            (Some(_), _) => format!("return V_{key};"),
            (None, Some(_)) if sig.gives() => format!("return ({})R_;", sig.c_ret()),
            _ => "return;".to_string(),
        };
        let done = if give.is_empty() { ret.clone() } else { format!("{{ {give}{ret} }}") };
        let mut cx = stmt::Body {
            pou: index,
            scope: &scope,
            out: &mut body,
            depth: 1,
            loops: Vec::new(),
            ret: ret.clone(),
            on_return: (!give.is_empty()).then(|| done.clone()),
        };
        self.stmts(&pou.body, &mut cx)?;
        self.fill_ref_inits(&mut decls, &scope)?;
        let mut c = String::new();
        match &sig.this {
            Some(t) => {
                let _ = writeln!(c, "/* METHOD {}.{} */", iecf_sema::namespaces::display(t), pou.name.text);
            }
            None => {
                let _ = writeln!(c, "/* FUNCTION {} */", pou.name.text);
            }
        }
        let _ = writeln!(c, "{head} {{");
        c.push_str(&decls);
        c.push_str(&body);
        let _ = writeln!(c, "    {done}\n}}\n");
        Ok((format!("{head};\n"), c))
    }
}

impl Gen<'_> {
    /// The user's FUNCTION_BLOCK `key`: its type into the header, after
    /// those of the blocks it contains, and its init and body functions.
    pub(crate) fn block(&mut self, key: &str, at: Span) -> R<()> {
        if self.blocks.contains_key(key) {
            return Ok(());
        }
        // A type first met inside an expression - a call of a FUNCTION
        // that takes or gives one - is written now: what the expression
        // has gathered so far is kept aside, and is as it was after.
        let str_temps = std::mem::take(&mut self.str_temps);
        let call_bufs = std::mem::take(&mut self.call_bufs);
        let sel_bufs = std::mem::take(&mut self.sel_bufs);
        let subst = std::mem::take(&mut self.subst);
        let gated_stores = std::mem::take(&mut self.gated_stores);
        let in_out_backs = std::mem::take(&mut self.in_out_backs);
        let split_lines = self.split_lines.take();
        let want_eno = self.want_eno.take();
        let keep = self.keep.take();
        let flags = (
            self.pre_ok,
            self.gating,
            self.seq,
            self.split_ok,
            self.eno_used,
            self.may_fault,
            self.may_flag,
            self.keep_ok,
        );
        self.const_ctx.push(key.to_string());
        let r = self.block_written(key, at);
        self.const_ctx.pop();
        self.str_temps = str_temps;
        self.call_bufs = call_bufs;
        self.sel_bufs = sel_bufs;
        self.subst = subst;
        self.gated_stores = gated_stores;
        self.in_out_backs = in_out_backs;
        self.split_lines = split_lines;
        self.want_eno = want_eno;
        self.keep = keep;
        (
            self.pre_ok,
            self.gating,
            self.seq,
            self.split_ok,
            self.eno_used,
            self.may_fault,
            self.may_flag,
            self.keep_ok,
        ) = flags;
        r
    }

    fn block_written(&mut self, key: &str, at: Span) -> R<()> {
        if self.block_busy.iter().any(|k| k == key) {
            return self.not_yet(at, "a FUNCTION_BLOCK that contains itself");
        }
        let pou = self.pous[key];
        // Its base first: the derived structure starts with it.
        let base = self.types.base(key).map(str::to_string);
        if let Some(b) = &base {
            self.block(b, at)?;
        }
        self.block_busy.push(key.to_string());
        let mut scope = Scope::default();
        self.implicit_globals(key, pou, &mut scope)?;
        let mut fields = String::new();
        let mut init = String::new();
        let mut params = Vec::new();
        let mut ranges = HashMap::new();
        let mut data = Vec::new();
        let mut blocks = Vec::new();
        let mut itfs = Vec::new();
        let mut in_data = Vec::new();
        let mut paths = HashMap::new();
        let mut temps = String::new();
        let mut members = Vec::new();
        let mut retentive = false;
        // R_EDGE and F_EDGE inputs: their triggers' calls, and names.
        let mut edge_lines = String::new();
        let mut edged: Vec<String> = Vec::new();
        // The instance knows its type, for the whole run (OO.md §4): a
        // pointer first in the structure of a hierarchy's root.
        if self.carries_type(key) && base.is_none() {
            let _ = writeln!(fields, "    const iecf_otype_t *type;");
        }
        if let Some(b) = &base {
            // A derived type's structure starts with its base's (C99
            // 6.7.2.1: a pointer to it is one to its base); what it
            // inherits is reached there (Ed.3 6.6.5.5.2).
            let _ = writeln!(fields, "    B_{b}_t base;");
            let _ = writeln!(init, "    B_{b}_init(ctx, &self->base, keep);");
            let inherited = self.inherit_scopes.get(b).cloned().unwrap_or_default();
            for (n, p) in &inherited.vars {
                scope.vars.insert(n.clone(), p.in_derived());
            }
            scope.poly.extend(inherited.poly.iter().cloned());
            let bs = self.blocks[b].clone();
            for n in bs
                .params
                .iter()
                .map(|p| &p.0)
                .chain(bs.data.iter().map(|d| &d.0))
                .chain(bs.blocks.iter().map(|d| &d.0))
                .chain(bs.itfs.iter().map(|d| &d.0))
                .chain(bs.in_data.iter().map(|d| &d.0))
            {
                paths.insert(n.clone(), format!("base.{}", bs.field(n)));
            }
            params.extend(bs.params);
            data.extend(bs.data);
            blocks.extend(bs.blocks);
            itfs.extend(bs.itfs);
            in_data.extend(bs.in_data);
            ranges.extend(bs.ranges);
            for m in self.block_members.get(b).cloned().unwrap_or_default() {
                members.push(match m.at {
                    symbols::At::Field(f) => symbols::Member { at: symbols::At::Field(format!("base.{f}")), ..m },
                    _ => m,
                });
            }
            retentive |= self.retentive.contains(b);
        }
        if self.carries_type(key) {
            let _ = writeln!(init, "    self->{} = &TY_{key};", self.type_field(key));
        }
        for b in &pou.var_blocks {
            let flags = if b.constant { symbols::CONSTANT } else { 0 }
                | if b.retain == Some(Retain::Retain) { symbols::RETAIN } else { 0 };
            retentive |= b.retain == Some(Retain::Retain);
            // Ed.3 6.5.6: a RETAIN variable is set at a cold start only, a
            // NON_RETAIN one at every start, one that is neither unless the
            // instance is RETAIN; `keep` is 0 cold, 1 warm, 2 warm and the
            // instance RETAIN (docs/c-output.md §7).
            let guard = match b.retain {
                Some(Retain::Retain) => Some("keep == 0"),
                Some(Retain::NonRetain) => None,
                None => Some("keep != 2"),
            };
            let mut sec = String::new();
            // A PUBLIC (or INTERNAL) variable is read and written from
            // outside as an input is (Ed.3 6.6.5.2 rule 7); the checker
            // said where.
            let public = b.kind == VarKind::Var
                && matches!(b.access, Some((iecf_syntax::ast::Access::Public | iecf_syntax::ast::Access::Internal, _)));
            let dir = match b.kind {
                VarKind::Input => Some(Dir::Input),
                VarKind::Output => Some(Dir::Output),
                VarKind::InOut => Some(Dir::InOut),
                VarKind::Var => None,
                VarKind::Temp => {
                    self.locals(b, "VAR_TEMP", &mut scope, &mut temps)?;
                    continue;
                }
                VarKind::External => {
                    for d in &b.decls {
                        for n in &d.names {
                            let k = n.text.to_ascii_uppercase();
                            match self.globals.get(&k).cloned() {
                                Some(p) => {
                                    scope.vars.insert(k, p);
                                }
                                None => return self.not_yet(n.span, "this VAR_EXTERNAL"),
                            }
                        }
                    }
                    continue;
                }
                VarKind::Global => return self.not_yet(b.span, "VAR_GLOBAL in a FUNCTION_BLOCK"),
            };
            for d in &b.decls {
                let decl = self.decl(d)?;
                if d.names.is_empty() {
                    self.unnamed_located(d, &decl, "self->", &mut fields, &mut sec)?;
                    continue;
                }
                for n in &d.names {
                    let k = n.text.to_ascii_uppercase();
                    let place = match (&decl, dir, &d.location) {
                        (Decl::Data(_), Some(_), Some(a)) => {
                            return self.not_yet(a.span, "a FUNCTION_BLOCK parameter at an address");
                        }
                        // An array or structure of instances (inst.rs): set
                        // up as an instance in it is, its other parts under
                        // the section's guard.
                        // An input or output that holds instances is one
                        // too, copied in or out whole by a call
                        // (docs/dependencies.md).
                        (Decl::Data(t), None | Some(Dir::Input | Dir::Output), None) if self.has_instance(t) => {
                            retentive |= self.keeps_in(t);
                            let _ = writeln!(fields, "    {};", self.declare(t, &format!("V_{k}")));
                            let lv = format!("self->V_{k}");
                            self.init_keep =
                                if b.retain == Some(Retain::Retain) { "keep != 0 ? 2 : 0" } else { "keep != 0" }.into();
                            self.init_guard = guard.map(str::to_string);
                            let r = self.init_data(&lv, t, d.init.as_ref(), &mut init);
                            self.init_keep = "0".into();
                            self.init_guard = None;
                            r?;
                            let p = Place::Data(Lv::C(lv), t.clone());
                            if public || dir.is_some() {
                                data.push((k.clone(), t.clone(), dir.unwrap_or(Dir::Input)));
                            }
                            members.push(symbols::member(&k, &format!("V_{k}"), &p, symbols::dir_flags(dir) | flags));
                            scope.vars.insert(k, p);
                            continue;
                        }
                        // An in-out: a pointer to the caller's variable,
                        // set by each call (stmt.rs); an ARRAY[*]'s to its
                        // `iecf_flex_t` (flex.rs).
                        (Decl::Data(t), Some(Dir::InOut), _)
                            if self.passes(t) && !matches!(t.kind, derived::Kind::Itf(_)) =>
                        {
                            if d.init.is_some() {
                                return self.not_yet(d.span, "an initial value of a VAR_IN_OUT");
                            }
                            let (field, lv) = match t.kind {
                                derived::Kind::Flex(..) => (format!("iecf_flex_t *V_{k}"), format!("self->V_{k}")),
                                _ => (self.declare(t, &format!("(*V_{k})")), format!("(*self->V_{k})")),
                            };
                            let _ = writeln!(fields, "    {field};");
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            in_data.push((k.clone(), t.clone()));
                            scope.vars.insert(k.clone(), Place::Data(Lv::C(lv), t.clone()));
                            continue;
                        }
                        // An enumeration's in-out, as the integer it is held in.
                        (Decl::Data(derived::DTy { kind: derived::Kind::Enum(_, ty), .. }), Some(Dir::InOut), None) => {
                            if d.init.is_some() {
                                return self.not_yet(d.span, "an initial value of a VAR_IN_OUT");
                            }
                            let _ = writeln!(fields, "    {} *V_{k};", c_type(*ty));
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            params.push((k.clone(), *ty, Dir::InOut));
                            scope.vars.insert(k.clone(), Place::Field(format!("(*self->V_{k})"), *ty));
                            continue;
                        }
                        // A subrange's in-out, as the integer it is held
                        // in: a store through it clamped to its bounds.
                        (
                            Decl::Data(t @ derived::DTy { kind: derived::Kind::Elem(ty), .. }),
                            Some(Dir::InOut),
                            None,
                        ) => {
                            if d.init.is_some() {
                                return self.not_yet(d.span, "an initial value of a VAR_IN_OUT");
                            }
                            let _ = writeln!(fields, "    {} *V_{k};", c_type(*ty));
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            params.push((k.clone(), *ty, Dir::InOut));
                            let t = derived::DTy { init: None, ..t.clone() };
                            scope.vars.insert(k.clone(), Place::Data(Lv::C(format!("(*self->V_{k})")), t));
                            continue;
                        }
                        (Decl::Data(t), dir, _) => {
                            let p = self.data_var("self->", &k, t, d, &mut fields, &mut sec)?;
                            if let Some(r) = t.range {
                                ranges.insert(k.clone(), r);
                            }
                            match (dir, p.elem()) {
                                (None, Some(e)) if public => params.push((k.clone(), e, Dir::Input)),
                                (None, None) if public => data.push((k.clone(), t.clone(), Dir::Input)),
                                (None, _) => {}
                                (Some(Dir::Input | Dir::Output), Some(e)) => {
                                    params.push((k.clone(), e, dir.unwrap_or(Dir::Input)))
                                }
                                (Some(io @ (Dir::Input | Dir::Output)), None) => data.push((k.clone(), t.clone(), io)),
                                _ => return self.not_yet(d.span, "a FUNCTION_BLOCK parameter of this type"),
                            }
                            members.push(symbols::member(&k, &format!("V_{k}"), &p, symbols::dir_flags(dir) | flags));
                            scope.vars.insert(k, p);
                            continue;
                        }
                        // An interface: NULL until set. An input is set by a
                        // call as itf.rs says; an input, an output or a
                        // PUBLIC one is read from outside as data is.
                        (Decl::Itf(i), None | Some(Dir::Input | Dir::Output), None) => {
                            let _ = writeln!(fields, "    iecf_itf_t V_{k};");
                            let place = Place::Itf(format!("self->V_{k}"), i.clone());
                            self.init_var(&place, d, "0", &mut sec)?;
                            if dir == Some(Dir::Input) {
                                itfs.push((k.clone(), i.clone()));
                            }
                            if dir.is_some() || public {
                                data.push((k.clone(), derived::DTy::itf(i), dir.unwrap_or(Dir::Input)));
                            }
                            scope.vars.insert(k, place);
                            continue;
                        }
                        // An instance, of its type or a derived one (Ed.3
                        // 6.6.8): its address, set by each call.
                        (Decl::Block(inner), Some(Dir::InOut), None) => {
                            if d.init.is_some() {
                                return self.not_yet(d.span, "an initial value of a VAR_IN_OUT");
                            }
                            let _ = writeln!(fields, "    B_{inner}_t *V_{k};");
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            blocks.push((k.clone(), inner.clone()));
                            scope.vars.insert(k.clone(), Place::Block(format!("(*self->V_{k})"), inner.clone()));
                            scope.poly.insert(k.clone());
                            continue;
                        }
                        // A standard block's instance: its address, set
                        // by each call, as a structure's is.
                        (Decl::Fb(fb), Some(Dir::InOut), None) => {
                            if d.init.is_some() {
                                return self.not_yet(d.span, "an initial value of a VAR_IN_OUT");
                            }
                            let t = derived::DTy { kind: derived::Kind::Std(fb.clone()), init: None, range: None };
                            let _ = writeln!(fields, "    {};", self.declare(&t, &format!("(*V_{k})")));
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            in_data.push((k.clone(), t));
                            scope.vars.insert(k.clone(), Place::Fb(format!("(*self->V_{k})"), fb.clone()));
                            continue;
                        }
                        (Decl::Elem(ty), Some(Dir::InOut), None) => {
                            if d.init.is_some() {
                                return self.not_yet(d.span, "an initial value of a VAR_IN_OUT");
                            }
                            let _ = writeln!(fields, "    {} *V_{k};", c_type(*ty));
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            params.push((k.clone(), *ty, Dir::InOut));
                            scope.vars.insert(k.clone(), Place::Field(format!("(*self->V_{k})"), *ty));
                            continue;
                        }
                        (Decl::Elem(ty), None, Some(a)) => self.located(a, *ty)?,
                        (Decl::Elem(ty), _, None) => {
                            let _ = writeln!(fields, "    {} V_{k};", c_type(*ty));
                            Place::Field(format!("self->V_{k}"), *ty)
                        }
                        (Decl::Fb(fb), None, None) => {
                            let _ = writeln!(fields, "    iecf_{}_t V_{k};", fb.stem);
                            Place::Fb(format!("self->V_{k}"), fb.clone())
                        }
                        // An input or output of a block's type: an instance
                        // of its own in this one, a call copying the one
                        // given into it or it out, whole (Ed.3 Table 42,
                        // 9a; docs/dependencies.md).
                        (Decl::Fb(fb), Some(io @ (Dir::Input | Dir::Output)), None) => {
                            let _ = writeln!(fields, "    iecf_{}_t V_{k};", fb.stem);
                            let t = derived::DTy { kind: derived::Kind::Std(fb.clone()), init: None, range: None };
                            data.push((k.clone(), t, io));
                            Place::Fb(format!("self->V_{k}"), fb.clone())
                        }
                        (Decl::Block(inner), Some(io @ (Dir::Input | Dir::Output)), None) => {
                            retentive |= self.retentive.contains(inner);
                            let _ = writeln!(fields, "    B_{inner}_t V_{k};");
                            let place = Place::Block(format!("self->V_{k}"), inner.clone());
                            self.init_var(&place, d, "keep != 0", &mut init)?;
                            let t = derived::DTy { kind: derived::Kind::Block(inner.clone()), init: None, range: None };
                            data.push((k.clone(), t, io));
                            members.push(symbols::member(
                                &k,
                                &format!("V_{k}"),
                                &place,
                                symbols::dir_flags(dir) | flags,
                            ));
                            scope.vars.insert(k, place);
                            continue;
                        }
                        (Decl::Block(inner), None, None) => {
                            // Its own type says what it keeps; the section
                            // says whether the instance is RETAIN (Ed.3 6.5.6).
                            retentive |= self.retentive.contains(inner);
                            let _ = writeln!(fields, "    B_{inner}_t V_{k};");
                            let place = Place::Block(format!("self->V_{k}"), inner.clone());
                            let keep = if b.retain == Some(Retain::Retain) { "keep != 0 ? 2 : 0" } else { "keep != 0" };
                            self.init_var(&place, d, keep, &mut init)?;
                            members.push(symbols::member(&k, &format!("V_{k}"), &place, flags));
                            scope.vars.insert(k, place);
                            continue;
                        }
                        _ => return self.not_yet(d.span, "this declaration in a FUNCTION_BLOCK"),
                    };
                    if let (Some(dir), Some(ty)) = (dir.or(public.then_some(Dir::Input)), place.elem()) {
                        params.push((k.clone(), ty, dir));
                    }
                    self.init_var(&place, d, "0", &mut sec)?;
                    members.push(symbols::member(&k, &format!("V_{k}"), &place, symbols::dir_flags(dir) | flags));
                    if let (Some(edge), Some(raw)) = (d.edge, place.lvalue()) {
                        let raw = raw.to_string();
                        self.edge_input(&k, edge, &raw, (&mut fields, &mut sec), &mut edge_lines, &mut scope);
                        edged.push(k);
                        continue;
                    }
                    scope.vars.insert(k, place);
                }
            }
            match guard {
                _ if sec.is_empty() => {}
                Some(g) => {
                    let _ = writeln!(init, "    if ({g}) {{");
                    for line in sec.lines() {
                        let _ = writeln!(init, "    {line}");
                    }
                    let _ = writeln!(init, "    }}");
                }
                None => init.push_str(&sec),
            }
        }
        // An instance kept whole keeps its steps (Ed.3 6.7.2).
        let chart = match self.chart(pou, "self->", Some("keep != 2"), &mut fields, &mut init, &mut scope) {
            Ok(c) => c,
            Err(e) => {
                self.block_busy.retain(|k| k != key);
                return Err(e);
            }
        };
        if retentive {
            self.retentive.insert(key.to_string());
        }
        // ENO used and not declared (Ed.3 6.6.1.5): a field, 1 as each
        // run starts; a caller's ENO is it and no flag raised in the run.
        let eno = self.types.implicit_eno(key);
        if eno {
            let _ = writeln!(fields, "    IEC_BOOL eno;");
            scope.vars.insert("ENO".into(), Place::Field("self->eno".into(), Elem::Bool));
        }
        // What a derived type inherits: not the VAR_TEMPs, not ENO, not
        // what is PRIVATE (Ed.3 6.6.5.5.2).
        let mut inherit = scope.clone();
        inherit.vars.remove("ENO");
        for b in &pou.var_blocks {
            if b.kind == VarKind::Temp || matches!(b.access, Some((Access::Private, _))) {
                for n in b.decls.iter().flat_map(|d| &d.names) {
                    inherit.vars.remove(&n.text.to_ascii_uppercase());
                }
            }
        }
        self.inherit_scopes.insert(key.to_string(), inherit);
        let in_outs: Vec<String> = params
            .iter()
            .filter(|p| p.2 == Dir::InOut)
            .map(|p| p.0.clone())
            .chain(blocks.iter().map(|b| b.0.clone()))
            .chain(in_data.iter().map(|b| b.0.clone()))
            .collect();
        self.blocks.insert(key.to_string(), BlockSig { params, data, ranges, blocks, paths, itfs, in_data });
        // THIS, the instance (Ed.3 6.6.5.7.2); the scope kept for the
        // methods, which reach the same variables through their `self`.
        scope.vars.insert("THIS".into(), Place::Block("(*self)".into(), key.to_string()));
        scope.poly.insert("THIS".into());
        let mut kept = scope.clone();
        // A method has no access to an R_EDGE or F_EDGE input (Ed.3
        // 6.6.5.4 rule 3).
        for k in &edged {
            kept.vars.remove(k);
        }
        for b in pou.var_blocks.iter().filter(|b| b.kind == VarKind::Temp) {
            for n in b.decls.iter().flat_map(|d| &d.names) {
                kept.vars.remove(&n.text.to_ascii_uppercase());
            }
        }
        for n in &in_outs {
            kept.vars.remove(n);
        }
        self.block_scopes.insert(key.to_string(), kept);
        // Each method of a type with a table, for it (oo.rs, itf.rs).
        if self.carries_type(key) {
            for m in pou.methods.iter().filter(|m| m.specs.abstract_.is_none()) {
                self.fn_sig(&iecf_sema::method_key(key, &m.name.text), m.name.span)?;
            }
        }
        self.block_members.insert(key.to_string(), members);
        let index = self.pou_names.len() as u16;
        self.pou_names.push(key.to_string());
        let p = self.prefix.clone();
        let mut body = edge_lines;
        let mut cx = stmt::Body {
            pou: index,
            scope: &scope,
            out: &mut body,
            depth: 1,
            loops: Vec::new(),
            ret: "return;".into(),
            on_return: None,
        };
        let r = match &chart {
            Some(ch) => self.chart_scan(ch, &mut cx),
            None => self.stmts(&pou.body, &mut cx),
        };
        self.block_busy.retain(|k| k != key);
        r?;
        self.fill_ref_inits(&mut init, &scope)?;
        self.fill_ref_inits(&mut temps, &scope)?;
        // Declared ahead of every structure (`block_fwd`), so a pointer to
        // it - an in-out of its own type - needs no definition first.
        let _ = writeln!(self.block_fwd, "typedef struct B_{key}_s B_{key}_t;");
        let _ = writeln!(self.block_types, "struct B_{key}_s {{");
        if fields.is_empty() {
            let _ = writeln!(self.block_types, "    char none_; /* C has no empty structure */");
        }
        self.block_types.push_str(&fields);
        let _ = writeln!(self.block_types, "}};\n");
        let c = &mut self.block_code;
        let what = if pou.kind == PouKind::Class { "CLASS" } else { "FUNCTION_BLOCK" };
        let _ = writeln!(c, "/* {what} {} */", pou.name.text);
        let _ = writeln!(c, "static void B_{key}_init({p}_t *ctx, B_{key}_t *self, int keep) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;\n    (void)keep;");
        c.push_str(&init);
        let _ = writeln!(c, "}}\n");
        // A CLASS has no body, and is never called (Ed.3 6.6.5.2).
        if pou.kind == PouKind::Class {
            return Ok(());
        }
        let _ = writeln!(c, "static void B_{key}({p}_t *ctx, B_{key}_t *self) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
        if eno {
            let _ = writeln!(c, "    self->eno = 1;");
        }
        c.push_str(&temps);
        c.push_str(&body);
        let _ = writeln!(c, "}}\n");
        Ok(())
    }
}

/// A FUNCTION's or method's parameters after ctx and self, as C: type
/// (its `*` too) and name. `eno` whether an `IEC_BOOL *eno` comes first.
pub(crate) fn sig_params(sig: &FnSig, eno: bool) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if eno {
        out.push(("IEC_BOOL *".to_string(), "eno".to_string()));
    }
    if sig.ret_data.is_some() {
        out.push(("void *".to_string(), "R_".to_string()));
    }
    for (n, t, d) in &sig.inputs {
        let (ty, name) = match (sig.in_blocks.get(n), d) {
            (Some(k), _) => (format!("B_{k}_t *"), format!("V_{n}")),
            _ if sig.in_itfs.contains_key(n) => ("iecf_itf_t ".to_string(), format!("V_{n}")),
            _ if sig.data.contains_key(n) && d.is_some() => ("const void *".to_string(), format!("P_{n}")),
            _ if sig.data.contains_key(n) => ("void *".to_string(), format!("V_{n}")),
            (None, Some(_)) => (format!("{} ", c_type(*t)), format!("V_{n}")),
            (None, None) => (format!("{} *", c_type(*t)), format!("V_{n}")),
        };
        out.push((ty, name));
    }
    out.extend(sig.outputs.iter().map(|(n, t, _)| {
        let ty = if sig.data.contains_key(n) { "void *".to_string() } else { format!("{} *", c_type(*t)) };
        (ty, format!("O_{n}"))
    }));
    out
}

/// `c`, the whole output, with each of its static functions that nothing
/// calls named in the start function, `(void)B_X;`: a block declared and
/// never called, a type only referred to. A compiler that warns of an
/// unused function then has none to warn of, and the output stays clean
/// with every warning on (docs/c-output.md §1) without an attribute.
fn name_unused(c: &str, start: &str) -> String {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut names: Vec<&str> = Vec::new();
    for l in c.lines().filter(|l| l.starts_with("static ") && l.ends_with(" {")) {
        let Some(head) = l.split('(').next() else { continue };
        let Some(i) = head.rfind(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) else { continue };
        names.push(&head[i + 1..]);
    }
    let mut lines = String::new();
    for n in names {
        // Used: named anywhere but on its own prototype and definition.
        let used = c.lines().filter(|l| !(l.starts_with("static ") && l.contains(&format!(" {n}(")))).any(|l| {
            l.match_indices(n).any(|(i, _)| {
                let b = l.as_bytes();
                (i == 0 || !word(b[i - 1])) && b.get(i + n.len()).is_none_or(|x| !word(*x))
            })
        });
        if !used && !start.contains(&format!(" {n}(")) {
            let _ = writeln!(lines, "    (void){n};");
        }
    }
    match c.find(start) {
        Some(at) if !lines.is_empty() => {
            let at = at + start.len();
            format!("{}{lines}{}", &c[..at], &c[at..])
        }
        _ => c.to_string(),
    }
}

/// A FUNCTION's parameter or result of a derived type at `lv`: an
/// instance is used as any instance is, the others as data.
/// The variable at `lv` of the elementary type `e`: of a subrange of
/// it when `range` says so, a store into it then clamped.
fn ranged(lv: String, e: Elem, range: Option<(i128, i128)>) -> Place {
    match range {
        Some(_) => Place::Data(Lv::C(lv), derived::DTy { kind: derived::Kind::Elem(e), init: None, range }),
        None => Place::Field(lv, e),
    }
}

fn data_place(lv: String, t: derived::DTy) -> Place {
    match t.kind {
        derived::Kind::Block(k) => Place::Block(lv, k),
        derived::Kind::Std(fb) => Place::Fb(lv, fb),
        _ => Place::Data(Lv::C(lv), t),
    }
}

/// The C type of an elementary type (runtime/iecf.h).
pub(crate) fn c_type(e: Elem) -> String {
    format!("IEC_{}", e.name())
}

#[cfg(test)]
mod tests;

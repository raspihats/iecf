//! iecf's C: a checked project in, the files of docs/c-output.md out.
//!
//! Built in slices (PROPOSAL.md §5 step 2), each turning on harness cases.
//! What a slice does not cover yet is E0009 at the construct, never C
//! that means something else. Covered: one CONFIGURATION, its tasks and
//! PROGRAM instances; variables of the integer, bit-string, BOOL, real,
//! time and date types, located or not (each its own width from its
//! address), and instances of the standard
//! function blocks; every ST statement; the integer operators,
//! conversions, shifts, ABS, MIN, MAX and LIMIT; + and - on durations;
//! the user's FUNCTIONs with inputs and locals, and FUNCTION_BLOCKs with
//! inputs, outputs, in-outs, locals and instances of other blocks.
//!
//! No I/O, like the rest of the core: it builds for wasm32.

mod dates;
mod derived;
mod enable;
mod expr;
mod names;
mod sfc;
mod stmt;
mod strings;
mod symbols;

use iecf_diagnostics::{Code, Diagnostic, SourceMap, Span};
use iecf_sema::Types;
use iecf_sema::stdlib::Dir;
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;
use names::{Place, Scope, StdFb};
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
}

/// A user's FUNCTION as its callers see it: `F_<NAME>(ctx, inputs...)`.
#[derive(Clone, Debug)]
pub(crate) struct FnSig {
    pub(crate) key: String,
    pub(crate) ret: Elem,
    /// Each input: name, type, and its initial value as C, for a call
    /// that does not give it (Ed.3 6.6.1.4).
    pub(crate) inputs: Vec<(String, Elem, String)>,
    /// Its body uses ENO without declaring it (Ed.3 6.6.1.5): an
    /// `IEC_BOOL *eno` after ctx, set to 1 as it starts.
    pub(crate) eno: bool,
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
    /// The user's FUNCTION_BLOCKs written so far, in the order their
    /// types must appear (a block after those it contains).
    pub(crate) blocks: HashMap<String, BlockSig>,
    block_busy: Vec<String>,
    block_types: String,
    block_code: String,
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
    /// Arguments of an EN/ENO call already computed, by their span.
    pub(crate) subst: HashMap<Span, enable::Sub>,
    /// Whether the expression being written is a statement's, which can
    /// have lines written before it (enable.rs).
    pub(crate) pre_ok: bool,
    /// Set by an EN/ENO call around the call it gates: a SPLIT may then
    /// write its lines, and leaves them here for inside `if (EN)`.
    pub(crate) split_ok: bool,
    pub(crate) split_lines: Option<Vec<String>>,
    /// The local an EN/ENO call passes as a FUNCTION's `eno`, taken by
    /// the outermost user call (enable.rs), and whether one took it.
    pub(crate) want_eno: Option<String>,
    pub(crate) eno_used: bool,
}

impl<'a> Gen<'a> {
    fn new(units: &'a [&'a SourceUnit], types: &'a Types, map: &'a SourceMap) -> Self {
        let mut pous = HashMap::new();
        let mut type_decls = HashMap::new();
        for u in units {
            for item in &u.items {
                match item {
                    Item::Pou(p) => {
                        pous.insert(p.name.text.to_ascii_uppercase(), p);
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
            blocks: HashMap::new(),
            block_busy: Vec::new(),
            block_types: String::new(),
            block_code: String::new(),
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
            want_eno: None,
            eno_used: false,
            subst: HashMap::new(),
            pre_ok: false,
            split_ok: false,
            split_lines: None,
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
        self.configuration(config).ok()
    }

    fn configuration(&mut self, config: &'a Configuration) -> R<Vec<(String, String)>> {
        let p = self.prefix.clone();
        let up = p.to_ascii_uppercase();

        // Globals: the configuration's, then each resource's.
        let mut globals = Scope::default();
        let mut fields = String::new(); // members of <p>_t
        let mut init = String::new(); // body of <p>_init and <p>_warm
        let mut rfields = String::new(); // members of <p>_retain_t
        let mut rinit = String::new(); // a cold start's, after `init`
        let mut links = String::new(); // each instance's pointer into ctx->retain
        for b in &config.globals {
            self.global_block(b, "", &mut globals, (&mut fields, &mut init), (&mut rfields, &mut rinit))?;
        }
        for r in &config.resources {
            let rp = r.name.as_ref().map(|n| format!("{}__", n.text.to_ascii_uppercase())).unwrap_or_default();
            for b in &r.globals {
                self.global_block(b, &rp, &mut globals, (&mut fields, &mut init), (&mut rfields, &mut rinit))?;
            }
        }

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
                    let (ty, code) = self.program(pou, index as u16, &globals)?;
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
        while let Some(key) = self.fn_queue.pop() {
            if done.contains(&key) {
                continue;
            }
            done.push(key.clone());
            let (proto, code) = self.function(&key)?;
            fn_protos.push_str(&proto);
            fn_code.push_str(&code);
        }

        // <p>.h
        let mut h = String::new();
        let guard = format!("{up}_H");
        let _ = writeln!(h, "/* {p}.h - written by iecf from CONFIGURATION {}. */", config.name.text);
        let _ = writeln!(h, "#ifndef {guard}\n#define {guard}\n\n#include \"iecf.h\"\n");
        for (area, n) in ["I", "Q", "M"].iter().zip(self.extent) {
            let _ = writeln!(h, "#define {up}_{area}_BYTES {n}u");
        }
        let _ = writeln!(h, "\n{}{}{pou_types}", self.data_types, self.block_types);
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
        c.push_str(&self.block_code);
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
        let names: Vec<String> = self.pou_names.iter().map(|n| format!("\"{n}\"")).collect();
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
                let global = match &e.kind {
                    ExprKind::Name(id) => globals.vars.get(&id.text.to_ascii_uppercase()).cloned(),
                    _ => None,
                };
                match global {
                    Some(Place::Field(g, t)) if Some(t) == place.elem() => {
                        let _ = writeln!(pre, "    ctx->{field}.V_{key} = &({g});");
                        bound.push(key);
                    }
                    _ => return self.not_yet(e.span, "an in-out connected to this (a global not at an address)"),
                }
                continue;
            }
            let Some(ty) = place.elem() else {
                return self.not_yet(name.span, "a connection of an array or structure");
            };
            if matches!(&place, Place::Data(_, t) if t.range.is_some()) {
                return self.not_yet(name.span, "a connection of a subrange");
            }
            let lv = match place.lvalue() {
                Some(lv) => lv.replacen("self->", &inst, 1),
                None => return self.not_yet(name.span, "a connection of this variable"),
            };
            match (a, d == dir) {
                (Arg::Input(_, e), true) => {
                    self.may_fault = false;
                    self.may_flag = false;
                    let v = self.value_as(e, ty, globals)?;
                    if self.may_fault || self.may_flag {
                        return self.not_yet(e.span, "a connection that can fault or be flagged");
                    }
                    let _ = writeln!(pre, "    {lv} = {v};");
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
                        _ => None,
                    };
                    let Some(to) = to else { return self.not_yet(target.span, "a connection to this") };
                    let Some(tt) = to.elem() else { return self.not_yet(target.span, "a connection to this") };
                    // Figure 11's implicit conversions widen: a cast, but
                    // not between the time and date types, whose units
                    // differ (TIME in ms, LTIME in ns): their conversions,
                    // exact when widening.
                    let v = if tt == ty {
                        v
                    } else if expr::is_time_or_date(ty) || expr::is_time_or_date(tt) {
                        self.may_flag = false;
                        let v = self.convert(target.span, v, ty, tt)?;
                        if self.may_flag {
                            return self
                                .not_yet(target.span, format!("a connection of {} to {}", ty.name(), tt.name()));
                        }
                        v
                    } else {
                        format!("(({})({v}))", c_type(tt))
                    };
                    let _ = writeln!(post, "    {}", to.store(&v));
                }
                _ => return self.not_yet(name.span, format!("the connection {}", name.text)),
            }
        }
        if let Some((k, ..)) = params.iter().find(|(k, _, d)| *d == Dir::InOut && !bound.contains(k)) {
            return self.not_yet(pc.span, format!("a program whose in-out {k} is not connected"));
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
                    _ => return self.not_yet(value.span, "an INTERVAL that is not a TIME literal"),
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
    fn global_block(
        &mut self,
        b: &VarBlock,
        rp: &str,
        globals: &mut Scope,
        plain: (&mut String, &mut String),
        retain: (&mut String, &mut String),
    ) -> R<()> {
        let is_retain = b.retain == Some(Retain::Retain);
        let (fields, init, owner) =
            if is_retain { (retain.0, retain.1, "ctx->retain.") } else { (plain.0, plain.1, "ctx->") };
        let flags = if b.constant { symbols::CONSTANT } else { 0 } | if is_retain { symbols::RETAIN } else { 0 };
        for d in &b.decls {
            let ty = match self.decl(d)? {
                Decl::Elem(ty) => ty,
                Decl::Data(t) => {
                    for n in &d.names {
                        let key = n.text.to_ascii_uppercase();
                        let place = self.data_var(owner, &format!("{rp}{key}"), &t, d, fields, init)?;
                        let m = symbols::member(&key, &format!("V_{rp}{key}"), &place, flags);
                        self.global_members.push((format!("{}{key}", rp.replace("__", ".")), m));
                        globals.vars.insert(key, place);
                    }
                    continue;
                }
                _ => return self.not_yet(d.ty.span(), "a global function block instance"),
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
            return self.not_yet(a.span, "a variable of a derived type at an address");
        }
        let _ = writeln!(fields, "    {};", self.declare(t, &format!("V_{k}")));
        let lv = format!("{owner}V_{k}");
        self.init_data(&lv, t, d.init.as_ref(), init)?;
        Ok(Place::Data(lv, t.clone()))
    }

    fn decl_of(&mut self, key: Span, at: Span) -> R<Decl> {
        match self.types.decl(key) {
            Some(Ty::Elem(e)) if expr::held(*e) => Ok(Decl::Elem(*e)),
            Some(Ty::Fb(name)) => match StdFb::named(name) {
                Some(fb) => Ok(Decl::Fb(fb)),
                None => {
                    let key = name.to_ascii_uppercase();
                    match self.pous.get(&key) {
                        Some(p) if p.kind == PouKind::FunctionBlock => {
                            self.block(&key, at)?;
                            Ok(Decl::Block(key))
                        }
                        _ => self.not_yet(at, format!("an instance of {name}")),
                    }
                }
            },
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
            Place::Fb(lv, fb) => {
                if d.init.is_some() {
                    return self.not_yet(d.span, "an initial value of a function block instance");
                }
                let _ = writeln!(out, "    iecf_{}_init(&{lv});", fb.stem);
                return Ok(());
            }
            Place::Data(lv, t) => {
                let (lv, t) = (lv.clone(), t.clone());
                return self.init_data(&lv, &t, d.init.as_ref(), out);
            }
            Place::Block(lv, key) => {
                if d.init.is_some() {
                    return self.not_yet(d.span, "an initial value of a function block instance");
                }
                let _ = writeln!(out, "    B_{key}_init(ctx, &{lv}, {keep});");
                return Ok(());
            }
            Place::Field(_, t) | Place::Located { ty: t, .. } => *t,
        };
        let value = match &d.init {
            Some(Init::Expr(e)) => {
                self.may_fault = false;
                self.may_flag = false;
                let code = self.value_as(e, ty, &Scope::default())?;
                if self.may_fault || self.may_flag {
                    return self.not_yet(e.span, "an initial value that can fault or be flagged");
                }
                Some(code)
            }
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
        let mut fields = String::new();
        let mut init = String::new();
        let mut temps = String::new();
        let mut members = Vec::new();
        // RETAIN variables: a structure of their own in ctx->retain, the
        // instance pointing at its part (docs/c-output.md §7).
        let mut rfields = String::new();
        let mut rinit = String::new();
        let mut params: Vec<(String, Place, Dir)> = Vec::new();
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
                        let Decl::Elem(ty) = self.decl(d)? else {
                            return self.not_yet(d.span, "a PROGRAM in-out of this type");
                        };
                        if d.init.is_some() || d.location.is_some() {
                            return self.not_yet(d.span, "an initial value or address on a VAR_IN_OUT");
                        }
                        for n in &d.names {
                            let k = n.text.to_ascii_uppercase();
                            let _ = writeln!(fields, "    {} *V_{k};", c_type(ty));
                            let _ = writeln!(init, "    self->V_{k} = NULL;");
                            let place = Place::Field(format!("(*self->V_{k})"), ty);
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
                    return self.not_yet(d.span, "a located variable without a name");
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
                        (Decl::Fb(_) | Decl::Block(_), Some(a)) => {
                            return self.not_yet(a.span, "a function block at an address");
                        }
                    };
                    self.init_var(&place, d, "0", init)?;
                    members.push(symbols::member(&k, &format!("V_{k}"), &place, flags));
                    if let Some(dir) = dir {
                        params.push((k.clone(), place.clone(), dir));
                    }
                    scope.vars.insert(k, place);
                }
            }
        }
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

        let p = self.prefix.clone();
        let mut c = String::new();
        let _ = writeln!(c, "/* PROGRAM {} */", pou.name.text);
        let _ = writeln!(c, "static void P_{key}_init({p}_t *ctx, P_{key}_t *self, int warm) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;\n    (void)warm;");
        c.push_str(&init);
        let _ = writeln!(c, "}}\n");
        if has_retain {
            // At a cold start only: a warm one keeps them.
            let _ = writeln!(c, "static void P_{key}_init_retain({p}_t *ctx, P_{key}_t *self) {{");
            let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
            c.push_str(&rinit);
            let _ = writeln!(c, "}}\n");
        }
        let mut body = String::new();
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
                Decl::Data(t) => {
                    for n in &d.names {
                        let k = n.text.to_ascii_uppercase();
                        let _ = writeln!(out, "    {};\n    (void)V_{k};", self.declare(&t, &format!("V_{k}")));
                        self.init_data(&format!("V_{k}"), &t, d.init.as_ref(), out)?;
                        scope.vars.insert(k.clone(), Place::Data(format!("V_{k}"), t.clone()));
                    }
                    continue;
                }
                _ => return self.not_yet(d.span, format!("a function block instance in {what}")),
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
        if let Some(s) = self.functions.get(key) {
            return Ok(s.clone());
        }
        let Some(pou) = self.pous.get(key).copied() else {
            return self.not_yet(at, "a call of something that is not a FUNCTION");
        };
        if pou.kind != PouKind::Function {
            return self.not_yet(at, "a call of a user's FUNCTION_BLOCK");
        }
        let ret = match &pou.return_type {
            Some(rt) => match self.decl_of(rt.span(), rt.span())? {
                Decl::Elem(e) => e,
                Decl::Fb(_) | Decl::Block(_) | Decl::Data(_) => return self.not_yet(rt.span(), "this result type"),
            },
            None => return self.not_yet(pou.name.span, "a FUNCTION without a result"),
        };
        let mut inputs = Vec::new();
        for b in &pou.var_blocks {
            match b.kind {
                VarKind::Input => {}
                VarKind::Var | VarKind::Temp => continue,
                _ => return self.not_yet(b.span, "a FUNCTION's outputs, in-outs or externals"),
            }
            for d in &b.decls {
                if d.edge.is_some() {
                    return self.not_yet(d.span, "R_EDGE and F_EDGE");
                }
                let Decl::Elem(ty) = self.decl(d)? else {
                    return self.not_yet(d.span, "a function block as an input");
                };
                let default = match &d.init {
                    Some(Init::Expr(e)) => self.value_as(e, ty, &Scope::default())?,
                    Some(_) => return self.not_yet(d.span, "this initial value"),
                    None => expr::zero(ty),
                };
                for n in &d.names {
                    inputs.push((n.text.to_ascii_uppercase(), ty, default.clone()));
                }
            }
        }
        let sig = FnSig { key: key.to_string(), ret, inputs, eno: self.types.implicit_eno(key) };
        self.functions.insert(key.to_string(), sig.clone());
        self.fn_queue.push(key.to_string());
        Ok(sig)
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
        let rt = c_type(sig.ret);
        let mut params: Vec<String> = sig.inputs.iter().map(|(n, t, _)| format!("{} V_{n}", c_type(*t))).collect();
        if sig.eno {
            params.insert(0, "IEC_BOOL *eno".into());
        }
        let head = format!(
            "static {rt} F_{key}({p}_t *ctx{}{})",
            if params.is_empty() { "" } else { ", " },
            params.join(", ")
        );

        let mut scope = Scope::default();
        let mut decls = String::new();
        let _ = writeln!(decls, "    {rt} V_{key} = {};", expr::zero(sig.ret));
        let _ = writeln!(decls, "    (void)ctx;");
        if sig.eno {
            let _ = writeln!(decls, "    *eno = 1;");
            scope.vars.insert("ENO".into(), Place::Field("(*eno)".into(), Elem::Bool));
        }
        scope.vars.insert(key.to_string(), Place::Field(format!("V_{key}"), sig.ret));
        for (n, t, _) in &sig.inputs {
            let _ = writeln!(decls, "    (void)V_{n};");
            scope.vars.insert(n.clone(), Place::Field(format!("V_{n}"), *t));
        }
        for b in pou.var_blocks.iter().filter(|b| matches!(b.kind, VarKind::Var | VarKind::Temp)) {
            self.locals(b, "a FUNCTION", &mut scope, &mut decls)?;
        }
        let mut body = String::new();
        let ret = format!("return V_{key};");
        let mut cx = stmt::Body {
            pou: index,
            scope: &scope,
            out: &mut body,
            depth: 1,
            loops: Vec::new(),
            ret: ret.clone(),
            on_return: None,
        };
        self.stmts(&pou.body, &mut cx)?;
        let mut c = String::new();
        let _ = writeln!(c, "/* FUNCTION {} */", pou.name.text);
        let _ = writeln!(c, "{head} {{");
        c.push_str(&decls);
        c.push_str(&body);
        let _ = writeln!(c, "    {ret}\n}}\n");
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
        if self.block_busy.iter().any(|k| k == key) {
            return self.not_yet(at, "a FUNCTION_BLOCK that contains itself");
        }
        let pou = self.pous[key];
        self.block_busy.push(key.to_string());
        let mut scope = Scope::default();
        let mut fields = String::new();
        let mut init = String::new();
        let mut params = Vec::new();
        let mut ranges = HashMap::new();
        let mut data = Vec::new();
        let mut temps = String::new();
        let mut members = Vec::new();
        let mut retentive = false;
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
                if d.edge.is_some() {
                    return self.not_yet(d.span, "R_EDGE and F_EDGE");
                }
                let decl = self.decl(d)?;
                if d.names.is_empty() {
                    return self.not_yet(d.span, "a located variable without a name");
                }
                for n in &d.names {
                    let k = n.text.to_ascii_uppercase();
                    let place = match (&decl, dir, &d.location) {
                        (Decl::Data(t), dir, _) => {
                            let p = self.data_var("self->", &k, t, d, &mut fields, &mut sec)?;
                            if let Some(r) = t.range {
                                ranges.insert(k.clone(), r);
                            }
                            match (dir, p.elem()) {
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
                    if let (Some(dir), Some(ty)) = (dir, place.elem()) {
                        params.push((k.clone(), ty, dir));
                    }
                    self.init_var(&place, d, "0", &mut sec)?;
                    members.push(symbols::member(&k, &format!("V_{k}"), &place, symbols::dir_flags(dir) | flags));
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
        self.blocks.insert(key.to_string(), BlockSig { params, data, ranges });
        self.block_members.insert(key.to_string(), members);
        let index = self.pou_names.len() as u16;
        self.pou_names.push(key.to_string());
        let p = self.prefix.clone();
        let mut body = String::new();
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
        let _ = writeln!(self.block_types, "typedef struct {{");
        if fields.is_empty() {
            let _ = writeln!(self.block_types, "    char none_; /* C has no empty structure */");
        }
        self.block_types.push_str(&fields);
        let _ = writeln!(self.block_types, "}} B_{key}_t;\n");
        let c = &mut self.block_code;
        let _ = writeln!(c, "/* FUNCTION_BLOCK {} */", pou.name.text);
        let _ = writeln!(c, "static void B_{key}_init({p}_t *ctx, B_{key}_t *self, int keep) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;\n    (void)keep;");
        c.push_str(&init);
        let _ = writeln!(c, "}}\n");
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

/// The C type of an elementary type (runtime/iecf.h).
pub(crate) fn c_type(e: Elem) -> String {
    format!("IEC_{}", e.name())
}

#[cfg(test)]
mod tests;

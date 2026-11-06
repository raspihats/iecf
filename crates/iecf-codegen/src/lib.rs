//! iecf's C: a checked project in, the files of docs/c-output.md out.
//!
//! Built in slices (PROPOSAL.md §5 step 2), each turning on harness cases.
//! What a slice does not cover yet is E0009 at the construct, never C
//! that means something else. Covered: one CONFIGURATION, its tasks and
//! PROGRAM instances; variables of the integer, bit-string, BOOL and
//! TIME/LTIME types, located or not, and instances of the standard
//! function blocks; every ST statement; the integer operators,
//! conversions, shifts, ABS, MIN, MAX and LIMIT; + and - on durations;
//! the user's FUNCTIONs with inputs and locals, and FUNCTION_BLOCKs with
//! inputs, outputs, in-outs, locals and instances of other blocks.
//!
//! No I/O, like the rest of the core: it builds for wasm32.

mod expr;
mod names;
mod stmt;

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
    Fb(StdFb),
    /// An instance of the user's FUNCTION_BLOCK, by its upper-cased name.
    Block(String),
}

/// A user's FUNCTION_BLOCK as its callers see it: `B_<NAME>_t`, its
/// inputs, outputs and in-outs (fields `V_<NAME>`, an in-out a pointer).
#[derive(Clone, Debug)]
pub(crate) struct BlockSig {
    pub(crate) params: Vec<(String, Elem, Dir)>,
}

/// A user's FUNCTION as its callers see it: `F_<NAME>(ctx, inputs...)`.
#[derive(Clone, Debug)]
pub(crate) struct FnSig {
    pub(crate) key: String,
    pub(crate) ret: Elem,
    /// Each input: name, type, and its initial value as C, for a call
    /// that does not give it (Ed.3 6.6.1.4).
    pub(crate) inputs: Vec<(String, Elem, String)>,
}

pub(crate) struct Gen<'a> {
    units: &'a [&'a SourceUnit],
    pub(crate) types: &'a Types,
    map: &'a SourceMap,
    pub(crate) diags: Vec<Diagnostic>,
    pub(crate) pous: HashMap<String, &'a Pou>,
    /// The project's prefix: the CONFIGURATION's name in lower case.
    pub(crate) prefix: String,
    /// POU names, in the order of the table `<prefix>_pou_names`.
    pou_names: Vec<String>,
    /// The highest byte addressed plus one, per area I, Q, M.
    pub(crate) extent: [u32; 3],
    /// Set when an expression calls something that can fault.
    pub(crate) may_fault: bool,
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
    /// The math functions called (Ed.3 Table 28): `IECF_SQRT` and `sqrt`.
    pub(crate) math: std::collections::BTreeSet<(String, &'static str)>,
}

impl<'a> Gen<'a> {
    fn new(units: &'a [&'a SourceUnit], types: &'a Types, map: &'a SourceMap) -> Self {
        let mut pous = HashMap::new();
        for u in units {
            for item in &u.items {
                if let Item::Pou(p) = item {
                    pous.insert(p.name.text.to_ascii_uppercase(), p);
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
            pou_names: Vec::new(),
            extent: [0; 3],
            may_fault: false,
            temp: 0,
            functions: HashMap::new(),
            fn_queue: Vec::new(),
            blocks: HashMap::new(),
            block_busy: Vec::new(),
            block_types: String::new(),
            block_code: String::new(),
            globals: HashMap::new(),
            math: Default::default(),
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
        let mut init = String::new(); // body of <p>_init
        for b in &config.globals {
            self.global_block(b, "", &mut globals, &mut fields, &mut init)?;
        }
        for r in &config.resources {
            let rp = r.name.as_ref().map(|n| format!("{}__", n.text.to_ascii_uppercase())).unwrap_or_default();
            for b in &r.globals {
                self.global_block(b, &rp, &mut globals, &mut fields, &mut init)?;
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
                if !pc.args.is_empty() {
                    return self.not_yet(pc.span, "a program instance with connections");
                }
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
                let _ = writeln!(init, "    P_{key}_init(ctx, &ctx->{field});");
                let _ = writeln!(init, "    if (ctx->fault.code != IECF_FAULT_NONE) return;");
                let task = pc.task.as_ref().map(|t| format!("{rp}{}", t.text.to_ascii_uppercase()));
                instances.push(Instance { field, pou: key, task });
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
        let _ = writeln!(h, "\n{}{pou_types}", self.block_types);
        let _ = writeln!(h, "typedef struct {{");
        let _ = writeln!(h, "    iecf_image_t image; /* the platform's, set before {p}_init */");
        let _ = writeln!(h, "    iecf_fault_t fault;");
        let _ = writeln!(h, "    int64_t now_ns;     /* the running task's time */");
        h.push_str(&fields);
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
        let _ = writeln!(h, "\n#endif /* {guard} */");

        // <p>.c
        let mut c = String::new();
        let _ = writeln!(c, "/* {p}.c - written by iecf from CONFIGURATION {}. */", config.name.text);
        let _ = writeln!(c, "#include \"{p}.h\"\n");
        // Table 28's functions from <math.h>, unless the platform gives
        // its own (docs/c-output.md §1); none, no <math.h>.
        for (m, f) in &self.math {
            let _ = writeln!(c, "#ifndef {m}\n#include <math.h>\n#define {m}(x) {f}(x)\n#endif");
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
        let _ = writeln!(c, "void {p}_init({p}_t *ctx) {{");
        let _ = writeln!(c, "    ctx->fault.code = IECF_FAULT_NONE;");
        let _ = writeln!(c, "    ctx->fault.pou = 0u;");
        let _ = writeln!(c, "    ctx->fault.line = 0u;");
        let _ = writeln!(c, "    ctx->now_ns = 0;");
        for ((area, n), field) in ["I", "Q", "M"].iter().zip(self.extent).zip(["i", "q", "m"]) {
            if n > 0 {
                let _ = writeln!(
                    c,
                    "    if (ctx->image.{field} == NULL || ctx->image.{field}_size < {up}_{area}_BYTES) {{\n        iecf_fail(&ctx->fault, IECF_FAULT_IMAGE);\n        return;\n    }}"
                );
            }
        }
        c.push_str(&init);
        let _ = writeln!(c, "}}\n");
        let _ = writeln!(c, "/* No RETAIN variable yet (docs/c-output.md §7): a warm start is a cold one. */");
        let _ = writeln!(c, "void {p}_warm({p}_t *ctx) {{\n    {p}_init(ctx);\n}}\n");
        for (name, _, _) in &tasks {
            let _ = writeln!(c, "void {p}_task_{name}({p}_t *ctx, int64_t now_ns) {{");
            let _ = writeln!(c, "    if (ctx->fault.code != IECF_FAULT_NONE) return;");
            let _ = writeln!(c, "    ctx->now_ns = now_ns;");
            let mine = |i: &&Instance| match &i.task {
                Some(t) => t == name,
                None => name == "_default",
            };
            for i in instances.iter().filter(mine) {
                let _ = writeln!(c, "    P_{}(ctx, &ctx->{});", i.pou, i.field);
                let _ = writeln!(c, "    if (ctx->fault.code != IECF_FAULT_NONE) return;");
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
        let _ = writeln!(c, "const char *const {p}_pou_names[] = {{{}}};", names.join(", "));

        Ok(vec![(format!("{p}.h"), h), (format!("{p}.c"), c), ("iecf.h".into(), IECF_H.into())])
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

    fn global_block(
        &mut self,
        b: &VarBlock,
        rp: &str,
        globals: &mut Scope,
        fields: &mut String,
        init: &mut String,
    ) -> R<()> {
        if b.retain == Some(Retain::Retain) {
            return self.not_yet(b.span, "VAR_GLOBAL RETAIN");
        }
        for d in &b.decls {
            let Decl::Elem(ty) = self.decl(d)? else {
                return self.not_yet(d.ty.span(), "a global function block instance");
            };
            for n in &d.names {
                let key = n.text.to_ascii_uppercase();
                let place = match &d.location {
                    Some(a) => self.located(a, ty)?,
                    None => {
                        let field = format!("V_{rp}{key}");
                        let _ = writeln!(fields, "    {} {field};", c_type(ty));
                        Place::Field(format!("ctx->{field}"), ty)
                    }
                };
                self.init_var(&place, d, init)?;
                globals.vars.insert(key, place);
            }
        }
        Ok(())
    }

    /// What a declaration declares, if this slice compiles it.
    pub(crate) fn decl(&mut self, d: &VarDecl) -> R<Decl> {
        self.decl_of(d.span, d.ty.span())
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
    fn init_var(&mut self, place: &Place, d: &VarDecl, out: &mut String) -> R<()> {
        let ty = match place {
            Place::Fb(lv, fb) => {
                if d.init.is_some() {
                    return self.not_yet(d.span, "an initial value of a function block instance");
                }
                let _ = writeln!(out, "    iecf_{}_init(&{lv});", fb.stem);
                return Ok(());
            }
            Place::Block(lv, key) => {
                if d.init.is_some() {
                    return self.not_yet(d.span, "an initial value of a function block instance");
                }
                let _ = writeln!(out, "    B_{key}_init(ctx, &{lv});");
                return Ok(());
            }
            Place::Field(_, t) | Place::Located { ty: t, .. } => *t,
        };
        let value = match &d.init {
            Some(Init::Expr(e)) => {
                self.may_fault = false;
                let code = self.value_as(e, ty, &Scope::default())?;
                if self.may_fault {
                    return self.not_yet(e.span, "an initial value that can fault");
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

    /// `name AT %QW2`: where it is, and the extent it adds.
    pub(crate) fn located(&mut self, a: &DirectAddress, ty: Elem) -> R<Place> {
        let Some((area, size, byte, bit)) = names::parse_address(&a.text) else {
            return self.not_yet(a.span, format!("the address {}", a.text));
        };
        let width = match size {
            'X' => 1,
            'B' => 1,
            'W' => 2,
            'D' => 4,
            _ => 8,
        };
        let fits = match size {
            'X' => ty == Elem::Bool,
            'B' => matches!(ty, Elem::Byte | Elem::Sint | Elem::Usint),
            'W' => matches!(ty, Elem::Word | Elem::Int | Elem::Uint),
            'D' => matches!(ty, Elem::Dword | Elem::Dint | Elem::Udint | Elem::Real),
            _ => matches!(ty, Elem::Lword | Elem::Lint | Elem::Ulint | Elem::Lreal),
        };
        if !fits {
            return self.not_yet(a.span, format!("{} at {}", ty.name(), a.text));
        }
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
        if !pou.sfc.is_empty() {
            return self.not_yet(pou.name.span, "SFC");
        }
        let mut scope = Scope::default();
        let mut fields = String::new();
        let mut init = String::new();
        for b in &pou.var_blocks {
            match b.kind {
                VarKind::Var => {}
                VarKind::Temp => return self.not_yet(b.span, "VAR_TEMP in a PROGRAM"),
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
                _ => return self.not_yet(b.span, "a PROGRAM's inputs, outputs or in-outs"),
            }
            if b.retain == Some(Retain::Retain) {
                return self.not_yet(b.span, "VAR RETAIN");
            }
            for d in &b.decls {
                let decl = self.decl(d)?;
                if d.names.is_empty() {
                    return self.not_yet(d.span, "a located variable without a name");
                }
                for n in &d.names {
                    let k = n.text.to_ascii_uppercase();
                    let place = match (&decl, &d.location) {
                        (Decl::Elem(ty), Some(a)) => self.located(a, *ty)?,
                        (Decl::Elem(ty), None) => {
                            let _ = writeln!(fields, "    {} V_{k};", c_type(*ty));
                            Place::Field(format!("self->V_{k}"), *ty)
                        }
                        (Decl::Fb(fb), None) => {
                            let _ = writeln!(fields, "    iecf_{}_t V_{k};", fb.stem);
                            Place::Fb(format!("self->V_{k}"), fb.clone())
                        }
                        (Decl::Block(key), None) => {
                            let _ = writeln!(fields, "    B_{key}_t V_{k};");
                            Place::Block(format!("self->V_{k}"), key.clone())
                        }
                        (Decl::Fb(_) | Decl::Block(_), Some(a)) => {
                            return self.not_yet(a.span, "a function block at an address");
                        }
                    };
                    self.init_var(&place, d, &mut init)?;
                    scope.vars.insert(k, place);
                }
            }
        }
        // Globals the program does not name stay out of its scope: only
        // VAR_EXTERNAL brings one in (Ed.3 6.5.2.2).
        let mut t = String::new();
        let _ = writeln!(t, "typedef struct {{");
        if fields.is_empty() {
            let _ = writeln!(t, "    char none_; /* C has no empty structure */");
        }
        t.push_str(&fields);
        let _ = writeln!(t, "}} P_{key}_t;\n");

        let p = self.prefix.clone();
        let mut c = String::new();
        let _ = writeln!(c, "/* PROGRAM {} */", pou.name.text);
        let _ = writeln!(c, "static void P_{key}_init({p}_t *ctx, P_{key}_t *self) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
        c.push_str(&init);
        let _ = writeln!(c, "}}\n");
        let mut body = String::new();
        let mut cx = stmt::Body {
            pou: index,
            scope: &scope,
            out: &mut body,
            depth: 1,
            loops: Vec::new(),
            ret: "return;".into(),
        };
        self.stmts(&pou.body, &mut cx)?;
        let _ = writeln!(c, "static void P_{key}({p}_t *ctx, P_{key}_t *self) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
        c.push_str(&body);
        let _ = writeln!(c, "}}\n");
        Ok((t, c))
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
                Decl::Fb(_) | Decl::Block(_) => return self.not_yet(rt.span(), "this result type"),
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
        let sig = FnSig { key: key.to_string(), ret, inputs };
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
        let params: Vec<String> = sig.inputs.iter().map(|(n, t, _)| format!("{} V_{n}", c_type(*t))).collect();
        let head = format!(
            "static {rt} F_{key}({p}_t *ctx{}{})",
            if params.is_empty() { "" } else { ", " },
            params.join(", ")
        );

        let mut scope = Scope::default();
        let mut decls = String::new();
        let _ = writeln!(decls, "    {rt} V_{key} = {};", expr::zero(sig.ret));
        let _ = writeln!(decls, "    (void)ctx;");
        scope.vars.insert(key.to_string(), Place::Field(format!("V_{key}"), sig.ret));
        for (n, t, _) in &sig.inputs {
            let _ = writeln!(decls, "    (void)V_{n};");
            scope.vars.insert(n.clone(), Place::Field(format!("V_{n}"), *t));
        }
        for b in pou.var_blocks.iter().filter(|b| matches!(b.kind, VarKind::Var | VarKind::Temp)) {
            for d in &b.decls {
                let Decl::Elem(ty) = self.decl(d)? else {
                    return self.not_yet(d.span, "a function block instance in a FUNCTION");
                };
                if d.location.is_some() {
                    return self.not_yet(d.span, "an address in a FUNCTION");
                }
                let value = match &d.init {
                    Some(Init::Expr(e)) => {
                        self.may_fault = false;
                        let v = self.value_as(e, ty, &Scope::default())?;
                        if self.may_fault {
                            return self.not_yet(e.span, "an initial value that can fault");
                        }
                        v
                    }
                    Some(_) => return self.not_yet(d.span, "this initial value"),
                    None => expr::zero(ty),
                };
                for n in &d.names {
                    let k = n.text.to_ascii_uppercase();
                    let _ = writeln!(decls, "    {} V_{k} = {value};\n    (void)V_{k};", c_type(ty));
                    scope.vars.insert(k.clone(), Place::Field(format!("V_{k}"), ty));
                }
            }
        }
        let mut body = String::new();
        let ret = format!("return V_{key};");
        let mut cx =
            stmt::Body { pou: index, scope: &scope, out: &mut body, depth: 1, loops: Vec::new(), ret: ret.clone() };
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
        if !pou.sfc.is_empty() {
            return self.not_yet(pou.name.span, "SFC");
        }
        self.block_busy.push(key.to_string());
        let mut scope = Scope::default();
        let mut fields = String::new();
        let mut init = String::new();
        let mut params = Vec::new();
        for b in &pou.var_blocks {
            if b.retain == Some(Retain::Retain) {
                return self.not_yet(b.span, "RETAIN");
            }
            let dir = match b.kind {
                VarKind::Input => Some(Dir::Input),
                VarKind::Output => Some(Dir::Output),
                VarKind::InOut => Some(Dir::InOut),
                VarKind::Var => None,
                VarKind::Temp => return self.not_yet(b.span, "VAR_TEMP in a FUNCTION_BLOCK"),
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
                            let _ = writeln!(fields, "    B_{inner}_t V_{k};");
                            Place::Block(format!("self->V_{k}"), inner.clone())
                        }
                        _ => return self.not_yet(d.span, "this declaration in a FUNCTION_BLOCK"),
                    };
                    if let (Some(dir), Some(ty)) = (dir, place.elem()) {
                        params.push((k.clone(), ty, dir));
                    }
                    self.init_var(&place, d, &mut init)?;
                    scope.vars.insert(k, place);
                }
            }
        }
        self.blocks.insert(key.to_string(), BlockSig { params });
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
        };
        let r = self.stmts(&pou.body, &mut cx);
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
        let _ = writeln!(c, "static void B_{key}_init({p}_t *ctx, B_{key}_t *self) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
        c.push_str(&init);
        let _ = writeln!(c, "}}\n");
        let _ = writeln!(c, "static void B_{key}({p}_t *ctx, B_{key}_t *self) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
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

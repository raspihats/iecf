//! iecf's C: a checked project in, the files of docs/c-output.md out.
//!
//! Built in slices (PROPOSAL.md §5 step 2), each turning on harness cases.
//! What a slice does not cover yet is E0009 at the construct, never C
//! that means something else. This slice: one CONFIGURATION, its tasks
//! and PROGRAM instances, variables of the integer, bit-string and BOOL
//! types, located or not, and every ST statement over them; the integer
//! operators, conversions, shifts, ABS, MIN, MAX and LIMIT.
//!
//! No I/O, like the rest of the core: it builds for wasm32.

mod expr;
mod names;
mod stmt;

use iecf_diagnostics::{Code, Diagnostic, SourceMap, Span};
use iecf_sema::Types;
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::*;
use names::{Place, Scope};
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

pub(crate) struct Gen<'a> {
    units: &'a [&'a SourceUnit],
    pub(crate) types: &'a Types,
    map: &'a SourceMap,
    pub(crate) diags: Vec<Diagnostic>,
    pous: HashMap<String, &'a Pou>,
    /// The project's prefix: the CONFIGURATION's name in lower case.
    pub(crate) prefix: String,
    /// POU names, in the order of the table `<prefix>_pou_names`.
    pou_names: Vec<String>,
    /// The highest byte addressed plus one, per area I, Q, M.
    pub(crate) extent: [u32; 3],
    /// Set when an expression calls a helper that can fault.
    pub(crate) may_fault: bool,
    /// For unique C temporaries.
    pub(crate) temp: u32,
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

        // <p>.h
        let mut h = String::new();
        let guard = format!("{up}_H");
        let _ = writeln!(h, "/* {p}.h - written by iecf from CONFIGURATION {}. */", config.name.text);
        let _ = writeln!(h, "#ifndef {guard}\n#define {guard}\n\n#include \"iecf.h\"\n");
        for (area, n) in ["I", "Q", "M"].iter().zip(self.extent) {
            let _ = writeln!(h, "#define {up}_{area}_BYTES {n}u");
        }
        let _ = writeln!(h, "\n{pou_types}");
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
            let ty = self.decl_type(d)?;
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
                self.init_var(&place, d, ty, init)?;
                globals.vars.insert(key, place);
            }
        }
        Ok(())
    }

    /// The elementary type of a declaration this slice compiles.
    pub(crate) fn decl_type(&mut self, d: &VarDecl) -> R<Elem> {
        match self.types.decl(d.span) {
            Some(Ty::Elem(e)) if e.is_int() || e.is_bit() => Ok(*e),
            Some(t) => {
                let t = t.to_string();
                self.not_yet(d.ty.span(), format!("a variable of type {t}"))
            }
            None => self.not_yet(d.ty.span(), "this declaration"),
        }
    }

    /// Code setting a variable to its initial value, into `out`.
    fn init_var(&mut self, place: &Place, d: &VarDecl, ty: Elem, out: &mut String) -> R<()> {
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
    fn located(&mut self, a: &DirectAddress, ty: Elem) -> R<Place> {
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
            'D' => matches!(ty, Elem::Dword | Elem::Dint | Elem::Udint),
            _ => matches!(ty, Elem::Lword | Elem::Lint | Elem::Ulint),
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
                VarKind::Var | VarKind::Temp => {}
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
                let ty = self.decl_type(d)?;
                if d.names.is_empty() {
                    return self.not_yet(d.span, "a located variable without a name");
                }
                for n in &d.names {
                    let k = n.text.to_ascii_uppercase();
                    let place = match &d.location {
                        Some(a) => self.located(a, ty)?,
                        None => {
                            let _ = writeln!(fields, "    {} V_{k};", c_type(ty));
                            Place::Field(format!("self->V_{k}"), ty)
                        }
                    };
                    self.init_var(&place, d, ty, &mut init)?;
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
        let mut cx = stmt::Body { pou: index, scope: &scope, out: &mut body, depth: 1, loops: Vec::new() };
        self.stmts(&pou.body, &mut cx)?;
        let _ = writeln!(c, "static void P_{key}({p}_t *ctx, P_{key}_t *self) {{");
        let _ = writeln!(c, "    (void)ctx;\n    (void)self;");
        c.push_str(&body);
        let _ = writeln!(c, "}}\n");
        Ok((t, c))
    }
}

/// The C type of an elementary type (runtime/iecf.h).
pub(crate) fn c_type(e: Elem) -> String {
    format!("IEC_{}", e.name())
}

#[cfg(test)]
mod tests;

//! CODESYS's and TwinCAT's programs as single instances (the
//! program-instances extension): a PROGRAM is its one instance, read and
//! written by its name from any POU, `PRG.x`, called, `PRG(...)`, and
//! given methods, `PRG.m(...)`. Ed.3 instantiates a program only in a
//! resource (6.6.4, 3) and has none of these.
//!
//! Written so before the check, after the namespaces are taken out: a
//! PROGRAM that some POU names so, or that declares a METHOD, is a
//! FUNCTION_BLOCK of its name, and its one instance a global variable,
//! `__PRG_<name>`, which every `PRG.x` and `PRG(...)` names; each POU
//! reaches it as it reaches a global it declares VAR_EXTERNAL (check.rs
//! `implicit_external`). A CONFIGURATION's `PROGRAM p WITH t : PRG(...)`
//! runs a PROGRAM of its own, `__PRG_RUN_<name>`, which calls that
//! instance with the connections given: a task and the POUs that name
//! it share the one instance, as in CODESYS. Each instance is RETAIN
//! when the configuration declares it so.
//!
//! A PROGRAM with what only a program has - a VAR_GLOBAL of its own, a
//! VAR_ACCESS, a partly located variable (`AT %I*`), a function block
//! instance the configuration gives a task of its own - is left as it is,
//! and naming it so stays an error.

use crate::codesys_oo::{expr, walk_pou};
use iecf_syntax::ast::*;
use std::collections::{HashMap, HashSet};

/// The one instance of a PROGRAM `name` made a block.
pub const INSTANCE: &str = "__PRG_";
/// The PROGRAM a configuration runs it with.
pub const RUN: &str = "__PRG_RUN_";

/// The global instance of the PROGRAM `name`.
pub fn instance(name: &str) -> String {
    format!("{INSTANCE}{}", name.to_ascii_uppercase())
}

/// Whether some PROGRAM is named by `PRG.x` or `PRG(...)`, or declares
/// a METHOD.
pub fn needed(units: &[&SourceUnit]) -> bool {
    let mut copy: Vec<SourceUnit> = units.iter().map(|u| (*u).clone()).collect();
    !made_blocks(&mut copy).is_empty()
}

/// The PROGRAMs made blocks, as above, each POU's `PRG.x` and
/// `PRG(...)` naming its instance, and the configurations running them.
pub fn rewrite(units: &mut [SourceUnit]) {
    let made = made_blocks(units);
    if made.is_empty() {
        return;
    }
    // Each use of a program's name.
    each(units, &mut |e| {
        if let Some(id) = program_name(e, &made) {
            id.text = instance(&id.text);
        }
    });
    // Each configuration's instances: run by a PROGRAM of their own; the
    // global instance RETAIN when one is.
    let mut runs: HashMap<String, Pou> = HashMap::new();
    let mut retained: HashSet<String> = HashSet::new();
    let pous: HashMap<String, Pou> = all_items(units)
        .filter_map(|i| match i {
            Item::Pou(p) if made.contains(&p.name.text.to_ascii_uppercase()) => {
                Some((p.name.text.to_ascii_uppercase(), p.clone()))
            }
            _ => None,
        })
        .collect();
    for unit in units.iter_mut() {
        for item in unit.items.iter_mut() {
            let Item::Configuration(c) = item else { continue };
            for pc in c.resources.iter_mut().flat_map(|r| r.programs.iter_mut()) {
                let key = pc.program_type.text.to_ascii_uppercase();
                let Some(p) = pous.get(&key) else { continue };
                if pc.retain == Some(Retain::Retain) {
                    retained.insert(key.clone());
                }
                runs.entry(key.clone()).or_insert_with(|| run_program(p, &pc.args));
                pc.program_type.text = format!("{RUN}{key}");
            }
        }
    }
    // Each program a block, its instance after it.
    fn blocks(
        items: &mut Vec<Item>,
        made: &HashSet<String>,
        retained: &HashSet<String>,
        runs: &mut HashMap<String, Pou>,
    ) {
        let mut out = Vec::with_capacity(items.len());
        for item in std::mem::take(items) {
            match item {
                Item::Pou(mut p) if p.kind == PouKind::Program && made.contains(&p.name.text.to_ascii_uppercase()) => {
                    let key = p.name.text.to_ascii_uppercase();
                    p.kind = PouKind::FunctionBlock;
                    let global = global_instance(&p, retained.contains(&key));
                    let run = runs.remove(&key);
                    out.push(Item::Pou(p));
                    out.push(Item::Globals(global));
                    if let Some(r) = run {
                        out.push(Item::Pou(r));
                    }
                }
                Item::Namespace(mut n) => {
                    blocks(&mut n.items, made, retained, runs);
                    out.push(Item::Namespace(n));
                }
                other => out.push(other),
            }
        }
        *items = out;
    }
    for unit in units.iter_mut() {
        blocks(&mut unit.items, &made, &retained, &mut runs);
    }
}

/// The PROGRAMs to make blocks: named so, or with a METHOD, and with
/// nothing only a program has.
fn made_blocks(units: &mut [SourceUnit]) -> HashSet<String> {
    let mut programs: HashSet<String> = HashSet::new();
    let mut fb_tasks: HashSet<String> = HashSet::new();
    for item in all_items(units) {
        match item {
            Item::Pou(p) if p.kind == PouKind::Program => {
                programs.insert(p.name.text.to_ascii_uppercase());
            }
            Item::Configuration(c) => {
                for pc in c.resources.iter().flat_map(|r| &r.programs) {
                    if !pc.fb_tasks.is_empty() {
                        fb_tasks.insert(pc.program_type.text.to_ascii_uppercase());
                    }
                }
            }
            _ => {}
        }
    }
    let convertible = |k: &String, p: &[Item]| {
        !fb_tasks.contains(k)
            && p.iter().any(|i| matches!(i, Item::Pou(q) if q.name.text.eq_ignore_ascii_case(k) && !only_a_program(q)))
    };
    let items: Vec<Item> = all_items(units).cloned().collect();
    let mut made: HashSet<String> = HashSet::new();
    for k in programs.iter() {
        let has_methods = items
            .iter()
            .any(|i| matches!(i, Item::Pou(q) if q.name.text.eq_ignore_ascii_case(k) && !q.methods.is_empty()));
        if has_methods && convertible(k, &items) {
            made.insert(k.clone());
        }
    }
    let names = programs;
    let mut named: HashSet<String> = HashSet::new();
    each(units, &mut |e| {
        if let Some(id) = program_name(e, &names) {
            named.insert(id.text.to_ascii_uppercase());
        }
    });
    for k in named {
        if convertible(&k, &items) {
            made.insert(k);
        }
    }
    made
}

/// Whether `p` has what only a program has: a VAR_GLOBAL, a VAR_ACCESS,
/// a partly located variable.
fn only_a_program(p: &Pou) -> bool {
    !p.accesses.is_empty()
        || p.var_blocks.iter().any(|b| {
            b.kind == VarKind::Global
                || b.decls.iter().any(|d| d.location.as_ref().is_some_and(|l| l.text.contains('*')))
        })
}

/// The program's name `e` holds as `PRG.x`'s base or `PRG(...)`'s
/// callee, `PRG` one of `names`.
fn program_name<'e>(e: &'e mut Expr, names: &HashSet<String>) -> Option<&'e mut Ident> {
    let base = match &mut e.kind {
        ExprKind::Member(b, _) => b,
        ExprKind::Call { callee, .. } => callee,
        _ => return None,
    };
    match &mut base.kind {
        ExprKind::Name(id) if names.contains(&id.text.to_ascii_uppercase()) => Some(id),
        _ => None,
    }
}

/// `VAR_GLOBAL __PRG_<name> : <name>; END_VAR`.
fn global_instance(p: &Pou, retain: bool) -> VarBlock {
    let at = p.name.span;
    VarBlock {
        kind: VarKind::Global,
        constant: false,
        input_constant: None,
        retain: retain.then_some(Retain::Retain),
        access: None,
        stat: None,
        inst: None,
        decls: vec![VarDecl {
            names: vec![Ident { text: instance(&p.name.text), span: at }],
            location: None,
            ty: TypeSpec::Named(p.name.clone()),
            edge: None,
            init: None,
            fb_args: None,
            span: at,
        }],
        span: at,
    }
}

/// `PROGRAM __PRG_RUN_<name>`: the inputs and in-outs the
/// configuration connects, `args`, and every output, as its own, and a
/// call of the instance with them.
fn run_program(p: &Pou, args: &[Arg]) -> Pou {
    let at = p.name.span;
    let connected: HashSet<String> = args
        .iter()
        .filter_map(|a| match a {
            Arg::Input(n, _) => Some(n.text.to_ascii_uppercase()),
            Arg::Output { name, .. } => Some(name.text.to_ascii_uppercase()),
            Arg::Positional(_) => None,
        })
        .collect();
    let mut var_blocks = Vec::new();
    let mut call_args = Vec::new();
    for b in p.var_blocks.iter().filter(|b| matches!(b.kind, VarKind::Input | VarKind::Output | VarKind::InOut)) {
        let mut b = b.clone();
        b.retain = None;
        for d in b.decls.iter_mut() {
            // Every output, which a TASK may read (`P.OUT`); the inputs
            // the configuration gives, the others the instance's own.
            if b.kind != VarKind::Output {
                d.names.retain(|n| connected.contains(&n.text.to_ascii_uppercase()));
            }
            d.edge = None;
            d.location = None;
            d.fb_args = None;
        }
        b.decls.retain(|d| !d.names.is_empty());
        for n in b.decls.iter().flat_map(|d| &d.names) {
            let own = Expr { kind: ExprKind::Name(n.clone()), span: n.span };
            call_args.push(match b.kind {
                VarKind::Output => Arg::Output { not: false, name: n.clone(), target: own },
                _ => Arg::Input(n.clone(), own),
            });
        }
        if !b.decls.is_empty() {
            var_blocks.push(b);
        }
    }
    let name = Ident { text: instance(&p.name.text), span: at };
    let call = Expr {
        kind: ExprKind::Call { callee: Box::new(Expr { kind: ExprKind::Name(name), span: at }), args: call_args },
        span: at,
    };
    Pou {
        kind: PouKind::Program,
        name: Ident { text: format!("{RUN}{}", p.name.text.to_ascii_uppercase()), span: at },
        return_type: None,
        var_blocks,
        accesses: Vec::new(),
        body: vec![Stmt { kind: StmtKind::Call(call), span: at }],
        sfc: Vec::new(),
        internal: false,
        using: Vec::new(),
        specs: Specs::default(),
        methods: Vec::new(),
        extends: None,
        interfaces: Vec::new(),
        header_semicolon: None,
        span: at,
    }
}

/// Every item of `units`, a namespace's too.
fn all_items(units: &[SourceUnit]) -> impl Iterator<Item = &Item> {
    fn flat<'a>(items: &'a [Item], out: &mut Vec<&'a Item>) {
        for i in items {
            out.push(i);
            if let Item::Namespace(n) = i {
                flat(&n.items, out);
            }
        }
    }
    let mut out = Vec::new();
    for u in units {
        flat(&u.items, &mut out);
    }
    out.into_iter()
}

/// Every expression of `units`: the POUs', the initial values and
/// FB_init arguments of their variables, of global lists and of
/// structures' fields.
fn each(units: &mut [SourceUnit], f: &mut dyn FnMut(&mut Expr)) {
    fn decls(ds: &mut [VarDecl], f: &mut dyn FnMut(&mut Expr)) {
        for d in ds.iter_mut() {
            if let Some(i) = &mut d.init {
                init(i, f);
            }
            if let Some((args, _)) = &mut d.fb_args {
                for a in args.iter_mut() {
                    match a {
                        Arg::Positional(x) | Arg::Input(_, x) | Arg::Output { target: x, .. } => expr(x, f),
                    }
                }
            }
        }
    }
    fn init(i: &mut Init, f: &mut dyn FnMut(&mut Expr)) {
        match i {
            Init::Expr(e) => expr(e, f),
            Init::Array { elems, .. } => {
                for el in elems.iter_mut() {
                    if let Some(r) = &mut el.repeat {
                        expr(r, f);
                    }
                    if let Some(v) = &mut el.value {
                        init(v, f);
                    }
                    el.more.iter_mut().for_each(|m| init(m, f));
                }
            }
            Init::Struct { fields, .. } => fields.iter_mut().for_each(|(_, v)| init(v, f)),
        }
    }
    fn pou(p: &mut Pou, f: &mut dyn FnMut(&mut Expr)) {
        for b in p.var_blocks.iter_mut() {
            for d in b.decls.iter_mut() {
                if let Some(i) = &mut d.init {
                    init(i, f);
                }
            }
        }
        for m in p.methods.iter_mut() {
            for b in m.var_blocks.iter_mut() {
                for d in b.decls.iter_mut() {
                    if let Some(i) = &mut d.init {
                        init(i, f);
                    }
                }
            }
        }
        // The bodies, and the FB_init arguments (codesys_oo.rs).
        walk_pou(p, f);
    }
    fn items(list: &mut [Item], f: &mut dyn FnMut(&mut Expr)) {
        for item in list.iter_mut() {
            match item {
                Item::Pou(p) => pou(p, f),
                Item::Globals(b) => decls(&mut b.decls, f),
                Item::Types(ts) => {
                    for t in ts.iter_mut() {
                        if let TypeDef::Struct { fields, .. } = &mut t.def {
                            decls(fields, f);
                        }
                    }
                }
                Item::Namespace(n) => items(&mut n.items, f),
                _ => {}
            }
        }
    }
    for unit in units.iter_mut() {
        items(&mut unit.items, f);
    }
}

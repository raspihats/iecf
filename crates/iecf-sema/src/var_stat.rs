//! CODESYS's and TwinCAT's static variables (the var-stat extension):
//! `VAR_STAT ... END_VAR` in a FUNCTION, a function block, a class, a
//! PROGRAM or a method declares variables that exist once for the POU,
//! however many instances it has, and keep their values from one call
//! to the next. Written as Ed.3 writes such a variable before anything
//! is checked: a global variable named by the POU's whole path, `T.x`
//! (`T__x`, as namespaces.rs names an element; a method's `T.M.x`),
//! which the POU declares VAR_EXTERNAL and uses by that name. A block
//! extending T reaches T's through T's VAR_EXTERNAL, as it reaches any
//! of T's variables; a method's are its own.

use crate::codesys_oo::{stmts, var_names};
use crate::namespaces::SEP;
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_syntax::ast::*;
use std::collections::HashMap;

/// Static variables: each name, upper-cased, and its global's name.
type Statics = Vec<(String, String)>;

/// Whether some POU has a VAR_STAT section.
pub fn needed(units: &[&SourceUnit]) -> bool {
    fn pou(p: &Pou) -> bool {
        p.var_blocks.iter().any(|b| b.stat.is_some()) || p.methods.iter().any(pou)
    }
    units.iter().any(|u| u.items.iter().any(|i| matches!(i, Item::Pou(p) if pou(p))))
}

/// Each VAR_STAT section made global variables, its POU's uses of them
/// renamed; a static variable named as another variable of its POU is
/// E0012.
pub fn rewrite(units: &mut [SourceUnit]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // Each block's and class's base and its static variables' global
    // names, for the blocks extending it.
    let mut types: HashMap<String, (Option<String>, Statics)> = HashMap::new();
    for unit in units.iter() {
        for item in &unit.items {
            if let Item::Pou(p) = item
                && matches!(p.kind, PouKind::FunctionBlock | PouKind::Class)
            {
                let stat = statics(&p.name.text, &p.var_blocks);
                types.insert(upper(&p.name.text), (p.extends.as_ref().map(|b| upper(&b.text)), stat));
            }
        }
    }
    let inherited = |base: Option<&String>| {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut at = base.cloned();
        let mut depth = 0;
        while let Some(t) = at {
            depth += 1;
            let Some((up, stat)) = types.get(&t) else { break };
            out.extend(stat.iter().cloned());
            at = if depth > 64 { None } else { up.clone() };
        }
        out
    };
    for unit in units.iter_mut() {
        let mut globals = Vec::new();
        for item in unit.items.iter_mut() {
            let Item::Pou(p) = item else { continue };
            let path = p.name.text.clone();
            let base = p.extends.as_ref().map(|b| upper(&b.text));
            let mut map = take(&path, &mut p.var_blocks, &mut globals, &mut diags);
            let own = var_names(&p.var_blocks);
            if matches!(p.kind, PouKind::FunctionBlock | PouKind::Class) {
                for (k, g) in inherited(base.as_ref()) {
                    if !own.contains(&k) && !map.iter().any(|(m, _)| *m == k) {
                        map.push((k, g));
                    }
                }
            }
            rename(&mut p.body, &map);
            for el in p.sfc.iter_mut() {
                match el {
                    SfcElement::Action(a) => rename(&mut a.body, &map),
                    SfcElement::Transition(t) => {
                        let mut body = vec![Stmt { kind: StmtKind::Call(t.condition.clone()), span: t.condition.span }];
                        rename(&mut body, &map);
                        if let Some(Stmt { kind: StmtKind::Call(c), .. }) = body.pop() {
                            t.condition = c;
                        }
                    }
                    SfcElement::Step(_) => {}
                }
            }
            for m in p.methods.iter_mut() {
                let mpath = format!("{path}{SEP}{}", m.name.text);
                let mine = take(&mpath, &mut m.var_blocks, &mut globals, &mut diags);
                let mut hidden = var_names(&m.var_blocks);
                hidden.push(upper(&m.name.text));
                let mut seen: Vec<(String, String)> = mine;
                for (k, g) in &map {
                    if !hidden.contains(k) && !seen.iter().any(|(m, _)| m == k) {
                        seen.push((k.clone(), g.clone()));
                    }
                }
                rename(&mut m.body, &seen);
            }
        }
        // The globals first, as a VAR_GLOBAL at the top of the file.
        for b in globals.into_iter().rev() {
            unit.items.insert(0, Item::Globals(b));
        }
    }
    diags
}

/// The static variables `blocks` declare: each name, upper-cased, and
/// its global's name under `path`.
fn statics(path: &str, blocks: &[VarBlock]) -> Vec<(String, String)> {
    blocks
        .iter()
        .filter(|b| b.stat.is_some())
        .flat_map(|b| b.decls.iter().flat_map(|d| d.names.iter()))
        .map(|n| (upper(&n.text), global(path, &n.text)))
        .collect()
}

fn global(path: &str, name: &str) -> String {
    format!("{path}{SEP}{name}")
}

/// `blocks`' VAR_STAT sections taken out: each a VAR_GLOBAL pushed on
/// `globals` and a VAR_EXTERNAL of it left in its place. Gives each
/// static variable's name and its global's.
fn take(
    path: &str,
    blocks: &mut [VarBlock],
    globals: &mut Vec<VarBlock>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<(String, String)> {
    let map = statics(path, blocks);
    if map.is_empty() {
        return map;
    }
    let others = var_names(&blocks.iter().filter(|b| b.stat.is_none()).cloned().collect::<Vec<_>>());
    for b in blocks.iter_mut().filter(|b| b.stat.is_some()) {
        for d in b.decls.iter() {
            for n in d.names.iter().filter(|n| others.contains(&upper(&n.text))) {
                diags.push(Diagnostic::new(
                    Code::Duplicate,
                    n.span,
                    format!("'{}' is declared twice: a VAR_STAT's variable is one of its POU's", n.text),
                ));
            }
        }
        let renamed = |d: &VarDecl, external: bool| {
            let mut d = d.clone();
            for n in d.names.iter_mut() {
                n.text = global(path, &n.text);
                if external {
                    // A place of its own: what is checked is keyed by it.
                    n.span = Span::new(n.span.file, n.span.start, n.span.start);
                }
            }
            if external {
                d.init = None;
                d.fb_args = None;
                d.location = None;
                d.span = Span::new(d.span.file, d.span.start, d.span.start);
            }
            d
        };
        globals.push(VarBlock {
            kind: VarKind::Global,
            stat: None,
            access: None,
            input_constant: None,
            decls: b.decls.iter().map(|d| renamed(d, false)).collect(),
            ..b.clone()
        });
        *b = VarBlock {
            kind: VarKind::External,
            stat: None,
            access: None,
            input_constant: None,
            retain: None,
            decls: b.decls.iter().map(|d| renamed(d, true)).collect(),
            ..b.clone()
        };
    }
    map
}

/// Each use in `body` of a name of `map` written as its global's name,
/// a FOR's control variable too.
fn rename(body: &mut [Stmt], map: &[(String, String)]) {
    if map.is_empty() {
        return;
    }
    let to = |id: &mut Ident| {
        let k = upper(&id.text);
        if let Some((_, g)) = map.iter().find(|(m, _)| *m == k) {
            id.text = g.clone();
        }
    };
    stmts(body, &mut |e| {
        if let ExprKind::Name(id) = &mut e.kind {
            to(id);
        }
    });
    for_vars(body, &to);
}

fn for_vars(body: &mut [Stmt], to: &dyn Fn(&mut Ident)) {
    for s in body {
        match &mut s.kind {
            StmtKind::For { var, body, .. } => {
                to(var);
                for_vars(body, to);
            }
            StmtKind::If { branches, else_body } => {
                for (_, b) in branches.iter_mut() {
                    for_vars(b, to);
                }
                if let Some(b) = else_body {
                    for_vars(b, to);
                }
            }
            StmtKind::Case { arms, else_body, .. } => {
                for a in arms.iter_mut() {
                    for_vars(&mut a.body, to);
                }
                if let Some(b) = else_body {
                    for_vars(b, to);
                }
            }
            StmtKind::While { body, .. } | StmtKind::Repeat { body, .. } => for_vars(body, to),
            _ => {}
        }
    }
}

fn upper(s: &str) -> String {
    s.to_ascii_uppercase()
}

/// Whether some method has a VAR_INST section.
pub fn inst_needed(units: &[&SourceUnit]) -> bool {
    units.iter().any(|u| {
        u.items.iter().any(
            |i| matches!(i, Item::Pou(p) if p.methods.iter().any(|m| m.var_blocks.iter().any(|b| b.inst.is_some()))),
        )
    })
}

/// CODESYS's VAR_INST (the var-inst extension): a method's variables
/// kept in its instance, not reset from call to call. Each is written as
/// a VAR of its block named by the block's and method's path, `T.M.x`
/// (`T__M__x`), set up once with the instance, and the method's uses of
/// `x` are that variable's. One named as another of the method's
/// variables is E0012. A VAR_INST outside a method is left for the
/// checker to refuse.
pub fn rewrite_inst(units: &mut [SourceUnit]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for unit in units.iter_mut() {
        for item in unit.items.iter_mut() {
            let Item::Pou(p) = item else { continue };
            if !matches!(p.kind, PouKind::FunctionBlock | PouKind::Class) {
                continue;
            }
            let mut kept = Vec::new();
            for m in p.methods.iter_mut() {
                if !m.var_blocks.iter().any(|b| b.inst.is_some()) {
                    continue;
                }
                let path = format!("{}{SEP}{}", p.name.text, m.name.text);
                let others = var_names(&m.var_blocks.iter().filter(|b| b.inst.is_none()).cloned().collect::<Vec<_>>());
                let mut map: Statics = Vec::new();
                let (inst, rest): (Vec<VarBlock>, Vec<VarBlock>) =
                    std::mem::take(&mut m.var_blocks).into_iter().partition(|b| b.inst.is_some());
                m.var_blocks = rest;
                for mut b in inst {
                    for d in b.decls.iter_mut() {
                        for n in d.names.iter_mut() {
                            if others.contains(&upper(&n.text)) {
                                diags.push(Diagnostic::new(
                                    Code::Duplicate,
                                    n.span,
                                    format!(
                                        "'{}' is declared twice: a VAR_INST's variable is one of its method's",
                                        n.text
                                    ),
                                ));
                            }
                            let g = global(&path, &n.text);
                            map.push((upper(&n.text), g.clone()));
                            n.text = g;
                        }
                    }
                    b.inst = None;
                    kept.push(b);
                }
                rename(&mut m.body, &map);
            }
            p.var_blocks.extend(kept);
        }
    }
    diags
}

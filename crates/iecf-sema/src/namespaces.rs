//! Namespaces (Ed.3 6.9), taken out before the check: each element of a
//! namespace becomes a declaration of its own, named by its whole path,
//! and each name that means it is written so. The checker and the
//! generator never see a NAMESPACE.
//!
//! The whole path is the names joined by `__`, `Standard__Timers__TON`:
//! no IEC name has two underscores together (Ed.3 6.1.2; the lexer
//! refuses one), so it is no other element's name, and it is a C name
//! as it stands (docs/c-output.md). [`display`] writes it back with dots
//! for a message or the symbol table.
//!
//! Which element a short name means, where Ed.3 is silent, is decided
//! as C#'s using directives, whose wording 6.9.4 follows
//! (NAMESPACES.md, D2): innermost first - the POU's own variables, then
//! what its USINGs bring in; the namespace it is in, then what that
//! namespace's USINGs bring in; each parent namespace the same; the
//! global namespace, then the file's USINGs. A namespace's own element
//! hides one a USING brings in; two USINGs at one step bringing in the
//! same name is E0029 where the name is used.

use crate::stdlib::reserved_name;
use crate::types::Elem;
use crate::{Extension, Options};
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_syntax::ast::*;
use std::collections::{HashMap, HashSet};

/// Between the parts of a namespace element's name.
pub const SEP: &str = "__";

/// `Standard__Timers__TON` as the user writes it: `Standard.Timers.TON`.
/// A leading `__`, CODESYS's `__ISVALIDREF`, stays.
pub fn display(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'_'
            && b.get(i + 1) == Some(&b'_')
            && i > 0
            && b[i - 1].is_ascii_alphanumeric()
            && b.get(i + 2).is_some_and(u8::is_ascii_alphanumeric)
        {
            out.push('.');
            i += 2;
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

fn low(s: &str) -> String {
    s.to_ascii_lowercase()
}

fn key(parts: &[String]) -> String {
    parts.join(".")
}

/// An elementary type's name, or STRING's: never a namespace's element.
fn elementary(name: &str) -> bool {
    Elem::from_name(name).is_some() || ["STRING", "WSTRING", "USTRING"].iter().any(|s| name.eq_ignore_ascii_case(s))
}

struct Element {
    /// The name every use is rewritten to.
    flat: String,
    /// The namespace it is in, lower case.
    container: Vec<String>,
    internal: bool,
    what: &'static str,
}

struct Ns {
    internal: bool,
    span: Span,
    /// As first declared, `Standard.Timers`.
    shown: String,
}

/// One step of D2's order: a namespace (empty: the global one) and the
/// namespaces its USINGs bring in, lower case.
#[derive(Clone, Default)]
struct Level {
    ns: Vec<String>,
    using: Vec<Vec<String>>,
    /// As written, resolved once every namespace is known.
    written: Vec<Vec<Ident>>,
}

/// Where a hoisted item stands: its levels, innermost first, the global
/// one last.
struct Placed {
    item: Item,
    levels: Vec<Level>,
}

enum Found {
    Element(String),
    Namespace(String),
    /// A name of the global namespace iecf does not rename: a POU or type
    /// outside every namespace, a global variable, a standard one.
    Global,
    Nothing,
}

struct Cx<'l> {
    levels: &'l [Level],
    using: Vec<Vec<String>>,
    locals: HashSet<String>,
    /// A FUNCTION's own name, the variable of its result: (short, whole).
    own: Option<(String, String)>,
}

impl Cx<'_> {
    fn here(&self) -> &[String] {
        self.levels.first().map_or(&[], |l| l.ns.as_slice())
    }
}

struct Pass<'o> {
    options: &'o Options,
    elements: HashMap<String, Element>,
    namespaces: HashMap<String, Ns>,
    /// Lower-case names of the global namespace's own elements and
    /// global variables.
    globals: HashSet<String>,
    diags: Vec<Diagnostic>,
}

/// Take every namespace out of `units`, in place; what it found wrong.
pub fn flatten(units: &mut [SourceUnit], options: &Options) -> Vec<Diagnostic> {
    let mut pass = Pass {
        options,
        elements: HashMap::new(),
        namespaces: HashMap::new(),
        globals: HashSet::new(),
        diags: Vec::new(),
    };
    let mut placed: Vec<Vec<Placed>> = Vec::new();
    for unit in units.iter_mut() {
        let items = std::mem::take(&mut unit.items);
        let mut out = Vec::new();
        let mut file = Level::default();
        let mut declared = false;
        for item in items {
            match item {
                Item::Using(u) => {
                    if declared {
                        pass.error(
                            Code::UnexpectedToken,
                            u.span,
                            "USING comes before the file's declarations (Ed.3 Annex A, POU_Decl)".to_string(),
                        );
                    }
                    file.written.extend(u.paths);
                }
                item => {
                    declared = true;
                    out.push(item);
                }
            }
        }
        let mut list = Vec::new();
        let mut stack = vec![file];
        for item in out {
            pass.hoist(item, &mut stack, &[], &mut list);
        }
        placed.push(list);
    }
    pass.check_clashes();
    for list in &mut placed {
        for p in list.iter_mut() {
            let levels = std::mem::take(&mut p.levels);
            let levels = pass.resolve_usings(levels);
            p.levels = levels;
        }
    }
    for (unit, list) in units.iter_mut().zip(placed) {
        for mut p in list {
            pass.rewrite_item(&mut p.item, &p.levels);
            unit.items.push(p.item);
        }
    }
    // A USING is resolved for each item it stands over: said once.
    let mut seen = HashSet::new();
    pass.diags.retain(|d| seen.insert((d.span, d.message.clone())));
    pass.diags
}

impl Pass<'_> {
    fn error(&mut self, code: Code, span: Span, message: String) {
        self.diags.push(Diagnostic::new(code, span, message));
    }

    // ---- taking the elements out ------------------------------------

    /// `item`, standing in the namespaces `stack` (innermost last) whose
    /// display names are `shown`, into `out`, its elements renamed.
    fn hoist(&mut self, item: Item, stack: &mut Vec<Level>, shown: &[String], out: &mut Vec<Placed>) {
        let in_ns = !shown.is_empty();
        let ns: Vec<String> = stack.last().map(|l| l.ns.clone()).unwrap_or_default();
        match item {
            Item::Namespace(n) => {
                let depth = stack.len();
                let mut shown = shown.to_vec();
                for (i, part) in n.name.iter().enumerate() {
                    if part.text.is_empty() {
                        return;
                    }
                    let mut path = stack.last().map(|l| l.ns.clone()).unwrap_or_default();
                    path.push(low(&part.text));
                    shown.push(part.text.clone());
                    let last = i + 1 == n.name.len();
                    let entry = self.namespaces.entry(key(&path)).or_insert(Ns {
                        internal: false,
                        span: part.span,
                        shown: shown.join("."),
                    });
                    if last && n.internal {
                        entry.internal = true;
                    }
                    let written =
                        if last { n.using.iter().flat_map(|u| u.paths.clone()).collect() } else { Vec::new() };
                    stack.push(Level { ns: path, using: Vec::new(), written });
                }
                for inner in n.items {
                    self.hoist(inner, stack, &shown, out);
                }
                stack.truncate(depth);
            }
            Item::Using(u) => {
                self.error(
                    Code::UnexpectedToken,
                    u.span,
                    "USING in a namespace comes right after its name (Ed.3 6.9.4)".to_string(),
                );
            }
            Item::Pou(mut p) => {
                if in_ns {
                    let what = p.kind.keyword();
                    self.element(&mut p.name, &ns, shown, p.internal, what);
                } else if !p.name.text.is_empty() {
                    self.globals.insert(low(&p.name.text));
                }
                out.push(Placed { item: Item::Pou(p), levels: stack.iter().rev().cloned().collect() });
            }
            Item::Types(mut decls) => {
                for d in &mut decls {
                    if in_ns {
                        self.element(&mut d.name, &ns, shown, false, "type");
                    } else if !d.name.text.is_empty() {
                        self.globals.insert(low(&d.name.text));
                    }
                }
                out.push(Placed { item: Item::Types(decls), levels: stack.iter().rev().cloned().collect() });
            }
            item => {
                if in_ns {
                    let (what, span) = match &item {
                        Item::Globals(b) => ("VAR_GLOBAL", b.span),
                        Item::Configuration(c) => ("a CONFIGURATION", c.span),
                        Item::ConfigList(_, at) => ("VAR_CONFIG", *at),
                        _ => unreachable!("namespaces, USINGs, POUs and types are handled above"),
                    };
                    self.error(
                        Code::UnexpectedToken,
                        span,
                        format!(
                            "{what} cannot be in a namespace, which holds types, functions, function blocks and programs (Ed.3 6.9.2): put it outside every NAMESPACE"
                        ),
                    );
                }
                let mut names = Vec::new();
                match &item {
                    Item::Globals(b) => names.extend(b.decls.iter().flat_map(|d| d.names.iter())),
                    Item::Configuration(c) => {
                        names.extend(c.globals.iter().flat_map(|b| b.decls.iter().flat_map(|d| d.names.iter())));
                        for r in &c.resources {
                            names.extend(r.globals.iter().flat_map(|b| b.decls.iter().flat_map(|d| d.names.iter())));
                        }
                    }
                    _ => {}
                }
                let names: Vec<String> = names.iter().map(|n| low(&n.text)).collect();
                self.globals.extend(names);
                out.push(Placed { item, levels: stack.iter().rev().cloned().collect() });
            }
        }
    }

    /// Name the element `name` of the namespace `ns` (shown as `shown`)
    /// by its whole path.
    fn element(&mut self, name: &mut Ident, ns: &[String], shown: &[String], internal: bool, what: &'static str) {
        if name.text.is_empty() {
            return;
        }
        if elementary(&name.text) {
            self.error(
                Code::ReservedName,
                name.span,
                format!("'{}' is reserved: it names a data type (Ed.3 6.1.3), in a namespace too", name.text),
            );
        }
        let mut path = ns.to_vec();
        path.push(low(&name.text));
        let mut flat = shown.to_vec();
        flat.push(name.text.clone());
        let flat = flat.join(SEP);
        self.elements.entry(key(&path)).or_insert(Element {
            flat: flat.clone(),
            container: ns.to_vec(),
            internal,
            what,
        });
        name.text = flat;
    }

    /// A namespace and an element of one name side by side (D8): which
    /// would `Motor.Start` mean?
    fn check_clashes(&mut self) {
        let mut found = Vec::new();
        for (k, ns) in &self.namespaces {
            let (parent, name) = k.rsplit_once('.').map_or(("", k.as_str()), |(p, n)| (p, n));
            let clash = if parent.is_empty() {
                if self.globals.contains(name) {
                    Some("a POU, type or global variable".to_string())
                } else {
                    reserved_name(name).map(str::to_string)
                }
            } else {
                self.elements.get(k).map(|e| format!("a {}", e.what))
            };
            if let Some(what) = clash {
                found.push((ns.span, ns.shown.clone(), what));
            }
        }
        found.sort_by_key(|(s, _, _)| (s.file, s.start));
        for (span, name, what) in found {
            self.error(
                Code::Duplicate,
                span,
                format!(
                    "namespace '{name}' has the name of {what} beside it: give one of them another name (NAMESPACES.md D8)"
                ),
            );
        }
    }

    /// Each level's USINGs, resolved: a namespace's name, found from
    /// that level outwards (C#).
    fn resolve_usings(&mut self, mut levels: Vec<Level>) -> Vec<Level> {
        for level in &mut levels {
            let written = std::mem::take(&mut level.written);
            let mut using = Vec::new();
            for path in &written {
                if let Some(ns) = self.using_target(path, &level.ns) {
                    using.push(ns);
                }
            }
            level.using = using;
        }
        levels
    }

    /// The namespace `USING path` names, written in namespace `from`.
    fn using_target(&mut self, path: &[Ident], from: &[String]) -> Option<Vec<String>> {
        let parts: Vec<String> = path.iter().map(|p| low(&p.text)).collect();
        if parts.iter().any(String::is_empty) {
            return None;
        }
        let span = Span::new(path[0].span.file, path[0].span.start, path[path.len() - 1].span.end);
        let shown: Vec<&str> = path.iter().map(|p| p.text.as_str()).collect();
        let shown = shown.join(".");
        for depth in (0..=from.len()).rev() {
            let mut full = from[..depth].to_vec();
            full.extend(parts.iter().cloned());
            let k = key(&full);
            if self.namespaces.contains_key(&k) {
                if !self.ns_visible(&full, from) {
                    self.error(Code::NamespaceAccess, span, format!("namespace '{shown}' is INTERNAL to its parent namespace, which this is not in (Ed.3 Figure 29)"));
                    return None;
                }
                return Some(full);
            }
            if let Some(e) = self.elements.get(&k) {
                let what = e.what;
                self.error(
                    Code::NamespaceAccess,
                    span,
                    format!("'{shown}' is a {what}, not a namespace: USING names namespaces (Ed.3 6.9.4)"),
                );
                return None;
            }
        }
        self.error(Code::NamespaceAccess, span, format!("there is no namespace '{shown}'"));
        None
    }

    // ---- who sees what (Ed.3 Figure 29) ------------------------------

    /// Each INTERNAL namespace among `ns` and its parents shows its
    /// elements only inside the namespace above it.
    fn ns_visible(&self, ns: &[String], from: &[String]) -> bool {
        (1..=ns.len()).all(|n| {
            let internal = self.namespaces.get(&key(&ns[..n])).is_some_and(|x| x.internal);
            !internal || from.starts_with(&ns[..n - 1])
        })
    }

    /// The namespace `k` (lower case, dotted) as first declared.
    fn ns_shown(&self, k: &str) -> String {
        self.namespaces.get(k).map_or_else(|| k.to_string(), |n| n.shown.clone())
    }

    fn element_visible(&self, e: &Element, from: &[String]) -> bool {
        (!e.internal || from.starts_with(&e.container)) && self.ns_visible(&e.container, from)
    }

    // ---- finding what a name means -----------------------------------

    /// What the first part of a name, `name` (lower case), means at `cx`;
    /// the caller has looked at the POU's variables. `None` when two
    /// USINGs bring it in, E0029 reported.
    fn first(&mut self, name: &str, cx: &Cx, span: Span) -> Option<Found> {
        if let Some(f) = self.brought(name, &cx.using, span) {
            return f;
        }
        let (global, inner) = cx.levels.split_last().map_or((None, &[][..]), |(g, i)| (Some(g), i));
        for level in inner {
            let mut path = level.ns.clone();
            path.push(name.to_string());
            let k = key(&path);
            if self.elements.contains_key(&k) {
                return Some(Found::Element(k));
            }
            if self.namespaces.contains_key(&k) {
                return Some(Found::Namespace(k));
            }
            if let Some(f) = self.brought(name, &level.using, span) {
                return f;
            }
        }
        if self.namespaces.contains_key(name) {
            return Some(Found::Namespace(name.to_string()));
        }
        if self.globals.contains(name) || reserved_name(name).is_some() {
            return Some(Found::Global);
        }
        if let Some(g) = global
            && let Some(f) = self.brought(name, &g.using, span)
        {
            return f;
        }
        Some(Found::Nothing)
    }

    /// The element `name` that the namespaces `using` bring in, if one
    /// does; `Some(None)` if more than one, E0029 reported.
    fn brought(&mut self, name: &str, using: &[Vec<String>], span: Span) -> Option<Option<Found>> {
        let mut hits: Vec<String> = Vec::new();
        for u in using {
            let mut path = u.clone();
            path.push(name.to_string());
            let k = key(&path);
            if self.elements.contains_key(&k) && !hits.contains(&k) {
                hits.push(k);
            }
        }
        match hits.len() {
            0 => None,
            1 => hits.pop().map(|k| Some(Found::Element(k))),
            _ => {
                let shown: Vec<String> = hits.iter().map(|k| display(&self.elements[k].flat)).collect();
                self.error(
                    Code::AmbiguousName,
                    span,
                    format!(
                        "'{name}' could be {}: USINGs bring in each; name the one meant in full",
                        shown.join(" or ")
                    ),
                );
                Some(None)
            }
        }
    }

    /// What the name `parts` means at `cx`: what it found and how many
    /// parts that took. `None` once an error is reported.
    fn path(&mut self, parts: &[Ident], cx: &Cx) -> Option<(Found, usize)> {
        let span = Span::new(parts[0].span.file, parts[0].span.start, parts[parts.len() - 1].span.end);
        let mut found;
        let mut i = 1;
        if parts[0].text == "_" {
            if !self.options.has(Extension::GlobalNamespace) {
                self.error(
                    Code::NamespaceAccess,
                    span,
                    "'_.' names the global namespace only with --extension global-namespace (Siemens and PLCnext write it so; Ed.3 gives no way)".to_string(),
                );
                return None;
            }
            if parts.len() == 1 {
                return Some((Found::Nothing, 1));
            }
            let name = low(&parts[1].text);
            found = if self.namespaces.contains_key(&name) {
                Found::Namespace(name)
            } else if self.globals.contains(&name) || reserved_name(&name).is_some() {
                Found::Global
            } else {
                Found::Nothing
            };
            i = 2;
        } else {
            found = self.first(&low(&parts[0].text), cx, parts[0].span)?;
        }
        loop {
            match &found {
                Found::Element(k) => {
                    let e = &self.elements[k];
                    if !self.element_visible(e, cx.here()) {
                        let what = e.what;
                        let shown = display(&e.flat);
                        self.error(
                            Code::NamespaceAccess,
                            span,
                            format!(
                                "{what} '{shown}' is INTERNAL: it is seen only inside its namespace (Ed.3 Figure 29)"
                            ),
                        );
                        return None;
                    }
                    return Some((found, i));
                }
                Found::Namespace(k) => {
                    let ns: Vec<String> = k.split('.').map(str::to_string).collect();
                    if !self.ns_visible(&ns, cx.here()) {
                        self.error(
                            Code::NamespaceAccess,
                            span,
                            format!("namespace '{}' is INTERNAL to the namespace above it, which this is not in (Ed.3 Figure 29)", self.ns_shown(k)),
                        );
                        return None;
                    }
                    if i == parts.len() {
                        return Some((found, i));
                    }
                    let next = format!("{k}.{}", low(&parts[i].text));
                    found = if self.elements.contains_key(&next) {
                        Found::Element(next)
                    } else if self.namespaces.contains_key(&next) {
                        Found::Namespace(next)
                    } else {
                        let shown: Vec<&str> = parts[..i].iter().map(|p| p.text.as_str()).collect();
                        self.error(
                            Code::NamespaceAccess,
                            parts[i].span,
                            format!("namespace '{}' has no '{}'", shown.join("."), parts[i].text),
                        );
                        return None;
                    };
                    i += 1;
                }
                Found::Global | Found::Nothing => return Some((found, i)),
            }
        }
    }

    // ---- rewriting the uses -------------------------------------------

    fn rewrite_item(&mut self, item: &mut Item, levels: &[Level]) {
        let mut cx = Cx { levels, using: Vec::new(), locals: HashSet::new(), own: None };
        match item {
            Item::Pou(p) => {
                let written: Vec<Vec<Ident>> = p.using.drain(..).flat_map(|u| u.paths).collect();
                let here = cx.here().to_vec();
                cx.using = written.iter().filter_map(|w| self.using_target(w, &here)).collect();
                self.rewrite_pou(p, &mut cx);
            }
            Item::Types(decls) => {
                for d in decls {
                    match &mut d.def {
                        TypeDef::Spec(s) => self.spec(s, &cx),
                        TypeDef::Struct { fields, .. } => {
                            for f in fields {
                                self.decl(f, &cx);
                            }
                        }
                    }
                    if let Some(i) = &mut d.init {
                        self.init(i, &cx);
                    }
                }
            }
            Item::Globals(b) => self.block(b, &cx),
            Item::Configuration(c) => {
                for b in &mut c.globals {
                    self.block(b, &cx);
                }
                for r in &mut c.resources {
                    for b in &mut r.globals {
                        self.block(b, &cx);
                    }
                    for t in &mut r.tasks {
                        self.args(&mut t.params, &cx);
                    }
                    for p in &mut r.programs {
                        self.type_name(&mut p.program_type, &cx);
                        self.args(&mut p.args, &cx);
                    }
                }
                for i in &mut c.config_inits {
                    self.config_init(i, &cx);
                }
            }
            Item::ConfigList(inits, _) => {
                for i in inits {
                    self.config_init(i, &cx);
                }
            }
            Item::Namespace(_) | Item::Using(_) => {}
        }
    }

    /// A POU's declarations and bodies, and its methods', each seeing
    /// its type's variables and THIS (Ed.3 6.6.5.4.3 rule 11).
    /// A chart's elements, and those of the charts its actions hold.
    fn chart(&mut self, sfc: &mut [SfcElement], cx: &mut Cx) {
        for el in sfc {
            match el {
                SfcElement::Step(s) => {
                    for a in &mut s.actions {
                        if let Some(t) = &mut a.time {
                            self.expr(t, cx);
                        }
                    }
                }
                SfcElement::Transition(t) => {
                    if let Some(e) = &mut t.priority {
                        self.expr(e, cx);
                    }
                    self.expr(&mut t.condition, cx);
                }
                SfcElement::Action(a) => {
                    self.stmts(&mut a.body, cx);
                    self.chart(&mut a.sfc, cx);
                }
            }
        }
    }

    fn rewrite_pou(&mut self, p: &mut Pou, cx: &mut Cx) {
        for b in &p.var_blocks {
            for d in &b.decls {
                cx.locals.extend(d.names.iter().map(|n| low(&n.text)));
            }
        }
        for (_, el) in sfc_elements(&p.sfc) {
            match el {
                SfcElement::Step(s) => {
                    cx.locals.insert(low(&s.name.text));
                }
                SfcElement::Action(a) => {
                    cx.locals.insert(low(&a.name.text));
                }
                SfcElement::Transition(_) => {}
            }
        }
        if matches!(p.kind, PouKind::FunctionBlock | PouKind::Class) {
            cx.locals.insert("this".into());
            cx.locals.insert("super".into());
        }
        if let Some(b) = &mut p.extends {
            self.type_name(b, cx);
        }
        for i in &mut p.interfaces {
            self.type_name(i, cx);
        }
        let own = cx.own.take();
        if p.kind == PouKind::Function {
            let short = p.name.text.rsplit(SEP).next().unwrap_or_default();
            cx.own = Some((low(short), p.name.text.clone()));
        }
        if let Some(t) = &mut p.return_type {
            self.spec(t, cx);
        }
        for b in &mut p.var_blocks {
            self.block(b, cx);
        }
        self.stmts(&mut p.body, cx);
        self.chart(&mut p.sfc, cx);
        cx.own = own;
        for m in &mut p.methods {
            let saved = cx.locals.clone();
            cx.locals.insert(low(&m.name.text));
            self.rewrite_pou(m, cx);
            cx.locals = saved;
        }
    }

    fn config_init(&mut self, i: &mut ConfigInit, cx: &Cx) {
        self.spec(&mut i.ty, cx);
        if let Some(v) = &mut i.init {
            self.init(v, cx);
        }
    }

    fn block(&mut self, b: &mut VarBlock, cx: &Cx) {
        for d in &mut b.decls {
            self.decl(d, cx);
        }
    }

    fn decl(&mut self, d: &mut VarDecl, cx: &Cx) {
        self.spec(&mut d.ty, cx);
        if let Some(i) = &mut d.init {
            self.init(i, cx);
        }
    }

    fn init(&mut self, i: &mut Init, cx: &Cx) {
        match i {
            Init::Expr(e) => self.expr(e, cx),
            Init::Array { elems, .. } => {
                for el in elems {
                    if let Some(r) = &mut el.repeat {
                        self.expr(r, cx);
                    }
                    if let Some(v) = &mut el.value {
                        self.init(v, cx);
                    }
                }
            }
            Init::Struct { fields, .. } => {
                for (_, v) in fields {
                    self.init(v, cx);
                }
            }
        }
    }

    fn spec(&mut self, s: &mut TypeSpec, cx: &Cx) {
        match s {
            TypeSpec::Named(id) => self.type_name(id, cx),
            TypeSpec::String { len, .. } => {
                if let Some(l) = len {
                    self.expr(l, cx);
                }
            }
            TypeSpec::Array { dims, elem, .. } => {
                for (lo, hi) in dims.iter_mut().flatten() {
                    self.expr(lo, cx);
                    self.expr(hi, cx);
                }
                self.spec(elem, cx);
            }
            TypeSpec::Ref { target, .. } => self.spec(target, cx),
            TypeSpec::Subrange { low, high, .. } => {
                self.expr(low, cx);
                self.expr(high, cx);
            }
            TypeSpec::Enum { values, .. } => {
                for v in values {
                    if let Some(e) = &mut v.value {
                        self.expr(e, cx);
                    }
                }
            }
        }
    }

    /// A type's name, or a program's in a CONFIGURATION, rewritten to the
    /// element it means.
    fn type_name(&mut self, id: &mut Ident, cx: &Cx) {
        if id.text.is_empty() || elementary(&id.text) {
            return;
        }
        let file = id.span.file;
        let mut at = id.span.start;
        let parts: Vec<Ident> = id
            .text
            .split('.')
            .map(|p| {
                let span = Span::new(file, at, at + p.len() as u32);
                at += p.len() as u32 + 1;
                Ident { text: p.to_string(), span }
            })
            .collect();
        // Once said here, the type is left unnamed: the checker says no more.
        let Some((found, used)) = self.path(&parts, cx) else {
            id.text.clear();
            return;
        };
        match found {
            Found::Element(k) if used == parts.len() => id.text = self.elements[&k].flat.clone(),
            Found::Namespace(k) => {
                let msg = format!("'{}' is a namespace, not a type", self.ns_shown(&k));
                self.error(Code::NamespaceAccess, id.span, msg);
                id.text.clear();
            }
            Found::Element(k) => {
                let e = &self.elements[&k];
                let msg = format!("{} '{}' has no '{}'", e.what, display(&e.flat), parts[used].text);
                self.error(Code::NamespaceAccess, parts[used].span, msg);
                id.text.clear();
            }
            Found::Global if parts[0].text == "_" && used == parts.len() => id.text = parts[1].text.clone(),
            Found::Global | Found::Nothing => {}
        }
    }

    fn stmts(&mut self, body: &mut [Stmt], cx: &Cx) {
        for s in body {
            self.stmt(s, cx);
        }
    }

    fn stmt(&mut self, s: &mut Stmt, cx: &Cx) {
        match &mut s.kind {
            StmtKind::Assign { target, value }
            | StmtKind::RefAssign { target, value }
            | StmtKind::AssignAttempt { target, value } => {
                self.expr(target, cx);
                self.expr(value, cx);
            }
            StmtKind::Call(e) => self.expr(e, cx),
            StmtKind::If { branches, else_body } => {
                for (c, b) in branches {
                    self.expr(c, cx);
                    self.stmts(b, cx);
                }
                if let Some(b) = else_body {
                    self.stmts(b, cx);
                }
            }
            StmtKind::Case { selector, arms, else_body } => {
                self.expr(selector, cx);
                for a in arms {
                    for l in &mut a.labels {
                        match l {
                            CaseLabel::Value(e) => self.expr(e, cx),
                            CaseLabel::Range(a, b) => {
                                self.expr(a, cx);
                                self.expr(b, cx);
                            }
                        }
                    }
                    self.stmts(&mut a.body, cx);
                }
                if let Some(b) = else_body {
                    self.stmts(b, cx);
                }
            }
            StmtKind::For { from, to, by, body, .. } => {
                self.expr(from, cx);
                self.expr(to, cx);
                if let Some(b) = by {
                    self.expr(b, cx);
                }
                self.stmts(body, cx);
            }
            StmtKind::While { cond, body } => {
                self.expr(cond, cx);
                self.stmts(body, cx);
            }
            StmtKind::Repeat { body, until } => {
                self.stmts(body, cx);
                self.expr(until, cx);
            }
            StmtKind::Exit | StmtKind::Continue | StmtKind::Return | StmtKind::Empty => {}
        }
    }

    fn args(&mut self, args: &mut [Arg], cx: &Cx) {
        for a in args {
            match a {
                Arg::Positional(x) | Arg::Input(_, x) | Arg::Output { target: x, .. } => self.expr(x, cx),
            }
        }
    }

    fn expr(&mut self, e: &mut Expr, cx: &Cx) {
        match &mut e.kind {
            ExprKind::Call { callee, args } => {
                if !self.callee(callee, cx) {
                    // Said here: the checker says no more.
                    e.kind = ExprKind::Error;
                    return;
                }
                self.args(args, cx);
            }
            ExprKind::Name(id) => {
                if let Some((short, flat)) = &cx.own
                    && !cx.locals.contains(short)
                    && low(&id.text) == *short
                {
                    id.text = flat.clone();
                }
            }
            ExprKind::Member(..) => {
                if let Some(parts) = chain(e)
                    && !cx.locals.contains(&low(&parts[0].text))
                    && !cx.own.as_ref().is_some_and(|(s, _)| *s == low(&parts[0].text))
                {
                    match self.path(&parts, cx) {
                        None => {
                            e.kind = ExprKind::Error;
                            return;
                        }
                        Some((Found::Namespace(k), _)) => {
                            let msg = format!(
                                "'{}' is a namespace, which holds no variables (Ed.3 6.9.2; NAMESPACES.md D7)",
                                self.ns_shown(&k)
                            );
                            self.error(Code::NamespaceAccess, e.span, msg);
                            e.kind = ExprKind::Error;
                            return;
                        }
                        Some((Found::Global, 2)) if parts[0].text == "_" => {
                            strip_global(e);
                            return;
                        }
                        _ => {}
                    }
                }
                if let ExprKind::Member(b, _) = &mut e.kind {
                    self.expr(b, cx);
                }
            }
            ExprKind::EnumValue { ty, .. } => self.type_name(ty, cx),
            ExprKind::BitAccess(b, _) | ExprKind::Deref(b) | ExprKind::Paren(b) | ExprKind::Unary(_, b) => {
                self.expr(b, cx)
            }
            ExprKind::Index(b, idx) => {
                self.expr(b, cx);
                for i in idx {
                    self.expr(i, cx);
                }
            }
            ExprKind::Binary(_, a, b) => {
                self.expr(a, cx);
                self.expr(b, cx);
            }
            ExprKind::Literal(_) | ExprKind::DirectAddress(_) | ExprKind::Error => {}
        }
    }

    /// What is called: a function named with its namespaces becomes one
    /// name, the function's whole one. FALSE once an error is reported.
    fn callee(&mut self, callee: &mut Expr, cx: &Cx) -> bool {
        let Some(parts) = chain(callee) else {
            self.expr(callee, cx);
            return true;
        };
        let first = low(&parts[0].text);
        if cx.locals.contains(&first) {
            self.expr(callee, cx);
            return true;
        }
        let span = callee.span;
        match self.path(&parts, cx) {
            None => return false,
            Some((Found::Element(k), used)) if used == parts.len() => {
                let text = self.elements[&k].flat.clone();
                callee.kind = ExprKind::Name(Ident { text, span });
            }
            Some((Found::Element(k), used)) => {
                let e = &self.elements[&k];
                let msg = format!("{} '{}' has no '{}'", e.what, display(&e.flat), parts[used].text);
                self.error(Code::NamespaceAccess, parts[used].span, msg);
                return false;
            }
            Some((Found::Namespace(k), _)) => {
                let msg = format!("'{}' is a namespace, not a function", self.ns_shown(&k));
                self.error(Code::NamespaceAccess, span, msg);
                return false;
            }
            Some((Found::Global, used)) if parts[0].text == "_" && used == parts.len() => {
                callee.kind = ExprKind::Name(Ident { text: parts[1].text.clone(), span });
            }
            _ => {}
        }
        true
    }
}

/// `a.b.c` as its names, when it is only names.
fn chain(e: &Expr) -> Option<Vec<Ident>> {
    match &e.kind {
        ExprKind::Name(id) => Some(vec![id.clone()]),
        ExprKind::Member(b, m) => {
            let mut v = chain(b)?;
            v.push(m.clone());
            Some(v)
        }
        _ => None,
    }
}

/// `_.g.f` made `g.f`: the global variable `g`.
fn strip_global(e: &mut Expr) {
    if let ExprKind::Member(b, m) = &mut e.kind {
        if matches!(&b.kind, ExprKind::Name(id) if id.text == "_") {
            let m = m.clone();
            e.kind = ExprKind::Name(m);
        } else {
            strip_global(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iecf_diagnostics::SourceMap;

    /// Each file's diagnostics, `code: message`, its names as written.
    fn check(files: &[&str], options: &Options) -> Vec<String> {
        let mut map = SourceMap::new();
        let mut units = Vec::new();
        let mut out: Vec<String> = Vec::new();
        for (i, text) in files.iter().enumerate() {
            let id = map.add(format!("f{i}.st"), text.to_string());
            let parsed = iecf_syntax::parse(id, text);
            out.extend(parsed.diagnostics.iter().map(|d| format!("{}: {}", d.code.id(), d.message)));
            units.push(parsed.unit);
        }
        let analysis = crate::analyze_project(&mut units, options);
        out.extend(analysis.diagnostics.iter().map(|d| format!("{}: {}", d.code.id(), display(&d.message))));
        out
    }

    fn one(text: &str) -> Vec<String> {
        check(&[text], &Options::default())
    }

    const LIB: &str = "NAMESPACE A
        FUNCTION f : INT f := 1; END_FUNCTION
        FUNCTION INTERNAL hidden : INT hidden := 2; END_FUNCTION
        FUNCTION usesHidden : INT usesHidden := hidden(); END_FUNCTION
        NAMESPACE Inner
            FUNCTION alsoHidden : INT alsoHidden := A.hidden(); END_FUNCTION
        END_NAMESPACE
        NAMESPACE INTERNAL Deep
            FUNCTION g : INT g := 3; END_FUNCTION
        END_NAMESPACE
        FUNCTION usesDeep : INT usesDeep := Deep.g(); END_FUNCTION
    END_NAMESPACE
    NAMESPACE B
        FUNCTION f : INT f := 4; END_FUNCTION
    END_NAMESPACE
    ";

    fn with_lib(body: &str) -> Vec<String> {
        one(&format!("{LIB}PROGRAM p\nVAR n : INT; END_VAR\n{body}\nEND_PROGRAM\n"))
    }

    #[test]
    fn display_gives_the_path_as_written() {
        assert_eq!(display("Standard__Timers__TON"), "Standard.Timers.TON");
        assert_eq!(display("'Plant__Main' and __ISVALIDREF"), "'Plant.Main' and __ISVALIDREF");
        assert_eq!(display("plain_name"), "plain_name");
    }

    #[test]
    fn a_name_with_its_namespaces_and_by_using() {
        assert!(with_lib("n := A.f() + B.f();").is_empty(), "{:?}", with_lib("n := A.f();"));
        assert!(one(&format!("{LIB}PROGRAM p USING A;\nVAR n : INT; END_VAR\nn := f();\nEND_PROGRAM\n")).is_empty());
    }

    #[test]
    fn internal_is_seen_only_inside_its_namespace() {
        // Inside A and its namespace Inner, hidden is seen (above, in LIB).
        assert_eq!(
            with_lib("n := A.hidden();"),
            ["E0028: FUNCTION 'A.hidden' is INTERNAL: it is seen only inside its namespace (Ed.3 Figure 29)"]
        );
    }

    #[test]
    fn an_internal_namespace_is_seen_in_the_one_above() {
        // A.usesDeep reaches Deep.g (above, in LIB).
        assert_eq!(
            with_lib("n := A.Deep.g();"),
            ["E0028: namespace 'A.Deep' is INTERNAL to the namespace above it, which this is not in (Ed.3 Figure 29)"]
        );
    }

    #[test]
    fn two_usings_bringing_one_name_is_an_error_where_it_is_used() {
        let src = format!("{LIB}PROGRAM p USING A, B;\nVAR n : INT; END_VAR\nn := A.f();\nEND_PROGRAM\n");
        assert!(one(&src).is_empty(), "unused, the clash is no error");
        let src = format!("{LIB}PROGRAM p USING A, B;\nVAR n : INT; END_VAR\nn := f();\nEND_PROGRAM\n");
        assert_eq!(one(&src), ["E0029: 'f' could be A.f or B.f: USINGs bring in each; name the one meant in full"]);
    }

    #[test]
    fn a_namespace_s_own_element_hides_one_using_brings_in() {
        let src = format!(
            "{LIB}NAMESPACE C USING A;
                FUNCTION f : DINT f := 5; END_FUNCTION
                FUNCTION_BLOCK user VAR d : DINT; END_VAR d := f(); END_FUNCTION_BLOCK
            END_NAMESPACE"
        );
        assert!(one(&src).is_empty(), "C.f, a DINT, not A.f: {:?}", one(&src));
    }

    #[test]
    fn using_does_not_bring_in_nested_namespaces() {
        // Ed.3 6.9.4 Example 2.
        let src = "NAMESPACE Standard.Timers FUNCTION_BLOCK TON END_FUNCTION_BLOCK END_NAMESPACE
            NAMESPACE Infeed USING Standard;
                FUNCTION_BLOCK user VAR t : Timers.TON; END_VAR END_FUNCTION_BLOCK
            END_NAMESPACE";
        assert_eq!(one(src), ["E0011: there is no type 'Timers.TON'"]);
    }

    #[test]
    fn a_global_using_is_its_file_s() {
        let lib = "NAMESPACE A FUNCTION f : INT f := 1; END_FUNCTION END_NAMESPACE";
        let with = "USING A;\nFUNCTION g : INT g := f(); END_FUNCTION";
        let without = "FUNCTION h : INT h := f(); END_FUNCTION";
        assert!(check(&[lib, with], &Options::default()).is_empty());
        assert_eq!(check(&[lib, with, without], &Options::default()), ["E0010: 'f' is not declared"]);
    }

    #[test]
    fn a_namespace_holds_no_variables() {
        assert_eq!(with_lib("n := A.x;"), ["E0028: namespace 'A' has no 'x'"],);
        assert_eq!(
            with_lib("n := A.Inner.y + 0; A.Inner := 1;"),
            [
                "E0028: namespace 'A.Inner' has no 'y'",
                "E0028: 'A.Inner' is a namespace, which holds no variables (Ed.3 6.9.2; NAMESPACES.md D7)"
            ]
        );
        assert_eq!(
            one("NAMESPACE A VAR_GLOBAL g : INT; END_VAR END_NAMESPACE"),
            [
                "E0005: VAR_GLOBAL cannot be in a namespace, which holds types, functions, function blocks and programs (Ed.3 6.9.2): put it outside every NAMESPACE"
            ]
        );
    }

    #[test]
    fn a_namespace_and_an_element_of_one_name_side_by_side() {
        assert_eq!(
            one("NAMESPACE Motor END_NAMESPACE\nFUNCTION_BLOCK Motor END_FUNCTION_BLOCK"),
            [
                "E0012: namespace 'Motor' has the name of a POU, type or global variable beside it: give one of them another name (NAMESPACES.md D8)"
            ]
        );
        assert_eq!(
            one("NAMESPACE A NAMESPACE m END_NAMESPACE FUNCTION m : INT m := 1; END_FUNCTION END_NAMESPACE"),
            [
                "E0012: namespace 'A.m' has the name of a FUNCTION beside it: give one of them another name (NAMESPACES.md D8)"
            ]
        );
    }

    #[test]
    fn the_global_namespace_is_named_only_with_its_extension() {
        let src = "NAMESPACE P FUNCTION ABS : INT VAR_INPUT x : INT; END_VAR ABS := x; END_FUNCTION
            FUNCTION g : INT g := _.ABS(-1); END_FUNCTION END_NAMESPACE";
        assert_eq!(
            one(src),
            [
                "E0028: '_.' names the global namespace only with --extension global-namespace (Siemens and PLCnext write it so; Ed.3 gives no way)"
            ]
        );
        assert!(check(&[src], &Options::default().with(Extension::GlobalNamespace)).is_empty());
    }

    #[test]
    fn what_a_namespace_cannot_be_given() {
        assert_eq!(
            one("NAMESPACE A FUNCTION_BLOCK INT END_FUNCTION_BLOCK END_NAMESPACE"),
            ["E0018: 'INT' is reserved: it names a data type (Ed.3 6.1.3), in a namespace too"]
        );
        assert_eq!(
            one("NAMESPACE A FUNCTION f : INT f := 1; END_FUNCTION END_NAMESPACE\nUSING A;"),
            ["E0005: USING comes before the file's declarations (Ed.3 Annex A, POU_Decl)",]
        );
        assert_eq!(
            one(
                "USING A.f, Nope;\nNAMESPACE A FUNCTION f : INT f := 1; END_FUNCTION END_NAMESPACE\nFUNCTION g : INT g := 1; END_FUNCTION"
            ),
            [
                "E0028: 'A.f' is a FUNCTION, not a namespace: USING names namespaces (Ed.3 6.9.4)",
                "E0028: there is no namespace 'Nope'",
            ]
        );
        assert_eq!(
            one(
                "NAMESPACE A FUNCTION f : INT f := 1; END_FUNCTION END_NAMESPACE\nNAMESPACE A FUNCTION f : INT f := 2; END_FUNCTION END_NAMESPACE"
            ),
            ["E0012: 'A.f' is declared twice: POUs and types share one namespace"]
        );
        assert_eq!(
            one("PROGRAM p VAR a__b : INT; END_VAR END_PROGRAM"),
            ["E0005: 'a__b' has two underscores together, which a name may not have (Ed.3 6.1.2)"]
        );
    }
}

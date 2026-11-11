//! Interfaces (OO.md step C, Ed.3 6.6.6).
//!
//! A variable of an INTERFACE is runtime/iecf.h's `iecf_itf_t`: the
//! instance it references, `self`, and that instance's type's table for
//! the interface; NULL when `self` is. The table of the interface `I` is
//! a structure of function pointers, `IT_<I>_t`, one `m_<M>` for each
//! method of `I` and of the interfaces it extends, each taking the
//! instance as `void *self`. Each type `T` that implements `I`, itself or
//! through a base, has its own, `I_<I>__<T>`, whose entries are thunks
//! `IM_<I>__<T>__<M>` calling the method `T` has, its own or the nearest
//! base's: an override below is a type of its own, with its own table.
//!
//! Every instance of such a type starts with the pointer to its type
//! (oo.rs), whose `itfs` list each interface it implements with its
//! table. An instance given where its type is known, a named one, takes
//! its table directly; THIS, an in-out and what a reference references
//! may be of a derived type, and an interface given another that extends
//! it holds the other's instance, whose type it does not know: each
//! finds the table in the instance's type (`iecf_itf_of`). `?=` looks the
//! same way, NULL when the type does not implement it.
//!
//! A method called through an interface is called through its table,
//! the interface checked first: NULL is a fault, the call not made, the
//! statement stopped (OO.md D8). An interface is never in the process
//! image nor in the symbol table.

use crate::expr::strip_paren;
use crate::names::{Lv, Place, Scope};
use crate::{Gen, R, sig_params};
use iecf_diagnostics::Span;
use iecf_sema::types::Ty;
use iecf_syntax::ast::{Expr, ExprKind, Init, Pou, PouKind};

/// The line an interface's initial value leaves, until `fill_itf_inits`.
fn marker(n: usize) -> String {
    format!("/*@ITF{n}@*/")
}
use std::fmt::Write as _;

impl Gen<'_> {
    /// Whether instances of `key` carry their type: it has a base or a
    /// derived type, or implements an interface (OO.md §4).
    pub(crate) fn carries_type(&self, key: &str) -> bool {
        self.in_hierarchy(key) || !self.types.implemented(key).is_empty()
    }

    /// The methods of the interface `i`, its bases' too, in the order of
    /// its table: the interface that declares each, and the prototype.
    pub(crate) fn itf_slots(&self, i: &str) -> Vec<(String, &Pou)> {
        let mut out: Vec<(String, &Pou)> = Vec::new();
        for k in self.types.itf_line(i).into_iter().rev() {
            let Some(p) = self.pous.get(&k).copied() else { continue };
            for m in p.methods.iter().filter(|m| !m.name.text.is_empty()) {
                if !out.iter().any(|(_, o)| o.name.text.eq_ignore_ascii_case(&m.name.text)) {
                    out.push((k.clone(), m));
                }
            }
        }
        out
    }

    /// Whether `key` names an INTERFACE.
    pub(crate) fn is_itf(&self, key: &str) -> bool {
        self.pous.get(key).is_some_and(|p| p.kind == PouKind::Interface)
    }

    /// `e` as an `iecf_itf_t` of the interface `want`: NULL, a variable
    /// of `want` or of an interface that extends it, or an instance of a
    /// type that implements it (the checker allowed it, Ed.3 6.6.6.5.1).
    pub(crate) fn itf_value(&mut self, e: &Expr, want: &str, scope: &Scope) -> R<String> {
        let e = strip_paren(e);
        if let ExprKind::Name(id) = &e.kind
            && id.is("NULL")
            && !scope.vars.contains_key("NULL")
        {
            return Ok("iecf_itf_null()".into());
        }
        self.itfs_used.insert(want.to_string());
        match self.types.expr(e.span).cloned() {
            Some(Ty::Itf(j)) => {
                let lv = match &e.kind {
                    // A FUNCTION's or method's result: the buffer its call
                    // writes (params.rs).
                    ExprKind::Call { callee, args } => {
                        if !self.pre_ok {
                            return self.not_yet(e.span, "a FUNCTION giving an interface here");
                        }
                        self.user_call(e, callee, args, scope)?
                    }
                    // A variable, an element, a field, a block's input or
                    // output.
                    _ => match self.data_path(e, scope)? {
                        Some((Lv::C(c), crate::derived::DTy { kind: crate::derived::Kind::Itf(_), .. })) => c,
                        _ => return self.not_yet(e.span, "this interface"),
                    },
                };
                // Another interface's instance: its table for this one is
                // its type's.
                Ok(if j == want { lv } else { format!("iecf_itf_as({lv}, &ITY_{want})") })
            }
            Some(Ty::Fb(_)) => {
                let Some(Place::Block(lv, s)) = self.call_instance(e, scope)? else {
                    return self.not_yet(e.span, "this instance given to an interface");
                };
                // Through a reference: checked on a line of its own, NULL
                // stopping the statement before its type is read (D8).
                let lv = if matches!(e.kind, ExprKind::Deref(_)) {
                    if !self.pre_ok {
                        return self.not_yet(e.span, "an instance through a reference given to an interface here");
                    }
                    let o = self.fresh("o");
                    self.str_temps.push(format!("B_{s}_t *{o} = &({lv});"));
                    self.str_temps.push(crate::enable::STOP.to_string());
                    self.may_fault = false;
                    format!("(*{o})")
                } else {
                    lv
                };
                Ok(if self.is_poly(e, &scope.poly) || matches!(e.kind, ExprKind::Deref(_)) {
                    format!("iecf_itf_of(&({lv}), &ITY_{want})")
                } else {
                    format!("iecf_itf_make(&({lv}), &I_{want}__{s})")
                })
            }
            _ => self.not_yet(e.span, "this value of an interface"),
        }
    }

    /// An interface's initial value into `out`: NULL when none is given
    /// (Ed.3 6.6.6.5.1 item 4); an instance or another interface, which
    /// may be declared after it, as a marker filled in when the POU's
    /// variables are all known (`fill_itf_inits`), as REF() is (refs.rs).
    pub(crate) fn init_itf(&mut self, lv: &str, i: &str, given: Option<&Init>, out: &mut String) -> R<()> {
        match given {
            None => {
                let _ = writeln!(out, "    {lv} = iecf_itf_null();");
            }
            Some(Init::Expr(x)) if matches!(&x.kind, ExprKind::Name(id) if id.is("NULL")) => {
                let _ = writeln!(out, "    {lv} = iecf_itf_null();");
            }
            Some(Init::Expr(x)) => {
                let n = self.itf_inits.len();
                self.itf_inits.push((lv.to_string(), x.clone(), i.to_string()));
                let _ = writeln!(out, "    {}", marker(n));
            }
            Some(Init::Array { span, .. } | Init::Struct { span, .. }) => {
                return self.not_yet(*span, "this initial value of an interface");
            }
        }
        Ok(())
    }

    /// The interfaces' initial values `text` holds markers of.
    pub(crate) fn fill_itf_inits(&mut self, text: &mut String, scope: &Scope) -> R<()> {
        for n in 0..self.itf_inits.len() {
            let m = marker(n);
            if !text.contains(&m) {
                continue;
            }
            let (lv, x, i) = self.itf_inits[n].clone();
            self.may_fault = false;
            self.may_flag = false;
            let was = std::mem::replace(&mut self.pre_ok, false);
            let v = self.itf_value(&x, &i, scope);
            self.pre_ok = was;
            let v = v?;
            if self.may_fault || self.may_flag {
                return self.not_yet(x.span, "an initial value that can fault or be flagged");
            }
            *text = text.replace(&m, &format!("{lv} = {v};"));
        }
        Ok(())
    }

    /// The statement storing `v`, an `iecf_itf_t` of the interface
    /// `from`, into `lv`, of `to`: the same, or one `from` extends.
    pub(crate) fn itf_store(&mut self, lv: &Lv, to: &str, v: &str, from: &str, at: Span) -> R<String> {
        let Some(c) = lv.c() else { return self.not_yet(at, "an interface stored here") };
        if to == from {
            return Ok(format!("{c} = {v};"));
        }
        self.itfs_used.insert(to.to_string());
        Ok(format!("{c} = iecf_itf_as({v}, &ITY_{to});"))
    }

    /// The interface `e` is, and its value: for `=` and `<>`, either side
    /// NULL or an interface.
    pub(crate) fn itf_side(&mut self, e: &Expr, other: &Expr, scope: &Scope) -> R<String> {
        let ty = |g: &Self, x: &Expr| match g.types.expr(x.span) {
            Some(Ty::Itf(i)) => Some(i.clone()),
            _ => None,
        };
        match ty(self, e).or_else(|| ty(self, other)) {
            Some(i) => self.itf_value(e, &i, scope),
            None => self.not_yet(e.span, "comparing this"),
        }
    }

    /// The interfaces' parts of the tables (oo.rs `oo_tables`): their C
    /// types, the prototypes of their thunks, their `ITY_`s; each type's
    /// tables and its list of them, `ITS_<T>`; and the thunks. `keys`
    /// are the types that carry their type, in the order of their `TY_`s.
    pub(crate) fn itf_tables(&mut self, keys: &[String]) -> R<ItfParts> {
        let p = self.prefix.clone();
        let mut used: std::collections::BTreeSet<String> = self.itfs_used.clone();
        for k in keys {
            used.extend(self.types.implemented(k));
        }
        let mut all = std::collections::BTreeSet::new();
        for i in &used {
            all.extend(self.types.itf_line(i));
        }
        let mut out = ItfParts::default();
        for i in &all {
            let slots: Vec<(String, String, iecf_diagnostics::Span)> = self
                .itf_slots(i)
                .into_iter()
                .map(|(d, m)| (d, m.name.text.to_ascii_uppercase(), m.name.span))
                .collect();
            let _ = writeln!(out.types, "typedef struct {{");
            if slots.is_empty() {
                let _ = writeln!(out.types, "    char none_; /* C has no empty structure */");
            }
            for (d, m, at) in &slots {
                let sig = self.fn_sig_q(&iecf_sema::method_key(d, m), *at, false)?;
                let rt = sig.c_ret();
                let ps: String = sig_params(&sig, false).into_iter().map(|(t, n)| format!(", {t}{n}")).collect();
                let _ = writeln!(out.types, "    {rt} (*m_{m})({p}_t *ctx, void *self{ps});");
            }
            let _ = writeln!(out.types, "}} IT_{i}_t;\n");
            let _ = writeln!(
                out.descs,
                "static const iecf_itype_t ITY_{i} = {{\"{}\"}};",
                iecf_sema::namespaces::display(i)
            );
        }
        for k in keys {
            let mine = self.types.implemented(k);
            let mut list = Vec::new();
            for i in &mine {
                let slots: Vec<(String, String)> =
                    self.itf_slots(i).into_iter().map(|(d, m)| (d, m.name.text.to_ascii_uppercase())).collect();
                let mut entries = Vec::new();
                for (d, m) in slots {
                    let f = self.itf_thunk(i, k, &d, &m, &mut out)?;
                    entries.push(format!(".m_{m} = {f}"));
                }
                let _ = writeln!(out.tables, "static const IT_{i}_t I_{i}__{k} = {{");
                if entries.is_empty() {
                    let _ = writeln!(out.tables, "    0");
                }
                for e in &entries {
                    let _ = writeln!(out.tables, "    {e},");
                }
                let _ = writeln!(out.tables, "}};");
                list.push(format!("{{&ITY_{i}, &I_{i}__{k}}}"));
            }
            if !list.is_empty() {
                let _ = writeln!(out.tables, "static const iecf_itfimpl_t ITS_{k}[] = {{{}}};\n", list.join(", "));
            }
            out.counts.insert(k.clone(), list.len());
        }
        Ok(out)
    }

    /// The entry of `k`'s table for `i` in the slot of `m`, declared by
    /// `d`: a thunk to the method `k` has, NULL when it has none (an
    /// ABSTRACT type, which no instance is).
    fn itf_thunk(&mut self, i: &str, k: &str, d: &str, m: &str, out: &mut ItfParts) -> R<String> {
        let p = self.prefix.clone();
        let line: Vec<String> = std::iter::once(k.to_string()).chain(self.types.ancestors(k)).collect();
        let found = line.into_iter().find_map(|t| {
            self.pous
                .get(&iecf_sema::method_key(&t, m))
                .copied()
                .map(|mp| (t, mp.specs.abstract_.is_some(), mp.name.span))
        });
        let Some((a, false, at)) = found else { return Ok("NULL".into()) };
        let slot = self.fn_sig_q(&iecf_sema::method_key(d, m), at, false)?;
        let sig = self.fn_sig(&iecf_sema::method_key(&a, m), at)?;
        let t = format!("IM_{i}__{k}__{m}");
        let rt = slot.c_ret();
        let ps: String = sig_params(&slot, false).into_iter().map(|(t, n)| format!(", {t}{n}")).collect();
        let head = format!("static {rt} {t}({p}_t *ctx, void *self{ps})");
        let mut args = vec!["ctx".to_string(), format!("(B_{a}_t *)self")];
        let mut pre = String::new();
        if sig.eno {
            pre.push_str("    IEC_BOOL eno_ = 1;\n");
            args.push("&eno_".into());
        }
        args.extend(sig_params(&slot, false).into_iter().map(|(_, n)| n));
        let ret = if sig.gives() { "return " } else { "" };
        let _ = writeln!(out.protos, "{head};");
        let _ = writeln!(out.thunks, "{head} {{\n{pre}    {ret}{}({});\n}}\n", sig.c_name, args.join(", "));
        Ok(t)
    }
}

/// What itf.rs adds to the tables (oo.rs `oo_tables`).
#[derive(Default)]
pub(crate) struct ItfParts {
    pub(crate) types: String,
    pub(crate) protos: String,
    pub(crate) descs: String,
    pub(crate) tables: String,
    pub(crate) thunks: String,
    /// Each type's count of interfaces, for its `TY_`.
    pub(crate) counts: std::collections::HashMap<String, usize>,
}

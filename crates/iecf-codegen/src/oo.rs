//! Inheritance and dynamic binding (OO.md step B, Ed.3 6.6.5.5 to
//! 6.6.5.8, 6.6.7.2.7 to 6.6.7.3).
//!
//! A derived type's structure starts with its base's, `B_<BASE>_t base;`
//! (lib.rs `block`), so a pointer to an instance is one to its base part
//! too (C99 6.7.2.1). The root of a hierarchy, and a type that
//! implements an interface (itf.rs), starts with `const iecf_otype_t
//! *type`, which each `_init` sets to its own `TY_<NAME>`: the instance
//! knows its type for the whole run. A type's `VT_<NAME>`
//! is its table: a structure of function pointers, its base's first,
//! then a body for the first function block of a hierarchy, then each
//! method it declares that can be overridden (not PRIVATE, not FINAL).
//! Each slot takes the type that declares it, `B_<I>_t *self`; a derived
//! type's implementation is reached through a thunk, `M_<T>__<M>__vt`,
//! which converts the pointer back.
//!
//! A call through THIS, a VAR_IN_OUT or a reference reads the slot of
//! the instance's table (6.6.5.6, 6.6.7.2.9 rule 4); `SUPER`, a named
//! instance, and a method nothing below overrides are called directly.

use crate::{Gen, R, sig_params};
use iecf_diagnostics::Span;
use iecf_syntax::ast::{Access, Expr, ExprKind, Pou, PouKind};
use std::fmt::Write as _;

impl Gen<'_> {
    /// Whether `key` has a base or a derived type: its instances carry
    /// their type, and it has a table.
    pub(crate) fn in_hierarchy(&self, key: &str) -> bool {
        self.types.base(key).is_some() || !self.types.descendants(key).is_empty()
    }

    fn depth(&self, key: &str) -> usize {
        self.types.ancestors(key).len()
    }

    /// From an instance of `from` to its part of type `to`, `from`
    /// itself or one of its bases: `.base.base`.
    pub(crate) fn up_path(&self, from: &str, to: &str) -> String {
        ".base".repeat(self.depth(from).saturating_sub(self.depth(to)))
    }

    /// The type pointer's field in an instance of `key`: `base.base.type`.
    pub(crate) fn type_field(&self, key: &str) -> String {
        format!("{}type", "base.".repeat(self.depth(key)))
    }

    fn method_of(&self, ty: &str, m: &str) -> Option<&Pou> {
        self.pous.get(&iecf_sema::method_key(ty, m)).copied()
    }

    /// The type whose table slot the method `m` of `a` fills: up its
    /// OVERRIDEs to the one that declared it first.
    fn introducer(&self, a: &str, m: &str) -> String {
        let mut at = a.to_string();
        while self.method_of(&at, m).is_some_and(|p| p.specs.override_.is_some()) {
            match self
                .types
                .ancestors(&at)
                .into_iter()
                .find(|b| self.method_of(b, m).is_some_and(|p| !matches!(p.specs.access, Some((Access::Private, _)))))
            {
                Some(b) => at = b,
                None => break,
            }
        }
        at
    }

    /// Whether the method `m` introduced by `i` has a slot: one that can
    /// be overridden, not PRIVATE (not inherited) nor FINAL.
    fn has_slot(&self, i: &str, m: &str) -> bool {
        self.method_of(i, m)
            .is_some_and(|p| p.specs.final_.is_none() && !matches!(p.specs.access, Some((Access::Private, _))))
    }

    /// Whether a slot passes ENO: when the method or an override of it
    /// uses ENO without declaring it (Ed.3 6.6.1.5).
    fn slot_eno(&self, i: &str, m: &str) -> bool {
        std::iter::once(i.to_string())
            .chain(self.types.descendants(i))
            .any(|t| self.method_of(&t, m).is_some() && self.types.implicit_eno(&iecf_sema::method_key(&t, m)))
    }

    /// The function block whose body slot a block `s` fills: the first of
    /// its line that is a function block (a class has no body).
    fn body_introducer(&self, s: &str) -> String {
        let mut at = s.to_string();
        while let Some(b) = self.types.base(&at) {
            if self.pous.get(b).is_some_and(|p| p.kind == PouKind::Class) {
                break;
            }
            at = b.to_string();
        }
        at
    }

    /// Whether the instance `base` names may be of a derived type: THIS,
    /// a VAR_IN_OUT, what a reference references.
    pub(crate) fn is_poly(&self, base: &Expr, poly: &std::collections::HashSet<String>) -> bool {
        match &base.kind {
            ExprKind::Paren(x) => self.is_poly(x, poly),
            ExprKind::Name(id) => poly.contains(&id.text.to_ascii_uppercase()),
            ExprKind::Deref(_) => true,
            _ => false,
        }
    }

    /// The method `key` (`A__M`) called on `lv`, an instance of `s`, bound
    /// at run time: the slot to call, the instance as the slot takes it,
    /// and the slot's signature. `None` when it is bound statically: not
    /// `poly`, no slot, or nothing below `s` overrides it (6.6.8.5 NOTE).
    pub(crate) fn dispatch(
        &mut self,
        lv: &str,
        s: &str,
        key: &str,
        poly: bool,
        at: Span,
    ) -> R<Option<(String, String, crate::FnSig)>> {
        let (a, m) = key.rsplit_once("__").unwrap_or(("", key));
        let i = self.introducer(a, m);
        let abstract_ = self.method_of(a, m).is_some_and(|p| p.specs.abstract_.is_some());
        let overridden = self.types.descendants(s).iter().any(|d| self.method_of(d, m).is_some());
        if !poly || !self.has_slot(&i, m) || !(overridden || abstract_) {
            return Ok(None);
        }
        let table = format!("((const VT_{s}_t *)({lv}).{}->table)", self.type_field(s));
        let slot = format!("{table}->{}m_{m}", "base.".repeat(self.depth(s) - self.depth(&i)));
        let mut sig = self.fn_sig_q(&iecf_sema::method_key(&i, m), at, false)?;
        sig.eno = self.slot_eno(&i, m);
        sig.c_name = slot.clone();
        Ok(Some((slot, format!("&({lv}){}", self.up_path(s, &i)), sig)))
    }

    /// The call of the block `s`'s body on `lv` through its table (Ed.3
    /// 6.6.7.2.9 rule 4), when a type below `s` has a body of its own.
    pub(crate) fn dispatch_body(&self, lv: &str, s: &str) -> Option<String> {
        if self.types.descendants(s).is_empty() {
            return None;
        }
        let i = self.body_introducer(s);
        let table = format!("((const VT_{s}_t *)({lv}).{}->table)", self.type_field(s));
        Some(format!(
            "{table}->{}body(ctx, &({lv}){});",
            "base.".repeat(self.depth(s) - self.depth(&i)),
            self.up_path(s, &i)
        ))
    }

    /// The slots `key` adds to its base's: `None` the body, else a
    /// method's upper-cased name.
    fn own_slots(&self, key: &str) -> Vec<Option<String>> {
        let Some(pou) = self.pous.get(key).copied() else { return Vec::new() };
        let mut out = Vec::new();
        let base_is_block =
            self.types.base(key).and_then(|b| self.pous.get(b)).is_some_and(|p| p.kind == PouKind::FunctionBlock);
        if pou.kind == PouKind::FunctionBlock && !base_is_block {
            out.push(None);
        }
        for m in &pou.methods {
            let n = m.name.text.to_ascii_uppercase();
            if m.specs.override_.is_none() && self.has_slot(key, &n) {
                out.push(Some(n));
            }
        }
        out
    }

    /// The tables of the types in a hierarchy generated: their C types,
    /// the prototypes they need, the tables and `TY_<NAME>`s, to stand
    /// before the blocks' code; and the thunks, after it.
    pub(crate) fn oo_tables(&mut self) -> R<(String, String)> {
        let p = self.prefix.clone();
        let mut keys: Vec<String> = self.blocks.keys().filter(|k| self.carries_type(k)).cloned().collect();
        keys.sort_by_key(|k| (self.depth(k), k.clone()));
        let (mut types, mut protos, mut tables, mut thunks, mut tys) =
            (String::new(), String::new(), String::new(), String::new(), String::new());
        let itfs = self.itf_tables(&keys)?;
        for k in &keys {
            let pou = self.pous[k.as_str()];
            // Its C type: its base's, then its own slots.
            let _ = writeln!(types, "typedef struct {{");
            if let Some(b) = self.types.base(k) {
                let _ = writeln!(types, "    VT_{b}_t base;");
            }
            let own = self.own_slots(k);
            if own.is_empty() && self.types.base(k).is_none() {
                let _ = writeln!(types, "    char none_; /* C has no empty structure */");
            }
            for slot in &own {
                match slot {
                    None => {
                        let _ = writeln!(types, "    void (*body)({p}_t *ctx, B_{k}_t *self);");
                    }
                    Some(m) => {
                        let sig = self.fn_sig_q(&iecf_sema::method_key(k, m), pou.name.span, false)?;
                        let rt = sig.c_ret();
                        let ps: Vec<String> = sig_params(&sig, self.slot_eno(k, m))
                            .into_iter()
                            .map(|(t, n)| format!(", {t}{n}"))
                            .collect();
                        let _ = writeln!(types, "    {rt} (*m_{m})({p}_t *ctx, B_{k}_t *self{});", ps.concat());
                    }
                }
            }
            let _ = writeln!(types, "}} VT_{k}_t;\n");
            if pou.kind == PouKind::FunctionBlock {
                let _ = writeln!(protos, "static void B_{k}({p}_t *ctx, B_{k}_t *self);");
            }
            // Its table: each slot of its line, filled with its own
            // implementation or the nearest base's.
            let mut entries = Vec::new();
            let line: Vec<String> = std::iter::once(k.clone()).chain(self.types.ancestors(k)).collect();
            for owner in line.iter().rev() {
                let path = "base.".repeat(self.depth(k) - self.depth(owner));
                for slot in self.own_slots(owner) {
                    match slot {
                        None => {
                            let f = if owner == k {
                                format!("B_{k}")
                            } else {
                                let t = format!("B_{k}__vt");
                                let _ = writeln!(protos, "static void {t}({p}_t *ctx, B_{owner}_t *self);");
                                let _ = writeln!(
                                    thunks,
                                    "static void {t}({p}_t *ctx, B_{owner}_t *self) {{\n    B_{k}(ctx, (B_{k}_t *)self);\n}}\n"
                                );
                                t
                            };
                            entries.push(format!(".{path}body = {f}"));
                        }
                        Some(m) => {
                            let f = self.slot_entry(k, owner, &m, &mut protos, &mut thunks)?;
                            entries.push(format!(".{path}m_{m} = {f}"));
                        }
                    }
                }
            }
            let _ = writeln!(tables, "static const VT_{k}_t VT_{k} = {{");
            if entries.is_empty() {
                let _ = writeln!(tables, "    0");
            }
            for e in &entries {
                let _ = writeln!(tables, "    {e},");
            }
            let _ = writeln!(tables, "}};");
            let base = self.types.base(k).map_or_else(|| "NULL".to_string(), |b| format!("&TY_{b}"));
            let n = itfs.counts.get(k).copied().unwrap_or(0);
            let list = if n == 0 { "NULL".to_string() } else { format!("ITS_{k}") };
            let _ = writeln!(
                tys,
                "static const iecf_otype_t TY_{k} = {{{base}, \"{}\", &VT_{k}, {list}, {n}u}};",
                iecf_sema::namespaces::display(k)
            );
        }
        if keys.is_empty() && itfs.types.is_empty() {
            return Ok((String::new(), String::new()));
        }
        let head = format!(
            "/* The method tables (OO.md §4): each type's, its tables for its interfaces, and its TY_ the instances point to. */\n\
             {types}{}{protos}{}\n{}{tables}{}{tys}\n",
            itfs.types, itfs.protos, itfs.descs, itfs.tables
        );
        thunks.push_str(&itfs.thunks);
        Ok((head, thunks))
    }

    /// What `k`'s table holds in the slot of method `m`, introduced by
    /// `owner`: the implementation of `k` or its nearest base, directly
    /// when its C signature is the slot's, else through a thunk; NULL for
    /// an ABSTRACT one, which no instance reaches (Ed.3 6.6.5.8).
    fn slot_entry(&mut self, k: &str, owner: &str, m: &str, protos: &mut String, thunks: &mut String) -> R<String> {
        let p = self.prefix.clone();
        let line: Vec<String> = std::iter::once(k.to_string()).chain(self.types.ancestors(k)).collect();
        let Some((a, mp)) = line.into_iter().find_map(|t| self.method_of(&t, m).map(|p| (t, p))) else {
            return Ok("NULL".into());
        };
        let at = mp.name.span;
        if mp.specs.abstract_.is_some() {
            return Ok("NULL".into());
        }
        let akey = iecf_sema::method_key(&a, m);
        let sig = self.fn_sig_q(&akey, at, true)?;
        let eno = self.slot_eno(owner, m);
        if a == owner && sig.eno == eno {
            return Ok(sig.c_name);
        }
        let t = format!("M_{akey}__vt");
        if thunks.contains(&format!(" {t}(")) {
            return Ok(t);
        }
        let rt = sig.c_ret();
        let ps = sig_params(&sig, eno);
        let head = format!(
            "static {rt} {t}({p}_t *ctx, B_{owner}_t *self{})",
            ps.iter().map(|(t, n)| format!(", {t}{n}")).collect::<String>()
        );
        let mut args = vec!["ctx".to_string(), format!("(B_{a}_t *)self")];
        args.extend(sig_params(&sig, sig.eno).into_iter().map(|(_, n)| n));
        let set_eno = if eno && !sig.eno { "    *eno = 1;\n" } else { "" };
        let ret = if sig.gives() { "return " } else { "" };
        let _ = writeln!(protos, "{head};");
        let _ = writeln!(thunks, "{head} {{\n{set_eno}    {ret}{}({});\n}}\n", sig.c_name, args.join(", "));
        Ok(t)
    }
}

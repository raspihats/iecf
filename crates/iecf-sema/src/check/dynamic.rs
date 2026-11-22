//! CODESYS's and TwinCAT's `__NEW` and `__DELETE` (the dynamic-memory
//! extension, docs/extensions.md): memory taken while the program runs
//! from the running task's pool, `--dynamic-memory BYTES` of it, and
//! given back. `__NEW(T)`, `__NEW(T, n)` is a `POINTER TO T`, assigned
//! whole to a pointer; `__DELETE(p)` a statement of its own, which sets
//! `p` to 0. Each pointer to a block is checked as it is used, a block
//! given back too (runtime/iecf.h).

use super::{Checker, upper};
use crate::types::Ty;
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_syntax::ast::*;

/// Whether `e`, its parentheses aside, is a call of `name`.
pub(super) fn is_call(e: &Expr, name: &str) -> bool {
    match &e.kind {
        ExprKind::Paren(x) => is_call(x, name),
        ExprKind::Call { callee, .. } => matches!(&callee.kind, ExprKind::Name(id) if id.is(name)),
        _ => false,
    }
}

impl<'a> Checker<'a> {
    /// `__NEW(T)`, `__NEW(T, n)`: a pointer to `n` (1 without it) values
    /// of the data type `T`, each its type's initial value; 0 when the
    /// task's pool has no room.
    pub(super) fn new_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let assigned = std::mem::take(&mut self.new_ok);
        self.saw_new = true;
        self.out.dynamic = true;
        let (t, n) = match args {
            [Arg::Positional(t)] => (t, None),
            [Arg::Positional(t), Arg::Positional(n)] => (t, Some(n)),
            _ => {
                self.error(
                    Code::BadCall,
                    span,
                    "__NEW takes a type and, for an array of them, how many: __NEW(T), __NEW(T, n)",
                );
                self.args_only(args);
                return Ty::Unknown;
            }
        };
        let ty = match &t.kind {
            ExprKind::Name(id) if self.variable(&upper(&id.text)).is_none() && self.names_type(&id.text) => {
                self.resolve_spec(&TypeSpec::Named(id.clone()), true)
            }
            // A block's or a class's instance: left out (docs/extensions.md).
            ExprKind::Name(id)
                if self.pous.get(&upper(&id.text)).is_some_and(|p| {
                    matches!(p.kind, PouKind::FunctionBlock | PouKind::Class | PouKind::Interface)
                }) =>
            {
                self.error(
                    Code::ReferenceNotKept,
                    t.span,
                    format!(
                        "__NEW of {}, an instance, is not built: a pointer points to data only, and an instance \
                         made while the program runs would need its FB_init and FB_exit; declare the instance",
                        id.text
                    ),
                );
                Ty::Unknown
            }
            _ => {
                self.expr(t);
                self.error(Code::BadCall, t.span, "__NEW takes a type's name first: __NEW(INT, n)");
                Ty::Unknown
            }
        };
        if let Some(n) = n {
            let nt = self.expr(n);
            if !nt.is_unknown() && !nt.is_int() {
                self.error(Code::OperandTypes, n.span, format!("__NEW's count is an integer, not {nt}"));
            }
        }
        if !ty.is_unknown() && !self.plain_data(&ty, 0) {
            self.error(
                Code::ReferenceNotKept,
                t.span,
                format!(
                    "__NEW gives memory for data - an elementary value, a STRING, an array or a structure of them - \
                     not {ty}: a pointer points to data only, so no block is made with __NEW (no FB_init, no FB_exit)"
                ),
            );
        }
        if !assigned {
            self.error(
                Code::BadCall,
                span,
                "__NEW is the whole value assigned to a pointer, `p := __NEW(T, n)`, as CODESYS and TwinCAT take it",
            );
        }
        if self.options.dynamic_memory().is_none() {
            self.error(
                Code::BadCall,
                span,
                "__NEW takes its memory from the running task's pool, whose size the build is given: \
                 --dynamic-memory BYTES",
            );
        }
        Ty::Pointer(Box::new(ty))
    }

    /// `__DELETE(p)`: the block `p` points to given back, `p` set to 0.
    pub(super) fn delete_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let alone = std::mem::take(&mut self.delete_ok);
        self.out.dynamic = true;
        let [Arg::Positional(p)] = args else {
            self.error(Code::BadCall, span, "__DELETE takes one pointer variable: __DELETE(p)");
            self.args_only(args);
            return Ty::Void;
        };
        let t = self.place(p);
        if !matches!(t, Ty::Pointer(_) | Ty::Unknown) {
            self.error(Code::OperandTypes, p.span, format!("__DELETE takes a pointer variable, not {t}"));
        }
        if !alone {
            self.error(Code::BadCall, span, "__DELETE gives no value: it is a statement of its own, `__DELETE(p);`");
        }
        Ty::Void
    }

    /// Whether a value of `ty` holds a pointer: itself, an element, a
    /// field, or a variable of a function block instance.
    pub(super) fn holds_pointer(&mut self, ty: &Ty, depth: usize) -> bool {
        if depth > 32 {
            return false;
        }
        match ty {
            Ty::Pointer(_) => true,
            Ty::Array { elem, .. } => self.holds_pointer(elem, depth + 1),
            Ty::Struct(k) => {
                let fields: Vec<&'a VarDecl> = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().collect(),
                    _ => Vec::new(),
                };
                fields.into_iter().any(|f| {
                    let t = self.resolve_spec(&f.ty, false);
                    self.holds_pointer(&t, depth + 1)
                })
            }
            Ty::Fb(k) => {
                let decls: Vec<&'a VarDecl> = match self.pous.get(k) {
                    Some(p) => p
                        .var_blocks
                        .iter()
                        .filter(|b| !matches!(b.kind, VarKind::Temp | VarKind::External | VarKind::InOut))
                        .flat_map(|b| b.decls.iter())
                        .collect(),
                    None => Vec::new(),
                };
                decls.into_iter().any(|d| {
                    let t = self.resolve_spec(&d.ty, false);
                    self.holds_pointer(&t, depth + 1)
                })
            }
            _ => false,
        }
    }

    /// In a project that calls `__NEW`, a pointer kept across a restart
    /// (W0006 so far) is refused, E0027: it may hold a block of a pool,
    /// which every start empties.
    pub(super) fn retained_pointers(&mut self) {
        if !self.saw_new {
            return;
        }
        for k in std::mem::take(&mut self.retained_ptrs) {
            let Some(d) = self.diags.get(k) else { continue };
            if d.code != Code::KeptReference {
                continue;
            }
            let span = d.span;
            self.diags[k] = Diagnostic::new(
                Code::ReferenceNotKept,
                span,
                "this RETAIN variable holds a pointer, and the project calls __NEW (dynamic-memory): a pointer may \
                 hold a block of a task's pool, which every start empties, so none is kept across a restart - \
                 declare it VAR NON_RETAIN",
            );
        }
    }
}

//! CODESYS's pointers, the pointers extension (POINTERS.md,
//! docs/extensions.md): `POINTER TO T`, `ADR(x)`, `SIZEOF(x)`, `p^`,
//! arithmetic and comparison in bytes. A pointer is an address inside
//! one variable, checked where it is used (the generator's fat pointer);
//! the checker keeps a pointer that may reach a variable gone when its
//! POU returns from being kept where it would outlive it.

use super::{Checker, upper};
use crate::Extension;
use crate::types::{Elem, Ty};
use iecf_diagnostics::{Code, Span};
use iecf_syntax::ast::*;

impl<'a> Checker<'a> {
    /// `POINTER TO to`, written at `span`: refused without the extension,
    /// and to anything but data.
    pub(super) fn pointer_type(&mut self, to: Ty, span: Span, report: bool) -> Ty {
        if report && !self.options.has(Extension::Pointers) {
            self.error(
                Code::UnexpectedToken,
                span,
                "POINTER TO is CODESYS's, not IEC 61131-3: Ed.3's REF_TO references a variable (Table 12), or \
                 --extension pointers",
            );
        }
        if report && self.options.has(Extension::Pointers) && !to.is_unknown() && !self.plain_data(&to, 0) {
            self.error(
                Code::ReferenceNotKept,
                span,
                format!(
                    "a pointer points to data - an elementary value, a STRING, an array or a structure of them, \
                     without a reference, an interface, an instance or a pointer in it - not {to}"
                ),
            );
        }
        Ty::Pointer(Box::new(to))
    }

    /// Whether a value of `ty` is data a pointer may point to, whose bytes
    /// it may read and write: no reference, interface, instance or
    /// pointer in it.
    pub(super) fn plain_data(&mut self, ty: &Ty, depth: usize) -> bool {
        if depth > 32 {
            return false;
        }
        match ty {
            Ty::Elem(_) | Ty::Str { .. } | Ty::Enum(_) | Ty::Unknown => true,
            Ty::Array { elem, .. } => self.plain_data(elem, depth + 1),
            Ty::Struct(k) => {
                let fields: Vec<&'a VarDecl> = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().collect(),
                    _ => Vec::new(),
                };
                fields.into_iter().all(|f| {
                    let t = self.resolve_spec(&f.ty, false);
                    self.plain_data(&t, depth + 1)
                })
            }
            _ => false,
        }
    }

    /// `ADR(x)`: a pointer to the variable, element or field `x`, of its
    /// type.
    pub(super) fn adr_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let [Arg::Positional(x)] = args else {
            self.error(Code::BadCall, span, "ADR takes one variable: ADR(x)");
            self.args_only(args);
            return Ty::Unknown;
        };
        if !is_place(x) {
            self.expr(x);
            self.error(
                Code::BadCall,
                x.span,
                "ADR takes a variable, an element or a field: a value has no address (store it in a variable first)",
            );
            return Ty::Unknown;
        }
        let t = self.expr(x);
        if let Some(root) = root_name(x)
            && let Some(v) = self.variable(&upper(&root.text))
            && v.constant
        {
            self.error(
                Code::NotAssignable,
                x.span,
                format!("ADR of '{}', a CONSTANT: a pointer could write it", root.text),
            );
        }
        if !t.is_unknown() && !self.plain_data(&t, 0) {
            self.error(
                Code::ReferenceNotKept,
                x.span,
                format!("ADR takes data - an elementary value, a STRING, an array or a structure of them - not {t}"),
            );
            return Ty::Unknown;
        }
        Ty::Pointer(Box::new(t))
    }

    /// `SIZEOF(x)`: the size in bytes of a variable or a type, of the
    /// smallest unsigned type that holds it, as CODESYS's (USINT, UINT,
    /// UDINT, ULINT). The value is the C type's on the platform, which
    /// the generator writes; the type is chosen here from iecf's layout,
    /// which no platform's exceeds.
    pub(super) fn sizeof_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let [Arg::Positional(x)] = args else {
            self.error(Code::BadCall, span, "SIZEOF takes one variable or type: SIZEOF(x)");
            self.args_only(args);
            return Ty::Unknown;
        };
        let t = match &x.kind {
            // A type's name, where no variable has it.
            ExprKind::Name(id) if self.variable(&upper(&id.text)).is_none() && self.names_type(&id.text) => {
                let t = self.resolve_spec(&TypeSpec::Named(id.clone()), true);
                self.out.sizeof_types.insert(x.span, t.clone());
                t
            }
            _ if is_place(x) => self.expr(x),
            _ => {
                self.expr(x);
                self.error(Code::BadCall, x.span, "SIZEOF takes a variable, an element, a field or a type");
                return Ty::Unknown;
            }
        };
        if matches!(t, Ty::Pointer(_)) {
            self.error(
                Code::BadCall,
                x.span,
                "SIZEOF of a pointer is no portable number - CODESYS gives 4 or 8 by the platform, and iecf's \
                 checked pointer is larger: for the size of what it points to write SIZEOF(p^), of one element \
                 SIZEOF(p^[0])",
            );
            return Ty::Unknown;
        }
        if t.is_unknown() {
            return Ty::Unknown;
        }
        if !self.plain_data(&t, 0) {
            self.error(
                Code::BadCall,
                x.span,
                format!("SIZEOF takes data - an elementary value, a STRING, an array or a structure of them - not {t}"),
            );
            return Ty::Unknown;
        }
        let n = self.layout_size(&t, 0).unwrap_or(u64::from(u32::MAX) + 1);
        Ty::Elem(if n <= 0xFF {
            Elem::Usint
        } else if n <= 0xFFFF {
            Elem::Uint
        } else if n <= 0xFFFF_FFFF {
            Elem::Udint
        } else {
            Elem::Ulint
        })
    }

    /// Whether `name` is a type: elementary, STRING, or the project's.
    fn names_type(&self, name: &str) -> bool {
        let k = upper(name);
        Elem::from_name(&k).is_some() || matches!(k.as_str(), "STRING" | "WSTRING") || self.types.contains_key(&k)
    }

    /// The size of `ty` in iecf's layout - each part aligned to its own
    /// size, a structure to its largest part's - which bounds the C
    /// type's size on any platform: the type of SIZEOF.
    fn layout_size(&mut self, ty: &Ty, depth: usize) -> Option<u64> {
        Some(self.layout(ty, depth)?.0)
    }

    /// Size and alignment.
    fn layout(&mut self, ty: &Ty, depth: usize) -> Option<(u64, u64)> {
        if depth > 32 {
            return None;
        }
        Some(match ty {
            Ty::Elem(e) => {
                let n = elem_size(*e);
                (n, n)
            }
            Ty::Str { wide, len } => {
                let n = len.unwrap_or(254) + 1;
                if *wide { (2 * n, 2) } else { (n, 1) }
            }
            Ty::Enum(_) => (8, 8),
            Ty::Array { elem, dims } => {
                let (n, a) = self.layout(elem, depth + 1)?;
                let mut count: u64 = 1;
                for (lo, hi) in dims.iter().map(|d| d.unwrap_or((0, 0))) {
                    count = count.checked_mul(u64::try_from(hi - lo + 1).ok()?)?;
                }
                (n.checked_mul(count)?, a)
            }
            Ty::Struct(k) => {
                let fields: Vec<&'a VarDecl> = match self.types.get(k).map(|t| &t.def) {
                    Some(TypeDef::Struct { fields, .. }) => fields.iter().collect(),
                    _ => return None,
                };
                let (mut size, mut align) = (0u64, 1u64);
                for f in fields {
                    let t = self.resolve_spec(&f.ty, false);
                    let (n, a) = self.layout(&t, depth + 1)?;
                    for _ in &f.names {
                        size = size.div_ceil(a) * a + n;
                    }
                    align = align.max(a);
                }
                (size.div_ceil(align) * align, align)
            }
            _ => return None,
        })
    }

    /// `+ - = <> < <= > >=` where an operand is a pointer: None when
    /// neither is. A pointer plus or minus an integer is a pointer of its
    /// type, moved by that many bytes; two pointers subtracted, a DINT;
    /// compared, BOOL - with 0 or NULL too, for = and <>.
    pub(super) fn pointer_binary(&mut self, op: BinaryOp, a: &Ty, b: &Ty, span: Span) -> Option<Ty> {
        use BinaryOp::*;
        let (pa, pb) = (matches!(a, Ty::Pointer(_)), matches!(b, Ty::Pointer(_)));
        if !pa && !pb {
            return None;
        }
        let int = |t: &Ty| t.is_int() || t.elem().and_then(Elem::unsigned_of).is_some();
        let null = |t: &Ty| matches!(t, Ty::IntLit(Some(0)) | Ty::Null);
        Some(match op {
            Add if pa && int(b) => a.clone(),
            Add if pb && int(a) => b.clone(),
            Sub if pa && int(b) => a.clone(),
            Sub if pa && pb => Ty::Elem(Elem::Dint),
            Eq | Ne if (pa && pb) || (pa && null(b)) || (pb && null(a)) => Ty::Elem(Elem::Bool),
            Lt | Le | Gt | Ge if pa && pb => Ty::Elem(Elem::Bool),
            _ => {
                self.error(
                    Code::OperandTypes,
                    span,
                    format!(
                        "a pointer takes + and - of an integer (bytes), - of another pointer, the comparisons \
                         with a pointer, and = and <> with 0; not {a} and {b} (an address is no number)"
                    ),
                );
                Ty::Unknown
            }
        })
    }

    /// `target := value`, `target` a pointer: refused where `value` may
    /// point into a variable gone when the POU returns and `target`
    /// outlives the call.
    pub(super) fn pointer_kept(&mut self, target: &Expr, value: &Expr) {
        if self.options.has(Extension::Pointers) && self.escapes(target) && self.may_point_to_temp(value) {
            self.temp_pointer_error(value.span);
        }
    }

    /// A pointer given to a block's input, which the instance keeps.
    pub(super) fn pointer_kept_arg(&mut self, value: &Expr) {
        if self.options.has(Extension::Pointers) && self.may_point_to_temp(value) {
            self.temp_pointer_error(value.span);
        }
    }

    fn temp_pointer_error(&mut self, at: Span) {
        self.error(
            Code::ReferenceNotKept,
            at,
            "this pointer may point into a variable gone when its POU returns - a FUNCTION's or method's, a \
             VAR_TEMP, an in-out's - and is kept where it outlives the call: take the address of a variable that \
             lives as long as the program, or keep the pointer in a temporary",
        );
    }

    /// Whether a pointer stored at `target` outlives the POU's call: all
    /// but the POU's own temporaries (not an in-out, an output or a
    /// FUNCTION's result).
    fn escapes(&mut self, target: &Expr) -> bool {
        match &target.kind {
            ExprKind::Paren(x) => self.escapes(x),
            ExprKind::Name(id) => {
                let k = upper(&id.text);
                let fn_like = self.pou_kind == Some(PouKind::Function);
                if fn_like && self.pou_key.rsplit("__").next() == Some(k.as_str()) {
                    return true;
                }
                match self.variable(&k) {
                    Some(v) if v.temp => matches!(self.own_section(&k), Some(VarKind::InOut | VarKind::Output)),
                    _ => true,
                }
            }
            ExprKind::Index(b, _) | ExprKind::Member(b, _) => self.escapes(b),
            _ => true,
        }
    }

    /// Whether the pointer `value` may point into a variable gone when the
    /// POU returns: the address of a temporary or of an in-out, or a
    /// pointer held in a temporary, given as an input, or reached
    /// through an in-out - whose value no one here can vouch for.
    fn may_point_to_temp(&mut self, value: &Expr) -> bool {
        match &value.kind {
            ExprKind::Paren(x) => self.may_point_to_temp(x),
            ExprKind::Call { callee, args } if matches!(&callee.kind, ExprKind::Name(id) if id.is("ADR")) => {
                match args.as_slice() {
                    [Arg::Positional(x)] => self.address_of_temp(x),
                    _ => false,
                }
            }
            ExprKind::Binary(BinaryOp::Add | BinaryOp::Sub, a, b) => {
                self.may_point_to_temp(a) || self.may_point_to_temp(b)
            }
            ExprKind::Name(_) | ExprKind::Index(..) | ExprKind::Member(..) => self.held_in_temp(value),
            _ => false,
        }
    }

    /// Whether `x`, whose address is taken, is a temporary or an in-out:
    /// through a pointer, whatever that pointer may point to.
    fn address_of_temp(&mut self, x: &Expr) -> bool {
        match &x.kind {
            ExprKind::Paren(b) | ExprKind::Index(b, _) | ExprKind::Member(b, _) => self.address_of_temp(b),
            ExprKind::Deref(p) => self.may_point_to_temp(p),
            ExprKind::Name(id) => {
                let k = upper(&id.text);
                match self.variable(&k) {
                    Some(v) => v.temp || self.own_section(&k) == Some(VarKind::InOut),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// Whether the variable `e` reads is the POU's temporary or in-out.
    fn held_in_temp(&mut self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Paren(b) | ExprKind::Index(b, _) | ExprKind::Member(b, _) => self.held_in_temp(b),
            ExprKind::Name(id) => {
                let k = upper(&id.text);
                match self.variable(&k) {
                    Some(v) => v.temp || self.own_section(&k) == Some(VarKind::InOut),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// The section the POU being checked declares `key` in, if it does.
    fn own_section(&self, key: &str) -> Option<VarKind> {
        let pou = self.cur_pou?;
        pou.var_blocks.iter().find_map(|b| b.decls.iter().any(|d| d.names.iter().any(|n| n.is(key))).then_some(b.kind))
    }
}

/// Whether `e` names a variable, an element or a field: what has an
/// address.
fn is_place(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Name(_) | ExprKind::Deref(_) => true,
        ExprKind::Paren(b) | ExprKind::Index(b, _) | ExprKind::Member(b, _) => is_place(b),
        _ => false,
    }
}

/// The variable a place is part of, unless through a pointer.
fn root_name(e: &Expr) -> Option<&Ident> {
    match &e.kind {
        ExprKind::Name(id) => Some(id),
        ExprKind::Paren(b) | ExprKind::Index(b, _) | ExprKind::Member(b, _) => root_name(b),
        _ => None,
    }
}

/// An elementary type's size in bytes, as runtime/iecf.h holds it.
fn elem_size(e: Elem) -> u64 {
    use Elem::*;
    match e {
        Bool | Sint | Usint | Byte | Char => 1,
        Int | Uint | Word | Wchar => 2,
        Dint | Udint | Dword | Real | Time | Date | Tod | Dt => 4,
        _ => 8,
    }
}

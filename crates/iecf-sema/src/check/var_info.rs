//! CODESYS's and TwinCAT's `__VARINFO(x)` (the var-info extension,
//! var_info.rs): a `__SYSTEM.VAR_INFO` of the variable, element, field
//! or bit `x`, from what the check knows of it.

use super::{Checker, type_class};
use crate::types::Ty;
use crate::{VarInfo, var_info::VAR_INFO};
use iecf_diagnostics::{Code, Span};
use iecf_syntax::ast::*;

impl<'a> Checker<'a> {
    pub(super) fn var_info_call(&mut self, args: &[Arg], span: Span) -> Ty {
        let gives = Ty::Struct(VAR_INFO.into());
        let [Arg::Positional(x)] = args else {
            self.error(Code::BadCall, span, "__VARINFO takes one variable: __VARINFO(x)");
            self.args_only(args);
            return gives;
        };
        let Some(symbol) = symbol(x) else {
            self.expr(x);
            self.error(Code::BadCall, x.span, "__VARINFO takes a variable, an element, a field or a bit of one");
            return gives;
        };
        let t = self.expr(x);
        if t.is_unknown() {
            return gives;
        }
        let elements = match &t {
            Ty::Array { elem, dims } => {
                let mut n: u64 = 1;
                for d in dims {
                    let Some((lo, hi)) = d else {
                        self.error(
                            Code::BadCall,
                            x.span,
                            "__VARINFO of an ARRAY[*]: how many elements it has, only the run knows",
                        );
                        return gives;
                    };
                    n = n.saturating_mul(u64::try_from(*hi - *lo + 1).unwrap_or(0));
                }
                Some((class(elem), n))
            }
            _ => None,
        };
        let mut inner = x;
        while let ExprKind::Paren(b) = &inner.kind {
            inner = b;
        }
        let bit = match &inner.kind {
            ExprKind::BitAccess(_, l) => match l.part() {
                Some((PartSize::Bit, n)) => u32::try_from(n).ok(),
                _ => None,
            },
            _ => None,
        };
        let type_name = t.to_string().replace("__", ".");
        self.out.var_infos.insert(span, VarInfo { symbol, type_name, class: class(&t), elements, bit });
        gives
    }
}

fn class(t: &Ty) -> usize {
    let c = type_class(t);
    crate::any::CLASSES.iter().position(|k| *k == c).unwrap_or(0)
}

/// The name `x` is known by: a variable's, a field's; of an element or a
/// bit, its array's or bit string's. None for what is no variable.
fn symbol(x: &Expr) -> Option<String> {
    match &x.kind {
        ExprKind::Name(id) | ExprKind::Member(_, id) => Some(id.text.clone()),
        ExprKind::Paren(b) | ExprKind::Index(b, _) | ExprKind::BitAccess(b, _) | ExprKind::Deref(b) => symbol(b),
        _ => None,
    }
}

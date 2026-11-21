//! `__VARINFO(x)` (the var-info extension, iecf-sema var_info.rs): a
//! `__SYSTEM.VAR_INFO` filled in a temporary before the statement, from
//! what the check worked out of `x` and the platform's C sizes. `x` is
//! not read, nor its indices worked out: what is known of it is its
//! declaration's.

use crate::derived::{DTy, Kind};
use crate::names::{Lv, Scope};
use crate::{Gen, R};
use iecf_sema::types::Ty;
use iecf_syntax::ast::*;

/// `__SYSTEM.MEMORY_AREA#MEM_MEMORY`, CODESYS's initial value: iecf has
/// no memory areas a runtime numbers.
const MEM_MEMORY: usize = 2;

impl Gen<'_> {
    /// The C lines filling `__VARINFO(x)`, the call `e`, and the
    /// temporary they fill.
    pub(crate) fn var_info_value(&mut self, e: &Expr, scope: &Scope) -> R<(Vec<String>, Lv, DTy)> {
        let Some(info) = self.types.var_info(e.span).cloned() else {
            return self.not_yet(e.span, "this __VARINFO");
        };
        let ExprKind::Call { args, .. } = &e.kind else { return self.not_yet(e.span, "this __VARINFO") };
        let [Arg::Positional(x)] = args.as_slice() else { return self.not_yet(e.span, "this __VARINFO") };
        let name = Ident { text: iecf_sema::var_info::VAR_INFO.into(), span: e.span };
        let Some(t) = self.derived(&TypeSpec::Named(name))? else {
            return self.not_yet(e.span, "__SYSTEM.VAR_INFO");
        };
        let x = crate::expr::strip_paren(x);
        // x's size in bits, and an array's element's.
        let (bits, elem_bits) = match (&x.kind, info.bit) {
            (_, Some(_)) => ("1".to_string(), "0".to_string()),
            (ExprKind::BitAccess(..), None) => match self.types.expr(x.span) {
                Some(Ty::Elem(el)) => (format!("sizeof({}) * 8", crate::c_type(*el)), "0".to_string()),
                _ => return self.not_yet(x.span, "__VARINFO of this"),
            },
            _ => {
                let Some(d) = self.of_place(x, scope)? else { return self.not_yet(x.span, "__VARINFO of this") };
                let c = self.declare(&d, "");
                let elem = match &d.kind {
                    Kind::Array(el, _) => format!("sizeof({}) * 8", self.declare(el, "")),
                    _ => "0".to_string(),
                };
                (format!("sizeof({c}) * 8"), elem)
            }
        };
        let v = self.fresh("vi");
        let (base, count) = info.elements.unwrap_or((0, 0));
        let lines = vec![
            format!("{};", self.declare(&t, &v)),
            format!("{v}.V_BYTEADDRESS = 0; {v}.V_BYTEOFFSET = 0; {v}.V_AREA = 0; {v}.V_BITADDRESS = 0;"),
            format!("{v}.V_BITNO = (IEC_INT){};", info.bit.unwrap_or(0)),
            format!("{v}.V_BITSIZE = (IEC_INT)({bits});"),
            format!("{v}.V_TYPECLASS = {};", info.class),
            format!("iecf_sinit({v}.V_TYPENAME, 79u, \"{}\");", c_text(&info.type_name)),
            format!("{v}.V_NUMELEMENTS = (IEC_UDINT){count}u;"),
            format!("{v}.V_BASETYPECLASS = {base};"),
            format!("{v}.V_ELEMBITSIZE = (IEC_UDINT)({elem_bits});"),
            format!("{v}.V_MEMORYAREA = {MEM_MEMORY};"),
            format!("iecf_sinit({v}.V_SYMBOL, 39u, \"{}\");", c_text(&info.symbol)),
            format!("iecf_sinit({v}.V_COMMENT, 79u, \"\");"),
        ];
        Ok((lines, Lv::C(v), t))
    }

    /// The derived type of the place `x`, an element's from its array's:
    /// no index worked out.
    fn of_place(&mut self, x: &Expr, scope: &Scope) -> R<Option<DTy>> {
        match &crate::expr::strip_paren(x).kind {
            ExprKind::Index(b, ix) => {
                let Some(d) = self.of_place(b, scope)? else { return Ok(None) };
                let Kind::Array(el, bounds) = d.kind else { return Ok(None) };
                // `a[i, j]` of an ARRAY[.., ..]: one element.
                if ix.len() != bounds.len() {
                    return Ok(None);
                }
                Ok(Some(*el))
            }
            ExprKind::Member(b, _) if matches!(&b.kind, ExprKind::Index(..)) => match self.types.expr(x.span) {
                Some(Ty::Elem(el)) => Ok(Some(DTy { kind: Kind::Elem(*el), init: None, range: None })),
                _ => Ok(None),
            },
            _ => self.place_type(x, scope),
        }
    }
}

/// `s` in a C string literal: names and type names, so no quote or
/// backslash; anything else outside ASCII's printable characters escaped.
fn c_text(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' | '\\' => format!("\\{c}"),
            ' '..='~' => c.to_string(),
            c => format!("\\x{:02x}", c as u32 & 0xff),
        })
        .collect()
}

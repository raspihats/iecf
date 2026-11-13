//! CONSTANT variables named where a constant expression is wanted: an
//! initial value (Ed.3 6.6.1.2.4, Figure 6). The variable is not in the
//! scope there - a FUNCTION's input is given its initial value by the
//! caller, a TYPE's by whatever declares a variable of it - so its name
//! stands for its own initial value, worked out in its place: the
//! POU's own `VAR CONSTANT`, or a `VAR_GLOBAL CONSTANT` through its
//! `VAR_EXTERNAL CONSTANT`, or directly in a TYPE, as the checker lets
//! each be named (docs/dependencies.md).

use crate::derived::{DTy, Kind, count};
use crate::expr::{arg_value, const_int, conversion_names, expr_duration, lit, math_function, trunc_names};
use crate::names::Scope;
use crate::{Decl, Gen, R, c_type};
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::{Arg, BinaryOp, Expr, ExprKind, Init, LiteralKind, PouKind, UnaryOp, VarDecl, VarKind};

/// The value of a constant expression, worked out by the compiler.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Cv {
    /// An integer, a bit string, or a duration in its type's unit.
    Int(i128),
    Real(f64),
    Bool(bool),
}

/// `f` as a result of type `ty`: a REAL's in single precision.
fn round_real(ty: Option<Elem>, f: f64) -> f64 {
    if ty == Some(Elem::Real) { f as f32 as f64 } else { f }
}

/// An initial value's mark while it is worked out, by where it is.
fn span_key(at: Span) -> String {
    format!("@{:?}:{}", at.file, at.start)
}

/// A number as a real.
fn cv_f64(v: Cv) -> Option<f64> {
    match v {
        Cv::Int(i) => Some(i as f64),
        Cv::Real(f) => Some(f),
        Cv::Bool(_) => None,
    }
}

impl<'a> Gen<'a> {
    /// The declaration of the CONSTANT `name` as the code at `at` sees
    /// it. A TYPE sees the global ones only; a POU its own, then - a
    /// method - its type's, then those of the type's bases, then the
    /// global ones, a name declared otherwise hiding what is further out.
    pub(crate) fn const_decl(&self, name: &str, at: Span) -> Option<&'a VarDecl> {
        let key = name.to_ascii_uppercase();
        let within = |s: Span| s.file == at.file && s.start <= at.start && at.end <= s.end;
        let in_type = self.type_decls.values().any(|t| within(t.span));
        let mut keys: Vec<String> = Vec::new();
        if !in_type && let Some(ctx) = self.const_ctx.last() {
            keys.push(ctx.clone());
            let mut owner = ctx.as_str();
            if self.pous.get(owner).is_some_and(|p| p.kind == PouKind::Method)
                && let Some((t, _)) = ctx.rsplit_once("__")
            {
                keys.push(t.to_string());
                owner = t;
            }
            keys.extend(self.types.ancestors(owner));
        }
        for k in &keys {
            let Some(pou) = self.pous.get(k.as_str()) else { continue };
            for b in pou.var_blocks.iter().filter(|b| b.constant) {
                for d in &b.decls {
                    if d.names.iter().any(|n| n.text.eq_ignore_ascii_case(&key)) {
                        return match b.kind {
                            VarKind::External => self.global_consts.get(&key).copied(),
                            _ => Some(d),
                        };
                    }
                }
            }
            // A name the POU declares otherwise hides one further out.
            let own = pou.var_blocks.iter().flat_map(|b| &b.decls).flat_map(|d| &d.names);
            if own.into_iter().any(|n| n.text.eq_ignore_ascii_case(&key)) {
                return None;
            }
        }
        self.global_consts.get(&key).copied()
    }

    /// E0032: a constant whose initial value names itself has no value.
    pub(crate) fn const_cycle<T>(&mut self, at: Span) -> R<T> {
        self.diags.push(Diagnostic::new(
            Code::NotConstant,
            at,
            "this constant's initial value names the constant itself: it has no value",
        ));
        Err(crate::Skip)
    }

    /// The initial value of the CONSTANT a bare name in `e` is, when it
    /// is one: what an initial value of a string, an array or a structure
    /// that names a constant stands for.
    pub(crate) fn const_init(&self, e: &Expr) -> Option<&'a Init> {
        match &e.kind {
            ExprKind::Paren(x) => self.const_init(x),
            ExprKind::Name(id) => self.const_decl(&id.text, id.span)?.init.as_ref(),
            _ => None,
        }
    }

    /// A CONSTANT, or a part of one that is an array or a structure
    /// (`KA[2]`, `KP.x`): its type, and the initial value it has - the
    /// one given, or its type's. An index outside the array has no value
    /// (`fold_bad`, E0032), as it would fault when the program runs.
    pub(crate) fn const_part(&mut self, e: &Expr) -> Option<(DTy, Option<Init>)> {
        match &e.kind {
            ExprKind::Paren(x) => self.const_part(x),
            ExprKind::Name(id) => {
                let d = self.const_decl(&id.text, id.span)?;
                // Its own mark, apart from that of a value being worked out.
                let key = format!("#{}", id.text.to_ascii_uppercase());
                if self.const_busy.contains(&key) {
                    return None;
                }
                let t = match self.decl(d).ok()? {
                    Decl::Data(t) => t,
                    Decl::Elem(e) => DTy { kind: Kind::Elem(e), init: None, range: None },
                    _ => return None,
                };
                let init = d.init.clone().or_else(|| t.init.clone());
                self.const_busy.push(key);
                let r = self.const_chase(t, init);
                self.const_busy.pop();
                r
            }
            ExprKind::Member(b, f) => {
                let (t, init) = self.const_part(b)?;
                let Kind::Struct(k) = &t.kind else { return None };
                // An OVERLAP's field is what its bytes hold, read at run time.
                if self.layouts.get(k).is_some_and(|l| l.overlap) {
                    return None;
                }
                let fields = self.structs.get(k)?.clone();
                let (_, ft) = fields.iter().find(|(n, _)| n.eq_ignore_ascii_case(&f.text))?;
                let given = match &init {
                    Some(Init::Struct { fields, .. }) => {
                        fields.iter().find(|(n, _)| n.text.eq_ignore_ascii_case(&f.text)).map(|(_, i)| i.clone())
                    }
                    Some(_) => return None,
                    None => None,
                };
                self.const_chase(ft.clone(), given.or_else(|| ft.init.clone()))
            }
            ExprKind::Index(b, idx) => {
                let (t, init) = self.const_part(b)?;
                let Kind::Array(elem, bounds) = &t.kind else { return None };
                if idx.len() != bounds.len() {
                    return None;
                }
                let mut flat: u64 = 0;
                for (i, (lo, hi)) in idx.iter().zip(bounds) {
                    let (Cv::Int(v), false) = self.fold_as(i, None)? else { return None };
                    if v < i128::from(*lo) || v > i128::from(*hi) {
                        self.fold_none(i.span, &format!("indexes element {v} of an array of {lo}..{hi}"));
                        return None;
                    }
                    flat = flat.checked_mul(count(*lo, *hi)?)?.checked_add((v - i128::from(*lo)) as u64)?;
                }
                let given = match &init {
                    Some(Init::Array { elems, .. }) => {
                        let mut from: u64 = 0;
                        let mut found = None;
                        for el in elems {
                            let n = match &el.repeat {
                                Some(r) => u64::try_from(self.cint(r)?).ok()?,
                                None => 1,
                            };
                            if flat < from.saturating_add(n) {
                                found = el.value.clone();
                                break;
                            }
                            from += n;
                        }
                        found
                    }
                    Some(_) => return None,
                    None => None,
                };
                let elem = (**elem).clone();
                let init = given.or_else(|| elem.init.clone());
                self.const_chase(elem, init)
            }
            _ => None,
        }
    }

    /// `init`, of an array, a structure or a string `t`, where it names
    /// a constant or a part of one: that one's own.
    fn const_chase(&mut self, t: DTy, mut init: Option<Init>) -> Option<(DTy, Option<Init>)> {
        if t.scalar().is_none() {
            let mut seen = 0;
            while let Some(Init::Expr(x)) = &init {
                let x = x.clone();
                if !matches!(
                    crate::expr::strip_paren(&x).kind,
                    ExprKind::Name(_) | ExprKind::Member(..) | ExprKind::Index(..)
                ) {
                    break;
                }
                // Each initial value once: one reached again names itself.
                let key = span_key(x.span);
                seen += 1;
                if seen > 64 || self.const_busy.contains(&key) {
                    self.fold_bad.get_or_insert((
                        Code::NotConstant,
                        x.span,
                        "this constant's initial value names the constant itself: it has no value".into(),
                    ));
                    return None;
                }
                self.const_busy.push(key);
                let part = self.const_part(&x);
                self.const_busy.pop();
                let Some((_, next)) = part else { break };
                init = next;
            }
        }
        Some((t, init))
    }

    /// For a whole array, structure or string given `x`: the initial
    /// value of the part of a constant `x` is (`Some(None)`: its type's
    /// own), `None` when it is no part of one.
    pub(crate) fn const_whole_part(&mut self, x: &Expr) -> R<Option<Option<Init>>> {
        if !matches!(crate::expr::strip_paren(x).kind, ExprKind::Member(..) | ExprKind::Index(..)) {
            return Ok(None);
        }
        self.fold_bad = None;
        let part = self.const_part(x);
        if let Some((code, at, why)) = self.fold_bad.take() {
            self.diags.push(Diagnostic::new(code, at, why));
            return Err(crate::Skip);
        }
        Ok(part.map(|(_, init)| init))
    }

    /// The scalar part of a constant `e` is (`KA[2]`), as C, where no
    /// variable is in scope: in an initial value.
    pub(crate) fn const_part_value(&mut self, e: &Expr, scope: &Scope) -> R<Option<String>> {
        let Some((t, init)) = self.const_part(e) else { return Ok(None) };
        let Some(elem) = t.scalar() else { return Ok(None) };
        Ok(Some(match init {
            Some(Init::Expr(x)) => {
                let root = span_key(x.span);
                if self.const_busy.contains(&root) {
                    return self.const_cycle(e.span);
                }
                self.const_busy.push(root);
                let v = self.value_as(&x, elem, scope);
                self.const_busy.pop();
                v?
            }
            Some(_) => return Ok(None),
            None => self.enum_initial(&t, None).unwrap_or_else(|_| crate::expr::zero(elem)),
        }))
    }

    /// The value of the elementary CONSTANT `name`, as C: its initial
    /// value worked out here, its type's own when it has none. `None`
    /// when `name` is no constant of an elementary type.
    pub(crate) fn const_value(&mut self, name: &str, at: Span) -> R<Option<String>> {
        let Some(d) = self.const_decl(name, at) else { return Ok(None) };
        let key = name.to_ascii_uppercase();
        if self.const_busy.contains(&key) {
            return self.const_cycle(at);
        }
        let ty = match self.decl(d)? {
            Decl::Elem(t) => t,
            Decl::Data(t) => match t.scalar() {
                Some(e) => e,
                None => return Ok(None),
            },
            _ => return Ok(None),
        };
        let v = match &d.init {
            Some(Init::Expr(x)) => {
                self.const_busy.push(key);
                let v = self.value_as(x, ty, &Scope::default());
                self.const_busy.pop();
                v?
            }
            Some(_) => return Ok(None),
            None => crate::expr::zero(ty),
        };
        Ok(Some(v))
    }
}

impl Gen<'_> {
    /// The elementary type of the constant `name`.
    fn const_type(&mut self, d: &VarDecl) -> Option<Elem> {
        match self.decl(d) {
            Ok(Decl::Elem(t)) => Some(t),
            Ok(Decl::Data(t)) => t.scalar(),
            _ => None,
        }
    }

    /// `v`, a value of type `from`, as the type `to`: an integer as a
    /// real, a real as a longer one; `None` where the two do not convert
    /// by widening or the value does not fit.
    fn cv_as(v: Cv, from: Option<Elem>, to: Elem) -> Option<Cv> {
        match v {
            Cv::Bool(_) => (to == Elem::Bool).then_some(v),
            Cv::Int(i) if to.is_real() => {
                // A duration is no number.
                if from.is_some_and(|f| !f.is_int()) {
                    return None;
                }
                let f = i as f64;
                Some(Cv::Real(if to == Elem::Real { f as f32 as f64 } else { f }))
            }
            Cv::Int(_) => (to != Elem::Bool).then_some(v),
            Cv::Real(f) if to.is_real() => Some(Cv::Real(if to == Elem::Real { f as f32 as f64 } else { f })),
            Cv::Real(_) => None,
        }
    }

    /// The value of `e` when it is a constant expression (Ed.3
    /// 6.6.1.2.4): literals, CONSTANT variables, the operators and the
    /// standard functions on them, worked out as the checker works an
    /// integer one out - exactly, not in the operands' width - and a REAL
    /// one in single precision. `None` for anything else, or one the
    /// compiler leaves to the run: a date, a string, a duration times a
    /// real. The mathematics of Table 28 and `**` are the target's: what
    /// is worked out through them (`fold_inexact`) only says that the
    /// target's value is one its type holds.
    pub(crate) fn fold(&mut self, e: &Expr) -> Option<Cv> {
        let ty = match self.types.expr(e.span) {
            Some(Ty::Elem(t)) => Some(*t),
            Some(Ty::Enum(k)) => {
                let k = k.clone();
                return self.fold_enum(e, &k);
            }
            // An untyped real is of the type wanted of it.
            Some(Ty::RealLit) => Some(self.fold_want.filter(|t| t.is_real()).unwrap_or(Elem::Lreal)),
            _ => None,
        };
        let v = self.fold_in(e, ty)?;
        if let Cv::Real(f) = v {
            let t = ty.map_or("its type", Elem::name);
            // A real that is infinite or no number: the run would flag it.
            if !f.is_finite() {
                self.fold_bad.get_or_insert((
                    Code::LiteralRange,
                    e.span,
                    format!("this constant expression is {f}, which {t} does not hold"),
                ));
                return None;
            }
            // The target's mathematics may differ from the compiler's in
            // the last digits: no nearer the type's limit than this.
            let max = if ty == Some(Elem::Real) { f64::from(f32::MAX) } else { f64::MAX };
            if self.fold_inexact && f.abs() > max / 4.0 {
                self.fold_bad.get_or_insert((
                    Code::LiteralRange,
                    e.span,
                    format!("this constant expression is about {f:e}, too near the largest {t} for an initial value"),
                ));
                return None;
            }
        }
        Some(v)
    }

    /// [`Self::fold`] with an untyped real literal in `e` of the type
    /// `want`, and whether the value went through the target's mathematics.
    fn fold_as(&mut self, e: &Expr, want: Option<Elem>) -> Option<(Cv, bool)> {
        let inner = want.or(self.fold_want);
        let was_want = std::mem::replace(&mut self.fold_want, inner);
        let was = std::mem::replace(&mut self.fold_inexact, false);
        let v = self.fold(e);
        let inexact = self.fold_inexact;
        self.fold_inexact |= was;
        self.fold_want = was_want;
        v.map(|v| (v, inexact))
    }

    /// An enumerated value, a named value, or a CONSTANT of such a type:
    /// its number.
    fn fold_enum(&mut self, e: &Expr, k: &str) -> Option<Cv> {
        let name = match &e.kind {
            ExprKind::Name(id) => id,
            ExprKind::EnumValue { value, .. } => value,
            ExprKind::Paren(x) => return self.fold_enum(x, k),
            // A part of a constant array or structure.
            ExprKind::Index(..) | ExprKind::Member(..) => {
                let (_, init) = self.const_part(e)?;
                self.enum_base(k, e.span).ok()?;
                return match init {
                    Some(Init::Expr(x)) => {
                        let root = span_key(x.span);
                        if self.const_busy.contains(&root) {
                            return None;
                        }
                        self.const_busy.push(root);
                        let v = self.fold(&x);
                        self.const_busy.pop();
                        v
                    }
                    Some(_) => None,
                    None => self.enums.get(k)?.values.first().map(|(_, n)| Cv::Int(*n)),
                };
            }
            _ => return None,
        };
        self.enum_base(k, e.span).ok()?;
        let up = name.text.to_ascii_uppercase();
        let info = self.enums.get(k)?;
        if let Some((_, n)) = info.values.iter().find(|(v, _)| *v == up) {
            return Some(Cv::Int(*n));
        }
        let first = info.values.first().map(|(_, n)| *n);
        if !matches!(e.kind, ExprKind::Name(_)) || self.const_busy.contains(&up) {
            return None;
        }
        let d = self.const_decl(&name.text, name.span)?;
        match &d.init {
            Some(Init::Expr(x)) => {
                self.const_busy.push(up);
                let v = self.fold(x);
                self.const_busy.pop();
                v
            }
            Some(_) => None,
            None => first.map(Cv::Int),
        }
    }

    /// E0032: a constant expression with no value.
    fn fold_none(&mut self, at: Span, why: &str) -> Option<Cv> {
        self.fold_bad.get_or_insert((
            Code::NotConstant,
            at,
            format!("this constant expression {why}: it has no value"),
        ));
        None
    }

    /// E0020: a constant `shown` that `t` does not hold.
    fn fold_range(&mut self, at: Span, shown: String, t: Elem) -> Option<Cv> {
        self.fold_bad.get_or_insert((
            Code::LiteralRange,
            at,
            format!("this constant expression is {shown}, which {} does not hold", t.name()),
        ));
        None
    }

    fn fold_in(&mut self, e: &Expr, ty: Option<Elem>) -> Option<Cv> {
        let real = |f: f64| round_real(ty, f);
        Some(match &e.kind {
            ExprKind::Paren(x) | ExprKind::Unary(UnaryOp::Plus, x) => self.fold(x)?,
            ExprKind::Literal(l) => match l.kind {
                LiteralKind::Bool => Cv::Bool(const_int(e)? != 0),
                LiteralKind::Int | LiteralKind::Based => match ty {
                    Some(Elem::Bool) => Cv::Bool(const_int(e)? != 0),
                    Some(t) if t.is_real() => Cv::Real(real(const_int(e)? as f64)),
                    _ => Cv::Int(const_int(e)?),
                },
                LiteralKind::Real => {
                    let f: f64 = l.text.replace('_', "").parse().ok()?;
                    Cv::Real(real(if l.negative { -f } else { f }))
                }
                LiteralKind::Time => {
                    let ns = expr_duration(l)?;
                    match ty? {
                        Elem::Time if ns % 1_000_000 == 0 => Cv::Int(ns / 1_000_000),
                        Elem::Ltime => Cv::Int(ns),
                        _ => return None,
                    }
                }
                _ => return None,
            },
            ExprKind::Name(id) => {
                let d = self.const_decl(&id.text, id.span)?;
                let key = id.text.to_ascii_uppercase();
                if self.const_busy.contains(&key) {
                    return None;
                }
                let t = self.const_type(d)?;
                match &d.init {
                    Some(Init::Expr(x)) => {
                        let from = match self.types.expr(x.span) {
                            Some(Ty::Elem(f)) => Some(*f),
                            _ => None,
                        };
                        self.const_busy.push(key);
                        let was = self.fold_want.replace(t);
                        let v = self.fold(x);
                        self.fold_want = was;
                        self.const_busy.pop();
                        Self::cv_as(v?, from, t)?
                    }
                    Some(_) => return None,
                    None if t == Elem::Bool => Cv::Bool(false),
                    None if t.is_real() => Cv::Real(0.0),
                    None => Cv::Int(0),
                }
            }
            ExprKind::Index(..) | ExprKind::Member(..) => {
                let (t, init) = self.const_part(e)?;
                let Kind::Elem(elem) = t.kind else { return None };
                match init {
                    Some(Init::Expr(x)) => {
                        let root = span_key(x.span);
                        if self.const_busy.contains(&root) {
                            return None;
                        }
                        let from = self.fold_ty(&x);
                        self.const_busy.push(root);
                        let was = self.fold_want.replace(elem);
                        let v = self.fold(&x);
                        self.fold_want = was;
                        self.const_busy.pop();
                        Self::cv_as(v?, from, elem)?
                    }
                    Some(_) => return None,
                    None if elem == Elem::Bool => Cv::Bool(false),
                    None if elem.is_real() => Cv::Real(0.0),
                    None => Cv::Int(0),
                }
            }
            ExprKind::Unary(UnaryOp::Neg, x) => match self.fold(x)? {
                Cv::Int(i) => Cv::Int(i.checked_neg()?),
                Cv::Real(f) => Cv::Real(-f),
                Cv::Bool(_) => return None,
            },
            ExprKind::Unary(UnaryOp::Not, x) => match self.fold(x)? {
                Cv::Bool(b) => Cv::Bool(!b),
                // A bit string's complement is in its width: the run's.
                _ => return None,
            },
            ExprKind::Binary(BinaryOp::Power, a, b) => self.fold_power(ty?, a, b)?,
            ExprKind::Binary(op, a, b) => self.fold_ops(e, *op, &[a.as_ref(), b.as_ref()], ty)?,
            ExprKind::Call { args, .. } => {
                let sc = self.types.std_call(e.span)?.clone();
                if sc.outputs.iter().any(|(_, s)| s.is_some()) {
                    return None;
                }
                let all: Vec<&Expr> = sc.inputs.iter().filter_map(|(_, s)| arg_value(args, *s)).collect();
                if all.len() != sc.inputs.len() {
                    return None;
                }
                self.fold_call(e, &sc.name, &all, ty)?
            }
            _ => return None,
        })
    }

    /// The type both sides of a comparison are compared in, as far as
    /// an untyped real literal among them needs one.
    fn fold_common(&self, xs: &[&Expr]) -> Option<Elem> {
        xs.iter().find_map(|x| self.fold_ty(x)).or(Some(Elem::Lreal))
    }

    /// `xs[0] op xs[1] op ...`: an operator, or its function (Ed.3 Tables
    /// 29-31, 33), a comparison holding when each neighbouring pair does.
    fn fold_ops(&mut self, e: &Expr, op: BinaryOp, xs: &[&Expr], ty: Option<Elem>) -> Option<Cv> {
        use BinaryOp::*;
        if let (Some(t), Mul | Div, [a, b]) = (ty.filter(|t| t.is_duration()), op, xs) {
            return self.fold_scale(e, op == Div, t, a, b);
        }
        let compares = matches!(op, Eq | Ne | Lt | Le | Gt | Ge);
        let want = if compares { self.fold_common(xs) } else { ty };
        let mut vs = Vec::new();
        for x in xs {
            vs.push((self.fold_as(x, want)?, self.fold_ty(x)));
        }
        if compares {
            let mut all = true;
            for w in vs.windows(2) {
                let (((x, xi), xt), ((y, yi), yt)) = (w[0], w[1]);
                // The target's mathematics decides a comparison of its
                // own: this one only says that there is nothing to flag.
                self.fold_inexact |= xi || yi;
                let (x, y) = Self::fold_mix(x, xt, y, yt)?;
                let Cv::Bool(b) = self.fold_op(e, op, x, y, Some(Elem::Bool))? else { return None };
                all &= b;
            }
            return Some(Cv::Bool(all));
        }
        let mut it = vs.into_iter();
        let ((mut acc, _), mut at) = it.next()?;
        for ((y, yi), yt) in it {
            // So it does whether a real divisor is zero.
            if yi && op == Div {
                return None;
            }
            let (x, y) = Self::fold_mix(acc, at, y, yt)?;
            acc = self.fold_op(e, op, x, y, ty)?;
            at = ty.or(at);
        }
        Some(acc)
    }

    /// A duration `t` times or over a number (Ed.3 Table 30): by an
    /// integer exactly, by a real in LREAL and rounded to the unit, to
    /// the even one from half-way.
    fn fold_scale(&mut self, e: &Expr, div: bool, t: Elem, a: &Expr, b: &Expr) -> Option<Cv> {
        let (d, n) = if self.fold_ty(a).is_some_and(Elem::is_duration) { (a, b) } else { (b, a) };
        if div && !std::ptr::eq(d, a) {
            return None;
        }
        let (Cv::Int(x), _) = self.fold_as(d, Some(t))? else { return None };
        let (n, inexact) = self.fold_as(n, Some(Elem::Lreal))?;
        let held = if t == Elem::Time { Elem::Dint } else { Elem::Lint };
        let (lo, hi) = held.int_range()?;
        let unit = if t == Elem::Time { "ms" } else { "ns" };
        let v = match n {
            Cv::Int(0) if div => return self.fold_none(e.span, "divides by zero"),
            Cv::Int(n) if div => x.checked_div(n)?,
            Cv::Int(n) => x.checked_mul(n)?,
            // Which whole unit is nearest is the target's to say.
            Cv::Real(_) if inexact => return None,
            Cv::Real(f) if div && f == 0.0 => return self.fold_none(e.span, "divides by zero"),
            Cv::Real(f) => {
                let r = if div { x as f64 / f } else { x as f64 * f }.round_ties_even();
                if !(r >= lo as f64 && r <= hi as f64) {
                    return self.fold_range(e.span, format!("{r:e} {unit}"), t);
                }
                r as i128
            }
            Cv::Bool(_) => return None,
        };
        if v < lo || v > hi {
            return self.fold_range(e.span, format!("{v} {unit}"), t);
        }
        Some(Cv::Int(v))
    }

    /// An integer beside a real is the real it widens to.
    fn fold_mix(x: Cv, xt: Option<Elem>, y: Cv, yt: Option<Elem>) -> Option<(Cv, Cv)> {
        Some(match (x, y) {
            (Cv::Int(_), Cv::Real(_)) => (Self::cv_as(x, xt, Elem::Lreal)?, y),
            (Cv::Real(_), Cv::Int(_)) => (x, Self::cv_as(y, yt, Elem::Lreal)?),
            p => p,
        })
    }

    fn fold_op(&mut self, e: &Expr, op: BinaryOp, x: Cv, y: Cv, ty: Option<Elem>) -> Option<Cv> {
        use BinaryOp::*;
        let real = |f: f64| round_real(ty, f);
        Some(match (op, x, y) {
            (Add, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_add(q)?),
            (Sub, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_sub(q)?),
            (Mul, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_mul(q)?),
            (Div, Cv::Int(_), Cv::Int(0)) => return self.fold_none(e.span, "divides by zero"),
            (Div, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_div(q)?),
            // As the run has it (docs/behaviour/integers.md).
            (Mod, Cv::Int(_), Cv::Int(0)) => Cv::Int(0),
            (Mod, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_rem(q)?),
            // Bit strings, which are never negative.
            (And, Cv::Int(p), Cv::Int(q)) if p >= 0 && q >= 0 => Cv::Int(p & q),
            (Or, Cv::Int(p), Cv::Int(q)) if p >= 0 && q >= 0 => Cv::Int(p | q),
            (Xor, Cv::Int(p), Cv::Int(q)) if p >= 0 && q >= 0 => Cv::Int(p ^ q),
            (Add, Cv::Real(p), Cv::Real(q)) => Cv::Real(real(p + q)),
            (Sub, Cv::Real(p), Cv::Real(q)) => Cv::Real(real(p - q)),
            (Mul, Cv::Real(p), Cv::Real(q)) if ty.is_some_and(Elem::is_real) => Cv::Real(real(p * q)),
            (Div, Cv::Real(_), Cv::Real(q)) if q.abs() == 0.0 => return self.fold_none(e.span, "divides by zero"),
            (Div, Cv::Real(p), Cv::Real(q)) if ty.is_some_and(Elem::is_real) => Cv::Real(real(p / q)),
            (And, Cv::Bool(p), Cv::Bool(q)) => Cv::Bool(p && q),
            (Or, Cv::Bool(p), Cv::Bool(q)) => Cv::Bool(p || q),
            (Xor, Cv::Bool(p), Cv::Bool(q)) => Cv::Bool(p != q),
            (Eq, p, q) => Cv::Bool(p == q),
            (Ne, p, q) => Cv::Bool(p != q),
            (Lt, Cv::Int(p), Cv::Int(q)) => Cv::Bool(p < q),
            (Le, Cv::Int(p), Cv::Int(q)) => Cv::Bool(p <= q),
            (Gt, Cv::Int(p), Cv::Int(q)) => Cv::Bool(p > q),
            (Ge, Cv::Int(p), Cv::Int(q)) => Cv::Bool(p >= q),
            (Lt, Cv::Real(p), Cv::Real(q)) => Cv::Bool(p < q),
            (Le, Cv::Real(p), Cv::Real(q)) => Cv::Bool(p <= q),
            (Gt, Cv::Real(p), Cv::Real(q)) => Cv::Bool(p > q),
            (Ge, Cv::Real(p), Cv::Real(q)) => Cv::Bool(p >= q),
            _ => return None,
        })
    }

    /// `a ** b`, a real `t` raised to any number: the target's `pow`.
    fn fold_power(&mut self, t: Elem, a: &Expr, b: &Expr) -> Option<Cv> {
        if !t.is_real() {
            return None;
        }
        let (x, _) = self.fold_as(a, Some(t))?;
        let (y, _) = self.fold_as(b, Some(Elem::Lreal))?;
        let (Cv::Real(x), Some(y)) = (Self::cv_as(x, self.fold_ty(a), t)?, cv_f64(y)) else { return None };
        self.fold_inexact = true;
        Some(Cv::Real(round_real(Some(t), x.powf(y))))
    }

    /// A standard function of constants: those whose result is exact.
    /// A string's or a date's, and a conversion to or from text, are
    /// left to the run.
    fn fold_call(&mut self, e: &Expr, name: &str, all: &[&Expr], ty: Option<Elem>) -> Option<Cv> {
        use BinaryOp::*;
        let op = match name {
            "ADD" | "ADD_TIME" | "ADD_LTIME" => Some(Add),
            "SUB" | "SUB_TIME" | "SUB_LTIME" => Some(Sub),
            "MUL" | "MUL_TIME" | "MUL_LTIME" => Some(Mul),
            "DIV" | "DIV_TIME" | "DIV_LTIME" => Some(Div),
            "MOD" => Some(Mod),
            "AND" => Some(And),
            "OR" => Some(Or),
            "XOR" => Some(Xor),
            "GT" => Some(Gt),
            "GE" => Some(Ge),
            "EQ" => Some(Eq),
            "LE" => Some(Le),
            "LT" => Some(Lt),
            "NE" => Some(Ne),
            _ => None,
        };
        if let Some(op) = op {
            return self.fold_ops(e, op, all, ty);
        }
        // The inputs that share the result's type, as the result's kind.
        let same = |g: &mut Self, xs: &[&Expr]| -> Option<Vec<Cv>> {
            let mut vs = Vec::new();
            for x in xs {
                let (v, _) = g.fold_as(x, ty)?;
                vs.push((v, g.fold_ty(x)));
            }
            let any_real = vs.iter().any(|(v, _)| matches!(v, Cv::Real(_)));
            vs.into_iter()
                .map(|(v, t)| match v {
                    Cv::Int(_) if any_real => Self::cv_as(v, t, ty.filter(|t| t.is_real()).unwrap_or(Elem::Lreal)),
                    v => Some(v),
                })
                .collect()
        };
        let less = |a: Cv, b: Cv| match (a, b) {
            (Cv::Int(p), Cv::Int(q)) => Some(p < q),
            (Cv::Real(p), Cv::Real(q)) => Some(p < q),
            (Cv::Bool(p), Cv::Bool(q)) => Some(!p & q),
            _ => None,
        };
        Some(match name {
            "MOVE" => self.fold_as(all.first()?, ty)?.0,
            "NOT" => match self.fold(all.first()?)? {
                Cv::Bool(b) => Cv::Bool(!b),
                _ => return None,
            },
            "ABS" => match self.fold_as(all.first()?, ty)?.0 {
                Cv::Int(i) => Cv::Int(i.checked_abs()?),
                Cv::Real(f) => Cv::Real(f.abs()),
                Cv::Bool(_) => return None,
            },
            "MIN" | "MAX" => {
                let vs = same(self, all)?;
                let mut acc = *vs.first()?;
                for v in &vs[1..] {
                    // As iecf.h's: the first of two that are the same.
                    let takes = if name == "MIN" { less(*v, acc)? } else { less(acc, *v)? };
                    if takes {
                        acc = *v;
                    }
                }
                acc
            }
            "LIMIT" => {
                let [mn, i, mx] = same(self, all)?[..] else { return None };
                let low = if less(i, mn)? { mn } else { i };
                if less(mx, low)? { mx } else { low }
            }
            "SEL" => {
                let [g, a, b] = all else { return None };
                let Cv::Bool(g) = self.fold(g)? else { return None };
                let [a, b] = same(self, &[a, b])?[..] else { return None };
                if g { b } else { a }
            }
            "MUX" => {
                let (k, ins) = all.split_first()?;
                let (Cv::Int(k), ki) = self.fold_as(k, None)? else { return None };
                let vs = same(self, ins)?;
                if ki {
                    return None;
                }
                match usize::try_from(k).ok().and_then(|k| vs.get(k)) {
                    Some(v) => *v,
                    None => return self.fold_none(e.span, &format!("selects MUX's input {k}, and there is none")),
                }
            }
            "SHL" | "SHR" | "ROL" | "ROR" => {
                let [i, n] = all else { return None };
                // Literals alone are of the type wanted of them.
                let ty = ty.or(self.fold_want);
                let w = ty.filter(|t| t.is_bit() && *t != Elem::Bool)?.bits()?;
                let (Cv::Int(x), Cv::Int(n)) = (self.fold_as(i, ty)?.0, self.fold(n)?) else { return None };
                if n < 0 {
                    return self.fold_none(e.span, "shifts by a negative count");
                }
                let mask = (1i128 << w) - 1;
                if x < 0 || x > mask {
                    return None;
                }
                let k = (n % i128::from(w)) as u32;
                Cv::Int(match name {
                    "SHL" if n >= i128::from(w) => 0,
                    "SHL" => (x << n) & mask,
                    "SHR" if n >= i128::from(w) => 0,
                    "SHR" => x >> n,
                    _ if k == 0 => x,
                    "ROL" => ((x << k) | (x >> (w - k))) & mask,
                    _ => ((x >> k) | (x << (w - k))) & mask,
                })
            }
            _ if math_function(name).is_some() => {
                let t = ty.filter(|t| t.is_real())?;
                let x = all.first()?;
                let (v, _) = self.fold_as(x, Some(t))?;
                let Cv::Real(x) = Self::cv_as(v, self.fold_ty(x), t)? else { return None };
                let f = match name {
                    "SQRT" => x.sqrt(),
                    "LN" => x.ln(),
                    "LOG" => x.log10(),
                    "EXP" => x.exp(),
                    "SIN" => x.sin(),
                    "COS" => x.cos(),
                    "TAN" => x.tan(),
                    "ASIN" => x.asin(),
                    "ACOS" => x.acos(),
                    _ => x.atan(),
                };
                self.fold_inexact = true;
                Cv::Real(round_real(Some(t), f))
            }
            "EXPT" => {
                let [a, b] = all else { return None };
                self.fold_power(ty?, a, b)?
            }
            // Functions of strings (Table 34) and of dates (Tables 30 and
            // 36), of constants: worked out here, as the runtime's do
            // (iecf.h); what the runtime would flag is refused.
            "LEN" => Cv::Int(self.fold_string(all.first()?, 0)?.len() as i128),
            "FIND" => {
                let [a, b] = all else { return None };
                let (a, b) = (self.fold_string(a, 0)?, self.fold_string(b, 0)?);
                if b.is_empty() || b.len() > a.len() {
                    Cv::Int(0)
                } else {
                    Cv::Int(a.windows(b.len()).position(|w| w == b.as_slice()).map_or(0, |i| i as i128 + 1))
                }
            }
            "CONCAT_DATE_TOD" => {
                let [d, tod] = all else { return None };
                let (d, tod) = (self.fold_date(d)?, self.fold_date(tod)?);
                self.fold_seconds(e.span, d + tod / 1000)?
            }
            "SUB_DATE_DATE" | "SUB_DT_DT" | "SUB_TOD_TOD" => {
                let [a, b] = all else { return None };
                let (a, b) = (self.fold_date(a)?, self.fold_date(b)?);
                let ms = if name == "SUB_TOD_TOD" { a - b } else { (a - b) * 1000 };
                if ms < i128::from(i32::MIN) || ms > i128::from(i32::MAX) {
                    return self.fold_range(e.span, format!("{ms} ms"), Elem::Time);
                }
                Cv::Int(ms)
            }
            "ADD_DT_TIME" | "SUB_DT_TIME" => {
                let [a, b] = all else { return None };
                let (a, ms) = (self.fold_date(a)?, self.fold_date(b)?);
                let ms = if name == "ADD_DT_TIME" { ms } else { -ms };
                self.fold_seconds(e.span, a + ms.div_euclid(1000))?
            }
            "CONCAT_DATE" | "CONCAT_TOD" | "CONCAT_DT" => {
                let mut parts = Vec::new();
                for x in all {
                    let Cv::Int(v) = self.fold(x)? else { return None };
                    parts.push(v);
                }
                let ymd = |p: &[i128]| {
                    let (y, m, d) = (p[0], p[1], p[2]);
                    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
                    let len = match m {
                        2 if leap => 29,
                        2 => 28,
                        4 | 6 | 9 | 11 => 30,
                        _ => 31,
                    };
                    ((1970..=2106).contains(&y) && (1..=12).contains(&m) && d >= 1 && d <= len).then(|| {
                        // H. Hinnant's days_from_civil, as iecf.h's.
                        let y = if m <= 2 { y - 1 } else { y };
                        let era = y.div_euclid(400);
                        let yoe = y - era * 400;
                        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
                        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
                        era * 146_097 + doe - 719_468
                    })
                };
                let hms = |p: &[i128]| {
                    ((0..=23).contains(&p[0])
                        && (0..=59).contains(&p[1])
                        && (0..=59).contains(&p[2])
                        && (0..=999).contains(&p[3]))
                    .then(|| ((p[0] * 60 + p[1]) * 60 + p[2], p[3]))
                };
                let value = match (name, parts.as_slice()) {
                    ("CONCAT_DATE", p @ [_, _, _]) => ymd(p).map(|days| days * 86_400),
                    ("CONCAT_TOD", p @ [_, _, _, _]) => hms(p).map(|(s, ms)| s * 1000 + ms),
                    ("CONCAT_DT", p @ [_, _, _, _, _, _, _]) => match (ymd(&p[..3]), hms(&p[3..])) {
                        (Some(days), Some((s, _))) => Some(days * 86_400 + s),
                        _ => None,
                    },
                    _ => return None,
                };
                match value {
                    Some(v) if name == "CONCAT_TOD" => Cv::Int(v),
                    Some(v) => self.fold_seconds(e.span, v)?,
                    None => {
                        self.fold_bad.get_or_insert((
                            Code::LiteralRange,
                            e.span,
                            format!(
                                "{name} of these parts is no {}: a part is out of its range",
                                &name["CONCAT_".len()..]
                            ),
                        ));
                        return None;
                    }
                }
            }
            _ if name.starts_with("STRING_TO_") || name.starts_with("WSTRING_TO_") => {
                // The text of a whole number, as written in decimal.
                let to = ty.filter(|t| t.is_int())?;
                let x = all.first()?;
                let units = self.fold_string(x, 0)?;
                let text: String = units.iter().map(|u| char::from_u32(u32::from(*u)).unwrap_or('?')).collect();
                let digits = text.strip_prefix(['-', '+']).unwrap_or(&text);
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                let v: i128 = text.strip_prefix('+').unwrap_or(&text).parse().ok()?;
                let (lo, hi) = to.int_range()?;
                if v < lo || v > hi {
                    return self.fold_range(e.span, v.to_string(), to);
                }
                Cv::Int(v)
            }
            _ => {
                // A conversion between numbers, rounding or truncating.
                let (from, to, trunc) = match (trunc_names(name), conversion_names(name)) {
                    (Some((f, t)), _) => (f, t, true),
                    (None, Some((f, t))) => (f, t, false),
                    _ => return None,
                };
                let x = all.first()?;
                let from = from.or(self.fold_ty(x));
                if !to.is_num() || from.is_some_and(|f| !f.is_num()) {
                    return None;
                }
                let (v, inexact) = self.fold_as(x, from.or(Some(to)))?;
                let v = match from {
                    Some(f) => Self::cv_as(v, self.fold_ty(x), f)?,
                    None => v,
                };
                match v {
                    Cv::Int(i) if to.is_real() => Cv::Real(round_real(Some(to), i as f64)),
                    Cv::Int(i) => {
                        let (lo, hi) = to.int_range()?;
                        if i < lo || i > hi {
                            return self.fold_range(e.span, i.to_string(), to);
                        }
                        Cv::Int(i)
                    }
                    Cv::Real(f) if to.is_real() => Cv::Real(round_real(Some(to), f)),
                    // Which integer is nearest is the target's to say.
                    Cv::Real(_) if inexact => return None,
                    Cv::Real(f) => {
                        let r = if trunc { f.trunc() } else { f.round_ties_even() };
                        let (lo, hi) = to.int_range()?;
                        if !(r >= lo as f64 && r <= hi as f64) {
                            return self.fold_range(e.span, f.to_string(), to);
                        }
                        Cv::Int(r as i128)
                    }
                    Cv::Bool(_) => return None,
                }
            }
        })
    }

    /// A DATE's or DT's seconds, a TOD's or TIME's milliseconds, of a
    /// constant: a literal, or what folds to one.
    fn fold_date(&mut self, e: &Expr) -> Option<i128> {
        match &e.kind {
            ExprKind::Paren(x) => self.fold_date(x),
            ExprKind::Literal(l) if l.kind == LiteralKind::Time => match iecf_sema::datetime::literal(&l.text) {
                Some(Ok((_, v))) => Some(i128::from(v)),
                Some(Err(_)) => None,
                None => crate::expr::duration_ns(&l.text).map(|ns| i128::from(ns) / 1_000_000),
            },
            _ => match self.fold(e)? {
                Cv::Int(i) => Some(i),
                _ => None,
            },
        }
    }

    /// Seconds as a DATE or DT: 1970-01-01 to 2106-02-07-06:28:15.
    fn fold_seconds(&mut self, at: Span, s: i128) -> Option<Cv> {
        if s < 0 || s > i128::from(u32::MAX) {
            self.fold_bad.get_or_insert((
                Code::LiteralRange,
                at,
                "this constant expression is a date outside 1970-01-01 .. 2106-02-07, which a DATE or DT does not hold"
                    .to_string(),
            ));
            return None;
        }
        Some(Cv::Int(s))
    }

    /// The characters of `e`, a STRING or WSTRING expression of
    /// constants: a literal, a CONSTANT, and Table 34's functions and
    /// the conversions of whole numbers on them, as iecf.h's do; None
    /// when it is not one, `fold_bad` set where the runtime would flag -
    /// a position or a length outside the string.
    pub(crate) fn fold_string(&mut self, e: &Expr, depth: usize) -> Option<Vec<u16>> {
        if depth > 32 {
            return None;
        }
        match &e.kind {
            ExprKind::Paren(x) => self.fold_string(x, depth + 1),
            ExprKind::Literal(l) if matches!(l.kind, LiteralKind::String | LiteralKind::WString) => {
                let w = l.kind == LiteralKind::WString;
                let units = crate::strings::literal_units(&l.text, w).ok()?;
                Some(units.into_iter().take_while(|c| *c != 0).collect())
            }
            ExprKind::Name(_) => match self.const_init(e)? {
                Init::Expr(x) => self.fold_string(x, depth + 1),
                _ => None,
            },
            ExprKind::Call { args, .. } => {
                let sc = self.types.std_call(e.span)?.clone();
                if sc.outputs.iter().any(|(_, s)| s.is_some()) {
                    return None;
                }
                let all: Vec<&Expr> = sc.inputs.iter().filter_map(|(_, s)| arg_value(args, *s)).collect();
                if all.len() != sc.inputs.len() {
                    return None;
                }
                let name = sc.name.as_str();
                let int = |g: &mut Self, x: &Expr| match g.fold(x)? {
                    Cv::Int(i) => Some(i),
                    _ => None,
                };
                let bad = |g: &mut Self, what: String| -> Option<Vec<u16>> {
                    g.fold_bad.get_or_insert((Code::LiteralRange, e.span, what));
                    None
                };
                match name {
                    "CONCAT" => {
                        let mut out = Vec::new();
                        for x in &all {
                            out.extend(self.fold_string(x, depth + 1)?);
                        }
                        Some(out)
                    }
                    "LEFT" | "RIGHT" => {
                        let s = self.fold_string(all.first()?, depth + 1)?;
                        let l = int(self, all.get(1)?)?;
                        if l < 0 || l as usize > s.len() {
                            return bad(self, format!("{name} takes {l} characters of a string of {}", s.len()));
                        }
                        let l = l as usize;
                        Some(if name == "LEFT" { s[..l].to_vec() } else { s[s.len() - l..].to_vec() })
                    }
                    "MID" | "DELETE" => {
                        let s = self.fold_string(all.first()?, depth + 1)?;
                        let (l, p) = (int(self, all.get(1)?)?, int(self, all.get(2)?)?);
                        if l < 0 || p < 1 || (p - 1 + l) as usize > s.len() {
                            return bad(
                                self,
                                format!(
                                    "{name} of {l} characters from position {p} is outside a string of {}",
                                    s.len()
                                ),
                            );
                        }
                        let (from, to) = ((p - 1) as usize, (p - 1 + l) as usize);
                        Some(if name == "MID" {
                            s[from..to].to_vec()
                        } else {
                            s[..from].iter().chain(&s[to..]).copied().collect()
                        })
                    }
                    "INSERT" => {
                        let (a, b) =
                            (self.fold_string(all.first()?, depth + 1)?, self.fold_string(all.get(1)?, depth + 1)?);
                        let p = int(self, all.get(2)?)?;
                        if p < 0 || p as usize > a.len() {
                            return bad(self, format!("INSERT after position {p} is outside a string of {}", a.len()));
                        }
                        let p = p as usize;
                        Some(a[..p].iter().chain(&b).chain(&a[p..]).copied().collect())
                    }
                    "REPLACE" => {
                        let (a, b) =
                            (self.fold_string(all.first()?, depth + 1)?, self.fold_string(all.get(1)?, depth + 1)?);
                        let (l, p) = (int(self, all.get(2)?)?, int(self, all.get(3)?)?);
                        if l < 0 || p < 1 || (p - 1 + l) as usize > a.len() {
                            return bad(
                                self,
                                format!(
                                    "REPLACE of {l} characters from position {p} is outside a string of {}",
                                    a.len()
                                ),
                            );
                        }
                        let (from, to) = ((p - 1) as usize, (p - 1 + l) as usize);
                        Some(a[..from].iter().chain(&b).chain(&a[to..]).copied().collect())
                    }
                    "STRING_TO_WSTRING" | "CHAR_TO_STRING" | "WCHAR_TO_WSTRING" | "CHAR_TO_WSTRING" => {
                        self.fold_string(all.first()?, depth + 1)
                    }
                    _ => {
                        // A whole number as its decimal text.
                        let from = name.strip_suffix("_TO_STRING").or_else(|| name.strip_suffix("_TO_WSTRING"))?;
                        let whole =
                            matches!(from, "SINT" | "INT" | "DINT" | "LINT" | "USINT" | "UINT" | "UDINT" | "ULINT");
                        if !whole {
                            return None;
                        }
                        let v = int(self, all.first()?)?;
                        Some(v.to_string().encode_utf16().collect())
                    }
                }
            }
            _ => None,
        }
    }

    fn fold_ty(&self, e: &Expr) -> Option<Elem> {
        match self.types.expr(e.span) {
            Some(Ty::Elem(t)) => Some(*t),
            _ => None,
        }
    }

    /// What in `e` is no constant (Ed.3 6.6.1.2.4): a variable, or the
    /// result of a FUNCTION of the program's, which the compiler does not
    /// run. `None` when all of it is literals, constants and standard
    /// functions.
    pub(crate) fn not_constant(&mut self, e: &Expr) -> Option<(Span, String)> {
        match &e.kind {
            ExprKind::Literal(_) | ExprKind::EnumValue { .. } => None,
            ExprKind::Paren(x) | ExprKind::Unary(_, x) => self.not_constant(x),
            ExprKind::Binary(_, a, b) => self.not_constant(a).or_else(|| self.not_constant(b)),
            ExprKind::Name(id) => {
                if self.const_decl(&id.text, id.span).is_some() || matches!(self.enum_const(e), Ok(Some(_))) {
                    return None;
                }
                Some((id.span, format!("'{}' is a variable", id.text)))
            }
            ExprKind::Call { callee, args } => {
                if self.types.std_call(e.span).is_none() {
                    let what = match &callee.kind {
                        ExprKind::Name(id) => format!("the result of {}", id.text),
                        _ => "the result of a method".to_string(),
                    };
                    return Some((callee.span, format!("{what} is known only as the program runs")));
                }
                args.iter().find_map(|a| match a {
                    Arg::Positional(x) | Arg::Input(_, x) => self.not_constant(x),
                    Arg::Output { target, .. } => Some((target.span, "a function's output is taken".to_string())),
                })
            }
            // A part of a constant array or structure is one.
            ExprKind::Index(b, idx) => self.not_constant(b).or_else(|| idx.iter().find_map(|i| self.not_constant(i))),
            ExprKind::Member(b, _) | ExprKind::BitAccess(b, _) => self.not_constant(b),
            ExprKind::Deref(x) => Some((x.span, "what a reference refers to is a variable".to_string())),
            ExprKind::DirectAddress(a) => Some((a.span, format!("{} is a variable", a.text))),
            _ => None,
        }
    }

    /// The initial value `e` of a variable of the elementary type `ty`,
    /// as C. A constant expression is worked out here, so nothing of it
    /// is left to the run; one it cannot hold is E0020, and what is no
    /// constant E0032. The mathematics of Table 28 and `**` are computed
    /// as the variable is set up, by the target's own, once the compiler
    /// has worked out that the result is one the type holds: nothing
    /// faults and nothing is flagged as the program starts.
    pub(crate) fn init_scalar(&mut self, e: &Expr, ty: Elem) -> R<String> {
        self.may_fault = false;
        self.may_flag = false;
        let from = self.fold_ty(e);
        self.fold_bad = None;
        self.fold_inexact = false;
        let was = self.fold_want.replace(ty);
        let folded = self.fold(e);
        self.fold_want = was;
        let inexact = std::mem::replace(&mut self.fold_inexact, false);
        if let Some((code, at, why)) = self.fold_bad.take() {
            self.diags.push(Diagnostic::new(code, at, why));
            return Err(crate::Skip);
        }
        if let Some(v) = folded.and_then(|v| Self::cv_as(v, from, ty)).filter(|_| !inexact) {
            let c = match v {
                Cv::Bool(b) => lit(i128::from(b), ty),
                Cv::Int(i) => lit(i, ty),
                Cv::Real(f) if f.is_finite() => {
                    let t = format!("{f:e}");
                    Some(if ty == Elem::Real {
                        format!("(({}){t}f)", c_type(ty))
                    } else {
                        format!("(({}){t})", c_type(ty))
                    })
                }
                Cv::Real(_) => None,
            };
            return match c {
                Some(c) => Ok(c),
                None => {
                    let shown = match v {
                        Cv::Int(i) => i.to_string(),
                        Cv::Real(f) => f.to_string(),
                        Cv::Bool(b) => b.to_string(),
                    };
                    self.diags.push(Diagnostic::new(
                        Code::LiteralRange,
                        e.span,
                        format!("this constant expression is {shown}, which {} does not hold", ty.name()),
                    ));
                    Err(crate::Skip)
                }
            };
        }
        if let Some((at, why)) = self.not_constant(e) {
            self.diags.push(Diagnostic::new(
                Code::NotConstant,
                at,
                format!("{why}: an initial value is a constant expression (Ed.3 6.6.1.2.4)"),
            ));
            return Err(crate::Skip);
        }
        let seq = std::mem::replace(&mut self.seq, false);
        let code = self.value_as(e, ty, &Scope::default());
        self.seq = seq;
        let code = code?;
        // Worked out above, all of it: the target's own value is one the
        // type holds, and nothing in it divides by zero.
        let checked = folded.is_some() && inexact;
        if !checked && (self.may_fault || self.may_flag) {
            return self.not_yet(e.span, "an initial value that can fault or be flagged");
        }
        self.may_fault = false;
        self.may_flag = false;
        Ok(code)
    }
}

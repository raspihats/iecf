//! CONSTANT variables named where a constant expression is wanted: an
//! initial value (Ed.3 6.6.1.2.4, Figure 6). The variable is not in the
//! scope there - a FUNCTION's input is given its initial value by the
//! caller, a TYPE's by whatever declares a variable of it - so its name
//! stands for its own initial value, worked out in its place: the
//! POU's own `VAR CONSTANT`, or a `VAR_GLOBAL CONSTANT` through its
//! `VAR_EXTERNAL CONSTANT`, or directly in a TYPE, as the checker lets
//! each be named (docs/dependencies.md).

use crate::expr::{const_int, expr_duration, lit};
use crate::names::Scope;
use crate::{Decl, Gen, R, c_type};
use iecf_diagnostics::{Code, Diagnostic, Span};
use iecf_sema::types::{Elem, Ty};
use iecf_syntax::ast::{BinaryOp, Expr, ExprKind, Init, LiteralKind, PouKind, UnaryOp, VarDecl, VarKind};

/// The value of a constant expression, worked out by the compiler.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Cv {
    /// An integer, a bit string, or a duration in its type's unit.
    Int(i128),
    Real(f64),
    Bool(bool),
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
    /// 6.6.1.2.4): literals, CONSTANT variables, and the operators on
    /// them, worked out as the checker works an integer one out - exactly,
    /// not in the operands' width - and a REAL one in single precision.
    /// `None` for anything else, or one the compiler leaves to the run:
    /// a date, a duration times a real.
    pub(crate) fn fold(&mut self, e: &Expr) -> Option<Cv> {
        let ty = match self.types.expr(e.span) {
            Some(Ty::Elem(t)) => Some(*t),
            Some(Ty::Enum(k)) => {
                let k = k.clone();
                return self.fold_enum(e, &k);
            }
            _ => None,
        };
        let real = |f: f64| if ty == Some(Elem::Real) { f as f32 as f64 } else { f };
        let v = self.fold_in(e, ty, real)?;
        // A real that is infinite or no number: the run would flag it.
        if let Cv::Real(f) = v
            && !f.is_finite()
        {
            let t = ty.map_or("its type", Elem::name);
            self.fold_bad.get_or_insert((
                Code::LiteralRange,
                e.span,
                format!("this constant expression is {f}, which {t} does not hold"),
            ));
            return None;
        }
        Some(v)
    }

    /// An enumerated value, a named value, or a CONSTANT of such a type:
    /// its number.
    fn fold_enum(&mut self, e: &Expr, k: &str) -> Option<Cv> {
        let name = match &e.kind {
            ExprKind::Name(id) => id,
            ExprKind::EnumValue { value, .. } => value,
            ExprKind::Paren(x) => return self.fold_enum(x, k),
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

    fn fold_in(&mut self, e: &Expr, ty: Option<Elem>, real: impl Fn(f64) -> f64) -> Option<Cv> {
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
                        let v = self.fold(x);
                        self.const_busy.pop();
                        Self::cv_as(v?, from, t)?
                    }
                    Some(_) => return None,
                    None if t == Elem::Bool => Cv::Bool(false),
                    None if t.is_real() => Cv::Real(0.0),
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
            ExprKind::Binary(op, a, b) => {
                let (x, y) = (self.fold(a)?, self.fold(b)?);
                // An integer beside a real is the real it widens to.
                let (x, y) = match (x, y) {
                    (Cv::Int(i), Cv::Real(_)) => (Self::cv_as(Cv::Int(i), self.fold_ty(a), Elem::Lreal)?, y),
                    (Cv::Real(_), Cv::Int(i)) => (x, Self::cv_as(Cv::Int(i), self.fold_ty(b), Elem::Lreal)?),
                    p => p,
                };
                use BinaryOp::*;
                match (op, x, y) {
                    (Add, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_add(q)?),
                    (Sub, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_sub(q)?),
                    (Mul, Cv::Int(p), Cv::Int(q)) => Cv::Int(p.checked_mul(q)?),
                    (Div, Cv::Int(_), Cv::Int(0)) => {
                        self.fold_bad.get_or_insert((
                            Code::NotConstant,
                            e.span,
                            "this constant expression divides by zero: it has no value".into(),
                        ));
                        return None;
                    }
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
                }
            }
            _ => return None,
        })
    }

    fn fold_ty(&self, e: &Expr) -> Option<Elem> {
        match self.types.expr(e.span) {
            Some(Ty::Elem(t)) => Some(*t),
            _ => None,
        }
    }

    /// The initial value `e` of a variable of the elementary type `ty`,
    /// as C. A constant expression is worked out here, so nothing of it
    /// is left to the run; one it cannot hold is E0020. Anything else is
    /// computed as the variable is set up, where there is no statement to
    /// stop or to flag: one that could fault or be flagged is refused.
    pub(crate) fn init_scalar(&mut self, e: &Expr, ty: Elem) -> R<String> {
        self.may_fault = false;
        self.may_flag = false;
        let from = self.fold_ty(e);
        self.fold_bad = None;
        let folded = self.fold(e);
        if let Some((code, at, why)) = self.fold_bad.take() {
            self.diags.push(Diagnostic::new(code, at, why));
            return Err(crate::Skip);
        }
        if let Some(v) = folded.and_then(|v| Self::cv_as(v, from, ty)) {
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
        let seq = std::mem::replace(&mut self.seq, false);
        let code = self.value_as(e, ty, &Scope::default());
        self.seq = seq;
        let code = code?;
        if self.may_fault || self.may_flag {
            return self.not_yet(e.span, "an initial value that can fault or be flagged");
        }
        Ok(code)
    }
}

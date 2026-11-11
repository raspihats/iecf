//! Variable-length arrays (Ed.3 6.5.3): an `ARRAY[*]` input, output or
//! in-out of a FUNCTION or method, a block's in-out, and `LOWER_BOUND` /
//! `UPPER_BOUND` (Table 15).
//!
//! Each is a pointer to an `iecf_flex_t` (iecf.h): the caller's
//! array, and each dimension's lower and upper bound in turn, as
//! `int64_t`. A call gives a C99 compound literal of one, which lives
//! until the caller's statement ends; an `ARRAY[*]` passed on is its own
//! pointer. An index into it is always checked against the bounds it
//! carries: out of them, a fault (IECF_FAULT_INDEX), as for any array.

use crate::derived::{DTy, Kind};
use crate::expr::{const_int, lit};
use crate::names::{Lv, Scope};
use crate::{Gen, R};
use iecf_diagnostics::Span;
use iecf_sema::StdCall;
use iecf_sema::types::Elem;
use iecf_syntax::ast::{Arg, Expr};
use std::fmt::Write as _;

const FAULT: &str = "&ctx->fault";

/// The C expression of a variable-length array given as the pointer
/// `p` (an in-out's `V_<n>`, an input's `P_<n>`, an output's `O_<n>`):
/// a pointer to its `iecf_flex_t`.
pub(crate) fn flex_at(p: &str) -> String {
    let c = if p.starts_with("P_") { "const " } else { "" };
    format!("(({c}iecf_flex_t *){p})")
}

impl Gen<'_> {
    /// The offset of index `i` in dimension `d` (from 0) of the
    /// variable-length array `desc`: always checked.
    pub(crate) fn flex_index(&mut self, i: &Expr, desc: &str, d: usize, scope: &Scope) -> R<String> {
        let (lo, hi) = (format!("{desc}->b[{}]", 2 * d), format!("{desc}->b[{}]", 2 * d + 1));
        self.may_fault = true;
        if let Some(v) = const_int(i) {
            let Ok(v) = i64::try_from(v) else { return self.not_yet(i.span, "this index") };
            return Ok(format!("iecf_index({FAULT}, INT64_C({v}), {lo}, {hi})"));
        }
        let t = self.elem_of(i)?;
        if !t.is_int() {
            return self.not_yet(i.span, format!("an index of type {}", t.name()));
        }
        let v = self.value(i, scope)?;
        Ok(if t.is_signed_int() {
            format!("iecf_index({FAULT}, (int64_t)({v}), {lo}, {hi})")
        } else {
            format!("iecf_index_u({FAULT}, (uint64_t)({v}), {lo}, {hi})")
        })
    }

    /// Element `offs` of the variable-length array `desc` of `elem`s,
    /// row-major as the caller's array is laid out.
    pub(crate) fn flex_elem(&self, desc: &str, elem: &DTy, offs: &[String]) -> Lv {
        let mut flat = offs[0].clone();
        for (k, o) in offs.iter().enumerate().skip(1) {
            flat = format!("({flat}) * iecf_flex_n({desc}, {k}u) + {o}");
        }
        Lv::C(format!("(({}){desc}->p)[{flat}]", self.declare(elem, "(*)")))
    }

    /// Every element of the variable-length array `desc` set to its
    /// type's initial value, into `out`: an output, as its call starts
    /// (Ed.3 6.6.2.2: a FUNCTION has no memory).
    pub(crate) fn flex_init(&mut self, desc: &str, t: &DTy, out: &mut String) -> R<()> {
        let Kind::Flex(elem, n) = &t.kind else { unreachable!("an ARRAY[*]") };
        let k = self.fresh("k");
        let total: Vec<String> = (0..*n).map(|d| format!("iecf_flex_n({desc}, {d}u)")).collect();
        let lv = format!("(({}){desc}->p)[{k}]", self.declare(elem, "(*)"));
        let mut body = String::new();
        self.init_data(&lv, elem, None, &mut body)?;
        let _ = writeln!(out, "    for (uint32_t {k} = 0; {k} < {}; {k}++) {{", total.join(" * "));
        for l in body.lines() {
            let _ = writeln!(out, "    {l}");
        }
        let _ = writeln!(out, "    }}");
        Ok(())
    }

    /// `x` given to an in-out `ARRAY[*]`: a pointer to an `iecf_flex_t`
    /// of the array in C memory, or the one an `ARRAY[*]` already is.
    pub(crate) fn flex_arg(&mut self, x: &Expr, scope: &Scope) -> R<String> {
        let seq = std::mem::replace(&mut self.seq, false);
        let found = self.data_path(x, scope);
        self.seq = seq;
        match found? {
            Some((Lv::C(c), DTy { kind: Kind::Array(_, bounds), .. })) => {
                let b: Vec<String> =
                    bounds.iter().flat_map(|(lo, hi)| [format!("INT64_C({lo})"), format!("INT64_C({hi})")]).collect();
                Ok(format!("&(iecf_flex_t){{(void *)&({c}), (const int64_t[]){{{}}}}}", b.join(", ")))
            }
            // Passed on: an input's stays one the callee only reads.
            Some((Lv::C(d), DTy { kind: Kind::Flex(..), .. })) if d.contains("const ") => {
                Ok(format!("(const void *){d}"))
            }
            Some((Lv::C(d), DTy { kind: Kind::Flex(..), .. })) => Ok(format!("(void *){d}")),
            Some(_) => self.not_yet(x.span, "an ARRAY[*] given an array at an address or through a reference"),
            None => self.not_yet(x.span, "this given to an ARRAY[*]"),
        }
    }

    /// `LOWER_BOUND(A, d)` or `UPPER_BOUND(A, d)`, a DINT: a constant for
    /// an array of fixed bounds and a constant `d`; else read where the
    /// bounds are, a `d` outside the array's dimensions a fault
    /// (IECF_FAULT_INDEX), as Ed.3 Table 15 calls it an error.
    pub(crate) fn array_bound(&mut self, e: &Expr, sc: &StdCall, args: &[Arg], scope: &Scope) -> R<String> {
        let upper = sc.name == "UPPER_BOUND";
        let arg = |n: usize| crate::expr::arg_value(args, sc.inputs[n].1);
        let (Some(arr), Some(dim)) = (arg(0), arg(1)) else { return self.not_yet(e.span, "this call") };
        let Some((lv, t)) = self.data_path(arr, scope)? else { return self.not_yet(arr.span, "this array") };
        let dint = crate::c_type(Elem::Dint);
        let at = |k: i128| 2 * (k - 1) as usize + usize::from(upper);
        let (n, b) = match (&t.kind, &lv) {
            (Kind::Array(_, bounds), _) => {
                if let Some(k) = const_int(dim) {
                    let (lo, hi) = bounds[(k - 1) as usize];
                    let v = if upper { hi } else { lo };
                    return Ok(lit(i128::from(v), Elem::Dint).unwrap_or_else(|| "0".into()));
                }
                let b: Vec<String> =
                    bounds.iter().flat_map(|(lo, hi)| [format!("INT64_C({lo})"), format!("INT64_C({hi})")]).collect();
                (bounds.len(), format!("(const int64_t[]){{{}}}", b.join(", ")))
            }
            (Kind::Flex(_, n), Lv::C(d)) => {
                if let Some(k) = const_int(dim) {
                    return Ok(format!("(({dint}){d}->b[{}])", at(k)));
                }
                (*n, format!("{d}->b"))
            }
            _ => return self.not_yet(arr.span, "this array"),
        };
        let t = self.elem_of(dim)?;
        let v = self.value(dim, scope)?;
        let v = if t.is_signed_int() { format!("(int64_t)({v})") } else { format!("iecf_dim_u((uint64_t)({v}))") };
        self.may_fault = true;
        Ok(format!("iecf_bound({FAULT}, {v}, {n}, {b}, {})", u8::from(upper)))
    }

    /// A whole variable-length array where only an element or a bound
    /// of it is compiled.
    pub(crate) fn flex_whole<T>(&mut self, at: Span) -> R<T> {
        self.not_yet(at, "a whole ARRAY[*] here (its elements, LOWER_BOUND and UPPER_BOUND are compiled)")
    }
}

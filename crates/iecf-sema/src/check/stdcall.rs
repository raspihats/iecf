//! Calls of the standard functions (Ed.3 6.6.2.5): arguments bound to
//! the inputs by position or by name (6.6.1.4, 6.6.2.5.1), each checked
//! against what the input takes, the result worked out from them.

use super::{Checker, upper};
use crate::Extension;
use crate::stdlib::{Class, Kind, Res, Sig, unify};
use crate::types::{Conversion, Elem, Ty};
use iecf_diagnostics::{Code, Span};
use iecf_syntax::ast::{Arg, Expr};

/// An input or output of the call, with what was passed to it.
struct Slot<'e> {
    name: String,
    kind: Kind,
    value: Option<&'e Expr>,
}

impl<'a> Checker<'a> {
    pub(super) fn std_call(&mut self, name: &str, sig: Sig, args: &[Arg], span: Span) -> Ty {
        let Some((inputs, outputs)) = self.bind(name, &sig, args, span) else {
            self.args_only(args);
            return match sig.result {
                Res::Ty(t) | Res::Convert(t) => t,
                _ => sig.typed.unwrap_or(Ty::Unknown),
            };
        };
        let mut types: Vec<Ty> = Vec::new();
        let mut common: Vec<(Ty, Span)> = Vec::new();
        for slot in &inputs {
            let Some(e) = slot.value else { continue };
            let t = self.expr(e);
            types.push(t.clone());
            let kind = match (&slot.kind, &sig.typed_k) {
                (Kind::Class(Class::AnyInt), Some(k)) if slot.name == "K" => Kind::Ty(k.clone()),
                (k, _) => k.clone(),
            };
            match kind {
                Kind::Ty(want) => {
                    self.coerce(&t, &want, e.span);
                }
                Kind::Class(_) if matches!(sig.result, Res::Op(..)) => {
                    if let Some(want) = &sig.typed {
                        self.coerce(&t, want, e.span);
                    }
                }
                Kind::Class(c) => self.in_class(name, &slot.name, c, &t, e.span),
                Kind::Common(c) => match &sig.typed {
                    Some(want) => {
                        self.coerce(&t, want, e.span);
                    }
                    None => {
                        if !t.is_unknown() && !c.has(&t) {
                            self.in_class(name, &slot.name, c, &t, e.span);
                        } else {
                            common.push((as_string(&t, c), e.span));
                        }
                    }
                },
            }
        }
        for slot in &outputs {
            let Some(e) = slot.value else { continue };
            let t = self.place(e);
            match &slot.kind {
                Kind::Ty(want) => {
                    self.coerce(want, &t, e.span);
                }
                Kind::Class(c) | Kind::Common(c) => self.in_class(name, &slot.name, *c, &t, e.span),
            }
        }
        let common = match &sig.typed {
            Some(t) => t.clone(),
            None => self.common_type(name, &common, span),
        };
        let slots = |v: &[Slot]| v.iter().map(|s| (s.name.clone(), s.value.map(|e| e.span))).collect();
        self.out.std_calls.insert(
            span,
            crate::StdCall {
                name: sig.base.clone().unwrap_or_else(|| upper(name)),
                called: upper(name),
                inputs: slots(&inputs),
                outputs: slots(&outputs),
                common: common.clone(),
                typed_k: sig.typed_k.clone(),
            },
        );
        match sig.result {
            Res::Common => match common {
                Ty::Str { wide, .. } => Ty::Str { wide, len: None },
                t => t,
            },
            Res::Ty(t) => t,
            Res::Op(op, _) => {
                if sig.typed.is_some() {
                    return common;
                }
                let mut it = types.into_iter();
                let first = it.next().unwrap_or(Ty::Unknown);
                it.fold(first, |acc, t| self.binary(op, &acc, &t, span))
            }
            Res::Convert(to) => {
                let from = types.first().cloned().unwrap_or(Ty::Unknown);
                if let (Some(a), Some(b)) = (figure_11_elem(&from), figure_11_elem(&to))
                    && a.conversion_to(b) == Conversion::None
                    && !ed2_string_conversion(a, b)
                    && !(time_integer_conversion(a, b) && self.options.has(Extension::TimeIntegerConversions))
                    && !(bit_string_to_bool(a, b) && self.options.has(Extension::BitStringToBool))
                {
                    let at = inputs.first().and_then(|s| s.value).map_or(span, |e| e.span);
                    let hint = if time_integer_conversion(a, b) {
                        " (CODESYS, TwinCAT and Siemens have it, in milliseconds: --extension time-integer-conversions)"
                    } else if bit_string_to_bool(a, b) {
                        " (CODESYS and TwinCAT have it, TRUE for any value but 0: --extension bit-string-to-bool; \
                         or write x <> 0)"
                    } else {
                        ""
                    };
                    self.error(
                        Code::TypeMismatch,
                        at,
                        format!("Ed.3 Figure 11 defines no conversion from {} to {}{hint}", a.name(), b.name()),
                    );
                }
                to
            }
            Res::Ref => Ty::Ref(Box::new(types.into_iter().next().unwrap_or(Ty::Unknown))),
        }
    }

    fn in_class(&mut self, name: &str, input: &str, c: Class, t: &Ty, span: Span) {
        if t.is_unknown() || c.has(t) {
            return;
        }
        self.error(Code::OperandTypes, span, format!("{input} of {name} takes {}, not {t}", c.name()));
    }

    /// The one type the common inputs share, by the implicit
    /// conversions; an error if they share none.
    fn common_type(&mut self, name: &str, tys: &[(Ty, Span)], span: Span) -> Ty {
        let Some((first, _)) = tys.first() else { return Ty::Unknown };
        let mut acc = first.clone();
        for (t, at) in &tys[1..] {
            match unify(&acc, t) {
                Some(u) => acc = u,
                None => {
                    let what = match (&acc, t) {
                        (Ty::Str { wide: a, .. }, Ty::Str { wide: b, .. }) if a != b => {
                            " (Ed.3 6.6.2.5.11: STRING and WSTRING do not mix)"
                        }
                        _ => "",
                    };
                    let at = if tys.len() > 1 { *at } else { span };
                    self.error(
                        Code::OperandTypes,
                        at,
                        format!("the inputs of {name} share one type, and {acc} and {t} have none{what}"),
                    );
                    return Ty::Unknown;
                }
            }
        }
        acc
    }

    /// The arguments put in the function's inputs and outputs, by
    /// position or by name. `None` after an error that leaves the call
    /// without a meaning to check further.
    fn bind<'e>(
        &mut self,
        name: &str,
        sig: &Sig,
        args: &'e [Arg],
        span: Span,
    ) -> Option<(Vec<Slot<'e>>, Vec<Slot<'e>>)> {
        let slot = |n: &str, k: &Kind| Slot { name: n.to_string(), kind: k.clone(), value: None };
        let mut inputs: Vec<Slot> = sig.inputs.iter().map(|(n, k)| slot(n, k)).collect();
        let mut outputs: Vec<Slot> = sig.outputs.iter().map(|(n, k)| slot(n, k)).collect();
        let formal = args.iter().any(|a| !matches!(a, Arg::Positional(_)));
        let positional = args.iter().any(|a| matches!(a, Arg::Positional(_)));
        if formal && positional {
            self.error(
                Code::BadCall,
                span,
                "a call passes its arguments all by name or all in order, not both (Ed.3 6.6.1.4)",
            );
            return None;
        }
        let fixed = inputs.len();
        if positional {
            let n = args.len();
            match &sig.ext {
                Some(ext) if n < fixed + ext.min => {
                    self.error(
                        Code::BadCall,
                        span,
                        format!("{name} takes at least {} inputs, not {n}", fixed + ext.min),
                    );
                    return None;
                }
                None if n < fixed || n > fixed + outputs.len() => {
                    let takes = if outputs.is_empty() {
                        format!("{fixed} input(s)")
                    } else {
                        format!("{fixed} input(s), then its {} output(s)", outputs.len())
                    };
                    self.error(Code::BadCall, span, format!("{name} takes {takes} in order, not {n} (Ed.3 6.6.1.4)"));
                    return None;
                }
                _ => {}
            }
            for (i, a) in args.iter().enumerate() {
                let Arg::Positional(e) = a else { continue };
                if i < fixed {
                    inputs[i].value = Some(e);
                } else if let Some(ext) = &sig.ext {
                    let n = ext.first as usize + i - fixed;
                    inputs.push(Slot { name: format!("IN{n}"), kind: ext.kind.clone(), value: Some(e) });
                } else {
                    outputs[i - fixed].value = Some(e);
                }
            }
            return Some((inputs, outputs));
        }
        let mut seen: Vec<String> = Vec::new();
        for a in args {
            let (id, is_output, value) = match a {
                Arg::Input(id, e) => (id, false, e),
                Arg::Output { name, target, .. } => (name, true, target),
                Arg::Positional(_) => continue,
            };
            let key = upper(&id.text);
            if seen.contains(&key) {
                self.error(Code::BadCall, id.span, format!("'{}' is passed twice", id.text));
                continue;
            }
            seen.push(key.clone());
            if key == "EN" || key == "ENO" {
                // EN and ENO (Ed.3 6.6.1.5), BOOL both.
                if (key == "ENO") != is_output {
                    self.error(
                        Code::BadCall,
                        id.span,
                        format!("{key} is passed with {}", if is_output { ":=" } else { "=>" }),
                    );
                }
                let t = if is_output { self.place(value) } else { self.expr(value) };
                self.coerce(&t, &Ty::Elem(Elem::Bool), value.span);
                continue;
            }
            let ext_index = sig.ext.as_ref().and_then(|ext| {
                let n: u32 = key.strip_prefix("IN")?.parse().ok()?;
                (n >= ext.first && !inputs.iter().any(|s| s.name == key)).then_some(ext.kind.clone())
            });
            if let Some(s) = inputs.iter_mut().find(|s| s.name == key) {
                s.value = Some(value);
            } else if let Some(kind) = ext_index {
                inputs.push(Slot { name: key.clone(), kind, value: Some(value) });
            } else if let Some(s) = outputs.iter_mut().find(|s| s.name == key) {
                s.value = Some(value);
                if !is_output {
                    self.error(
                        Code::BadCall,
                        id.span,
                        format!("'{}' is an output: take it with {} => ...", id.text, id.text),
                    );
                }
                continue;
            } else {
                self.error(Code::BadCall, id.span, format!("{name} has no input or output '{}'", id.text));
                if is_output {
                    self.place(value)
                } else {
                    self.expr(value)
                };
                continue;
            }
            if is_output {
                self.error(
                    Code::BadCall,
                    id.span,
                    format!("'{}' is an input: pass it with {} := ...", id.text, id.text),
                );
            }
        }
        // Extensible inputs in the order of their numbers, which a formal
        // call may give in any order (6.6.2.5.1).
        let mut ext_inputs = inputs.split_off(fixed);
        ext_inputs.sort_by_key(|s| s.name[2..].parse::<u32>().unwrap_or(0));
        inputs.extend(ext_inputs);
        Some((inputs, outputs))
    }
}

/// CHAR and WCHAR as one-character strings, where any characters go
/// (Ed.3 Table 34, NOTE 2), so they share a type with strings.
/// And an integer literal where a real goes as a real literal: `SQRT(2)`
/// is a REAL.
fn as_string(t: &Ty, c: Class) -> Ty {
    if c == Class::AnyReal && matches!(t, Ty::IntLit(_)) {
        return Ty::RealLit;
    }
    match (c, t.elem()) {
        (Class::AnyChars, Some(Elem::Char)) => Ty::Str { wide: false, len: None },
        (Class::AnyChars, Some(Elem::Wchar)) => Ty::Str { wide: true, len: None },
        _ => t.clone(),
    }
}

/// A conversion between a string and a type that is not a character
/// type. Figure 11 marks these "-", none defined; but Ed.2 defined them
/// (Table 22, note e: the string holds the value as its literal is
/// written), and Ed.3 states it is "fully compatible" with Ed.2 without
/// listing their removal (Annex B). iecf takes them as kept: PROPOSAL.md
/// §3, to be read again against Ed.4.
fn ed2_string_conversion(a: Elem, b: Elem) -> bool {
    let string = |e: Elem| matches!(e, Elem::String | Elem::Wstring);
    let char = |e: Elem| matches!(e, Elem::Char | Elem::Wchar);
    (string(a) && !string(b) && !char(b)) || (string(b) && !string(a) && !char(a))
}

/// TIME or LTIME on one side, an integer or a bit string other than BOOL
/// on the other: the conversions every big vendor has and Figure 11 does
/// not (docs/extensions.md, time-integer-conversions).
fn time_integer_conversion(a: Elem, b: Elem) -> bool {
    let number = |e: Elem| e.is_int() || (e.is_bit() && e != Elem::Bool);
    (a.is_duration() && number(b)) || (b.is_duration() && number(a))
}

/// BYTE, WORD, DWORD or LWORD to BOOL: in Ed.3 Table 25, not in Figure 11
/// (PROPOSAL.md §6.8).
fn bit_string_to_bool(a: Elem, b: Elem) -> bool {
    b == Elem::Bool && matches!(a, Elem::Byte | Elem::Word | Elem::Dword | Elem::Lword)
}

/// A type's row or column in Figure 11, if it has one.
fn figure_11_elem(t: &Ty) -> Option<Elem> {
    match t {
        Ty::Elem(e) => Some(*e),
        Ty::Str { wide: false, .. } => Some(Elem::String),
        Ty::Str { wide: true, .. } => Some(Elem::Wstring),
        _ => None,
    }
}

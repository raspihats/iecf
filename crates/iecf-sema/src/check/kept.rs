//! method-in-outs: a method's own VAR_IN_OUT it binds to a block's
//! in-out that a method of the block uses after the call - kept past
//! the method's call, as the block keeps it. The method is not refused
//! for it; each call of the method is, where it gives a variable that
//! may be gone by then (E0027), the rule moving to the call
//! (decided 2026-11-25, florin).

use super::*;

/// A call giving a method's in-out a variable that may be gone after
/// the call: refused once the in-out is found kept.
pub(super) struct InOutCall {
    /// The method's key.
    callee: String,
    /// The in-out, upper-cased.
    param: String,
    at: Span,
    /// The calling method's own in-out given, which is then kept too.
    from: Option<(String, String)>,
}

impl<'a> Checker<'a> {
    /// The in-out of the method being checked that `arg` is or is part
    /// of, upper-cased: not through a pointer.
    pub(super) fn own_method_inout(&self, arg: &Expr) -> Option<String> {
        if !self.cur_pou.is_some_and(|p| p.kind == PouKind::Method) {
            return None;
        }
        let k = upper(&self.root_name(arg)?.text);
        (self.own_section(&k) == Some(VarKind::InOut)).then_some(k)
    }

    /// method-in-outs: `arg`, which may be gone after the call, given to
    /// the method `key`'s in-out `param`: kept for the end, when it is
    /// known whether the method keeps it.
    pub(super) fn method_inout_given(&mut self, key: &str, param: &str, arg: &Expr) {
        if !self.options.has(Extension::MethodInOuts)
            || !self.pous.get(key).is_some_and(|p| p.kind == PouKind::Method)
            || !self.address_of_temp(arg)
        {
            return;
        }
        let from = self.own_method_inout(arg).map(|p| (self.pou_key.clone(), p));
        self.inout_calls.push(InOutCall { callee: key.to_string(), param: upper(param), at: arg.span, from });
    }

    /// Whether the method `kept`, keeping its in-out, may be the one a
    /// call of `callee` runs: the same method of the same type, a type
    /// derived from it, or a type behind an interface.
    fn may_run(&self, kept: &str, callee: &str) -> bool {
        let (Some((ko, kn)), Some((co, cn))) = (crate::split_key(kept), crate::split_key(callee)) else {
            return kept == callee;
        };
        kn == cn && (self.derives(ko, co) || self.pous.get(co).is_some_and(|p| p.kind == PouKind::Interface))
    }

    /// Why a call of `callee` keeps its in-out `param`, if it may.
    fn kept_why(&self, callee: &str, param: &str) -> Option<&String> {
        self.kept_inouts.iter().find(|((m, p), _)| p == param && self.may_run(m, callee)).map(|(_, why)| why)
    }

    /// At the end: each method's in-outs it keeps, through the methods it
    /// gives them to; then each call giving one a variable that may be
    /// gone, refused.
    pub(super) fn kept_inouts_checked(&mut self) {
        let calls = std::mem::take(&mut self.inout_calls);
        loop {
            let mut more = Vec::new();
            for c in &calls {
                if let Some(from) = &c.from
                    && !self.kept_inouts.contains_key(from)
                    && self.kept_why(&c.callee, &c.param).is_some()
                {
                    let why = format!(
                        "it gives it to the in-out {} of {}, which keeps it",
                        c.param,
                        crate::namespaces::display(&c.callee)
                    );
                    more.push((from.clone(), why));
                }
            }
            if more.is_empty() {
                break;
            }
            self.kept_inouts.extend(more);
        }
        let mut seen = HashSet::new();
        for c in &calls {
            if c.from.is_some() || !seen.insert(c.at) {
                continue;
            }
            let Some(why) = self.kept_why(&c.callee, &c.param).cloned() else { continue };
            self.error(
                Code::ReferenceNotKept,
                c.at,
                format!(
                    "{} keeps its in-out {} past the call - {why} (method-in-outs): this variable may be gone by \
                     then - a VAR_TEMP, a FUNCTION's or method's, an in-out - so give one that lives as long as \
                     the instance",
                    crate::namespaces::display(&c.callee),
                    c.param
                ),
            );
        }
    }
}

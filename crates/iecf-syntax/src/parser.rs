//! Tokens to a syntax tree: hand-written recursive descent following Ed.3
//! Annex A.
//!
//! Recovery: a mistake is reported once, at the token where the grammar
//! stopped fitting, and the parser skips to the end of what it was
//! reading - the `;` of a statement or declaration, the END keyword of a
//! block - then carries on. A block whose END never comes is reported at
//! its start (E0006). The whole file is always parsed: an error in one
//! POU never hides the next.

use crate::ast::*;
use crate::lexer::lex;
use crate::token::{Kw, Token, TokenKind};
use iecf_diagnostics::{Code, Diagnostic, FileId, Span};

/// A file's tree and every diagnostic found reading it, lexical included.
#[derive(Debug)]
pub struct Parse {
    pub unit: SourceUnit,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(file: FileId, text: &str) -> Parse {
    let (tokens, mut diagnostics) = lex(file, text);
    let mut tokens: Vec<Token> = tokens.into_iter().filter(|t| !t.kind.is_trivia()).collect();
    let keyword_names = soft_keywords(text, &mut tokens);
    let mut p = Parser {
        file,
        text,
        toks: tokens,
        pos: 0,
        last_end: 0,
        diags: Vec::new(),
        last_error_at: None,
        fb_tasks: Vec::new(),
        in_body: false,
        chained: Vec::new(),
        chain_rest: Vec::new(),
        missing_semis: Vec::new(),
        properties: Vec::new(),
        stop_words: Vec::new(),
        fb_args: None,
    };
    let mut unit = p.source_unit();
    unit.keyword_names = keyword_names;
    unit.chained_assignments = std::mem::take(&mut p.chained);
    unit.missing_semicolons = std::mem::take(&mut p.missing_semis);
    unit.properties = std::mem::take(&mut p.properties);
    diagnostics.append(&mut p.diags);
    diagnostics.sort_by_key(|d| (d.span.start, d.span.end));
    Parse { unit, diagnostics }
}

/// STEP, ON, R_EDGE and F_EDGE are keywords Ed.3 reserves (6.1.3) and
/// CODESYS does not: code written for it names variables `step` and
/// `on`. Each becomes a name where its keyword cannot stand - STEP not
/// before a step's name, ON not after `RESOURCE name`, R_EDGE and F_EDGE
/// not after a declaration's BOOL - and its place is kept for the
/// checker, which takes it only with the keyword-names extension. The
/// grammar has each keyword in that one place only, so a program that
/// uses them as keywords reads as before.
fn soft_keywords(text: &str, toks: &mut [Token]) -> Vec<(Span, String)> {
    let mut names = Vec::new();
    for i in 0..toks.len() {
        let TokenKind::Kw(k @ (Kw::Step | Kw::On | Kw::REdge | Kw::FEdge)) = toks[i].kind else { continue };
        let prev = |n: usize| i.checked_sub(n).map(|j| toks[j]);
        let word = |t: Token| &text[t.span.start as usize..t.span.end as usize];
        let keyword = match k {
            Kw::Step => toks.get(i + 1).is_some_and(|t| t.kind == TokenKind::Ident),
            Kw::On => {
                prev(1).is_some_and(|t| t.kind == TokenKind::Ident)
                    && prev(2).is_some_and(|t| t.kind == TokenKind::Kw(Kw::Resource))
            }
            _ => prev(1).is_some_and(|t| t.kind == TokenKind::Ident && word(t).eq_ignore_ascii_case("BOOL")),
        };
        if !keyword {
            toks[i].kind = TokenKind::Ident;
            names.push((toks[i].span, word(toks[i]).to_string()));
        }
    }
    names
}

struct Parser<'a> {
    file: FileId,
    text: &'a str,
    toks: Vec<Token>,
    pos: usize,
    /// End of the last token consumed: where a node's span ends.
    last_end: u32,
    diags: Vec<Diagnostic>,
    /// The token index of the last error, so one spot gets one error.
    last_error_at: Option<usize>,
    /// Inside a POU's statements, where a direct address is deprecated.
    in_body: bool,
    /// `fb WITH task` read among a program configuration's elements
    /// (Ed.3 Annex A, FB_Task): the instance and the task.
    fb_tasks: Vec<(Ident, Ident)>,
    /// Each chained assignment's place (`a := b := 0;`).
    chained: Vec<Span>,
    /// A chained assignment's statements after its first, which the
    /// statement list takes next.
    chain_rest: Vec<Stmt>,
    /// Where a structured statement's `;` is missing (optional-semicolons).
    missing_semis: Vec<Span>,
    /// Each CODESYS PROPERTY's place (properties).
    properties: Vec<Span>,
    /// `T(args)` just read as a type: its FB_init arguments, for the
    /// declaration being read (fb-init).
    fb_args: Option<(Vec<Arg>, Span)>,
    /// Words, not keywords, that end a statement list here (END_GET,
    /// END_SET in a property).
    stop_words: Vec<&'static str>,
}

/// Every name `from` in `body`, a property's in its getter, made `to`.
fn rename_in(body: &mut [Stmt], from: &str, to: &str) {
    fn expr(e: &mut Expr, from: &str, to: &str) {
        match &mut e.kind {
            ExprKind::Name(id) if id.is(from) => id.text = to.to_string(),
            ExprKind::Member(x, _)
            | ExprKind::BitAccess(x, _)
            | ExprKind::Deref(x)
            | ExprKind::Unary(_, x)
            | ExprKind::Paren(x) => expr(x, from, to),
            ExprKind::Index(x, ix) => {
                expr(x, from, to);
                ix.iter_mut().for_each(|i| expr(i, from, to));
            }
            ExprKind::Binary(_, x, y) => {
                expr(x, from, to);
                expr(y, from, to);
            }
            ExprKind::Call { callee, args } => {
                expr(callee, from, to);
                for a in args.iter_mut() {
                    match a {
                        Arg::Positional(x) | Arg::Input(_, x) | Arg::Output { target: x, .. } => expr(x, from, to),
                    }
                }
            }
            _ => {}
        }
    }
    for s in body {
        match &mut s.kind {
            StmtKind::Assign { target, value }
            | StmtKind::RefAssign { target, value }
            | StmtKind::AssignAttempt { target, value } => {
                expr(target, from, to);
                expr(value, from, to);
            }
            StmtKind::Call(e) => expr(e, from, to),
            StmtKind::If { branches, else_body } => {
                for (c, b) in branches.iter_mut() {
                    expr(c, from, to);
                    rename_in(b, from, to);
                }
                if let Some(b) = else_body {
                    rename_in(b, from, to);
                }
            }
            StmtKind::Case { selector, arms, else_body } => {
                expr(selector, from, to);
                for a in arms.iter_mut() {
                    rename_in(&mut a.body, from, to);
                }
                if let Some(b) = else_body {
                    rename_in(b, from, to);
                }
            }
            StmtKind::For { from: f, to: t, by, body, .. } => {
                expr(f, from, to);
                expr(t, from, to);
                if let Some(b) = by {
                    expr(b, from, to);
                }
                rename_in(body, from, to);
            }
            StmtKind::While { cond, body } => {
                expr(cond, from, to);
                rename_in(body, from, to);
            }
            StmtKind::Repeat { body, until } => {
                rename_in(body, from, to);
                expr(until, from, to);
            }
            _ => {}
        }
    }
}

/// Whether an expression has a call anywhere in it.
fn has_call(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Call { .. } => true,
        ExprKind::Member(x, _)
        | ExprKind::BitAccess(x, _)
        | ExprKind::Deref(x)
        | ExprKind::Unary(_, x)
        | ExprKind::Paren(x) => has_call(x),
        ExprKind::Index(x, ix) => has_call(x) || ix.iter().any(has_call),
        ExprKind::Binary(_, x, y) => has_call(x) || has_call(y),
        _ => false,
    }
}

/// Keywords that end or start a structure. A statement list stops at
/// one; a declaration list too.
fn is_structural(kw: Kw) -> bool {
    use Kw::*;
    matches!(
        kw,
        Program
            | EndProgram
            | Function
            | EndFunction
            | FunctionBlock
            | EndFunctionBlock
            | Class
            | EndClass
            | Interface
            | EndInterface
            | Method
            | EndMethod
            | PropertyGet
            | PropertySet
            | EndProperty
            | Namespace
            | EndNamespace
            | Using
            | Var
            | VarInput
            | VarOutput
            | VarInOut
            | VarTemp
            | VarExternal
            | VarGlobal
            | VarAccess
            | VarConfig
            | EndVar
            | Type
            | EndType
            | EndStruct
            | Elsif
            | Else
            | EndIf
            | EndCase
            | EndFor
            | EndWhile
            | Until
            | EndRepeat
            | Step
            | InitialStep
            | EndStep
            | Transition
            | EndTransition
            | Action
            | EndAction
            | Configuration
            | EndConfiguration
            | Resource
            | EndResource
            | Task
    )
}

fn var_kind(kw: Kw) -> Option<VarKind> {
    Some(match kw {
        Kw::Var => VarKind::Var,
        Kw::VarInput => VarKind::Input,
        Kw::VarOutput => VarKind::Output,
        Kw::VarInOut => VarKind::InOut,
        Kw::VarTemp => VarKind::Temp,
        Kw::VarExternal => VarKind::External,
        Kw::VarGlobal => VarKind::Global,
        _ => return None,
    })
}

fn is_var_block_start(kw: Kw) -> bool {
    var_kind(kw).is_some() || matches!(kw, Kw::VarAccess | Kw::VarConfig)
}

fn binary_op(kind: TokenKind) -> Option<(BinaryOp, u8)> {
    // Ed.3 Table 71, lowest to highest; unary operators bind tighter
    // than `**`.
    Some(match kind {
        TokenKind::Kw(Kw::Or) => (BinaryOp::Or, 1),
        TokenKind::Kw(Kw::Xor) => (BinaryOp::Xor, 2),
        TokenKind::Kw(Kw::And) | TokenKind::Amp => (BinaryOp::And, 3),
        TokenKind::Eq => (BinaryOp::Eq, 4),
        TokenKind::Ne => (BinaryOp::Ne, 4),
        TokenKind::Lt => (BinaryOp::Lt, 5),
        TokenKind::Le => (BinaryOp::Le, 5),
        TokenKind::Gt => (BinaryOp::Gt, 5),
        TokenKind::Ge => (BinaryOp::Ge, 5),
        TokenKind::Plus => (BinaryOp::Add, 6),
        TokenKind::Minus => (BinaryOp::Sub, 6),
        TokenKind::Star => (BinaryOp::Mul, 7),
        TokenKind::Slash => (BinaryOp::Div, 7),
        TokenKind::Kw(Kw::Mod) => (BinaryOp::Mod, 7),
        TokenKind::Power => (BinaryOp::Power, 8),
        _ => return None,
    })
}

impl Parser<'_> {
    // ---- the cursor ------------------------------------------------

    fn kind(&self) -> TokenKind {
        self.nth(0)
    }

    fn nth(&self, n: usize) -> TokenKind {
        self.toks.get(self.pos + n).map_or(TokenKind::Eof, |t| t.kind)
    }

    fn tok(&self) -> Token {
        self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.kind() == kind
    }

    fn at_kw(&self, kw: Kw) -> bool {
        self.kind() == TokenKind::Kw(kw)
    }

    fn at_eof(&self) -> bool {
        self.at(TokenKind::Eof)
    }

    fn bump(&mut self) -> Token {
        let t = self.tok();
        if t.kind != TokenKind::Eof {
            self.pos += 1;
            self.last_end = t.span.end;
        }
        t
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: Kw) -> bool {
        self.eat(TokenKind::Kw(kw))
    }

    fn text_of(&self, t: Token) -> &str {
        &self.text[t.span.start as usize..t.span.end as usize]
    }

    fn span_from(&self, start: u32) -> Span {
        Span::new(self.file, start, self.last_end.max(start))
    }

    fn start(&self) -> u32 {
        self.tok().span.start
    }

    // ---- diagnostics -----------------------------------------------

    fn found(&self) -> String {
        let t = self.tok();
        match t.kind {
            TokenKind::Ident => format!("'{}'", self.text_of(t)),
            TokenKind::Kw(kw) => kw.as_str().to_string(),
            k => k.describe().to_string(),
        }
    }

    /// E0005 at the current token, unless this token already has an error.
    fn error_here(&mut self, message: String) {
        if self.last_error_at == Some(self.pos) {
            return;
        }
        self.last_error_at = Some(self.pos);
        let span = self.tok().span;
        self.diags.push(Diagnostic::new(Code::UnexpectedToken, span, message));
    }

    fn expected(&mut self, what: &str) {
        let found = self.found();
        self.error_here(format!("expected {what}, found {found}"));
    }

    fn expect(&mut self, kind: TokenKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.expected(kind.describe());
        false
    }

    fn expect_kw(&mut self, kw: Kw) -> bool {
        if self.eat_kw(kw) {
            return true;
        }
        self.expected(kw.as_str());
        false
    }

    /// The END keyword of a block opened at `opener`. Missing, the block
    /// is reported at its start, and nothing is consumed: what is here
    /// probably belongs to an outer block.
    fn close(&mut self, end: Kw, opener: Span, what: &str) {
        if self.eat_kw(end) {
            return;
        }
        let found = self.found();
        if self.last_error_at != Some(self.pos) {
            self.last_error_at = Some(self.pos);
            self.diags.push(Diagnostic::new(
                Code::UnclosedBlock,
                opener,
                format!("{what} is never closed: expected {}, found {found}", end.as_str()),
            ));
        }
    }

    fn not_yet(&mut self, span: Span, what: &str) {
        self.diags.push(Diagnostic::new(Code::NotYetSupported, span, format!("{what} is not compiled by iecf yet")));
    }

    /// Skip a construct iecf does not read yet, through its END keyword.
    fn skip_construct(&mut self, end: Kw, what: &str) {
        let start = self.tok().span;
        self.bump();
        self.not_yet(start, what);
        while !self.at_eof() && !self.at_kw(end) {
            self.bump();
        }
        self.eat_kw(end);
    }

    fn error_at(&mut self, span: Span, message: String) {
        self.last_error_at = Some(self.pos);
        self.diags.push(Diagnostic::new(Code::UnexpectedToken, span, message));
    }

    /// Past `end`, after an error was reported for what it closes.
    fn skip_construct_quietly(&mut self, end: Kw) {
        self.bump();
        while !self.at_eof() && !self.at_kw(end) {
            self.bump();
        }
        self.eat_kw(end);
    }

    fn skip_until_semi_or(&mut self, stop: &[Kw]) {
        while !self.at_eof() {
            if self.eat(TokenKind::Semi) {
                return;
            }
            if let TokenKind::Kw(kw) = self.kind()
                && (stop.contains(&kw) || is_structural(kw))
            {
                return;
            }
            self.bump();
        }
    }

    /// A keyword where a name is being declared or assigned: `step : INT;`,
    /// `on := TRUE;`. Said plainly, and skipped to its `;`, instead of
    /// reading the keyword as the construct it starts.
    fn keyword_as_name(&mut self) -> bool {
        let TokenKind::Kw(kw) = self.kind() else { return false };
        if !matches!(self.nth(1), TokenKind::Colon | TokenKind::Comma | TokenKind::Assign | TokenKind::Kw(Kw::At)) {
            return false;
        }
        let span = self.tok().span;
        let text = self.text_of(self.tok()).to_string();
        self.last_error_at = Some(self.pos);
        self.diags.push(Diagnostic::new(
            Code::UnexpectedToken,
            span,
            format!("'{text}' is the keyword {} and cannot be a name (Ed.3 6.1.3): choose another name", kw.as_str()),
        ));
        self.bump();
        self.skip_until_semi_or(&[]);
        true
    }

    /// A name, or a placeholder after reporting that one was expected.
    fn ident(&mut self, what: &str) -> Ident {
        if self.at(TokenKind::Ident) {
            let t = self.bump();
            return Ident { text: self.text_of(t).to_string(), span: t.span };
        }
        self.expected(what);
        let at = self.tok().span.start;
        Ident { text: String::new(), span: Span::new(self.file, at, at) }
    }

    // ---- files and items -------------------------------------------

    fn source_unit(&mut self) -> SourceUnit {
        SourceUnit { items: self.items(None), ..SourceUnit::default() }
    }

    /// Declarations until `end` (END_NAMESPACE), or the end of the file.
    fn items(&mut self, end: Option<Kw>) -> Vec<Item> {
        let mut items = Vec::new();
        while !self.at_eof() && !end.is_some_and(|e| self.at_kw(e)) {
            let before = self.pos;
            match self.kind() {
                TokenKind::Kw(Kw::Program) => items.push(Item::Pou(self.pou(PouKind::Program))),
                TokenKind::Kw(Kw::Function) => items.push(Item::Pou(self.pou(PouKind::Function))),
                TokenKind::Kw(Kw::FunctionBlock) => items.push(Item::Pou(self.pou(PouKind::FunctionBlock))),
                // CODESYS 2.3 exports write `FUNCTIONBLOCK`; CODESYS 3
                // refuses it too (C0098). One error, then the block read
                // as the FUNCTION_BLOCK meant, so nothing else cascades.
                TokenKind::Ident if self.text_of(self.tok()).eq_ignore_ascii_case("FUNCTIONBLOCK") => {
                    self.diags.push(Diagnostic::new(
                        Code::UnexpectedToken,
                        self.tok().span,
                        "FUNCTIONBLOCK is no keyword: write FUNCTION_BLOCK (Ed.3 Annex A, FB_Decl; CODESYS 3 \
                         refuses it too, error C0098)",
                    ));
                    self.last_error_at = Some(self.pos);
                    items.push(Item::Pou(self.pou(PouKind::FunctionBlock)));
                }
                TokenKind::Kw(Kw::Type) => items.push(Item::Types(self.types())),
                TokenKind::Kw(Kw::VarGlobal) => {
                    if let Some(b) = self.var_block() {
                        items.push(Item::Globals(b));
                    }
                }
                TokenKind::Kw(Kw::Configuration) => items.push(Item::Configuration(self.configuration())),
                TokenKind::Kw(Kw::VarConfig) => {
                    let at = self.tok().span;
                    let mut inits = Vec::new();
                    self.config_init_block(&mut inits);
                    items.push(Item::ConfigList(inits, at));
                }
                TokenKind::Kw(Kw::Class) => items.push(Item::Pou(self.pou(PouKind::Class))),
                TokenKind::Kw(Kw::Interface) => items.push(Item::Pou(self.pou(PouKind::Interface))),
                TokenKind::Kw(Kw::Namespace) => items.push(Item::Namespace(self.namespace())),
                TokenKind::Kw(Kw::Using) => items.push(Item::Using(self.using())),
                _ => {
                    self.expected(
                        "PROGRAM, FUNCTION, FUNCTION_BLOCK, CLASS, TYPE, VAR_GLOBAL, CONFIGURATION or NAMESPACE",
                    );
                    self.bump();
                    while !self.at_eof()
                        && !matches!(
                            self.kind(),
                            TokenKind::Kw(
                                Kw::Program
                                    | Kw::Function
                                    | Kw::FunctionBlock
                                    | Kw::Type
                                    | Kw::VarGlobal
                                    | Kw::Configuration
                                    | Kw::Class
                                    | Kw::Interface
                                    | Kw::Namespace
                                    | Kw::EndNamespace
                                    | Kw::Using
                            )
                        )
                    {
                        self.bump();
                    }
                    if end.is_none() && self.at_kw(Kw::EndNamespace) {
                        self.expected("a declaration: END_NAMESPACE closes no NAMESPACE here");
                        self.bump();
                    }
                }
            }
            if self.pos == before {
                self.bump();
            }
        }
        items
    }

    /// `NAMESPACE [INTERNAL] A.B USING ...; declarations END_NAMESPACE`
    /// (Ed.3 6.9.2).
    fn namespace(&mut self) -> Namespace {
        let start = self.start();
        let kw = self.bump();
        let internal = self.access_spec();
        let name = self.dotted("the namespace's name");
        let opener = Span::new(self.file, kw.span.start, self.last_end.max(kw.span.end));
        let mut using = Vec::new();
        while self.at_kw(Kw::Using) {
            using.push(self.using());
        }
        let items = self.items(Some(Kw::EndNamespace));
        let shown: Vec<&str> = name.iter().map(|n| n.text.as_str()).collect();
        self.close(Kw::EndNamespace, opener, &format!("NAMESPACE {}", shown.join(".")));
        Namespace { name, internal, using, items, span: self.span_from(start) }
    }

    /// `USING A.B, C;` (Ed.3 6.9.4).
    fn using(&mut self) -> Using {
        let start = self.start();
        self.bump();
        let mut paths = Vec::new();
        loop {
            paths.push(self.dotted("a namespace's name"));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        if !self.eat(TokenKind::Semi) {
            self.expected("';' after USING");
            self.skip_until_semi_or(&[]);
        }
        Using { paths, span: self.span_from(start) }
    }

    /// `A.B.C`: a name and the names after it, each after a `.`.
    fn dotted(&mut self, what: &str) -> Vec<Ident> {
        let mut parts = vec![self.ident(what)];
        while self.at(TokenKind::Dot) && self.nth(1) == TokenKind::Ident {
            self.bump();
            parts.push(self.ident(what));
        }
        parts
    }

    /// `A.B.C` as one name: a type's, or a program's in a CONFIGURATION,
    /// named with its namespaces (Ed.3 6.9.3).
    fn dotted_name(&mut self, what: &str) -> Ident {
        let parts = self.dotted(what);
        if parts.len() == 1 {
            return parts.into_iter().next().unwrap_or_else(|| unreachable!());
        }
        let text: Vec<&str> = parts.iter().map(|p| p.text.as_str()).collect();
        let span = Span::new(self.file, parts[0].span.start, parts[parts.len() - 1].span.end);
        Ident { text: text.join("."), span }
    }

    /// `INTERNAL` or `PUBLIC` before a POU's or namespace's name (Ed.3
    /// 6.9.2, as its examples write it): TRUE for INTERNAL. Read only
    /// there, so either stays a name anywhere else.
    fn access_spec(&mut self) -> bool {
        if !self.at(TokenKind::Ident) || self.nth(1) != TokenKind::Ident {
            return false;
        }
        let text = self.text_of(self.tok()).to_ascii_uppercase();
        if text != "INTERNAL" && text != "PUBLIC" {
            return false;
        }
        self.bump();
        text == "INTERNAL"
    }

    /// The specifiers before a name, in any order (OO.md D2): each a
    /// word read as one only where another name follows it, so all stay
    /// names elsewhere. `words` are those this place takes; INTERNAL and
    /// PUBLIC before a CLASS's or FUNCTION_BLOCK's name are its
    /// namespace's (6.9.2), returned as `internal`.
    fn specs(&mut self, method: bool) -> (Specs, bool) {
        let mut specs = Specs::default();
        let mut internal = false;
        while self.at(TokenKind::Ident) && self.nth(1) == TokenKind::Ident {
            let t = self.tok();
            let text = self.text_of(t).to_ascii_uppercase();
            let access = match text.as_str() {
                "PUBLIC" => Some(Access::Public),
                "PRIVATE" if method => Some(Access::Private),
                "PROTECTED" if method => Some(Access::Protected),
                "INTERNAL" => Some(Access::Internal),
                _ => None,
            };
            let slot = match (text.as_str(), access) {
                (_, Some(a)) if method => {
                    if specs.access.is_some() {
                        self.error_here(format!("a second access specifier, {text}: a method has one"));
                    }
                    specs.access = Some((a, t.span));
                    self.bump();
                    continue;
                }
                (_, Some(a)) => {
                    internal = a == Access::Internal;
                    self.bump();
                    continue;
                }
                ("FINAL", _) => &mut specs.final_,
                ("ABSTRACT", _) => &mut specs.abstract_,
                ("OVERRIDE", _) if method => &mut specs.override_,
                _ => break,
            };
            if slot.is_some() {
                self.error_here(format!("{text} is written twice"));
            }
            *slot = Some(t.span);
            self.bump();
        }
        (specs, internal)
    }

    /// CODESYS's `AND_THEN` and `OR_ELSE`, names to the standard, read as
    /// operators where one may follow an operand: AND's and OR's place
    /// in Table 71, as CODESYS ranks them.
    fn short_circuit_op(&self) -> Option<(BinaryOp, u8)> {
        if self.at_word(&["AND_THEN"]) {
            Some((BinaryOp::AndThen, 3))
        } else if self.at_word(&["OR_ELSE"]) {
            Some((BinaryOp::OrElse, 1))
        } else {
            None
        }
    }

    /// Whether the current token begins CODESYS's `VAR_STAT` section: the
    /// name VAR_STAT where a POU's sections are, a variable's name,
    /// CONSTANT, RETAIN, NON_RETAIN or END_VAR after it. The standard
    /// reserves no such word, so VAR_STAT stays a name elsewhere.
    fn at_var_stat(&self) -> bool {
        self.at_word(&["VAR_STAT"])
            && matches!(
                self.nth(1),
                TokenKind::Ident | TokenKind::Kw(Kw::Constant | Kw::Retain | Kw::NonRetain | Kw::EndVar)
            )
    }

    /// Whether the current token is a name spelled as one of `words`.
    fn at_word(&self, words: &[&str]) -> bool {
        self.at(TokenKind::Ident) && {
            let t = self.text_of(self.tok());
            words.iter().any(|w| t.eq_ignore_ascii_case(w))
        }
    }

    /// CODESYS's `PROPERTY [specifiers] name : T`, then `GET [VAR...]
    /// statements END_GET` and `SET ... END_SET`, either or both, then
    /// END_PROPERTY: the methods `PROPERTY_GET_name : T`, whose statements set
    /// `name` as its result, and `PROPERTY_SET_name`, with the input `name`
    /// (properties). Its place is kept for the checker.
    fn property(&mut self) -> Vec<Pou> {
        let kw = self.bump();
        let (specs, _) = self.specs(true);
        let name = self.ident("the property's name");
        self.properties.push(Span::new(self.file, kw.span.start, name.span.end));
        self.expect(TokenKind::Colon);
        let ty = self.type_spec();
        self.eat(TokenKind::Semi);
        let mut out = Vec::new();
        loop {
            if self.at_kw(Kw::EndProperty) {
                self.bump();
                break;
            }
            let get = self.at_word(&["GET"]);
            if !get && !self.at_word(&["SET"]) {
                self.expected("GET, SET or END_PROPERTY");
                self.skip_construct_quietly(Kw::EndProperty);
                break;
            }
            let acc = self.start();
            self.bump();
            let mut var_blocks = Vec::new();
            while let TokenKind::Kw(k) = self.kind() {
                if !is_var_block_start(k) {
                    break;
                }
                if let Some(b) = self.var_block() {
                    var_blocks.push(b);
                }
            }
            let end = if get { "END_GET" } else { "END_SET" };
            let saved = std::mem::replace(&mut self.stop_words, vec![end]);
            let was_in_body = std::mem::replace(&mut self.in_body, true);
            let mut body = self.stmts(&[Kw::EndProperty]);
            self.in_body = was_in_body;
            self.stop_words = saved;
            if self.at_word(&[end]) {
                self.bump();
            } else {
                self.expected(end);
            }
            let upper = name.text.to_ascii_uppercase();
            let method = |n: String, return_type, var_blocks, body| Pou {
                kind: PouKind::Method,
                name: Ident { text: n, span: name.span },
                return_type,
                var_blocks,
                accesses: Vec::new(),
                body,
                sfc: Vec::new(),
                internal: false,
                using: Vec::new(),
                specs,
                methods: Vec::new(),
                extends: None,
                interfaces: Vec::new(),
                span: Span::new(self.file, acc, self.last_end),
            };
            if get {
                // The property's name is the getter's result.
                let result = format!("PROPERTY_GET_{upper}");
                rename_in(&mut body, &upper, &result);
                out.push(method(result, Some(ty.clone()), var_blocks, body));
            } else {
                let input = VarBlock {
                    kind: VarKind::Input,
                    constant: false,
                    input_constant: None,
                    retain: None,
                    access: None,
                    stat: None,
                    decls: vec![VarDecl {
                        names: vec![name.clone()],
                        location: None,
                        ty: ty.clone(),
                        edge: None,
                        init: None,
                        fb_args: None,
                        span: name.span,
                    }],
                    span: name.span,
                };
                let mut blocks = vec![input];
                blocks.extend(var_blocks);
                out.push(method(format!("PROPERTY_SET_{upper}"), None, blocks, body));
            }
        }
        out
    }

    fn pou(&mut self, kind: PouKind) -> Pou {
        let start = self.start();
        let opener_kw = self.bump();
        let (specs, internal) = match kind {
            PouKind::FunctionBlock | PouKind::Class => self.specs(false),
            PouKind::Method => self.specs(true),
            _ => (Specs::default(), self.access_spec()),
        };
        let name = self.ident(if kind == PouKind::Method { "the method's name" } else { "the POU's name" });
        let opener = Span::new(self.file, opener_kw.span.start, name.span.end.max(opener_kw.span.end));
        let (end, what) = match kind {
            PouKind::Program => (Kw::EndProgram, "PROGRAM"),
            PouKind::Function => (Kw::EndFunction, "FUNCTION"),
            PouKind::FunctionBlock => (Kw::EndFunctionBlock, "FUNCTION_BLOCK"),
            PouKind::Class => (Kw::EndClass, "CLASS"),
            PouKind::Method => (Kw::EndMethod, "METHOD"),
            PouKind::Interface => (Kw::EndInterface, "INTERFACE"),
        };
        let mut return_type = None;
        if matches!(kind, PouKind::Function | PouKind::Method) && self.eat(TokenKind::Colon) {
            return_type = Some(self.type_spec());
        }
        let mut using = Vec::new();
        while self.at_kw(Kw::Using) {
            using.push(self.using());
        }
        let mut extends = None;
        let mut interfaces = Vec::new();
        if matches!(kind, PouKind::FunctionBlock | PouKind::Class) {
            // One base (Ed.3 6.6.5.5.2 rule 3: "Multi-inheritance is not
            // supported").
            if self.eat_kw(Kw::Extends) {
                extends = Some(self.dotted_name("the base type"));
                if self.at(TokenKind::Comma) {
                    let s = self.tok().span;
                    self.error_at(
                        s,
                        format!("a {what} extends one base type (Ed.3 6.6.5.5.2 rule 3); IMPLEMENTS takes several interfaces"),
                    );
                    while self.eat(TokenKind::Comma) {
                        self.ident("a name");
                    }
                }
            }
            // Any number of interfaces (Ed.3 6.6.6.4).
            if self.eat_kw(Kw::Implements) {
                interfaces = self.name_list("an interface's name");
            }
        }
        // An interface extends any number (Ed.3 6.6.6.3).
        if kind == PouKind::Interface && self.eat_kw(Kw::Extends) {
            interfaces = self.name_list("an interface's name");
        }
        let mut var_blocks = Vec::new();
        let mut accesses = Vec::new();
        let mut methods = Vec::new();
        loop {
            match self.kind() {
                TokenKind::Kw(Kw::VarAccess) => self.access_block(&mut accesses),
                TokenKind::Kw(kw) if is_var_block_start(kw) => {
                    if let Some(b) = self.var_block() {
                        var_blocks.push(b);
                    }
                }
                TokenKind::Ident if self.at_var_stat() => {
                    if let Some(b) = self.var_block() {
                        var_blocks.push(b);
                    }
                }
                TokenKind::Kw(Kw::Method)
                    if matches!(kind, PouKind::FunctionBlock | PouKind::Class | PouKind::Interface) =>
                {
                    methods.push(self.pou(PouKind::Method));
                }
                TokenKind::Kw(Kw::Method) => {
                    let s = self.tok().span;
                    self.error_at(s, format!("a METHOD belongs to a CLASS or a FUNCTION_BLOCK, not a {what}"));
                    self.skip_construct_quietly(Kw::EndMethod);
                }
                // CODESYS's PROPERTY (properties): its GET and SET as methods.
                TokenKind::Ident
                    if matches!(kind, PouKind::FunctionBlock | PouKind::Class | PouKind::Interface)
                        && self.at_word(&["PROPERTY"])
                        && self.nth(1) == TokenKind::Ident =>
                {
                    let ps = self.property();
                    methods.extend(ps);
                }
                TokenKind::Kw(Kw::PropertyGet | Kw::PropertySet) => {
                    self.skip_construct(Kw::EndProperty, "a property (Ed.4)")
                }
                _ => break,
            }
        }
        // A method's body is a body too, inside its type's declarations.
        let was_in_body = self.in_body;
        self.in_body = true;
        let body = if matches!(kind, PouKind::Class | PouKind::Interface) {
            // Ed.3 6.6.5.2: "A class has no body. A class may define only
            // methods."; an interface only prototypes (6.6.6.1).
            if !self.at_kw(end) && !self.at_eof() {
                let s = self.tok().span;
                let why = if kind == PouKind::Class {
                    "a CLASS has no body: only its variables and methods (Ed.3 6.6.5.2)"
                } else {
                    "an INTERFACE has no body: only its methods' prototypes (Ed.3 6.6.6.1)"
                };
                self.error_at(s, why.to_string());
                self.stmts(&[end]);
            }
            Vec::new()
        } else {
            self.stmts(&[end])
        };
        let mut sfc = Vec::new();
        loop {
            match self.kind() {
                TokenKind::Kw(Kw::Step | Kw::InitialStep) if !matches!(kind, PouKind::Method | PouKind::Interface) => {
                    sfc.push(SfcElement::Step(self.step()))
                }
                TokenKind::Kw(Kw::Transition) if !matches!(kind, PouKind::Method | PouKind::Interface) => {
                    sfc.push(SfcElement::Transition(self.transition()))
                }
                TokenKind::Kw(Kw::Action) if !matches!(kind, PouKind::Method | PouKind::Interface) => {
                    sfc.push(SfcElement::Action(self.action()))
                }
                TokenKind::Kw(Kw::Method) if kind == PouKind::FunctionBlock => {
                    let s = self.tok().span;
                    self.error_at(
                        s,
                        "a METHOD comes before the function block's body, after its declarations (Ed.3 6.6.7.2.3)"
                            .to_string(),
                    );
                    methods.push(self.pou(PouKind::Method));
                }
                _ => break,
            }
        }
        // A body is in one language (Annex A, FB_Body: SFC | ... |
        // Stmt_List), and an ACTION is part of an SFC (Sfc_Network).
        if let (false, Some(el)) = (body.is_empty(), sfc.first()) {
            let at = match el {
                SfcElement::Step(x) => x.span,
                SfcElement::Transition(x) => x.span,
                SfcElement::Action(x) => x.span,
            };
            self.diags.push(Diagnostic::new(
                Code::UnexpectedToken,
                at,
                "a body is statements or an SFC, not both (Ed.3 Annex A, FB_Body; an ACTION is a part of an SFC): \
                 put the statements in an action of the SFC",
            ));
        }
        self.in_body = was_in_body;
        let name_text = if name.text.is_empty() { String::new() } else { format!(" {}", name.text) };
        self.close(end, opener, &format!("{what}{name_text}"));
        Pou {
            kind,
            name,
            return_type,
            var_blocks,
            accesses,
            body,
            sfc,
            internal,
            using,
            specs,
            methods,
            extends,
            interfaces,
            span: self.span_from(start),
        }
    }

    /// `a, B.c, d`: names, each with its namespaces.
    fn name_list(&mut self, what: &str) -> Vec<Ident> {
        let mut out = vec![self.dotted_name(what)];
        while self.eat(TokenKind::Comma) {
            out.push(self.dotted_name(what));
        }
        out
    }

    // ---- declarations ----------------------------------------------

    fn var_block(&mut self) -> Option<VarBlock> {
        let start = self.start();
        let kw_tok = self.bump();
        // CODESYS's VAR_STAT, a name to the standard (see at_var_stat).
        let stat = (kw_tok.kind == TokenKind::Ident).then_some(kw_tok.span);
        let kw = match kw_tok.kind {
            TokenKind::Kw(kw) => kw,
            _ if stat.is_some() => Kw::Var,
            _ => unreachable!("var_block at a keyword"),
        };
        let what = if stat.is_some() { "VAR_STAT" } else { kw.as_str() };
        let Some(kind) = var_kind(kw) else {
            self.not_yet(kw_tok.span, kw.as_str());
            while !self.at_eof() && !self.at_kw(Kw::EndVar) {
                self.bump();
            }
            self.eat_kw(Kw::EndVar);
            return None;
        };
        let mut constant = false;
        let mut constant_at = None;
        let mut retain = None;
        let mut access = None;
        loop {
            if let Some(a) = self.var_access() {
                if access.is_some() {
                    self.error_at(a.1, "a second access specifier: a VAR section has one".to_string());
                }
                access = Some(a);
            } else if self.at_kw(Kw::Constant) {
                constant_at = Some(self.bump().span);
                constant = true;
            } else if self.eat_kw(Kw::Retain) {
                retain = Some(Retain::Retain);
            } else if self.eat_kw(Kw::NonRetain) {
                retain = Some(Retain::NonRetain);
            } else {
                break;
            }
        }
        let mut decls = Vec::new();
        // An empty block is legal from Ed.3 on (Annex A, `Var_Decls`).
        while !self.at_eof() && !self.at_kw(Kw::EndVar) {
            if self.keyword_as_name() {
                continue;
            }
            if let TokenKind::Kw(k) = self.kind()
                && is_structural(k)
            {
                break;
            }
            let before = self.pos;
            if let Some(d) = self.var_decl(&[Kw::EndVar]) {
                decls.push(d);
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.close(Kw::EndVar, kw_tok.span, what);
        let input_constant = if kind == VarKind::Input { constant_at } else { None };
        let constant = constant && input_constant.is_none();
        Some(VarBlock { kind, constant, input_constant, retain, access, stat, decls, span: self.span_from(start) })
    }

    /// PUBLIC, PRIVATE, PROTECTED or INTERNAL after `VAR` (Ed.3
    /// 6.6.5.10), among its other specifiers: a word read as one only
    /// where no `:`, `,` or `AT` follows it, so a variable may still have
    /// that name.
    fn var_access(&mut self) -> Option<(Access, Span)> {
        if !self.at(TokenKind::Ident)
            || matches!(self.nth(1), TokenKind::Colon | TokenKind::Comma | TokenKind::Kw(Kw::At))
        {
            return None;
        }
        let a = match self.text_of(self.tok()).to_ascii_uppercase().as_str() {
            "PUBLIC" => Access::Public,
            "PRIVATE" => Access::Private,
            "PROTECTED" => Access::Protected,
            "INTERNAL" => Access::Internal,
            _ => return None,
        };
        Some((a, self.bump().span))
    }

    /// `a, b AT %IX0.0 : TYPE R_EDGE := init;` - and the same shape for a
    /// STRUCT's fields.
    fn var_decl(&mut self, end: &[Kw]) -> Option<VarDecl> {
        let start = self.start();
        let mut names = Vec::new();
        if !self.at_kw(Kw::At) {
            if !self.at(TokenKind::Ident) {
                self.expected("a variable name");
                self.skip_until_semi_or(end);
                return None;
            }
            loop {
                names.push(self.ident("a variable name"));
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }
        let mut location = None;
        if self.eat_kw(Kw::At) {
            // A relative location, `%B2`, is a structure field's place in
            // its structure (Ed.3 6.4.4.7); the checker says where it may be.
            if self.at(TokenKind::DirectAddress) || self.at(TokenKind::RelativeAddress) {
                let t = self.bump();
                location = Some(DirectAddress { text: self.text_of(t).to_string(), span: t.span });
            } else {
                self.expected("a direct address such as %IX0.0");
            }
        }
        if !self.expect(TokenKind::Colon) {
            self.skip_until_semi_or(end);
            return None;
        }
        self.fb_args = None;
        let ty = self.type_spec();
        let edge = if self.eat_kw(Kw::REdge) {
            Some(Edge::Rising)
        } else if self.eat_kw(Kw::FEdge) {
            Some(Edge::Falling)
        } else {
            None
        };
        let init = if self.eat(TokenKind::Assign) { Some(self.decl_init()) } else { None };
        if !self.eat(TokenKind::Semi) {
            self.expected("';' after the declaration");
            self.skip_until_semi_or(end);
        }
        let fb_args = self.fb_args.take();
        Some(VarDecl { names, location, ty, edge, init, fb_args, span: self.span_from(start) })
    }

    fn type_spec(&mut self) -> TypeSpec {
        let start = self.start();
        match self.kind() {
            TokenKind::Kw(Kw::Array) => {
                self.bump();
                self.expect(TokenKind::LBracket);
                let mut dims = Vec::new();
                loop {
                    if self.eat(TokenKind::Star) {
                        dims.push(None);
                    } else {
                        let low = self.expr();
                        self.expect(TokenKind::DotDot);
                        let high = self.expr();
                        dims.push(Some((low, high)));
                    }
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RBracket);
                self.expect_kw(Kw::Of);
                let elem = Box::new(self.type_spec());
                TypeSpec::Array { dims, elem, span: self.span_from(start) }
            }
            TokenKind::Kw(Kw::RefTo) => {
                self.bump();
                let target = Box::new(self.type_spec());
                TypeSpec::Ref { target, implicit: false, span: self.span_from(start) }
            }
            TokenKind::LParen => self.enum_spec(None, start),
            // CODESYS's REFERENCE TO; the checker says it needs its extension.
            TokenKind::Ident
                if self.text_of(self.tok()).eq_ignore_ascii_case("REFERENCE")
                    && self.nth(1) == TokenKind::Kw(Kw::To) =>
            {
                self.bump();
                self.bump();
                let target = Box::new(self.type_spec());
                TypeSpec::Ref { target, implicit: true, span: self.span_from(start) }
            }
            // CODESYS's POINTER TO; the checker says it needs its extension.
            TokenKind::Ident
                if self.text_of(self.tok()).eq_ignore_ascii_case("POINTER") && self.nth(1) == TokenKind::Kw(Kw::To) =>
            {
                self.bump();
                self.bump();
                let target = Box::new(self.type_spec());
                TypeSpec::Pointer { target, span: self.span_from(start) }
            }
            TokenKind::Ident => {
                let name = self.dotted_name("a type");
                let is_string = ["STRING", "WSTRING", "USTRING"].iter().any(|s| name.is(s));
                if is_string {
                    let close = match self.kind() {
                        TokenKind::LBracket => Some(TokenKind::RBracket),
                        TokenKind::LParen => Some(TokenKind::RParen),
                        _ => None,
                    };
                    let parens = close == Some(TokenKind::RParen);
                    let len = close.map(|close| {
                        self.bump();
                        let e = self.expr();
                        self.expect(close);
                        Box::new(e)
                    });
                    return TypeSpec::String { name, len, parens, span: self.span_from(start) };
                }
                // `fb : T(args)`, CODESYS's FB_init arguments (fb-init): a
                // type that is no elementary type, which alone take a
                // subrange or named values. Kept for the declaration.
                // Every elementary type's: a subrange's or named values'
                // base, refused there if it is no integer (E0013).
                let int_like = [
                    "SINT",
                    "INT",
                    "DINT",
                    "LINT",
                    "USINT",
                    "UINT",
                    "UDINT",
                    "ULINT",
                    "BYTE",
                    "WORD",
                    "DWORD",
                    "LWORD",
                    "BOOL",
                    "REAL",
                    "LREAL",
                    "TIME",
                    "LTIME",
                    "DATE",
                    "LDATE",
                    "TIME_OF_DAY",
                    "TOD",
                    "LTIME_OF_DAY",
                    "LTOD",
                    "DATE_AND_TIME",
                    "DT",
                    "LDATE_AND_TIME",
                    "LDT",
                    "CHAR",
                    "WCHAR",
                ]
                .iter()
                .any(|t| name.is(t));
                if self.at(TokenKind::LParen) && !int_like {
                    let open = self.bump();
                    let args = self.args();
                    self.fb_args = Some((args, Span::new(self.file, open.span.start, self.last_end)));
                    return TypeSpec::Named(name);
                }
                if self.at(TokenKind::LParen) {
                    let enum_like = self.nth(1) == TokenKind::Ident
                        && matches!(self.nth(2), TokenKind::Assign | TokenKind::Comma | TokenKind::RParen);
                    if enum_like {
                        return self.enum_spec(Some(name), start);
                    }
                    self.bump();
                    let low = Box::new(self.expr());
                    self.expect(TokenKind::DotDot);
                    let high = Box::new(self.expr());
                    self.expect(TokenKind::RParen);
                    return TypeSpec::Subrange { base: name, low, high, span: self.span_from(start) };
                }
                TypeSpec::Named(name)
            }
            _ => {
                self.expected("a type");
                TypeSpec::Named(Ident { text: String::new(), span: Span::new(self.file, start, start) })
            }
        }
    }

    fn enum_spec(&mut self, base: Option<Ident>, start: u32) -> TypeSpec {
        self.expect(TokenKind::LParen);
        let mut values = Vec::new();
        loop {
            let name = self.ident("an enumerated value");
            let value = if self.eat(TokenKind::Assign) { Some(self.expr()) } else { None };
            values.push(EnumValue { name, value });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen);
        // CODESYS's base type after the values, `(a, b) DWORD`: an
        // integer or bit string type's name, which nothing else here
        // starts with (the checker allows it with codesys-enums).
        let after = base.is_none()
            && self.at(TokenKind::Ident)
            && ["INT", "UINT", "SINT", "USINT", "DINT", "UDINT", "LINT", "ULINT", "BYTE", "WORD", "DWORD", "LWORD"]
                .iter()
                .any(|t| self.text_of(self.tok()).eq_ignore_ascii_case(t));
        let base = if after { Some(self.ident("a base type")) } else { base };
        TypeSpec::Enum { base, values, base_after: after, span: self.span_from(start) }
    }

    fn init(&mut self) -> Init {
        let start = self.start();
        if self.eat(TokenKind::LBracket) {
            let elems = if self.at(TokenKind::RBracket) { Vec::new() } else { self.array_elems() };
            self.expect(TokenKind::RBracket);
            return Init::Array { elems, bare: false, span: self.span_from(start) };
        }
        if self.at(TokenKind::LParen) && self.nth(1) == TokenKind::Ident && self.nth(2) == TokenKind::Assign {
            self.bump();
            let fields = self.init_fields();
            self.expect(TokenKind::RParen);
            return Init::Struct { fields, span: self.span_from(start) };
        }
        Init::Expr(self.expr())
    }

    /// An array's initial values, `1, 2, 3(0)`, up to what ends the list.
    fn array_elems(&mut self) -> Vec<ArrayInitElem> {
        let mut elems = Vec::new();
        loop {
            if matches!(self.kind(), TokenKind::Int) && self.nth(1) == TokenKind::LParen {
                let repeat = self.primary();
                let open = self.start();
                self.bump();
                let value = if self.at(TokenKind::RParen) {
                    None
                } else if self.at(TokenKind::Ident) && self.nth(1) == TokenKind::Assign {
                    // `50(PT := T#100ms)`, as Ed.3 Table 11 row 5b
                    // writes a repeated structure's or instance's
                    // value: the repeat's parentheses are its own.
                    let fields = self.init_fields();
                    Some(Init::Struct { fields, span: self.span_from(open) })
                } else {
                    Some(self.init())
                };
                // A list repeated, `2(1, 2, 3)`: Annex A's
                // Array_Elem_Init has one value, 6.4.4.5.2 "a
                // list which may use parentheses to express
                // repetitions", and its example this.
                let mut more = Vec::new();
                while value.is_some() && self.eat(TokenKind::Comma) {
                    more.push(self.init());
                }
                self.expect(TokenKind::RParen);
                elems.push(ArrayInitElem { repeat: Some(repeat), value, more });
            } else {
                elems.push(ArrayInitElem { repeat: None, value: Some(self.init()), more: Vec::new() });
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        elems
    }

    /// A declaration's initial value after `:=`. Without brackets, a list
    /// of an array's values, `1, 2, 3` or `3(7)`, is CODESYS 2.3's way:
    /// an `Init::Array` marked `bare`, which the checker accepts only
    /// with the bare-array-init extension. A `,` after a value, or a
    /// count before `(`, cannot start anything else there.
    fn decl_init(&mut self) -> Init {
        let start = self.start();
        if matches!(self.kind(), TokenKind::Int) && self.nth(1) == TokenKind::LParen {
            let elems = self.array_elems();
            return Init::Array { elems, bare: true, span: self.span_from(start) };
        }
        let first = self.init();
        if !self.at(TokenKind::Comma) {
            return first;
        }
        let mut elems = vec![ArrayInitElem { repeat: None, value: Some(first), more: Vec::new() }];
        self.bump();
        elems.extend(self.array_elems());
        Init::Array { elems, bare: true, span: self.span_from(start) }
    }

    /// `x := 1, y := 2` of a structure's initial value, up to its `)`.
    fn init_fields(&mut self) -> Vec<(Ident, Init)> {
        let mut fields = Vec::new();
        loop {
            let name = self.ident("a field name");
            self.expect(TokenKind::Assign);
            fields.push((name, self.init()));
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        fields
    }

    fn types(&mut self) -> Vec<TypeDecl> {
        let opener = self.bump().span;
        let mut decls = Vec::new();
        // CODESYS's `TYPE INTERNAL X :`, `TYPE PUBLIC X :`: the section's.
        let spec_at = self.tok().span;
        let before = self.pos;
        let internal = self.access_spec();
        let access = (self.pos != before).then_some((internal, spec_at));
        while !self.at_eof() && !self.at_kw(Kw::EndType) {
            if let TokenKind::Kw(k) = self.kind()
                && is_structural(k)
            {
                break;
            }
            let before = self.pos;
            let start = self.start();
            if !self.at(TokenKind::Ident) {
                self.expected("a type name");
                self.skip_until_semi_or(&[Kw::EndType]);
            } else {
                let name = self.ident("a type name");
                // CODESYS's structure extended (struct-extends): its base.
                let extends =
                    if self.eat_kw(Kw::Extends) { Some(self.dotted_name("the base structure")) } else { None };
                if self.expect(TokenKind::Colon) {
                    let def = if self.at_kw(Kw::Struct) {
                        let s = self.start();
                        let struct_kw = self.bump().span;
                        let overlap = self.eat_kw(Kw::Overlap);
                        let mut fields = Vec::new();
                        while !self.at_eof() && !self.at_kw(Kw::EndStruct) {
                            if self.keyword_as_name() {
                                continue;
                            }
                            if let TokenKind::Kw(k) = self.kind()
                                && is_structural(k)
                            {
                                break;
                            }
                            let b = self.pos;
                            if let Some(f) = self.var_decl(&[Kw::EndStruct]) {
                                fields.push(f);
                            }
                            if self.pos == b {
                                self.bump();
                            }
                        }
                        self.close(Kw::EndStruct, struct_kw, "STRUCT");
                        TypeDef::Struct { overlap, fields, span: self.span_from(s) }
                    } else {
                        TypeDef::Spec(self.type_spec())
                    };
                    let init = if self.eat(TokenKind::Assign) { Some(self.decl_init()) } else { None };
                    let mut no_semi = None;
                    if !self.eat(TokenKind::Semi) {
                        // `END_STRUCT END_TYPE`, or `END_STRUCT` and the
                        // next type: CODESYS's and TwinCAT's way, recorded
                        // for the checker (struct-semicolon).
                        let next_decl = self.at(TokenKind::Ident) && self.nth(1) == TokenKind::Colon;
                        if matches!(def, TypeDef::Struct { .. })
                            && init.is_none()
                            && (self.at_kw(Kw::EndType) || next_decl)
                        {
                            let at = self.tok().span.start;
                            no_semi = Some(Span::new(self.file, at, at));
                        } else {
                            self.expected("';' after the type declaration");
                            self.skip_until_semi_or(&[Kw::EndType]);
                        }
                    }
                    decls.push(TypeDecl { name, access, extends, def, init, no_semi, span: self.span_from(start) });
                } else {
                    self.skip_until_semi_or(&[Kw::EndType]);
                }
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.close(Kw::EndType, opener, "TYPE");
        decls
    }

    // ---- statements ------------------------------------------------

    /// Statements until one of `stop`, a structural keyword, or the end.
    fn stmts(&mut self, stop: &[Kw]) -> Vec<Stmt> {
        let mut out = Vec::new();
        loop {
            if self.keyword_as_name() {
                continue;
            }
            match self.kind() {
                TokenKind::Eof => break,
                TokenKind::Kw(kw) if stop.contains(&kw) || is_structural(kw) => break,
                TokenKind::Ident if self.at_word(&self.stop_words.clone()) => break,
                _ => {}
            }
            let before = self.pos;
            if let Some(s) = self.stmt() {
                out.push(s);
                out.append(&mut self.chain_rest);
            }
            if self.pos == before {
                self.bump();
            }
        }
        out
    }

    fn can_start_stmt(&self) -> bool {
        matches!(
            self.kind(),
            TokenKind::Ident
                | TokenKind::DirectAddress
                | TokenKind::Semi
                | TokenKind::Kw(
                    Kw::If | Kw::Case | Kw::For | Kw::While | Kw::Repeat | Kw::Exit | Kw::Continue | Kw::Return
                )
        )
    }

    fn end_stmt(&mut self) {
        if !self.eat(TokenKind::Semi) {
            self.expected("';' after the statement");
        }
    }

    /// `a := b := c := 0;` (CODESYS, TwinCAT, Siemens): assigned right
    /// to left, `c := 0; b := c; a := b;`, each inner target read back,
    /// so it has its own type. Its place kept for the checker
    /// (chained-assignment); an inner target with a call in it is
    /// refused, as reading it back would call again.
    fn chained_assignment(&mut self, start: u32, first: Expr, second: Expr) -> Option<Stmt> {
        let mut targets = vec![first, second];
        let mut value = Expr { kind: ExprKind::Error, span: Span::new(self.file, self.last_end, self.last_end) };
        while self.eat(TokenKind::Assign) {
            value = self.expr();
            if self.at(TokenKind::Assign) {
                targets.push(value.clone());
            }
        }
        self.end_stmt();
        let span = self.span_from(start);
        self.chained.push(span);
        for t in &targets[1..] {
            if has_call(t) {
                self.diags.push(Diagnostic::new(
                    Code::UnexpectedToken,
                    t.span,
                    "a chained assignment reads each inner target back: one with a call in it would call again; \
                     assign it on its own",
                ));
            }
        }
        // The innermost first: `c := 0;`, then each target from the one
        // inside it.
        let mut stmts = Vec::new();
        let mut from = value;
        for t in targets.into_iter().rev() {
            let at = Span::new(self.file, t.span.start, span.end);
            let next = t.clone();
            stmts.push(Stmt { kind: StmtKind::Assign { target: t, value: from }, span: at });
            from = next;
        }
        let first = stmts.remove(0);
        self.chain_rest = stmts;
        Some(first)
    }

    fn stmt(&mut self) -> Option<Stmt> {
        if !self.can_start_stmt() {
            self.expected("a statement");
            self.skip_until_semi_or(&[]);
            return None;
        }
        let start = self.start();
        // `name:` and `JMP name;` (CODESYS's, the jump-labels extension;
        // the checker says it needs it). In a CASE's body `name:` is a
        // case label, read before this.
        if self.at(TokenKind::Ident) && self.nth(1) == TokenKind::Colon {
            let id = self.ident("a label");
            self.bump();
            return Some(Stmt { kind: StmtKind::Label(id), span: self.span_from(start) });
        }
        if self.at(TokenKind::Ident)
            && self.text_of(self.tok()).eq_ignore_ascii_case("JMP")
            && self.nth(1) == TokenKind::Ident
        {
            self.bump();
            let id = self.ident("a label");
            self.end_stmt();
            return Some(Stmt { kind: StmtKind::Jmp(id), span: self.span_from(start) });
        }
        let kind = match self.kind() {
            TokenKind::Semi => {
                self.bump();
                return Some(Stmt { kind: StmtKind::Empty, span: self.span_from(start) });
            }
            TokenKind::Kw(Kw::If) => self.if_stmt(),
            TokenKind::Kw(Kw::Case) => self.case_stmt(),
            TokenKind::Kw(Kw::For) => {
                let opener = self.bump().span;
                let var = self.ident("the loop variable");
                self.expect(TokenKind::Assign);
                let from = self.expr();
                self.expect_kw(Kw::To);
                let to = self.expr();
                let by = if self.eat_kw(Kw::By) { Some(self.expr()) } else { None };
                self.expect_kw(Kw::Do);
                let body = self.stmts(&[Kw::EndFor]);
                self.close(Kw::EndFor, opener, "FOR");
                StmtKind::For { var, from, to, by, body }
            }
            TokenKind::Kw(Kw::While) => {
                let opener = self.bump().span;
                let cond = self.expr();
                self.expect_kw(Kw::Do);
                let body = self.stmts(&[Kw::EndWhile]);
                self.close(Kw::EndWhile, opener, "WHILE");
                StmtKind::While { cond, body }
            }
            TokenKind::Kw(Kw::Repeat) => {
                let opener = self.bump().span;
                let body = self.stmts(&[Kw::Until]);
                let until = if self.eat_kw(Kw::Until) {
                    self.expr()
                } else {
                    self.close(Kw::Until, opener, "REPEAT");
                    Expr { kind: ExprKind::Error, span: Span::new(self.file, self.last_end, self.last_end) }
                };
                self.close(Kw::EndRepeat, opener, "REPEAT");
                StmtKind::Repeat { body, until }
            }
            TokenKind::Kw(Kw::Exit) => {
                self.bump();
                StmtKind::Exit
            }
            TokenKind::Kw(Kw::Continue) => {
                self.bump();
                StmtKind::Continue
            }
            TokenKind::Kw(Kw::Return) => {
                self.bump();
                StmtKind::Return
            }
            _ => {
                let target = self.expr();
                if self.eat(TokenKind::Assign) {
                    let value = self.expr();
                    if self.at(TokenKind::Assign) {
                        return self.chained_assignment(start, target, value);
                    }
                    StmtKind::Assign { target, value }
                } else if self.eat(TokenKind::AssignAttempt) {
                    let value = self.expr();
                    StmtKind::AssignAttempt { target, value }
                } else if self.at(TokenKind::Ident)
                    && self.text_of(self.tok()).eq_ignore_ascii_case("REF")
                    && self.nth(1) == TokenKind::Eq
                {
                    // CODESYS's `r REF= x`; the checker says it needs its extension.
                    self.bump();
                    self.bump();
                    let value = self.expr();
                    StmtKind::RefAssign { target, value }
                } else if matches!(target.kind, ExprKind::Call { .. }) {
                    StmtKind::Call(target)
                } else {
                    let message = if matches!(target.kind, ExprKind::Binary(BinaryOp::Eq, ..)) {
                        "'=' compares; to assign, write ':='".to_string()
                    } else if matches!(target.kind, ExprKind::Error) {
                        String::new()
                    } else {
                        "an expression on its own is not a statement: assign it with ':=', or call a function or block"
                            .to_string()
                    };
                    if !message.is_empty() && self.last_error_at != Some(self.pos) {
                        self.last_error_at = Some(self.pos);
                        self.diags.push(Diagnostic::new(Code::UnexpectedToken, target.span, message));
                    }
                    self.skip_until_semi_or(&[]);
                    return None;
                }
            }
        };
        // END_IF and its kin with no `;` (CODESYS, TwinCAT): kept for the
        // checker, which takes it with optional-semicolons.
        let structured = matches!(
            kind,
            StmtKind::If { .. }
                | StmtKind::Case { .. }
                | StmtKind::For { .. }
                | StmtKind::While { .. }
                | StmtKind::Repeat { .. }
        );
        if structured && !self.at(TokenKind::Semi) && self.last_error_at.is_none_or(|e| e < self.pos) {
            self.missing_semis.push(Span::new(self.file, self.last_end, self.last_end));
        } else {
            self.end_stmt();
        }
        Some(Stmt { kind, span: self.span_from(start) })
    }

    fn if_stmt(&mut self) -> StmtKind {
        let opener = self.bump().span;
        let mut branches = Vec::new();
        let cond = self.expr();
        self.expect_kw(Kw::Then);
        let body = self.stmts(&[Kw::Elsif, Kw::Else, Kw::EndIf]);
        branches.push((cond, body));
        while self.eat_kw(Kw::Elsif) {
            let cond = self.expr();
            self.expect_kw(Kw::Then);
            let body = self.stmts(&[Kw::Elsif, Kw::Else, Kw::EndIf]);
            branches.push((cond, body));
        }
        let else_body = if self.eat_kw(Kw::Else) { Some(self.stmts(&[Kw::EndIf])) } else { None };
        self.close(Kw::EndIf, opener, "IF");
        StmtKind::If { branches, else_body }
    }

    fn case_stmt(&mut self) -> StmtKind {
        let opener = self.bump().span;
        let selector = self.expr();
        self.expect_kw(Kw::Of);
        let mut arms = Vec::new();
        while !self.at_eof() && !self.at_kw(Kw::Else) && !self.at_kw(Kw::EndCase) {
            if let TokenKind::Kw(k) = self.kind()
                && is_structural(k)
            {
                break;
            }
            let before = self.pos;
            let labels = self.case_labels();
            if !self.expect(TokenKind::Colon) {
                self.skip_until_semi_or(&[Kw::EndCase]);
            }
            let mut body = Vec::new();
            loop {
                match self.kind() {
                    TokenKind::Eof => break,
                    TokenKind::Kw(kw) if kw == Kw::Else || kw == Kw::EndCase || is_structural(kw) => break,
                    _ => {}
                }
                if self.at_case_label() {
                    break;
                }
                let b = self.pos;
                if let Some(s) = self.stmt() {
                    body.push(s);
                    body.append(&mut self.chain_rest);
                }
                if self.pos == b {
                    self.bump();
                }
            }
            arms.push(CaseArm { labels, body });
            if self.pos == before {
                self.bump();
            }
        }
        // Ed.3 Case_Stmt: 'CASE' Expression 'OF' Case_Selection + ...
        if arms.is_empty() && (self.at_kw(Kw::Else) || self.at_kw(Kw::EndCase)) {
            let span = self.tok().span;
            self.error_at(
                span,
                "a CASE has at least one list of values before ELSE or END_CASE (Ed.3 7.3.3.3.3)".to_string(),
            );
        }
        let else_body = if self.eat_kw(Kw::Else) { Some(self.stmts(&[Kw::EndCase])) } else { None };
        self.close(Kw::EndCase, opener, "CASE");
        StmtKind::Case { selector, arms, else_body }
    }

    fn case_labels(&mut self) -> Vec<CaseLabel> {
        let mut labels = Vec::new();
        loop {
            let low = self.expr();
            if self.eat(TokenKind::DotDot) {
                let high = self.expr();
                labels.push(CaseLabel::Range(low, high));
            } else {
                labels.push(CaseLabel::Value(low));
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        labels
    }

    /// Does a CASE label start here (`1, 2..5:`, `Red:`)? Read ahead,
    /// then put everything back: cursor and diagnostics.
    fn at_case_label(&mut self) -> bool {
        if !matches!(
            self.kind(),
            TokenKind::Int
                | TokenKind::Based
                | TokenKind::Real
                | TokenKind::Time
                | TokenKind::String
                | TokenKind::WString
                | TokenKind::Minus
                | TokenKind::Plus
                | TokenKind::Ident
                | TokenKind::Kw(Kw::True | Kw::False)
        ) {
            return false;
        }
        let (pos, last_end, n, last_error_at) = (self.pos, self.last_end, self.diags.len(), self.last_error_at);
        self.case_labels();
        let is_label = self.at(TokenKind::Colon);
        self.pos = pos;
        self.last_end = last_end;
        self.diags.truncate(n);
        self.last_error_at = last_error_at;
        is_label
    }

    // ---- expressions -----------------------------------------------

    fn expr(&mut self) -> Expr {
        self.binary(1)
    }

    fn binary(&mut self, min: u8) -> Expr {
        let mut lhs = self.unary();
        while let Some((op, prec)) = binary_op(self.kind()).or_else(|| self.short_circuit_op()) {
            if prec < min {
                break;
            }
            self.bump();
            let rhs = self.binary(prec + 1);
            let span = lhs.span.to(rhs.span);
            lhs = Expr { kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)), span };
        }
        lhs
    }

    fn unary(&mut self) -> Expr {
        let op = match self.kind() {
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Plus => UnaryOp::Plus,
            TokenKind::Kw(Kw::Not) => UnaryOp::Not,
            _ => return self.postfix(),
        };
        let start = self.start();
        self.bump();
        let operand = self.unary();
        Expr { kind: ExprKind::Unary(op, Box::new(operand)), span: self.span_from(start) }
    }

    fn postfix(&mut self) -> Expr {
        let start = self.start();
        let mut e = self.primary();
        if !matches!(e.kind, ExprKind::Name(_) | ExprKind::DirectAddress(_) | ExprKind::Paren(_)) {
            return e;
        }
        loop {
            match self.kind() {
                TokenKind::Dot => {
                    self.bump();
                    // `w.%B1`, `w.%X3`: a part of a bit string (Ed.3 Table
                    // 17); the literal's text keeps the size letter.
                    if self.at(TokenKind::RelativeAddress) {
                        let t = self.bump();
                        let text = self.text_of(t).to_string();
                        if text.contains('.') {
                            self.error_at(
                                t.span,
                                format!(
                                    "'{text}' is no part of a bit string: '%', X, B, W, D or L, then a number, as w.%B1"
                                ),
                            );
                        }
                        let lit =
                            Literal { kind: LiteralKind::Int, type_prefix: None, negative: false, text, span: t.span };
                        e = Expr { kind: ExprKind::BitAccess(Box::new(e), lit), span: self.span_from(start) };
                    } else if self.at(TokenKind::Int) {
                        let t = self.bump();
                        let lit = Literal {
                            kind: LiteralKind::Int,
                            type_prefix: None,
                            negative: false,
                            text: self.text_of(t).to_string(),
                            span: t.span,
                        };
                        e = Expr { kind: ExprKind::BitAccess(Box::new(e), lit), span: self.span_from(start) };
                    } else {
                        let member = self.ident("a member name");
                        e = Expr { kind: ExprKind::Member(Box::new(e), member), span: self.span_from(start) };
                    }
                }
                TokenKind::LBracket => {
                    self.bump();
                    let mut index = vec![self.expr()];
                    while self.eat(TokenKind::Comma) {
                        index.push(self.expr());
                    }
                    self.expect(TokenKind::RBracket);
                    e = Expr { kind: ExprKind::Index(Box::new(e), index), span: self.span_from(start) };
                }
                TokenKind::Caret => {
                    self.bump();
                    e = Expr { kind: ExprKind::Deref(Box::new(e)), span: self.span_from(start) };
                }
                TokenKind::LParen
                    if matches!(
                        e.kind,
                        ExprKind::Name(_) | ExprKind::Member(..) | ExprKind::Index(..) | ExprKind::Deref(_)
                    ) =>
                {
                    self.bump();
                    let args = self.args();
                    // `fb WITH task` belongs to a program configuration.
                    for (fb, _) in std::mem::take(&mut self.fb_tasks) {
                        self.error_at(fb.span, format!("expected an argument, found '{} WITH': a task is given to a function block instance in a CONFIGURATION's program configuration (Ed.3 6.8.2)", fb.text));
                    }
                    e = Expr { kind: ExprKind::Call { callee: Box::new(e), args }, span: self.span_from(start) };
                }
                _ => return e,
            }
        }
    }

    /// Arguments after `(`, through `)`.
    fn args(&mut self) -> Vec<Arg> {
        let mut args = Vec::new();
        if self.eat(TokenKind::RParen) {
            return args;
        }
        loop {
            let named = self.at(TokenKind::Ident);
            if named && self.nth(1) == TokenKind::Kw(Kw::With) {
                // `fb1 WITH t2`, a program configuration's FB_Task.
                let fb = self.ident("a function block instance");
                self.bump();
                let task = self.ident("a task name");
                self.fb_tasks.push((fb, task));
            } else if named && self.nth(1) == TokenKind::Assign {
                let name = self.ident("a parameter name");
                self.bump();
                args.push(Arg::Input(name, self.expr()));
            } else if named && self.nth(1) == TokenKind::Arrow {
                let name = self.ident("a parameter name");
                self.bump();
                args.push(Arg::Output { not: false, name, target: self.expr() });
            } else if self.at_kw(Kw::Not) && self.nth(1) == TokenKind::Ident && self.nth(2) == TokenKind::Arrow {
                self.bump();
                let name = self.ident("a parameter name");
                self.bump();
                args.push(Arg::Output { not: true, name, target: self.expr() });
            } else {
                args.push(Arg::Positional(self.expr()));
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen);
        args
    }

    fn literal(&mut self, kind: LiteralKind, type_prefix: Option<Ident>, negative: bool, start: u32) -> Expr {
        let t = self.bump();
        let lit =
            Literal { kind, type_prefix, negative, text: self.text_of(t).to_string(), span: self.span_from(start) };
        Expr { span: lit.span, kind: ExprKind::Literal(lit) }
    }

    fn primary(&mut self) -> Expr {
        let start = self.start();
        match self.kind() {
            TokenKind::Int => self.literal(LiteralKind::Int, None, false, start),
            TokenKind::Real => self.literal(LiteralKind::Real, None, false, start),
            TokenKind::Based => self.literal(LiteralKind::Based, None, false, start),
            TokenKind::Time => self.literal(LiteralKind::Time, None, false, start),
            TokenKind::String => self.literal(LiteralKind::String, None, false, start),
            TokenKind::WString => self.literal(LiteralKind::WString, None, false, start),
            TokenKind::Kw(Kw::True | Kw::False) => self.literal(LiteralKind::Bool, None, false, start),
            TokenKind::Ident if self.nth(1) == TokenKind::Hash => self.typed_literal(start),
            TokenKind::Ident => {
                let id = self.ident("a name");
                Expr { span: id.span, kind: ExprKind::Name(id) }
            }
            // `AND(a, b)`, `MOD(a, b)`: the operators are standard
            // functions too (Ed.3 6.6.2.5, Tables 29 and 31), and the
            // FBD and LD generators call them that way. `NOT(x)` needs
            // nothing: it is NOT applied to `(x)`.
            TokenKind::Kw(kw @ (Kw::And | Kw::Or | Kw::Xor | Kw::Mod)) if self.nth(1) == TokenKind::LParen => {
                let t = self.bump();
                let id = Ident { text: kw.as_str().to_string(), span: t.span };
                Expr { span: id.span, kind: ExprKind::Name(id) }
            }
            TokenKind::DirectAddress => {
                let t = self.bump();
                let text = self.text_of(t).to_string();
                if self.in_body && text.ends_with('*') {
                    self.diags.push(Diagnostic::new(
                        Code::BadLocation,
                        t.span,
                        format!("{text} is a declaration's placeholder, completed by VAR_CONFIG: a body uses the variable's name"),
                    ));
                } else if self.in_body {
                    self.diags.push(Diagnostic::new(
                        Code::DirectAddressInBody,
                        t.span,
                        format!(
                            "{text} is a direct address in a body, deprecated since Ed.3: declare a name AT {text} and use the name"
                        ),
                    ));
                }
                Expr { span: t.span, kind: ExprKind::DirectAddress(DirectAddress { text, span: t.span }) }
            }
            TokenKind::LParen => {
                self.bump();
                let inner = self.expr();
                self.expect(TokenKind::RParen);
                Expr { kind: ExprKind::Paren(Box::new(inner)), span: self.span_from(start) }
            }
            _ => {
                self.expected("an expression");
                let at = self.tok().span.start;
                Expr { kind: ExprKind::Error, span: Span::new(self.file, at, at) }
            }
        }
    }

    /// `INT#5`, `INT#-5`, `REAL#1.5`, `BOOL#TRUE`, `STRING#'x'`,
    /// `Color#Red`.
    fn typed_literal(&mut self, start: u32) -> Expr {
        let ty = self.ident("a type");
        self.bump(); // '#'
        let negative = if self.eat(TokenKind::Minus) {
            true
        } else {
            self.eat(TokenKind::Plus);
            false
        };
        let kind = match self.kind() {
            TokenKind::Int => LiteralKind::Int,
            TokenKind::Real => LiteralKind::Real,
            TokenKind::Based => LiteralKind::Based,
            TokenKind::String => LiteralKind::String,
            TokenKind::WString => LiteralKind::WString,
            TokenKind::Kw(Kw::True | Kw::False) => LiteralKind::Bool,
            TokenKind::Ident if !negative => {
                let value = self.ident("an enumerated value");
                return Expr { kind: ExprKind::EnumValue { ty, value }, span: self.span_from(start) };
            }
            _ => {
                self.expected(&format!("a value after '{}#'", ty.text));
                return Expr { kind: ExprKind::Error, span: self.span_from(start) };
            }
        };
        self.literal(kind, Some(ty), negative, start)
    }

    // ---- SFC -------------------------------------------------------

    fn step(&mut self) -> Step {
        let start = self.start();
        let initial = self.at_kw(Kw::InitialStep);
        let opener = self.bump().span;
        let name = self.ident("the step's name");
        self.expect(TokenKind::Colon);
        let mut actions = Vec::new();
        while self.at(TokenKind::Ident) {
            let a_start = self.start();
            let action = self.ident("an action name");
            let mut qualifier = None;
            let mut time = None;
            let mut indicators = Vec::new();
            if self.expect(TokenKind::LParen) {
                if self.at(TokenKind::Ident) {
                    let q = self.ident("a qualifier");
                    // Ed.3 Table 59 (P1 and P0 too, which Annex A's
                    // Action_Qualifier leaves out).
                    if !["N", "R", "S", "P", "P1", "P0", "L", "D", "SD", "DS", "SL"].iter().any(|n| q.is(n)) {
                        self.diags.push(Diagnostic::new(
                            Code::UnexpectedToken,
                            q.span,
                            format!(
                                "'{}' is no action qualifier: N, R, S, P, P1, P0, L, D, SD, DS or SL (Ed.3 Table 59)",
                                q.text
                            ),
                        ));
                    }
                    qualifier = Some(q);
                }
                let timed = qualifier.as_ref().is_some_and(|q| ["L", "D", "SD", "DS", "SL"].iter().any(|t| q.is(t)));
                while self.eat(TokenKind::Comma) {
                    if timed && time.is_none() {
                        time = Some(self.expr());
                    } else {
                        indicators.push(self.ident("an indicator variable"));
                    }
                }
                self.expect(TokenKind::RParen);
                // A timed qualifier takes its duration (Annex A,
                // Action_Qualifier: `( 'L' | 'D' | ... ) ',' Action_Time`).
                if let Some(q) = qualifier.as_ref().filter(|_| timed && time.is_none()) {
                    self.diags.push(Diagnostic::new(
                        Code::UnexpectedToken,
                        q.span,
                        format!(
                            "{} takes a duration: {}({}, T#5s) (Ed.3 Annex A, Action_Qualifier)",
                            q.text.to_ascii_uppercase(),
                            action.text,
                            q.text
                        ),
                    ));
                }
            }
            if !self.eat(TokenKind::Semi) {
                self.expected("';' after the action association");
                self.skip_until_semi_or(&[Kw::EndStep]);
            }
            actions.push(ActionAssociation { action, qualifier, time, indicators, span: self.span_from(a_start) });
        }
        let what = if initial { "INITIAL_STEP" } else { "STEP" };
        self.close(Kw::EndStep, opener, what);
        Step { initial, name, actions, span: self.span_from(start) }
    }

    fn step_names(&mut self) -> Vec<Ident> {
        if self.eat(TokenKind::LParen) {
            let mut names = vec![self.ident("a step name")];
            while self.eat(TokenKind::Comma) {
                names.push(self.ident("a step name"));
            }
            self.expect(TokenKind::RParen);
            names
        } else {
            vec![self.ident("a step name")]
        }
    }

    fn transition(&mut self) -> Transition {
        let start = self.start();
        let opener = self.bump().span;
        let name = if self.at(TokenKind::Ident) { Some(self.ident("a name")) } else { None };
        let mut priority = None;
        if self.eat(TokenKind::LParen) {
            let p = self.ident("PRIORITY");
            if !p.is("PRIORITY") && !p.text.is_empty() {
                self.diags.push(Diagnostic::new(Code::UnexpectedToken, p.span, "expected PRIORITY"));
            }
            self.expect(TokenKind::Assign);
            let e = self.expr();
            // Annex A, Transition: `'PRIORITY' ':=' Unsigned_Int`.
            let literal = matches!(&e.kind, ExprKind::Literal(l)
                if l.kind == LiteralKind::Int && l.type_prefix.is_none() && !l.negative);
            if !literal && !matches!(e.kind, ExprKind::Error) {
                self.diags.push(Diagnostic::new(
                    Code::UnexpectedToken,
                    e.span,
                    "a transition's PRIORITY is an unsigned integer literal (Ed.3 Annex A, Transition)",
                ));
            }
            priority = Some(e);
            self.expect(TokenKind::RParen);
        }
        self.expect_kw(Kw::From);
        let from = self.step_names();
        self.expect_kw(Kw::To);
        let to = self.step_names();
        let condition = if self.eat(TokenKind::Assign) {
            let c = self.expr();
            if !self.eat(TokenKind::Semi) {
                self.expected("';' after the condition");
            }
            c
        } else {
            if self.at(TokenKind::Colon) {
                let s = self.tok().span;
                self.not_yet(s, "a transition condition in LD or FBD");
                while !self.at_eof() && !self.at_kw(Kw::EndTransition) {
                    self.bump();
                }
            } else {
                self.expected("':=' and the condition");
            }
            Expr { kind: ExprKind::Error, span: Span::new(self.file, self.last_end, self.last_end) }
        };
        self.close(Kw::EndTransition, opener, "TRANSITION");
        Transition { name, priority, from, to, condition, span: self.span_from(start) }
    }

    fn action(&mut self) -> Action {
        let start = self.start();
        let opener = self.bump().span;
        let name = self.ident("the action's name");
        self.expect(TokenKind::Colon);
        // An action whose body is an SFC (Ed.3 6.7.4.1; Annex A, Action :
        // ... FB_Body): a chart of its own, with its steps, transitions
        // and actions.
        if matches!(self.kind(), TokenKind::Kw(Kw::InitialStep | Kw::Step | Kw::Transition)) {
            let mut sfc = Vec::new();
            loop {
                match self.kind() {
                    TokenKind::Kw(Kw::Step | Kw::InitialStep) => sfc.push(SfcElement::Step(self.step())),
                    TokenKind::Kw(Kw::Transition) => sfc.push(SfcElement::Transition(self.transition())),
                    TokenKind::Kw(Kw::Action) => sfc.push(SfcElement::Action(self.action())),
                    _ => break,
                }
            }
            if !self.at_eof() && !self.at_kw(Kw::EndAction) {
                let at = self.tok().span;
                self.diags.push(Diagnostic::new(
                    Code::UnexpectedToken,
                    at,
                    "a body is statements or an SFC, not both (Ed.3 Annex A, FB_Body; an ACTION is a part of an \
                     SFC): put the statements in an action of the SFC",
                ));
                self.stmts(&[Kw::EndAction]);
            }
            self.close(Kw::EndAction, opener, "ACTION");
            return Action { name, body: Vec::new(), sfc, span: self.span_from(start) };
        }
        let body = self.stmts(&[Kw::EndAction]);
        self.close(Kw::EndAction, opener, "ACTION");
        Action { name, body, sfc: Vec::new(), span: self.span_from(start) }
    }

    // ---- configuration ---------------------------------------------

    fn configuration(&mut self) -> Configuration {
        let start = self.start();
        let opener = self.bump().span;
        let name = self.ident("the configuration's name");
        let mut globals = Vec::new();
        let mut config_inits = Vec::new();
        let mut accesses = Vec::new();
        self.config_var_blocks(&mut globals, &mut config_inits, &mut accesses);
        let mut resources = Vec::new();
        if self.at_kw(Kw::Resource) {
            while self.at_kw(Kw::Resource) {
                resources.push(self.resource());
            }
        } else {
            let r_start = self.start();
            let (tasks, programs) = self.resource_body();
            resources.push(Resource {
                name: None,
                on: None,
                globals: Vec::new(),
                tasks,
                programs,
                span: self.span_from(r_start),
            });
        }
        self.config_var_blocks(&mut globals, &mut config_inits, &mut accesses);
        self.close(Kw::EndConfiguration, opener, "CONFIGURATION");
        Configuration { name, globals, resources, config_inits, accesses, span: self.span_from(start) }
    }

    /// `VAR_GLOBAL`, `VAR_ACCESS` and `VAR_CONFIG` blocks of a
    /// configuration. The grammar puts globals before the resources and
    /// the other two after; any order is read, the checker does not mind.
    fn config_var_blocks(
        &mut self,
        globals: &mut Vec<VarBlock>,
        inits: &mut Vec<ConfigInit>,
        accesses: &mut Vec<AccessDecl>,
    ) {
        loop {
            match self.kind() {
                TokenKind::Kw(Kw::VarConfig) => self.config_init_block(inits),
                TokenKind::Kw(Kw::VarAccess) => self.access_block(accesses),
                TokenKind::Kw(Kw::VarGlobal) => {
                    if let Some(b) = self.var_block() {
                        globals.push(b);
                    }
                }
                _ => return,
            }
        }
    }

    /// `VAR_ACCESS name : path : type READ_WRITE; ... END_VAR` (Ed.3 Annex
    /// A, Access_Decls and Prog_Access_Decls).
    fn access_block(&mut self, out: &mut Vec<AccessDecl>) {
        let opener = self.bump().span;
        while !self.at_eof() && !self.at_kw(Kw::EndVar) {
            if let TokenKind::Kw(k) = self.kind()
                && is_structural(k)
            {
                break;
            }
            let before = self.pos;
            let start = self.start();
            if !self.at(TokenKind::Ident) {
                self.expected("an access name, as in `ALPHA : STATION_2.P1.y1 : BYTE READ_ONLY;`");
                self.skip_until_semi_or(&[Kw::EndVar]);
            } else {
                let name = self.ident("an access name");
                self.expect(TokenKind::Colon);
                // `R.%IX1.0`: a resource before a direct address.
                let resource = if self.at(TokenKind::Ident)
                    && self.nth(1) == TokenKind::Dot
                    && self.nth(2) == TokenKind::DirectAddress
                {
                    let r = self.ident("a resource's name");
                    self.bump();
                    Some(r)
                } else {
                    None
                };
                let path = self.expr();
                self.expect(TokenKind::Colon);
                let ty = self.type_spec();
                let mut read_write = false;
                if self.at(TokenKind::Ident) {
                    let word = self.text_of(self.tok()).to_ascii_uppercase();
                    match word.as_str() {
                        "READ_WRITE" | "READ_ONLY" => {
                            read_write = word == "READ_WRITE";
                            self.bump();
                        }
                        _ => self.expected("READ_ONLY, READ_WRITE or ';'"),
                    }
                }
                out.push(AccessDecl { name, resource, path, ty, read_write, span: self.span_from(start) });
                if !self.eat(TokenKind::Semi) {
                    self.expected("';'");
                    self.skip_until_semi_or(&[Kw::EndVar]);
                }
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.close(Kw::EndVar, opener, "VAR_ACCESS");
    }

    /// `VAR_CONFIG path AT %QB25 : BYTE := 0; ... END_VAR`
    fn config_init_block(&mut self, inits: &mut Vec<ConfigInit>) {
        let opener = self.bump().span;
        while !self.at_eof() && !self.at_kw(Kw::EndVar) {
            if let TokenKind::Kw(k) = self.kind()
                && is_structural(k)
            {
                break;
            }
            let before = self.pos;
            if let Some(i) = self.config_init() {
                inits.push(i);
            }
            if self.pos == before {
                self.bump();
            }
        }
        self.close(Kw::EndVar, opener, "VAR_CONFIG");
    }

    fn config_init(&mut self) -> Option<ConfigInit> {
        let start = self.start();
        if !self.at(TokenKind::Ident) {
            self.expected("an instance path such as STATION_1.P1.COUNT");
            self.skip_until_semi_or(&[Kw::EndVar]);
            return None;
        }
        let mut path = vec![self.ident("a name")];
        while self.eat(TokenKind::Dot) {
            path.push(self.ident("a name"));
        }
        let mut location = None;
        if self.eat_kw(Kw::At) {
            if self.at(TokenKind::DirectAddress) {
                let t = self.bump();
                location = Some(DirectAddress { text: self.text_of(t).to_string(), span: t.span });
            } else {
                self.expected("a direct address such as %QB25");
            }
        }
        if !self.expect(TokenKind::Colon) {
            self.skip_until_semi_or(&[Kw::EndVar]);
            return None;
        }
        let ty = self.type_spec();
        let init = if self.eat(TokenKind::Assign) { Some(self.init()) } else { None };
        if !self.eat(TokenKind::Semi) {
            self.expected("';' after the entry");
            self.skip_until_semi_or(&[Kw::EndVar]);
        }
        Some(ConfigInit { path, location, ty, init, span: self.span_from(start) })
    }

    fn resource(&mut self) -> Resource {
        let start = self.start();
        let opener = self.bump().span;
        let name = self.ident("the resource's name");
        self.expect_kw(Kw::On);
        let on = self.ident("the resource type");
        let mut globals = Vec::new();
        while self.at_kw(Kw::VarGlobal) {
            if let Some(b) = self.var_block() {
                globals.push(b);
            }
        }
        let (tasks, programs) = self.resource_body();
        self.close(Kw::EndResource, opener, "RESOURCE");
        Resource { name: Some(name), on: Some(on), globals, tasks, programs, span: self.span_from(start) }
    }

    fn resource_body(&mut self) -> (Vec<Task>, Vec<ProgramConfig>) {
        let mut tasks = Vec::new();
        let mut programs = Vec::new();
        loop {
            let start = self.start();
            if self.eat_kw(Kw::Task) {
                let name = self.ident("the task's name");
                let params = if self.expect(TokenKind::LParen) { self.args() } else { Vec::new() };
                if !self.eat(TokenKind::Semi) {
                    self.expected("';' after the task");
                    self.skip_until_semi_or(&[]);
                }
                tasks.push(Task { name, params, span: self.span_from(start) });
            } else if self.eat_kw(Kw::Program) {
                let retain = if self.eat_kw(Kw::Retain) {
                    Some(Retain::Retain)
                } else if self.eat_kw(Kw::NonRetain) {
                    Some(Retain::NonRetain)
                } else {
                    None
                };
                let name = self.ident("the program instance's name");
                let task = if self.eat_kw(Kw::With) { Some(self.ident("a task name")) } else { None };
                self.expect(TokenKind::Colon);
                let program_type = self.dotted_name("the program's type");
                self.fb_tasks.clear();
                let args = if self.eat(TokenKind::LParen) { self.args() } else { Vec::new() };
                let fb_tasks = std::mem::take(&mut self.fb_tasks);
                if !self.eat(TokenKind::Semi) {
                    self.expected("';' after the program");
                    self.skip_until_semi_or(&[]);
                }
                programs.push(ProgramConfig {
                    retain,
                    name,
                    task,
                    program_type,
                    args,
                    fb_tasks,
                    span: self.span_from(start),
                });
            } else {
                break;
            }
        }
        (tasks, programs)
    }
}

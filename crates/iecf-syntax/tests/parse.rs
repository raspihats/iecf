use iecf_diagnostics::FileId;
use iecf_syntax::ast::*;
use iecf_syntax::parse;

fn ok(text: &str) -> SourceUnit {
    let p = parse(FileId(0), text);
    assert!(p.diagnostics.is_empty(), "{:#?}", p.diagnostics);
    p.unit
}

/// `(code, line)` of each diagnostic, line 1-based.
fn diags(text: &str) -> Vec<(&'static str, usize)> {
    parse(FileId(0), text)
        .diagnostics
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect()
}

fn messages(text: &str) -> Vec<String> {
    parse(FileId(0), text).diagnostics.into_iter().map(|d| d.message).collect()
}

fn program_body(text: &str) -> Vec<Stmt> {
    let unit = ok(&format!("PROGRAM p\nVAR a, b, c : INT; x : BOOL; END_VAR\n{text}\nEND_PROGRAM\n"));
    match unit.items.into_iter().next() {
        Some(Item::Pou(p)) => p.body,
        other => panic!("{other:?}"),
    }
}

fn expr_of(text: &str) -> Expr {
    match program_body(&format!("a := {text};")).remove(0).kind {
        StmtKind::Assign { value, .. } => value,
        other => panic!("{other:?}"),
    }
}

/// The expression in prefix form: `(+ a (* b c))`.
fn sexpr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Name(id) => id.text.clone(),
        ExprKind::Literal(l) => {
            let sign = if l.negative { "-" } else { "" };
            match &l.type_prefix {
                Some(t) => format!("{}#{sign}{}", t.text, l.text),
                None => format!("{sign}{}", l.text),
            }
        }
        ExprKind::Binary(op, l, r) => format!("({op:?} {} {})", sexpr(l), sexpr(r)),
        ExprKind::Unary(op, x) => format!("({op:?} {})", sexpr(x)),
        ExprKind::Paren(x) => sexpr(x),
        ExprKind::Member(x, m) => format!("{}.{}", sexpr(x), m.text),
        ExprKind::BitAccess(x, b) => format!("{}.{}", sexpr(x), b.text),
        ExprKind::Index(x, i) => format!("{}[{}]", sexpr(x), i.iter().map(sexpr).collect::<Vec<_>>().join(",")),
        ExprKind::Deref(x) => format!("{}^", sexpr(x)),
        ExprKind::Call { callee, args } => {
            let args: Vec<String> = args
                .iter()
                .map(|a| match a {
                    Arg::Positional(e) => sexpr(e),
                    Arg::Input(n, e) => format!("{}:={}", n.text, sexpr(e)),
                    Arg::Output { not, name, target } => {
                        format!("{}{}=>{}", if *not { "NOT " } else { "" }, name.text, sexpr(target))
                    }
                })
                .collect();
            format!("{}({})", sexpr(callee), args.join(","))
        }
        ExprKind::EnumValue { ty, value } => format!("{}#{}", ty.text, value.text),
        ExprKind::DirectAddress(d) => d.text.clone(),
        ExprKind::Error => "<error>".into(),
    }
}

#[test]
fn the_corpus_parses_clean() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus");
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if !matches!(path.extension().and_then(|e| e.to_str()), Some("st" | "gvl")) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let p = parse(FileId(0), &text);
        assert!(p.diagnostics.is_empty(), "{}: {:#?}", path.display(), p.diagnostics);
        assert!(!p.unit.items.is_empty(), "{}", path.display());
        n += 1;
    }
    assert!(n >= 7, "corpus has {n} files");
}

#[test]
fn precedence_follows_ed3_table_71() {
    assert_eq!(sexpr(&expr_of("a + b * c")), "(Add a (Mul b c))");
    assert_eq!(sexpr(&expr_of("a - b - c")), "(Sub (Sub a b) c)");
    assert_eq!(sexpr(&expr_of("a OR b AND c")), "(Or a (And b c))");
    assert_eq!(sexpr(&expr_of("a OR b XOR c")), "(Or a (Xor b c))");
    assert_eq!(sexpr(&expr_of("a & b = c")), "(And a (Eq b c))");
    assert_eq!(sexpr(&expr_of("a = b < c")), "(Eq a (Lt b c))");
    assert_eq!(sexpr(&expr_of("a MOD b + c")), "(Add (Mod a b) c)");
    // Unary binds tighter than ** (Ed.3 Table 71): -2 ** 2 is (-2) ** 2.
    assert_eq!(sexpr(&expr_of("-a ** b")), "(Power (Neg a) b)");
    assert_eq!(sexpr(&expr_of("NOT a AND b")), "(And (Not a) b)");
    assert_eq!(sexpr(&expr_of("(a + b) * c")), "(Mul (Add a b) c)");
}

#[test]
fn postfix_calls_and_literals() {
    assert_eq!(sexpr(&expr_of("p.x[1, i].y^")), "p.x[1,i].y^");
    assert_eq!(sexpr(&expr_of("w.3")), "w.3");
    assert_eq!(sexpr(&expr_of("F(a, IN := b, Q => c, NOT Q2 => d)")), "F(a,IN:=b,Q=>c,NOT Q2=>d)");
    assert_eq!(sexpr(&expr_of("AND(OR(a, b), c)")), "AND(OR(a,b),c)");
    assert_eq!(sexpr(&expr_of("INT#-5")), "INT#-5");
    assert_eq!(sexpr(&expr_of("Color#Red")), "Color#Red");
    assert_eq!(sexpr(&expr_of("F()")), "F()");
}

#[test]
fn statements() {
    let body = program_body(
        "IF x THEN a := 1; ELSIF NOT x THEN a := 2; ELSE ; END_IF;\n\
         CASE a OF 1, 2..5: b := 1; 6: ; ELSE c := 0; END_CASE;\n\
         FOR a := 1 TO 10 BY 2 DO EXIT; END_FOR;\n\
         WHILE a > 0 DO a := a - 1; CONTINUE; END_WHILE;\n\
         REPEAT a := a + 1; UNTIL a > 9 END_REPEAT;\n\
         RETURN;",
    );
    let kinds: Vec<&str> = body
        .iter()
        .map(|s| match &s.kind {
            StmtKind::If { .. } => "if",
            StmtKind::Case { .. } => "case",
            StmtKind::For { .. } => "for",
            StmtKind::While { .. } => "while",
            StmtKind::Repeat { .. } => "repeat",
            StmtKind::Return => "return",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["if", "case", "for", "while", "repeat", "return"]);
    let StmtKind::Case { arms, else_body, .. } = &body[1].kind else { unreachable!() };
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].labels.len(), 2);
    assert!(matches!(arms[0].labels[1], CaseLabel::Range(..)));
    assert!(arms[1].body.iter().all(|s| s.kind == StmtKind::Empty));
    assert_eq!(else_body.as_ref().map(Vec::len), Some(1));
}

#[test]
fn declarations() {
    let unit = ok("PROGRAM p\n\
                   VAR RETAIN a, b AT %QW0 : INT := 3; END_VAR\n\
                   VAR AT %IX0.1 : BOOL; END_VAR\n\
                   VAR_INPUT e : BOOL F_EDGE; END_VAR\n\
                   VAR s : STRING[20]; r : REF_TO INT; m : ARRAY[*] OF INT; END_VAR\n\
                   VAR END_VAR\n\
                   END_PROGRAM");
    let Item::Pou(p) = &unit.items[0] else { unreachable!() };
    assert_eq!(p.var_blocks.len(), 5);
    let first = &p.var_blocks[0];
    assert_eq!(first.retain, Some(Retain::Retain));
    assert_eq!(first.decls[0].names.len(), 2);
    assert_eq!(first.decls[0].location.as_ref().unwrap().text, "%QW0");
    assert!(p.var_blocks[1].decls[0].names.is_empty(), "a located variable without a name");
    assert_eq!(p.var_blocks[2].decls[0].edge, Some(Edge::Falling));
    let ty = |i: usize| &p.var_blocks[3].decls[i].ty;
    assert!(matches!(ty(0), TypeSpec::String { len: Some(_), .. }));
    assert!(matches!(ty(1), TypeSpec::Ref { .. }));
    assert!(matches!(ty(2), TypeSpec::Array { dims, .. } if dims == &vec![None]));
    // An empty VAR block is legal from Ed.3 on.
    assert!(p.var_blocks[4].decls.is_empty());
}

#[test]
fn sfc_and_configuration() {
    let unit = ok("PROGRAM p\nVAR go : BOOL; END_VAR\n\
                   INITIAL_STEP A: Run(N); Pulse(L, T#2s); END_STEP\n\
                   STEP B: END_STEP\n\
                   TRANSITION t1 (PRIORITY := 1) FROM A TO (B) := go; END_TRANSITION\n\
                   TRANSITION FROM B TO A := NOT go; END_TRANSITION\n\
                   ACTION Run: go := TRUE; END_ACTION\n\
                   ACTION Pulse: ; END_ACTION\n\
                   END_PROGRAM\n\
                   CONFIGURATION C\n\
                   VAR_GLOBAL g : INT; END_VAR\n\
                   RESOURCE R ON PLC\n\
                   TASK t(INTERVAL := T#10ms, PRIORITY := 0);\n\
                   PROGRAM RETAIN i WITH t : p;\n\
                   END_RESOURCE\n\
                   END_CONFIGURATION\n");
    let Item::Pou(p) = &unit.items[0] else { unreachable!() };
    assert_eq!(p.sfc.len(), 6);
    let SfcElement::Step(a) = &p.sfc[0] else { unreachable!() };
    assert!(a.initial);
    assert!(a.actions[1].time.is_some());
    let SfcElement::Transition(t) = &p.sfc[2] else { unreachable!() };
    assert!(t.priority.is_some());
    let Item::Configuration(c) = &unit.items[1] else { unreachable!() };
    assert_eq!(c.globals.len(), 1);
    let r = &c.resources[0];
    assert_eq!(r.tasks[0].params.len(), 2);
    assert_eq!(r.programs[0].retain, Some(Retain::Retain));
    assert!(r.programs[0].task.as_ref().unwrap().is("T"));
}

#[test]
fn sfc_grammar() {
    // Ed.3 Annex A: Action_Qualifier (Table 61's names; a timed one with
    // its duration), Transition's PRIORITY an Unsigned_Int, and FB_Body
    // one language - an ACTION is a part of an SFC.
    let text = "PROGRAM p\nVAR go : BOOL; END_VAR\n\
                INITIAL_STEP A: Run(Z); Run(L); Run(P1); END_STEP\n\
                TRANSITION (PRIORITY := go) FROM A TO A := go; END_TRANSITION\n\
                TRANSITION (PRIORITY := -1) FROM A TO A := go; END_TRANSITION\n\
                ACTION Run: go := TRUE; END_ACTION\n\
                END_PROGRAM\n";
    assert_eq!(diags(text), vec![("E0005", 3), ("E0005", 3), ("E0005", 4), ("E0005", 5)]);
    let m = messages(text);
    assert_eq!(m[0], "'Z' is no action qualifier: N, R, S, P, P1, P0, L, D, SD, DS or SL (Ed.3 Table 61)");
    assert_eq!(m[1], "L takes a duration: Run(L, T#5s) (Ed.3 Annex A, Action_Qualifier)");
    assert_eq!(m[2], "a transition's PRIORITY is an unsigned integer literal (Ed.3 Annex A, Transition)");
    let beside = "PROGRAM p\nVAR go : BOOL; END_VAR\ngo := TRUE;\nACTION Run: go := FALSE; END_ACTION\nEND_PROGRAM\n";
    assert_eq!(diags(beside), vec![("E0005", 4)]);
}

#[test]
fn an_action_whose_body_is_an_sfc() {
    // Ed.3 6.7.4.1; Annex A, Action : ... FB_Body: a chart of its own,
    // with its actions; one language, as any body.
    let text = "PROGRAM p\nVAR go : BOOL; END_VAR\n\
                INITIAL_STEP a: sub(N); END_STEP\n\
                ACTION sub:\n\
                INITIAL_STEP i0: inner(N); END_STEP\n\
                TRANSITION FROM i0 TO i0 := go; END_TRANSITION\n\
                ACTION inner: go := TRUE; END_ACTION\n\
                END_ACTION\n\
                END_PROGRAM\n";
    assert_eq!(diags(text), vec![]);
    let unit = parse(FileId(0), text).unit;
    let Item::Pou(p) = &unit.items[0] else { unreachable!() };
    let SfcElement::Action(sub) = &p.sfc[1] else { unreachable!() };
    assert!(sub.body.is_empty());
    assert_eq!(sub.sfc.len(), 3);
    let SfcElement::Action(inner) = &sub.sfc[2] else { unreachable!() };
    assert_eq!(inner.body.len(), 1);
    assert_eq!(sfc_elements(&p.sfc).iter().map(|(c, _)| *c).collect::<Vec<_>>(), vec![0, 0, 1, 1, 1]);
    let both = "PROGRAM p\nVAR go : BOOL; END_VAR\nINITIAL_STEP a: sub(N); END_STEP\n\
                ACTION sub:\nINITIAL_STEP i0: END_STEP\ngo := TRUE;\nEND_ACTION\nEND_PROGRAM\n";
    assert_eq!(diags(both), vec![("E0005", 6)]);
}

#[test]
fn an_error_in_one_pou_does_not_hide_the_next() {
    let text = "PROGRAM a\nVAR x : INT; END_VAR\nx := ;\nEND_PROGRAM\n\
                PROGRAM b\nVAR y : INT; END_VAR\nIF y THEN\ny := 1;\nEND_PROGRAM\n";
    assert_eq!(diags(text), vec![("E0005", 3), ("E0006", 7)]);
    let unit = parse(FileId(0), text).unit;
    assert_eq!(unit.items.len(), 2, "both POUs are in the tree");
}

#[test]
fn one_mistake_one_error() {
    // A missing THEN: one error, and the IF still parses.
    assert_eq!(
        diags("PROGRAM p\nVAR x : BOOL; END_VAR\nIF x\n  x := FALSE;\nEND_IF;\nEND_PROGRAM"),
        vec![("E0005", 4)]
    );
    // A missing ';' after a statement.
    assert_eq!(diags("PROGRAM p\nVAR x : BOOL; END_VAR\nx := TRUE\nx := FALSE;\nEND_PROGRAM"), vec![("E0005", 4)]);
    // A missing END_VAR is reported at the VAR.
    assert_eq!(diags("PROGRAM p\nVAR x : BOOL;\nx := TRUE;\nEND_PROGRAM"), vec![("E0006", 2), ("E0005", 3)]);
}

#[test]
fn helpful_messages() {
    let m = messages("PROGRAM p\nVAR x : INT; END_VAR\nx = 1;\nEND_PROGRAM");
    assert_eq!(m, ["'=' compares; to assign, write ':='"]);
    let m = messages("PROGRAM p\nVAR on : BOOL; END_VAR\nEND_PROGRAM");
    assert_eq!(m, ["'on' is the keyword ON and cannot be a name (Ed.3 6.1.3): choose another name"]);
}

#[test]
fn deprecated_and_removed_forms_warn() {
    let text = "PROGRAM p\nVAR lamp AT %QX0.0 : BOOL; m : WORD := 8#17; END_VAR\n%QX0.1 := lamp;\nEND_PROGRAM";
    assert_eq!(diags(text), vec![("W0001", 2), ("W0002", 3)]);
}

#[test]
fn ed4_constructs_say_not_yet() {
    let text = "FUNCTION_BLOCK F\nPROPERTY_GET p : INT\np := 1;\nEND_PROPERTY\nEND_FUNCTION_BLOCK\n";
    assert_eq!(diags(text), vec![("E0009", 2)]);
}

/// INTERFACE, its EXTENDS of any number and its prototypes; IMPLEMENTS
/// of any number; `?=` (Ed.3 6.6.6).
#[test]
fn interfaces_and_assignment_attempts() {
    let unit =
        ok("INTERFACE I EXTENDS A, Lib.B\nMETHOD m : INT\nVAR_INPUT x : INT; END_VAR\nEND_METHOD\nEND_INTERFACE\n\
                   FUNCTION_BLOCK F EXTENDS G IMPLEMENTS I, Lib.J\nEND_FUNCTION_BLOCK\n\
                   PROGRAM p\nVAR a, b : I; END_VAR\na ?= b;\nEND_PROGRAM\n");
    let [Item::Pou(i), Item::Pou(f), Item::Pou(p)] = unit.items.as_slice() else { panic!("{unit:?}") };
    assert_eq!(i.kind, PouKind::Interface);
    let names = |v: &[iecf_syntax::ast::Ident]| v.iter().map(|n| n.text.clone()).collect::<Vec<_>>();
    assert_eq!(names(&i.interfaces), ["A", "Lib.B"]);
    assert_eq!(i.methods.len(), 1);
    assert!(i.methods[0].body.is_empty());
    assert_eq!(f.extends.as_ref().map(|e| e.text.as_str()), Some("G"));
    assert_eq!(names(&f.interfaces), ["I", "Lib.J"]);
    assert!(matches!(p.body[0].kind, iecf_syntax::ast::StmtKind::AssignAttempt { .. }));
    // An interface has no body.
    assert_eq!(diags("INTERFACE I\nMETHOD m\nEND_METHOD\nx := 1;\nEND_INTERFACE\n"), vec![("E0005", 4)]);
}

/// EXTENDS, one base, its name with its namespaces (Ed.3 6.6.5.5.2).
#[test]
fn extends_names_one_base() {
    let unit = ok("FUNCTION_BLOCK F EXTENDS Lib.G\nEND_FUNCTION_BLOCK\nCLASS C EXTENDS K\nEND_CLASS\n");
    let bases: Vec<String> = unit
        .items
        .iter()
        .filter_map(|i| match i {
            iecf_syntax::ast::Item::Pou(p) => p.extends.as_ref().map(|b| b.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(bases, ["Lib.G", "K"]);
    assert_eq!(diags("CLASS C EXTENDS A, B\nEND_CLASS\n"), vec![("E0005", 1)]);
}

/// CLASS, METHOD, their specifiers in any order (OO.md D2), and `VAR
/// PUBLIC`; each specifier a word only where a name follows it.
#[test]
fn classes_and_methods() {
    let unit = ok("CLASS FINAL C\n\
                   VAR PUBLIC RETAIN limit : INT; END_VAR\n\
                   VAR public, final : BOOL; END_VAR\n\
                   METHOD PUBLIC FINAL up : INT\nVAR_INPUT inc : INT; END_VAR\nup := inc;\nEND_METHOD\n\
                   METHOD OVERRIDE PROTECTED down\nEND_METHOD\n\
                   METHOD final\nEND_METHOD\n\
                   END_CLASS\n\
                   FUNCTION_BLOCK B\nVAR_INPUT i : INT; END_VAR\nMETHOD m\nEND_METHOD\ni := 1;\nEND_FUNCTION_BLOCK\n");
    let [Item::Pou(c), Item::Pou(b)] = unit.items.as_slice() else { panic!("{unit:?}") };
    assert_eq!(c.kind, PouKind::Class);
    assert!(c.specs.final_.is_some() && c.body.is_empty());
    assert_eq!(c.var_blocks[0].access.map(|a| a.0), Some(Access::Public));
    assert_eq!(c.var_blocks[0].retain, Some(Retain::Retain));
    assert_eq!(c.var_blocks[1].access, None);
    assert_eq!(c.var_blocks[1].decls[0].names.len(), 2);
    let names: Vec<&str> = c.methods.iter().map(|m| m.name.text.as_str()).collect();
    assert_eq!(names, ["up", "down", "final"]);
    let up = &c.methods[0];
    assert_eq!(up.kind, PouKind::Method);
    assert_eq!(up.specs.access.map(|a| a.0), Some(Access::Public));
    assert!(up.specs.final_.is_some() && up.return_type.is_some() && up.body.len() == 1);
    let down = &c.methods[1];
    assert!(down.specs.override_.is_some());
    assert_eq!(down.specs.access.map(|a| a.0), Some(Access::Protected));
    assert_eq!(c.methods[2].specs, Specs::default());
    assert_eq!((b.methods.len(), b.body.len()), (1, 1));
}

#[test]
fn what_classes_and_methods_cannot_have() {
    // A class has no body; a method belongs to a class or a block, before
    // a block's body; one access specifier, each word once.
    let text = "CLASS C\nVAR x : INT; END_VAR\nx := 1;\nEND_CLASS\n\
                PROGRAM p\nMETHOD m\nEND_METHOD\nEND_PROGRAM\n\
                FUNCTION_BLOCK F\nVAR x : INT; END_VAR\nx := 1;\nMETHOD m\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                CLASS D\nMETHOD PUBLIC PRIVATE m\nEND_METHOD\nMETHOD FINAL FINAL n\nEND_METHOD\nEND_CLASS\n";
    assert_eq!(diags(text), vec![("E0005", 3), ("E0005", 6), ("E0005", 12), ("E0005", 16), ("E0005", 18)]);
}

/// The editor feeds the parser half-typed text all day: every prefix of
/// every corpus file must parse, without a panic and without hanging,
/// and every diagnostic's span must lie inside the text.
#[test]
fn every_truncation_parses() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if !matches!(path.extension().and_then(|e| e.to_str()), Some("st" | "gvl")) {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (cut, _) in text.char_indices() {
            let prefix = &text[..cut];
            for d in parse(FileId(0), prefix).diagnostics {
                assert!(d.span.end as usize <= prefix.len(), "{}: cut at {cut}: {d:?}", path.display());
            }
        }
    }
}

#[test]
fn var_config_as_in_ed3_figure_28() {
    // The VAR_CONFIG block of the standard's own example (Ed.3 6.8.2).
    let unit = ok("CONFIGURATION CELL_1\n\
                   RESOURCE STATION_1 ON PROCESSOR_TYPE_1\n\
                   PROGRAM P1 : F;\n\
                   END_RESOURCE\n\
                   VAR_CONFIG\n\
                   STATION_1.P1.COUNT: INT:= 1;\n\
                   STATION_2.P1.TIME1: TON:= (PT:= T#4.5s);\n\
                   STATION_2.P4.FB1.C2 AT %QB25: BYTE;\n\
                   END_VAR\n\
                   END_CONFIGURATION\n");
    let Some(Item::Configuration(c)) = unit.items.first() else { panic!("{unit:?}") };
    assert_eq!(c.config_inits.len(), 3);
    let last = &c.config_inits[2];
    let path: Vec<_> = last.path.iter().map(|i| i.text.as_str()).collect();
    assert_eq!(path, ["STATION_2", "P4", "FB1", "C2"]);
    assert_eq!(last.location.as_ref().map(|l| l.text.as_str()), Some("%QB25"));
    assert!(matches!(c.config_inits[1].init, Some(Init::Struct { .. })));
}

#[test]
fn partly_addresses() {
    let unit = ok("FUNCTION_BLOCK v\nVAR c AT %Q* : BYTE; m AT %m* : WORD; END_VAR\nEND_FUNCTION_BLOCK\n");
    let Some(Item::Pou(p)) = unit.items.first() else { panic!() };
    let loc = p.var_blocks[0].decls[0].location.as_ref().unwrap();
    assert!(loc.is_partly());
    assert_eq!(loc.area(), Some('Q'));
    assert_eq!(p.var_blocks[0].decls[1].location.as_ref().unwrap().area(), Some('M'));
    // The asterisk replaces the size too.
    let m = messages("FUNCTION_BLOCK v\nVAR c AT %QX* : BOOL; END_VAR\nEND_FUNCTION_BLOCK\n");
    assert_eq!(m, ["'%QX*' is not a direct address: '*' takes the place of the size and the numbers, as in %I*"]);
}

#[test]
fn an_element_called_and_a_repeated_instance_value() {
    // Ed.3 Figure 8 d, `TONs[12](...)`, and Table 11 row 5b's
    // `[50(PT:=T#100ms)]`, as well as the grammar's `50((PT:=...))`.
    let unit =
        ok("PROGRAM p\nVAR ts : ARRAY[1..2] OF TON := [2(PT := T#1s)]; END_VAR\nts[1](IN := TRUE);\nEND_PROGRAM\n");
    let [Item::Pou(p)] = unit.items.as_slice() else { panic!("{unit:?}") };
    let StmtKind::Call(e) = &p.body[0].kind else { panic!("{:?}", p.body) };
    let ExprKind::Call { callee, .. } = &e.kind else { panic!("{e:?}") };
    assert!(matches!(callee.kind, ExprKind::Index(..)), "{callee:?}");
    let Some(Init::Array { elems, .. }) = &p.var_blocks[0].decls[0].init else { panic!("{p:?}") };
    assert!(elems[0].repeat.is_some());
    assert!(
        matches!(&elems[0].value, Some(Init::Struct { fields, .. }) if fields.len() == 1 && fields[0].0.text == "PT"),
        "{elems:?}"
    );
    let unit = ok("TYPE T2 : ARRAY[1..2] OF TON := [2((PT := T#1s))]; END_TYPE\n");
    let [Item::Types(ts)] = unit.items.as_slice() else { panic!("{unit:?}") };
    let Some(Init::Array { elems, .. }) = &ts[0].init else { panic!("{ts:?}") };
    assert!(matches!(&elems[0].value, Some(Init::Struct { .. })), "{elems:?}");
}

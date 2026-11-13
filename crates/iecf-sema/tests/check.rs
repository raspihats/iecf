use iecf_diagnostics::FileId;

/// `(code, line, message)` for each diagnostic of a one-file project,
/// parse errors included (there should be none).
fn check(text: &str) -> Vec<(&'static str, usize, String)> {
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(!parsed.diagnostics.iter().any(|d| d.is_error()), "parse: {:#?}", parsed.diagnostics);
    iecf_sema::check(&[&parsed.unit])
        .into_iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1, d.message))
        .collect()
}

fn codes(text: &str) -> Vec<(&'static str, usize)> {
    check(text).into_iter().map(|(c, l, _)| (c, l)).collect()
}

/// A program with a fixed set of variables and `body` as its body. The
/// body starts on line 10.
fn prog(body: &str) -> String {
    format!(
        "PROGRAM p\n\
         VAR\n\
         \x20 b : BOOL; i : INT; d : DINT; r : REAL; lr : LREAL;\n\
         \x20 w : WORD; bt : BYTE; t : TIME; s : STRING; ws : WSTRING;\n\
         \x20 us : USINT; si : SINT; arr : ARRAY[1..3] OF INT; tm : TON;\n\
         END_VAR\n\
         VAR CONSTANT K : INT := 3; END_VAR\n\
         VAR u : UINT; END_VAR\n\
         VAR q : LINT; END_VAR\n\
         {body}\n\
         END_PROGRAM\n"
    )
}

#[test]
fn a_correct_program_is_silent() {
    let body = "d := i;\nlr := r;\nr := i;\nw := bt;\nb := w.3;\nd := d + i;\nlr := r * 2.0;\n\
                t := t + T#1s;\nt := t * 2;\ntm(IN := b, PT := T#500ms);\nb := tm.Q;\nt := tm.ET;\n\
                i := arr[i];\nb := i > 3 AND d <> 0;\ns := CONCAT(s, 'x');\ni := LEN(s);\n\
                r := INT_TO_REAL(i);\ni := REAL_TO_INT(r);\ni := MAX(i, 5);\nIF b THEN i := 1; END_IF;\n\
                FOR i := 1 TO 3 DO arr[i] := 0; EXIT; END_FOR;\nWHILE NOT b DO b := TRUE; END_WHILE;\n\
                CASE i OF 1, 2..4: d := 0; END_CASE;\nus := 200;\nsi := -128;\nq := d;\nu := us;";
    assert_eq!(check(&prog(body)), vec![]);
}

#[test]
fn e0010_unknown_names() {
    let found = check(&prog("i := nope;\ni := tm.QQ;\n"));
    assert_eq!(found.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(), vec![("E0010", 10), ("E0010", 11)]);
    assert_eq!(found[0].2, "'nope' is not declared");
    assert_eq!(found[1].2, "TON has no input or output 'QQ'");
}

#[test]
fn e0010_globals_need_var_external() {
    let text = "VAR_GLOBAL g : INT; END_VAR\nPROGRAM p\nVAR x : INT; END_VAR\nx := g;\nEND_PROGRAM\n\
                PROGRAM q\nVAR_EXTERNAL g : INT; END_VAR\nVAR y : INT; END_VAR\ny := g;\nEND_PROGRAM\n";
    let found = check(text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].2, "'g' is a global variable: declare it VAR_EXTERNAL to use it here");
}

#[test]
fn e0010_internals_of_a_function_block() {
    let text = "FUNCTION_BLOCK F\nVAR_OUTPUT q : BOOL; END_VAR\nVAR hidden : INT; END_VAR\nq := hidden > 0;\nEND_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR f : F; x : INT; b : BOOL; END_VAR\nf();\nb := f.q;\nx := f.hidden;\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0010", 10)]);
}

#[test]
fn e0011_unknown_types() {
    let text = "PROGRAM p\nVAR a : TIMER; b : ARRAY[1..2] OF Nope; c : TON; END_VAR\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0011", 2), ("E0011", 2)]);
}

#[test]
fn e0012_duplicates_ignore_case() {
    let text = "PROGRAM p\nVAR count : INT; Count : DINT; END_VAR\nEND_PROGRAM\nFUNCTION_BLOCK P\nEND_FUNCTION_BLOCK\n";
    assert_eq!(codes(text), vec![("E0012", 2), ("E0012", 4)]);
}

#[test]
fn e0013_conversions_follow_figure_11() {
    let found = check(&prog("i := d;\nr := d;\nbt := w;\ni := r;\nb := i;\ns := ws;\nt := d;\n"));
    let messages: Vec<&str> = found.iter().map(|(_, _, m)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "DINT does not convert to INT implicitly: use DINT_TO_INT",
            "DINT does not convert to REAL implicitly: use DINT_TO_REAL",
            "WORD does not convert to BYTE implicitly: use WORD_TO_BYTE",
            "REAL does not convert to INT implicitly: use REAL_TO_INT",
            "INT does not convert to BOOL",
            "WSTRING does not convert to STRING: use WSTRING_TO_STRING",
            "DINT does not convert to TIME",
        ]
    );
    assert!(found.iter().all(|(c, _, _)| *c == "E0013"));
}

#[test]
fn e0014_operands() {
    let found = check(&prog("i := i + b;\nr := r + d;\ni := i AND 1;\ni := i MOD r;\nb := arr[r] > 0;\nb := i.3;\n"));
    assert_eq!(
        found.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(),
        vec![("E0014", 10), ("E0014", 11), ("E0014", 12), ("E0014", 13), ("E0014", 14), ("E0014", 15),]
    );
    assert_eq!(found[1].2, "+ takes two numbers of one type, not REAL and DINT: convert one side, e.g. DINT_TO_REAL");
}

#[test]
fn e0015_what_cannot_be_assigned() {
    let found = codes(&prog("K := 4;\ntm.Q := TRUE;\n%IX0.0 := TRUE;\n"));
    assert_eq!(found, vec![("E0015", 10), ("E0015", 11), ("E0015", 12)]);
}

#[test]
fn e0016_calls() {
    let text = "FUNCTION F : INT\nVAR_INPUT a, b : INT; END_VAR\nF := a + b;\nEND_FUNCTION\n\
                PROGRAM p\nVAR t : TON; x : INT; q : BOOL; END_VAR\n\
                x := F(1, b := 2);\nx := F(1);\nx := F(a := 1, c := 2);\nTON(IN := q);\nt(IN := q, Q := q);\nx := F(1, 2);\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0016", 7), ("E0016", 8), ("E0016", 9), ("E0016", 10), ("E0016", 11)]);
}

#[test]
fn e0017_external_must_match_global() {
    let text = "VAR_GLOBAL g : INT; END_VAR\nVAR_GLOBAL CONSTANT k : INT := 1; END_VAR\n\
                PROGRAM p\nVAR_EXTERNAL nope : INT; g : DINT; k : INT; END_VAR\nEND_PROGRAM\n";
    let found = check(text);
    assert_eq!(
        found.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(),
        vec![("E0017", 4), ("E0017", 4), ("E0017", 4)]
    );
    assert_eq!(found[1].2, "global 'g' is INT, so its VAR_EXTERNAL must be INT, not DINT");
    assert_eq!(found[2].2, "global 'k' is CONSTANT: declare it VAR_EXTERNAL CONSTANT");
}

#[test]
fn e0018_reserved_names() {
    let found = check("PROGRAM p\nVAR dt : TIME; ton : BOOL; lt : INT; ok : INT; END_VAR\nEND_PROGRAM\n");
    let messages: Vec<&str> = found.iter().map(|(_, _, m)| m.as_str()).collect();
    assert_eq!(
        messages,
        [
            "'dt' is reserved: it names a data type (Ed.3 6.1.3); choose another name",
            "'ton' is reserved: it names a standard function block (Ed.3 6.1.3); choose another name",
            "'lt' is reserved: it names a standard function (Ed.3 6.1.3); choose another name",
        ]
    );
}

#[test]
fn e0019_conditions_are_bool() {
    assert_eq!(
        codes(&prog("IF i THEN ; END_IF;\nWHILE w DO ; END_WHILE;\nREPEAT ; UNTIL 1 END_REPEAT;\n")),
        vec![("E0019", 10), ("E0019", 11), ("E0019", 12),]
    );
}

#[test]
fn e0020_literal_ranges() {
    assert_eq!(
        codes(&prog("si := 200;\nus := -1;\ni := INT#40000;\nb := 2;\n")),
        vec![("E0020", 10), ("E0020", 11), ("E0020", 12), ("E0020", 13),]
    );
    // DATE, TOD and DT as iecf holds them (docs/dependencies.md): from
    // 1970 to 2106, whole milliseconds in a TOD, whole seconds in a DT.
    assert_eq!(
        codes(&prog(
            "b := D#1969-12-31 = D#2023-02-29;\nb := TOD#24:00:00 > TOD#12:00:00.0001;\n\
             b := DT#2026-11-07-12:34:56.5 = DT#2026-11-07-12:34:56.000;\nb := D#2106-02-07 > D#1970-01-01;\n"
        )),
        vec![("E0020", 10), ("E0020", 10), ("E0020", 11), ("E0020", 11), ("E0020", 12)]
    );
}

#[test]
fn e0021_exit_outside_a_loop() {
    assert_eq!(
        codes(&prog("EXIT;\nFOR i := 1 TO 2 DO EXIT; END_FOR;\nCONTINUE;\n")),
        vec![("E0021", 10), ("E0021", 12)]
    );
}

#[test]
fn sfc_names() {
    let text = "PROGRAM p\nVAR go, flag : BOOL; END_VAR\n\
                INITIAL_STEP A: Run(N); flag(N); Missing(N); END_STEP\n\
                STEP B: END_STEP\n\
                TRANSITION FROM A TO C := go; END_TRANSITION\n\
                TRANSITION FROM B TO A := B.X AND B.T > T#1s; END_TRANSITION\n\
                ACTION Run: go := NOT A.X; END_ACTION\n\
                END_PROGRAM\n";
    let found = check(text);
    assert_eq!(found.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(), vec![("E0010", 3), ("E0010", 5)]);
    assert_eq!(found[0].2, "no action or BOOL variable 'Missing'");
    assert_eq!(found[1].2, "no step 'C'");
}

#[test]
fn configuration() {
    let text = "PROGRAM p\nEND_PROGRAM\nFUNCTION f : INT\nf := 1;\nEND_FUNCTION\n\
                CONFIGURATION C\nRESOURCE R ON PLC\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 0);\nTASK u(INTERVAL := 10, SPEED := 1);\n\
                PROGRAM a WITH t : p;\nPROGRAM b WITH nope : f;\nPROGRAM c : missing;\n\
                END_RESOURCE\nEND_CONFIGURATION\n";
    assert_eq!(codes(text), vec![("E0013", 9), ("E0016", 9), ("E0010", 11), ("E0016", 11), ("E0010", 12)]);
}

#[test]
fn a_program_s_in_outs_are_connected() {
    // Ed.3 6.6.1.4 rule 2 and 6.8.2: each in-out of a program instance
    // connected with := to a variable of its type, not a constant.
    let text = "PROGRAM p\nVAR_IN_OUT n : INT; END_VAR\nn := n + 1;\nEND_PROGRAM\n\
                CONFIGURATION C\nVAR_GLOBAL g : INT; w : DINT; END_VAR\nVAR_GLOBAL CONSTANT k : INT := 1; END_VAR\n\
                RESOURCE R ON PLC\nTASK t(INTERVAL := T#10ms, PRIORITY := 0);\n\
                PROGRAM a WITH t : p(n := g);\nPROGRAM b WITH t : p;\nPROGRAM c WITH t : p(n := 3);\n\
                PROGRAM d WITH t : p(n := w);\nPROGRAM e WITH t : p(n := k);\nEND_RESOURCE\nEND_CONFIGURATION\n";
    let got = check(text);
    let at: Vec<(&str, usize)> = got.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(at, vec![("E0016", 11), ("E0015", 12), ("E0013", 12), ("E0013", 13), ("E0015", 14)], "{got:?}");
    assert!(got[0].2.contains("must be connected to a global variable"), "{got:?}");
}

#[test]
fn eno_in_a_body_without_declaring_it() {
    // Ed.3 6.6.1.5 rules 2 and 7: a FUNCTION or FUNCTION_BLOCK may read
    // and set ENO; a PROGRAM has none, nothing calls it with ENO.
    let text = "FUNCTION f : INT\nVAR_INPUT x : INT; END_VAR\nf := x;\nENO := x > 0;\nEND_FUNCTION\n\
                FUNCTION_BLOCK b\nVAR x : BOOL; END_VAR\nIF NOT ENO THEN x := TRUE; END_IF;\nENO := FALSE;\nEND_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR q : BOOL; END_VAR\nENO := q;\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0010", 13)]);
}

#[test]
fn one_error_not_a_cascade() {
    // `nope` is reported once; the expressions built on it say nothing.
    assert_eq!(
        codes(&prog("i := (nope + 1) * 2 - arr[nope];\nIF nope > 3 THEN ; END_IF;\n")),
        vec![("E0010", 10), ("E0010", 10), ("E0010", 11),]
    );
}

#[test]
fn names_are_seen_across_files() {
    let a = iecf_syntax::parse(FileId(0), "FUNCTION_BLOCK Motor\nVAR_INPUT run : BOOL; END_VAR\nEND_FUNCTION_BLOCK\n");
    let b = iecf_syntax::parse(FileId(1), "PROGRAM p\nVAR m : Motor; END_VAR\nm(run := TRUE);\nEND_PROGRAM\n");
    assert!(iecf_sema::check(&[&a.unit, &b.unit]).is_empty());
}

fn corpus() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../iecf-syntax/tests/corpus");
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("st" | "gvl")))
        .map(|p| (p.display().to_string(), std::fs::read_to_string(&p).unwrap()))
        .collect();
    files.sort();
    files
}

/// Each corpus file is its own project (several have a PROGRAM main),
/// checked with the corpus's global variable list beside it.
#[test]
fn the_corpus_checks_clean() {
    let files = corpus();
    let (gvl_name, gvl) = files.iter().find(|(n, _)| n.ends_with(".gvl")).expect("a .gvl in the corpus");
    let globals = iecf_syntax::parse(FileId(1), gvl);
    for (name, text) in files.iter().filter(|(n, _)| n != gvl_name) {
        let parsed = iecf_syntax::parse(FileId(0), text);
        let found = iecf_sema::check(&[&parsed.unit, &globals.unit]);
        assert!(found.is_empty(), "{name}: {found:#?}");
    }
}

/// Half-typed text reaches the checker too: no truncation of any corpus
/// file may make it panic.
#[test]
fn every_truncation_checks() {
    for (name, text) in corpus() {
        for (cut, _) in text.char_indices().step_by(3) {
            let parsed = iecf_syntax::parse(FileId(0), &text[..cut]);
            for d in iecf_sema::check(&[&parsed.unit]) {
                assert!(d.span.end as usize <= cut, "{name} cut at {cut}: {d:?}");
            }
        }
    }
}

/// Ed.3 6.5.5.4 and 6.8.2: `*` addresses completed by VAR_CONFIG. Line
/// numbers: the configuration starts on line 12.
const PARTLY: &str = "TYPE hw : STRUCT in AT %I* : BOOL; spare : INT; END_STRUCT; END_TYPE\n\
    FUNCTION_BLOCK valve\n\
    VAR coil AT %Q* : BOOL; open AT %I* : BOOL; END_VAR\n\
    VAR_OUTPUT done : BOOL; END_VAR\n\
    VAR_INPUT cmd : BOOL; END_VAR\n\
    END_FUNCTION_BLOCK\n\
    PROGRAM line\n\
    VAR pump : valve; panel : hw; count : INT; END_VAR\n\
    VAR CONSTANT k : INT := 1; END_VAR\n\
    pump(cmd := panel.in); count := count + k;\n\
    END_PROGRAM\n";

fn partly(config: &str) -> Vec<(&'static str, usize, String)> {
    check(&format!("{PARTLY}{config}"))
}

#[test]
fn var_config_completes_every_partly_address() {
    let config = "CONFIGURATION plant\n\
                  RESOURCE cpu ON PLC\n\
                  TASK fast (INTERVAL := T#10ms, PRIORITY := 1);\n\
                  PROGRAM main WITH fast : line;\n\
                  PROGRAM spare : line;\n\
                  END_RESOURCE\n\
                  VAR_CONFIG\n\
                  cpu.main.pump.coil AT %QX0.0 : BOOL;\n\
                  CPU.MAIN.PUMP.OPEN AT %IX0.1 : BOOL;\n\
                  cpu.main.panel.in AT %IX0.2 : BOOL;\n\
                  cpu.main.count : INT := 100;\n\
                  cpu.main.pump : valve := (cmd := TRUE);\n\
                  cpu.spare.pump.coil AT %QX1.0 : BOOL;\n\
                  cpu.spare.pump.open AT %IX1.1 : BOOL;\n\
                  END_VAR\n\
                  END_CONFIGURATION\n";
    let found = partly(config);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "E0023");
    assert_eq!(found[0].1, 16, "at the instance that lacks it");
    assert_eq!(
        found[0].2,
        "'cpu.spare.panel.in' is declared AT %I* and VAR_CONFIG gives it no address: add `cpu.spare.panel.in AT %I...: BOOL;`"
    );
}

#[test]
fn var_config_without_named_resources() {
    let config = "CONFIGURATION plant\n\
                  PROGRAM main : line;\n\
                  VAR_CONFIG\n\
                  main.pump.coil AT %QX0.0 : BOOL;\n\
                  main.pump.open AT %IX0.1 : BOOL;\n\
                  main.panel.in AT %IX0.2 : BOOL;\n\
                  END_VAR\n\
                  END_CONFIGURATION\n";
    assert_eq!(partly(config), vec![]);
}

#[test]
fn e0024_entries_that_do_not_fit() {
    let config = "CONFIGURATION plant\n\
                  PROGRAM main : line;\n\
                  VAR_CONFIG\n\
                  main.pump.coil AT %IX0.0 : BOOL;\n\
                  main.pump.open AT %IX0.1 : BOOL;\n\
                  main.panel.in AT %IX0.2 : BOOL;\n\
                  main.count AT %MW4 : INT;\n\
                  main.k : INT := 2;\n\
                  main : line;\n\
                  END_VAR\n\
                  END_CONFIGURATION\n";
    let found = partly(config);
    let got: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(got, vec![("E0024", 15), ("E0024", 18), ("E0024", 19), ("E0024", 20)], "{found:?}");
    assert_eq!(found[0].2, "'main.pump.coil' is declared AT %Q*: its address is a %Q one");
}

#[test]
fn var_config_paths_and_types() {
    let config = "CONFIGURATION plant\n\
                  RESOURCE cpu ON PLC PROGRAM main : line; END_RESOURCE\n\
                  VAR_CONFIG\n\
                  cpu.main.pump.coil AT %QX0.0 : INT;\n\
                  cpu.main.pump.open AT %IX0.1 : BOOL;\n\
                  cpu.main.panel.in AT %IX0.2 : BOOL;\n\
                  gpu.main.count : INT;\n\
                  cpu.other.count : INT;\n\
                  cpu.main.nope : INT;\n\
                  cpu.main.count.x : INT;\n\
                  cpu.main.count : INT := 'text';\n\
                  cpu.main.count : INT;\n\
                  END_VAR\n\
                  END_CONFIGURATION\n";
    let found = partly(config);
    let got: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        got,
        vec![("E0013", 15), ("E0010", 18), ("E0010", 19), ("E0010", 20), ("E0010", 21), ("E0013", 22), ("E0012", 23)],
        "{found:?}"
    );
}

#[test]
fn e0022_where_a_partly_address_may_not_stand() {
    let text = "FUNCTION_BLOCK a\n\
                VAR_INPUT x AT %I* : BOOL; END_VAR\n\
                VAR_TEMP y AT %Q* : BOOL; END_VAR\n\
                VAR AT %M* : BOOL; END_VAR\n\
                VAR_OUTPUT o AT %Q* : BOOL; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK b\n\
                VAR z AT %Q* : BOOL; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION f : BOOL\n\
                VAR v AT %IX0.0 : BOOL; END_VAR\n\
                f := v;\n\
                END_FUNCTION\n\
                FUNCTION_BLOCK c\n\
                VAR_INPUT one : b; END_VAR\n\
                VAR many : ARRAY[1..2] OF b; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                VAR_GLOBAL g AT %I* : BOOL; gb : b; END_VAR\n";
    let got = codes(text);
    assert_eq!(
        got,
        vec![
            ("E0022", 2),
            ("E0022", 3),
            ("E0022", 4),
            ("E0022", 5),
            ("E0022", 11),
            ("E0022", 15),
            ("E0022", 16),
            ("E0022", 18),
            ("E0022", 18)
        ]
    );
}

#[test]
fn e0022_a_var_block_is_plain_located_or_partly() {
    let text = "PROGRAM p\n\
                VAR n : INT; lamp AT %QX0.0 : BOOL; END_VAR\n\
                VAR a AT %IX0.0 : BOOL; b AT %IX0.1 : BOOL; END_VAR\n\
                VAR m : INT; k : INT; END_VAR\n\
                END_PROGRAM\n\
                FUNCTION_BLOCK f\n\
                VAR q AT %Q* : BOOL; r : BOOL; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                VAR_GLOBAL g : INT; h AT %QX1.0 : BOOL; END_VAR\n";
    assert_eq!(codes(text), vec![("E0022", 2), ("E0022", 7)]);
}

#[test]
fn a_partly_address_in_a_body_is_an_error() {
    let text = "PROGRAM p\nVAR x : BOOL; END_VAR\nx := %I*;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let ids: Vec<_> = parsed.diagnostics.iter().map(|d| d.code.id()).collect();
    assert_eq!(ids, vec!["E0022"]);
}

#[test]
fn implicit_externals_extension() {
    use iecf_sema::{Extension, Options};
    let text = "VAR_GLOBAL g : INT; tick : TON; END_VAR\n\
                VAR_GLOBAL CONSTANT LIMIT_HI : INT := 9; END_VAR\n\
                PROGRAM p\n\
                VAR x : INT; END_VAR\n\
                x := g + LIMIT_HI;\n\
                tick(IN := TRUE, PT := T#1s);\n\
                LIMIT_HI := 1;\n\
                x := nope;\n\
                END_PROGRAM\n\
                PROGRAM q\n\
                VAR g : BOOL; END_VAR\n\
                g := TRUE;\n\
                END_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lines = |ds: Vec<iecf_diagnostics::Diagnostic>| {
        ds.iter().map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1)).collect::<Vec<_>>()
    };
    // The standard: every use of a global without VAR_EXTERNAL is E0010.
    let strict = lines(iecf_sema::check(&[&parsed.unit]));
    assert_eq!(strict, vec![("E0010", 5), ("E0010", 5), ("E0010", 6), ("E0010", 7), ("E0010", 8)]);
    // The extension: globals are seen, CONSTANT still holds, a local
    // still hides a global, an unknown name is still unknown.
    let options = Options::default().with(Extension::ImplicitExternals);
    let ext = lines(iecf_sema::check_with(&[&parsed.unit], &options));
    assert_eq!(ext, vec![("E0015", 7), ("E0010", 8)]);
    assert_eq!(Extension::from_name("IMPLICIT-externals"), Some(Extension::ImplicitExternals));
}

#[test]
fn standard_functions_by_their_tables() {
    // Ed.3 Tables 22-37, each family called as the standard writes it.
    let body = "r := ABS(r);\nlr := SQRT(lr);\nr := ATAN2(Y := r, X := 1.0);\ni := ADD(i, 1, i);\n\
                d := MUL(IN1 := d, IN2 := i);\nt := SUB(t, T#1s);\nt := DIV(t, 2);\ni := MOD(i, 3);\n\
                r := EXPT(r, i);\nw := SHL(IN := w, N := 3);\nw := AND(w, bt, 16#0F);\nw := NOT(w);\n\
                i := SEL(G := b, IN0 := i, IN1 := 5);\nd := MAX(i, d, 7);\ni := LIMIT(MN := 0, IN := i, MX := 9);\n\
                i := MUX(K := 2, IN0 := 1, IN2 := 2, IN4 := i);\nb := GT(i, 3, 1);\nb := NE(s, 'x');\n\
                i := LEN(s);\ns := LEFT(IN := s, L := 3);\ns := MID(s, 2, 2);\ns := CONCAT('a', s, 'b');\n\
                s := INSERT(IN1 := s, IN2 := 'xy', P := 2);\ns := REPLACE(s, 'x', 1, 2);\ni := FIND(s, 'b');\n\
                t := MUL_TIME(t, 2.5);\nus := DAY_OF_WEEK(D#2026-11-05);\nr := INT_TO_REAL(i);\n\
                d := TO_DINT(w);\ni := TRUNC_INT(r);\ns := INT_TO_STRING(i);\nlr := STRING_TO_LREAL(s);\n\
                i := ADD_INT(i, si);\nr := ADD_REAL(i, r);\nr := MUX_UINT_REAL(u, r, 1.0);\nw := MOVE(w);";
    assert_eq!(check(&prog(body)), vec![]);
}

#[test]
fn trunc_alone_is_deprecated_and_gives_dint() {
    // Ed.3 Table 22 2a; DINT as CODESYS, TwinCAT and Siemens (docs/dependencies.md).
    assert_eq!(
        codes(&prog("d := TRUNC(r);\nq := trunc(lr);\ni := TRUNC(r);")),
        vec![("W0003", 10), ("W0003", 11), ("W0003", 12), ("E0013", 12),]
    );
    assert_eq!(codes(&prog("d := TRUNC(i);")), vec![("W0003", 10), ("E0014", 10)], "an integer is not ANY_REAL");
}

#[test]
fn standard_functions_split_takes_outputs() {
    let text = "PROGRAM p\nVAR d : DATE; y : UINT; m, dd : USINT; r : REAL; END_VAR\n\
                SPLIT_DATE(d, y, m, dd);\n\
                SPLIT_DATE(IN := d, YEAR => y, MONTH => m, DAY => dd);\n\
                SPLIT_DATE(IN := d, YEAR => r);\n\
                SPLIT_DATE(IN := d, YEAR := y);\n\
                END_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0014", 5), ("E0016", 6)]);
}

#[test]
fn standard_functions_wrong_calls() {
    let body = "r := SQRT(i);\nr := SQRT(2) + ATAN2(1, r);\ni := ADD(i);\ni := SEL(i, 1, 2);\ni := MAX(i, s);\n\
                s := CONCAT(s, ws);\nw := SHL(w, r);\ni := LIMIT(IN := i, LO := 0);\ni := ABS(i, i);\n\
                i := MOD(r, 2);\nd := TIME_TO_DINT(t);\ni := ADD_INT(i, d);\nb := SEL(b, IN0 := b, IN1 := b);";
    let found = check(&prog(body));
    let got: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        got,
        vec![
            ("E0014", 10),
            ("E0016", 12),
            ("E0013", 13),
            ("E0014", 14),
            ("E0014", 15),
            ("E0014", 16),
            ("E0016", 17),
            ("E0016", 18),
            ("E0014", 19),
            ("E0013", 20),
            ("E0013", 21),
            ("E0016", 22)
        ],
        "{found:#?}"
    );
    assert_eq!(found[0].2, "IN of SQRT takes ANY_REAL, not INT");
    assert_eq!(
        found[4].2,
        "the inputs of CONCAT share one type, and STRING and WSTRING have none (Ed.3 6.6.2.5.11: STRING and WSTRING do not mix)"
    );
    assert_eq!(
        found[9].2,
        "Ed.3 Figure 11 defines no conversion from TIME to DINT (CODESYS, TwinCAT and Siemens have it, in milliseconds: --extension time-integer-conversions)"
    );
}

#[test]
fn var_config_list_extension() {
    use iecf_sema::{Extension, Options};
    // The CODESYS help's own example (Variable Configuration, VAR_CONFIG).
    let text = "FUNCTION_BLOCK locio\n\
                VAR xLocIn AT %I* : BOOL; xLocOut AT %Q* : BOOL; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM PLC_PRG\n\
                VAR locioVar1 : locio; locioVar2 : locio; END_VAR\n\
                END_PROGRAM\n\
                VAR_CONFIG\n\
                PLC_PRG.locioVar1.xLocIn AT %IX1.0 : BOOL;\n\
                PLC_PRG.locioVar1.xLocOut AT %QX0.0 : BOOL;\n\
                PLC_PRG.locioVar2.xLocIn AT %IX1.0 : BOOL;\n\
                END_VAR\n\
                VAR_CONFIG\n\
                PLC_PRG.locioVar2.xLocOut AT %QX0.3 : BOOL;\n\
                Other.x AT %QX0.4 : BOOL;\n\
                END_VAR\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lines = |ds: Vec<iecf_diagnostics::Diagnostic>| {
        ds.iter().map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1)).collect::<Vec<_>>()
    };
    // The standard: VAR_CONFIG belongs in a CONFIGURATION.
    assert_eq!(lines(iecf_sema::check(&[&parsed.unit])), vec![("E0025", 7), ("E0025", 12)]);
    // The extension: read as CODESYS reads it.
    let on = Options::default().with(Extension::VarConfigList);
    assert_eq!(lines(iecf_sema::check_with(&[&parsed.unit], &on)), vec![("E0010", 14)]);
    // A * left without an address is reported at the PROGRAM.
    let short = text.replace("PLC_PRG.locioVar2.xLocOut AT %QX0.3 : BOOL;\n", "");
    let parsed = iecf_syntax::parse(FileId(0), &short);
    let found = iecf_sema::check_with(&[&parsed.unit], &on);
    let codes: Vec<_> = found.iter().map(|d| d.code.id()).collect();
    assert_eq!(codes, ["E0023", "E0010"]);
    assert!(found[0].message.starts_with("'PLC_PRG.locioVar2.xLocOut' is declared AT %Q*"), "{}", found[0].message);
    // With a CONFIGURATION in the project, VAR_CONFIG goes inside it.
    let with_config = format!("{text}CONFIGURATION c\nPROGRAM PLC_PRG : PLC_PRG;\nEND_CONFIGURATION\n");
    let parsed = iecf_syntax::parse(FileId(0), &with_config);
    let codes: Vec<_> = iecf_sema::check_with(&[&parsed.unit], &on).iter().map(|d| d.code.id()).collect();
    assert_eq!(codes, ["E0025", "E0025", "E0023", "E0023", "E0023", "E0023"]);
}

#[test]
fn time_integer_conversions_extension() {
    use iecf_sema::{Extension, Options};
    let text = prog(
        "d := TIME_TO_DINT(t);\nt := DINT_TO_TIME(d);\nt := TO_TIME(u);\nw := TIME_TO_WORD(t);\nb := TIME_TO_BOOL(t);",
    );
    let parsed = iecf_syntax::parse(FileId(0), &text);
    let codes = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(|d| d.code.id()).collect::<Vec<_>>();
    assert_eq!(codes(&Options::default()), ["E0013"; 5]);
    let on = Options::default().with(Extension::TimeIntegerConversions);
    assert_eq!(codes(&on), ["E0013"], "TIME to BOOL is no number");
}

#[test]
fn w0004_an_address_s_size_prefix_against_its_type() {
    // Ed.3 6.5.5.3: the address is where it starts; warned when the size
    // prefix is not the type's size. Only a BOOL starts at a bit.
    let text = "PROGRAM p\n\
                VAR\n\
                \x20 t AT %MD0 : TIME; l AT %ML8 : LTIME; stamp AT %MD16 : DT; lstamp AT %ML24 : LDT;\n\
                \x20 w AT %MW40 : TIME;\n\
                \x20 r AT %IB12 : REAL;\n\
                \x20 x AT %QB3 : BOOL;\n\
                \x20 y AT %QW4 : BOOL;\n\
                \x20 n AT %MX50.3 : INT;\n\
                \x20 c AT %ML60 : TOD;\n\
                END_VAR\n\
                END_PROGRAM\n";
    let found = check(text);
    let at: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(at, vec![("W0004", 4), ("W0004", 5), ("W0004", 6), ("W0004", 7), ("E0022", 8), ("W0004", 9)]);
    assert_eq!(found[0].2, "TIME at %MW40 takes bytes 40 to 43: TIME is 4 bytes, W names 2 bytes");
    assert_eq!(found[1].2, "REAL at %IB12 takes bytes 12 to 15: REAL is 4 bytes, B names 1 byte");
    assert_eq!(found[2].2, "a BOOL at %QB3 takes byte 3 whole: TRUE is 1, and any value but 0 reads TRUE");
    assert_eq!(found[4].2, "INT at %MX50.3: only a BOOL starts at a bit; a 16-bit value starts at a byte");
    assert_eq!(found[5].2, "TOD at %ML60 takes bytes 60 to 63: TOD is 4 bytes, L names 8 bytes");
}

#[test]
fn w0004_on_the_address_var_config_completes() {
    let text = "PROGRAM line\n\
                VAR t AT %M* : TIME; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                RESOURCE cpu ON PLC\n\
                TASK fast (INTERVAL := T#10ms);\n\
                PROGRAM main WITH fast : line;\n\
                END_RESOURCE\n\
                VAR_CONFIG\n\
                cpu.main.t AT %MW4 : TIME;\n\
                END_VAR\n\
                END_CONFIGURATION\n";
    assert_eq!(codes(text), vec![("W0004", 10)]);
    assert_eq!(codes(&text.replace("%MW4", "%MD4")), vec![]);
}

#[test]
fn bit_string_to_bool_extension() {
    use iecf_sema::{Extension, Options};
    let text = prog("b := BYTE_TO_BOOL(bt);\nb := WORD_TO_BOOL(w);\nb := TO_BOOL(w);\nb := INT_TO_BOOL(i);");
    let parsed = iecf_syntax::parse(FileId(0), &text);
    let codes = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(|d| d.code.id()).collect::<Vec<_>>();
    assert_eq!(codes(&Options::default()), ["E0013"; 4]);
    let on = Options::default().with(Extension::BitStringToBool);
    assert_eq!(codes(&on), ["E0013"], "an integer is no bit string");
}

#[test]
fn an_array_or_structure_at_an_address() {
    // From a byte, its parts back to back; the size prefix compared with
    // its first element's or field's (a BOOL a byte).
    let text = "TYPE pt : STRUCT ok : BOOL; x : INT; END_STRUCT; END_TYPE\n\
                PROGRAM p\n\
                VAR\n\
                \x20 a AT %QW6 : ARRAY[0..9] OF INT;\n\
                \x20 b AT %QB0 : ARRAY[0..9] OF INT;\n\
                \x20 c AT %MB0 : pt;\n\
                \x20 d AT %MW4 : pt;\n\
                \x20 e AT %MX9.0 : ARRAY[0..1] OF BOOL;\n\
                \x20 s AT %MB20 : STRING[4];\n\
                \x20 g AT %MW30 : ARRAY[0..1, 0..1] OF pt;\n\
                END_VAR\n\
                END_PROGRAM\n";
    let found = check(text);
    let at: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(at, vec![("W0004", 5), ("W0004", 7), ("E0022", 8), ("W0004", 10)]);
    assert_eq!(found[0].2, "ARRAY[0..9] OF INT at %QB0: its first part is INT, 2 bytes, and B names 1 byte");
    assert_eq!(found[2].2, "ARRAY[0..1] OF BOOL at %MX9.0: an array, structure or string starts at a byte");
}

#[test]
fn two_arrays_are_one_type_only_with_the_same_bounds() {
    let text = "PROGRAM p\n\
                VAR a : ARRAY[0..3] OF INT; b : ARRAY[1..4] OF INT; c : ARRAY[0..3] OF INT; END_VAR\n\
                a := c;\n\
                a := b;\n\
                END_PROGRAM\n";
    let found = check(text);
    assert_eq!(found.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(), vec![("E0013", 4)]);
    assert_eq!(found[0].2, "ARRAY[1..4] OF INT does not convert to ARRAY[0..3] OF INT");
}

#[test]
fn ref_takes_a_variable_that_outlives_its_pou() {
    // Ed.3 6.4.4.10.3: not a temporary; and iecf's own rules
    // (docs/dependencies.md): not a CONSTANT, not a block's output from
    // outside; an input address warns.
    let text = "VAR_GLOBAL gin AT %IW0 : INT; END_VAR\n\
                VAR_GLOBAL CONSTANT gc : INT := 1; END_VAR\n\
                FUNCTION f : INT\n\
                VAR_INPUT i : INT; END_VAR\n\
                VAR r : REF_TO INT; END_VAR\n\
                r := REF(i);\n\
                f := 0;\n\
                END_FUNCTION\n\
                FUNCTION_BLOCK fb\n\
                VAR_INPUT x : INT; END_VAR\n\
                VAR_OUTPUT q : INT; END_VAR\n\
                VAR_TEMP t : INT; END_VAR\n\
                VAR r : REF_TO INT; w : WORD; END_VAR\n\
                r := REF(t);\n\
                r := REF(x);\n\
                r := REF(ENO);\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM p\n\
                VAR_EXTERNAL gin : INT; END_VAR\n\
                VAR_EXTERNAL CONSTANT gc : INT; END_VAR\n\
                VAR i : fb; r : REF_TO INT; w : WORD; END_VAR\n\
                r := REF(gin);\n\
                r := REF(gc);\n\
                r := REF(i.q);\n\
                r := REF(i.x);\n\
                r := REF(3);\n\
                r := REF(w.3);\n\
                END_PROGRAM\n";
    let found = check(text);
    let at: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        at,
        vec![
            ("E0026", 6),
            ("E0026", 14),
            ("E0026", 16),
            ("W0005", 22),
            ("E0026", 23),
            ("E0026", 24),
            ("E0026", 26),
            ("E0026", 27),
        ]
    );
    assert!(found[0].2.starts_with("'i' is a variable of a FUNCTION, gone when the POU returns"), "{}", found[0].2);
    assert!(found[1].2.starts_with("'t' is a VAR_TEMP"), "{}", found[1].2);
    assert_eq!(found[4].2, "'gc' is CONSTANT: a reference could write it");
    assert_eq!(found[5].2, "q is an output of FB: only the block writes it");
}

#[test]
fn a_reference_is_not_in_out_retain_or_at_an_address() {
    let text = "TYPE pr : STRUCT x : INT; r : REF_TO INT; END_STRUCT; END_TYPE\n\
                FUNCTION_BLOCK holder\n\
                VAR_IN_OUT io : REF_TO INT; END_VAR\n\
                VAR r : REF_TO INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM p\n\
                VAR RETAIN rr : REF_TO INT; s : pr; h : holder; n : INT; END_VAR\n\
                VAR loc AT %MW0 : pr; END_VAR\n\
                END_PROGRAM\n";
    let at: Vec<_> = check(text).into_iter().filter(|(c, _, _)| *c != "W0004").map(|(c, l, _)| (c, l)).collect();
    assert_eq!(at, vec![("E0027", 3), ("E0027", 7), ("E0027", 7), ("E0027", 7), ("E0027", 8)]);
}

#[test]
fn references_compare_with_eq_and_ne_only() {
    let text = "TYPE node : STRUCT v : INT; next : REF_TO node; END_STRUCT; END_TYPE\n\
                PROGRAM p\n\
                VAR a : INT; r, s : REF_TO INT; d : REF_TO DINT; b : BOOL; n : node; END_VAR\n\
                b := r = s;\n\
                b := r <> NULL;\n\
                b := NULL = r;\n\
                b := r < s;\n\
                b := r = d;\n\
                r := d;\n\
                r := REF(a);\n\
                n.next := REF(n);\n\
                a := n.next^.v + r^;\n\
                END_PROGRAM\n";
    let at: Vec<_> = codes(text);
    assert_eq!(at, vec![("E0014", 7), ("E0014", 8), ("E0013", 9)]);
}

/// `(code, line)` for each diagnostic with the reference-to extension on,
/// or off.
fn codes_ref(text: &str, on: bool) -> Vec<(&'static str, usize)> {
    use iecf_sema::{Extension, Options};
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(!parsed.diagnostics.iter().any(|d| d.is_error()), "parse: {:#?}", parsed.diagnostics);
    let o = if on { Options::default().with(Extension::ReferenceTo) } else { Options::default() };
    iecf_sema::check_with(&[&parsed.unit], &o)
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect()
}

#[test]
fn reference_to_extension() {
    let text = "TYPE pt : STRUCT x : INT; END_STRUCT; END_TYPE\n\
                FUNCTION_BLOCK fb\n\
                VAR_INPUT r : REFERENCE TO INT; END_VAR\n\
                r := r + 1;\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM p\n\
                VAR a : INT; d : DINT; s : pt; ra, rb : REFERENCE TO INT; rp : REFERENCE TO pt; f : fb; ok : BOOL; END_VAR\n\
                ra REF= a;\n\
                rb REF= ra;\n\
                ra := ra + 1;\n\
                rp REF= s;\n\
                rp.x := ra;\n\
                ok := __ISVALIDREF(rb);\n\
                f(r := a);\n\
                ra REF= 0;\n\
                END_PROGRAM\n";
    assert_eq!(codes_ref(text, true), vec![]);
    // Without it: the type unknown at each declaration, REF= not IEC.
    let off = codes_ref(text, false);
    assert!(off.contains(&("E0011", 3)) && off.contains(&("E0011", 7)), "{off:?}");
    assert!(off.contains(&("E0005", 8)) && off.contains(&("E0005", 15)), "{off:?}");
}

#[test]
fn reference_to_binds_what_ref_could_take() {
    let text = "FUNCTION_BLOCK fb\n\
                VAR_INPUT r : REFERENCE TO INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM p\n\
                VAR a : INT; d : DINT; ra : REFERENCE TO INT; rd : REFERENCE TO DINT; f : fb; ok : BOOL; END_VAR\n\
                VAR CONSTANT k : INT := 1; END_VAR\n\
                ra REF= d;\n\
                ra REF= rd;\n\
                ra REF= k;\n\
                ra REF= a + 1;\n\
                f(r := d);\n\
                a REF= a;\n\
                ok := __ISVALIDREF(a);\n\
                END_PROGRAM\n";
    assert_eq!(
        codes_ref(text, true),
        vec![("E0013", 7), ("E0013", 8), ("E0026", 9), ("E0026", 10), ("E0013", 11), ("E0013", 12), ("E0014", 13)]
    );
}

#[test]
fn reference_to_where_it_cannot_be() {
    let text = "FUNCTION_BLOCK fb\n\
                VAR_IN_OUT io : REFERENCE TO INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM p\n\
                VAR a : INT; END_VAR\n\
                VAR arr : ARRAY[0..1] OF REFERENCE TO INT; END_VAR\n\
                VAR rr : REF_TO REFERENCE TO INT; END_VAR\n\
                VAR r2 : REFERENCE TO REF_TO INT; END_VAR\n\
                VAR RETAIN kept : REFERENCE TO INT; END_VAR\n\
                VAR init : REFERENCE TO INT := a; END_VAR\n\
                END_PROGRAM\n";
    assert_eq!(
        codes_ref(text, true),
        vec![("E0027", 2), ("E0027", 6), ("E0027", 7), ("E0027", 8), ("E0027", 9), ("E0013", 10)]
    );
}

#[test]
fn a_short_date_meets_a_long_operand_widened() {
    // Ed.3 6.6.1.6, rule 7: the short date widened, so the result is long
    // and cannot go back into the short type implicitly.
    let text = "PROGRAM p\n\
                VAR dt1 : DT; ldt1 : LDT; t : TIME; ltm : LTIME; tod1 : TOD; ltod1 : LTOD; d1 : DATE; ld1 : LDATE; END_VAR\n\
                ldt1 := dt1 + ltm;\n\
                ltm := ldt1 - dt1;\n\
                ltm := dt1 - ldt1;\n\
                ltod1 := tod1 - ltm;\n\
                ltm := ld1 - d1;\n\
                dt1 := dt1 + t;\n\
                dt1 := dt1 + ltm;\n\
                tod1 := tod1 + ltm;\n\
                t := ldt1 - dt1;\n\
                END_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0013", 9), ("E0013", 10), ("E0013", 11)]);
}

#[test]
fn signed_and_unsigned_meet_in_the_next_signed_type() {
    // As Siemens SCL (docs/dependencies.md): the narrowest signed
    // integer both widen to; none for ULINT and LINT.
    let text = "PROGRAM p\n\
                VAR i : INT; ui : UINT; si : SINT; us : USINT; d : DINT; ud : UDINT; l : LINT; ul : ULINT; ok : BOOL; END_VAR\n\
                d := i + ui;\n\
                i := si + us;\n\
                l := d * ud;\n\
                ok := i < ui;\n\
                d := MAX(i, ui);\n\
                d := si - ui;\n\
                i := i + ui;\n\
                l := ul + l;\n\
                END_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0013", 9), ("E0014", 10)]);
}

#[test]
fn a_typed_function_takes_what_widens_to_its_type() {
    // Ed.3 6.6.2.4: the typed function's inputs are its type, reached
    // by an implicit conversion; LEN has no typed form (no common input).
    let text = "PROGRAM p\n\
                VAR i : INT; d : DINT; r : REAL; lr : LREAL; bt : BYTE; w : WORD; s : STRING; k : UINT; END_VAR\n\
                d := ADD_DINT(i, d, 3);\n\
                w := NOT_WORD(bt);\n\
                lr := MUX_UINT_LREAL(k, r, lr);\n\
                i := ADD_INT(d, i);\n\
                r := SQRT_REAL(lr);\n\
                i := LEN_STRING(s);\n\
                END_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0013", 6), ("E0013", 7), ("E0010", 8)]);
}

#[test]
fn a_char_literal_is_one_character() {
    // Ed.3 Table 6: CHAR#'X' (3b), 'X' a CHAR where one is wanted (1b);
    // Table 5: CHAR#16#41.
    let text = "PROGRAM p\n\
                VAR c : CHAR; s : STRING; ok : BOOL; END_VAR\n\
                c := CHAR#'a';\n\
                c := 'b';\n\
                c := CHAR#16#41;\n\
                ok := c = 'a';\n\
                s := c;\n\
                c := CHAR#'ab';\n\
                c := 'ab';\n\
                c := CHAR#16#100;\n\
                ok := c = 'ab';\n\
                c := s;\n\
                END_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0020", 8), ("E0013", 9), ("E0020", 10), ("E0014", 11), ("E0013", 12)]);
}

#[test]
fn a_wchar_literal_is_one_utf16_unit() {
    // Ed.3 Table 6: WCHAR#"X" and WCHAR#'X' (4b, 4d), "X" a WCHAR where
    // one is wanted (2b), never a CHAR; a character above U+FFFF is two
    // units (docs/dependencies.md); WSTRING_TO_WCHAR as Table 27 row 2.
    let text = "PROGRAM p\n\
                VAR c : CHAR; wc : WCHAR; ws : WSTRING; s : STRING; ok : BOOL; END_VAR\n\
                wc := WCHAR#\"a\";\n\
                wc := WCHAR#'b';\n\
                wc := \"$263A\";\n\
                wc := WCHAR#16#263A;\n\
                ok := wc = \"a\";\n\
                ws := wc;\n\
                ws := WSTRING#'q$41';\n\
                wc := WSTRING_TO_WCHAR(ws);\n\
                ws := \"${1F600}\";\n\
                wc := \"${1F600}\";\n\
                wc := WCHAR#\"ab\";\n\
                wc := WCHAR#16#10000;\n\
                c := \"a\";\n\
                wc := 'a';\n\
                ok := wc = 'a';\n\
                s := STRING#\"a\";\n\
                ws := \"${D800}\";\n\
                END_PROGRAM\n";
    assert_eq!(
        codes(text),
        vec![
            ("E0013", 12),
            ("E0020", 13),
            ("E0020", 14),
            ("E0013", 15),
            ("E0013", 16),
            ("E0014", 17),
            ("E0020", 18),
            ("E0020", 19)
        ]
    );
}

#[test]
fn a_function_without_a_result_is_a_statement() {
    // Ed.3 6.6.2.1: "A function without result shall not be called
    // inside an expression"; its name holds nothing to assign.
    let text = "FUNCTION note\n\
                VAR_INPUT a : INT; END_VAR\n\
                VAR_OUTPUT q : INT; END_VAR\n\
                q := a;\n\
                note := 1;\n\
                END_FUNCTION\n\
                PROGRAM p\n\
                VAR k : INT; END_VAR\n\
                note(a := 1, q => k);\n\
                k := note(a := 1);\n\
                k := 1 + note(2);\n\
                END_PROGRAM\n";
    let found = check(text);
    let at: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(at, vec![("E0015", 5), ("E0016", 10), ("E0014", 11)]);
    assert!(found[0].2.starts_with("note has no result to assign"), "{}", found[0].2);
}

#[test]
fn a_character_of_a_string() {
    // Ed.3 6.4.2: s[i], from 1, a CHAR of a STRING; its error cases.
    let text = "FUNCTION f : INT\n\
                VAR_IN_OUT x : CHAR; END_VAR\n\
                f := 0;\n\
                END_FUNCTION\n\
                PROGRAM p\n\
                VAR s : STRING[10]; t : STRING; c : CHAR; a : ARRAY[0..1] OF STRING[4]; i : INT; r : REF_TO CHAR; ws : WSTRING; wc : WCHAR; END_VAR\n\
                c := s[2];\n\
                s[i] := 'x';\n\
                a[1][3] := a[0][i + 1];\n\
                t := s[1];\n\
                wc := ws[1];\n\
                c := s[0];\n\
                c := s[11];\n\
                c := s[1, 2];\n\
                c := s[REAL#1.0];\n\
                s[2] := t;\n\
                r := REF(s[1]);\n\
                i := f(x := s[1]);\n\
                i := f(s[1]);\n\
                END_PROGRAM\n";
    let found = check(text);
    let at: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        at,
        vec![
            ("E0014", 12),
            ("E0014", 13),
            ("E0014", 14),
            ("E0014", 15),
            ("E0013", 16),
            ("E0026", 17),
            ("E0015", 18),
            ("E0015", 19)
        ]
    );
    assert_eq!(found[0].2, "STRING[10] has characters 1..10, not 0");
    assert_eq!(found[2].2, "STRING[10] takes one position in [ ], not 2");
}

#[test]
fn named_values_are_used_as_their_base_type() {
    // Ed.3 6.4.4.3: other constants assigned, values from calculations;
    // an enumeration (6.4.4.2) takes its own values only.
    let text = "TYPE
  col : INT (red := 1, green := 5, blue) := green;
  lv : WORD (lo := 16#10, hi := 16#20);
  mode : (idle, run);
END_TYPE
FUNCTION_BLOCK B
VAR_IN_OUT io : col; END_VAR
END_FUNCTION_BLOCK
PROGRAM p
VAR c : col; x : INT; w : WORD; m : mode; b : B; mi : (low, top) := top; END_VAR
c := 27;
c := red + 1;
x := c + col#blue;
w := lv#hi OR lo;
b(io := c);
b(io := x);
CASE c OF red: x := 1; green - 3, blue: x := 2; END_CASE;
IF mi = low THEN mi := top; END_IF;
m := 1;
x := m;
m := idle + 1;
END_PROGRAM
";
    assert_eq!(codes(text), vec![("E0013", 19), ("E0013", 20), ("E0014", 21)]);
}

#[test]
fn a_named_value_is_a_constant_its_base_type_holds() {
    let text = "TYPE
  a : SINT (lo := 1, big := 200);
  b : USINT (top := 255, over);
  c : REAL (x := 1);
  d : INT (v := 1.5);
END_TYPE
";
    assert_eq!(codes(text), vec![("E0020", 2), ("E0020", 3), ("E0013", 4), ("E0013", 5)]);
}

#[test]
fn named_values_with_no_base_type_are_int() {
    // As CODESYS and TwinCAT have it (docs/dependencies.md): used with
    // an INT as it is; a value an INT cannot hold needs its type named.
    let text = "TYPE
  gear : (slow := 2, fast := 4);
  far : (away := 100000);
  wide : DINT (off := 100000);
END_TYPE
PROGRAM p
VAR g : gear; x : INT; d : DINT; w : wide; END_VAR
x := x + g;
d := w + 1;
x := w;
END_PROGRAM
";
    assert_eq!(codes(text), vec![("E0020", 3), ("E0013", 10)]);
}

#[test]
fn an_enumeration_has_no_more_values_than_an_int_numbers() {
    let many = |n: usize| (0..n).map(|i| format!("v{i}")).collect::<Vec<_>>().join(", ");
    let ok = format!("TYPE e : ({}); END_TYPE\n", many(32768));
    assert_eq!(codes(&ok), vec![]);
    let over = format!("TYPE e : ({}); END_TYPE\n", many(32769));
    assert_eq!(codes(&over), vec![("E0020", 1)]);
    let place = format!("PROGRAM p\nVAR x : ({}); END_VAR\nEND_PROGRAM\n", many(32769));
    assert_eq!(codes(&place), vec![("E0020", 2)]);
}

#[test]
fn named_values_declared_in_place() {
    // Ed.3 6.4.4.3, in a variable's own declaration: used as the base
    // type; a shared name is one value when both declarations agree.
    let text = "PROGRAM p
VAR CONSTANT c : INT := 4; END_VAR
VAR
  x : INT (lo := 1, hi := 5) := hi;
  x2 : INT (lo := 1, top := c + 1);
  y : (p3 := 3, q4);
  w : WORD (m1 := 16#10);
  z : (lo := 2, zz);
  bad : SINT (big := 200);
  dup : (d1, d1);
  plain : (a, b);
  i : INT; ww : WORD;
END_VAR
i := x + 1;
x := 7;
i := y + top + p3;
ww := w OR m1;
i := zz;
i := lo;
i := plain;
plain := 1;
END_PROGRAM
";
    let found = codes(text);
    assert_eq!(found, vec![("E0020", 9), ("E0012", 10), ("E0010", 19), ("E0013", 20), ("E0013", 21)], "{found:?}");
}

#[test]
fn an_in_out_is_given_its_type_exactly_by_name_or_in_order() {
    // docs/behaviour/strings.md: a STRING in-out takes a variable of its
    // length; STRING is STRING[80].
    let text = "FUNCTION App : INT\nVAR_IN_OUT s : STRING; n : INT; END_VAR\nApp := n;\nEND_FUNCTION\n\
                PROGRAM p\nVAR v : INT; d : DINT; s4 : STRING[4]; s80 : STRING[80]; END_VAR\n\
                v := App(s80, v);\nv := App(s := s80, n := v);\n\
                v := App(s4, v);\nv := App(s := s4, n := v);\nv := App(s80, d);\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0013", 9), ("E0013", 10), ("E0013", 11)]);
}

#[test]
fn a_program_connection_names_what_ed3s_grammar_lets_it() {
    // Ed.3 6.8.2: Prog_Data_Source, Data_Sink, Global_Var_Access.
    let text = "TYPE PT : STRUCT x : INT; END_STRUCT; END_TYPE\n\
                PROGRAM main\nVAR_INPUT a : INT; b : INT; c : INT; d : INT; END_VAR\nVAR_OUTPUT o : INT; o2 : INT; END_VAR\nEND_PROGRAM\n\
                CONFIGURATION cf\nVAR_GLOBAL g : INT; gp : PT; ga : ARRAY [1..2] OF INT; END_VAR\nRESOURCE r ON x\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\nPROGRAM p WITH t : main(\na := -5,\nb := gp.x,\n\
                c := ga[1],\nd := g + 1,\no => gp.x,\no2 => ga[2]);\nEND_RESOURCE\nEND_CONFIGURATION\n";
    assert_eq!(codes(text), vec![("E0016", 13), ("E0016", 14), ("E0016", 16)]);
}

#[test]
fn a_partial_access_is_within_its_bit_string() {
    // Ed.3 6.6.1.3, Table 17.
    let text = "PROGRAM p\nVAR w : WORD; b : BYTE; i : INT; x : BOOL; d : DWORD; END_VAR\nVAR m AT %MB0 : BYTE; END_VAR\n\
                x := w.%X15;\nb := w.%B1;\nx := b.8;\nb := w.%B2;\nd := w.%D0;\nx := i.1;\nx := %MB0.%X1;\nEND_PROGRAM\n";
    let got: Vec<(&str, usize)> = codes(text).into_iter().filter(|(c, _)| c.starts_with('E')).collect();
    assert_eq!(got, vec![("E0014", 6), ("E0014", 7), ("E0014", 8), ("E0014", 9), ("E0014", 10)]);
}

#[test]
fn a_block_holding_an_instance_of_itself_is_e0011() {
    let text = "TYPE H : STRUCT a : ARRAY [1..2] OF S2; END_STRUCT; END_TYPE\n\
                FUNCTION_BLOCK S1\nVAR x : S1; END_VAR\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK S2\nVAR h : H; END_VAR\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK S3\nVAR_IN_OUT x : S3; END_VAR\nVAR r : REF_TO S3; END_VAR\nEND_FUNCTION_BLOCK\n";
    let mut got = codes(text);
    got.sort();
    assert_eq!(got, vec![("E0011", 2), ("E0011", 5)]);
}

#[test]
fn a_recursive_call_is_e0016() {
    // Ed.3 6.6.1: implementer specific; iecf refuses it (docs/dependencies.md).
    let text = "FUNCTION Ping : INT\nVAR_INPUT n : INT; END_VAR\nPing := Pong(n - 1);\nEND_FUNCTION\n\
                FUNCTION Pong : INT\nVAR_INPUT n : INT; END_VAR\nPong := Ping(n - 1);\nEND_FUNCTION\n\
                FUNCTION Fact : DINT\nVAR_INPUT n : DINT; END_VAR\nFact := n * Fact(n - 1);\nEND_FUNCTION\n\
                FUNCTION_BLOCK A\nMETHOD PUBLIC m : INT\nm := 1;\nEND_METHOD\nMETHOD PUBLIC n : INT\nn := THIS.m();\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK B EXTENDS A\nMETHOD PUBLIC OVERRIDE m : INT\nm := SUPER.m() + THIS.n();\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK C EXTENDS A\nMETHOD PUBLIC OVERRIDE m : INT\nm := SUPER.m();\nEND_METHOD\nEND_FUNCTION_BLOCK\n";
    let got: Vec<(&str, usize, String)> = check(text);
    let rec: Vec<&String> = got.iter().filter(|(c, _, _)| *c == "E0016").map(|(_, _, m)| m).collect();
    assert!(rec.iter().any(|m| m.contains("PING -> PONG -> PING")), "{got:#?}");
    assert!(rec.iter().any(|m| m.contains("here is the function's result")), "{got:#?}");
    // A.n may run B.m, which calls THIS.n: a cycle through the override.
    assert!(rec.iter().any(|m| m.contains("A.N -> B.M -> A.N") || m.contains("B.M -> A.N -> B.M")), "{got:#?}");
    // SUPER.m() is A.m itself: C has no cycle.
    assert!(!rec.iter().any(|m| m.contains("C.M")), "{got:#?}");
    assert_eq!(rec.len(), 3, "{got:#?}");
}

/// A derived block's instance has its base's `*` variables, completed
/// by its path as its own (Ed.3 6.6.5.5.2); a standard block's input
/// takes an initial value of its type; a structure with `*` components
/// is at no address of its own.
#[test]
fn var_config_reaches_inherited_and_standard_variables() {
    let text = "FUNCTION_BLOCK base\n\
                VAR coil AT %Q* : BOOL; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK derived EXTENDS base\n\
                END_FUNCTION_BLOCK\n\
                TYPE hw : STRUCT in AT %I* : BOOL; END_STRUCT; END_TYPE\n\
                PROGRAM line\n\
                VAR d : derived; t : TON; END_VAR\n\
                VAR x AT %MB0 : hw; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                RESOURCE cpu ON PLC\n\
                TASK fast (INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM main WITH fast : line;\n\
                END_RESOURCE\n\
                VAR_CONFIG\n\
                cpu.main.d.coil AT %QX0.0 : BOOL;\n\
                cpu.main.t.PT : TIME := T#2s;\n\
                cpu.main.t.Q : BOOL := T#1s;\n\
                cpu.main.t.NOPE : BOOL := TRUE;\n\
                cpu.main.x.in AT %IX0.0 : BOOL;\n\
                END_VAR\n\
                END_CONFIGURATION\n";
    let found: Vec<(&str, usize)> = check(text).into_iter().map(|(c, l, _)| (c, l)).collect();
    assert_eq!(found, vec![("E0022", 9), ("E0013", 19), ("E0010", 20)]);
}

/// Each CONFIGURATION has its own globals (Ed.3 6.5.2.2): two may name
/// theirs alike, and a POU's VAR_EXTERNAL is checked in each that runs
/// it; one no configuration runs, against any global of the name.
#[test]
fn globals_belong_to_their_configuration() {
    let text = "PROGRAM main\n\
                VAR_EXTERNAL g : INT; k : REAL; END_VAR\n\
                END_PROGRAM\n\
                FUNCTION_BLOCK lib\n\
                VAR_EXTERNAL g : INT; nowhere : BOOL; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                CONFIGURATION a\n\
                VAR_GLOBAL g : INT; k : REAL; END_VAR\n\
                RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE\n\
                END_CONFIGURATION\n\
                CONFIGURATION b\n\
                VAR_GLOBAL g : INT; k : INT; END_VAR\n\
                RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE\n\
                END_CONFIGURATION\n\
                CONFIGURATION A\n\
                END_CONFIGURATION\n\
                CONFIGURATION iecf\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0017", 2), ("E0017", 5), ("E0012", 15), ("E0018", 17)], "{found:?}");
    assert!(found[0].2.contains("in resource R of CONFIGURATION B, which runs MAIN"), "{found:?}");
}

/// A RESOURCE's globals are its own (Ed.3 6.5.2.2): two resources may
/// name theirs alike, not as their configuration's; a POU is checked in
/// each resource that runs it. A FUNCTION reaches one global wherever it
/// runs, and a VAR_EXTERNAL CONSTANT one value.
#[test]
fn globals_belong_to_their_resource() {
    let text = "FUNCTION f : INT\n\
                VAR_EXTERNAL g : INT; END_VAR\n\
                f := g;\n\
                END_FUNCTION\n\
                FUNCTION_BLOCK cb\n\
                VAR_EXTERNAL CONSTANT lim : INT; same : INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK only1\n\
                VAR_EXTERNAL x1 : INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR_EXTERNAL g : INT; END_VAR\n\
                VAR c : cb; o : only1; END_VAR\n\
                g := f();\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                VAR_GLOBAL cg : INT; END_VAR\n\
                RESOURCE r1 ON PLC\n\
                VAR_GLOBAL g : INT; x1 : INT; cg : INT; END_VAR\n\
                VAR_GLOBAL CONSTANT lim : INT := 3; same : INT := 4; END_VAR\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM p WITH t : main;\n\
                END_RESOURCE\n\
                RESOURCE r2 ON PLC\n\
                VAR_GLOBAL g : INT; END_VAR\n\
                VAR_GLOBAL CONSTANT lim : INT := 5; same : INT := 4; END_VAR\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM p WITH t : main;\n\
                END_RESOURCE\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0017", 2), ("E0017", 6), ("E0017", 9), ("E0012", 19)], "{found:?}");
    assert!(
        found[0].2.contains("'g' is a different global in resource R1 of CONFIGURATION PLANT and in resource R2"),
        "{found:?}"
    );
    assert!(found[1].2.contains("'lim' is CONSTANT"), "{found:?}");
    assert!(found[2].2.contains("in resource R2 of CONFIGURATION PLANT, which runs ONLY1"), "{found:?}");
    assert!(found[3].2.contains("in this resource and in its CONFIGURATION"), "{found:?}");
}

/// Two resources each with their own `g`, a PROGRAM and a block both
/// run: nothing to report.
#[test]
fn two_resources_name_a_global_alike() {
    let text = "FUNCTION_BLOCK counter\n\
                VAR_EXTERNAL g : INT; END_VAR\n\
                g := g + 1;\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR_EXTERNAL g : INT; END_VAR\n\
                VAR c : counter; END_VAR\n\
                c();\n\
                g := g + 1;\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                RESOURCE r1 ON PLC VAR_GLOBAL g : INT; END_VAR TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE\n\
                RESOURCE r2 ON PLC VAR_GLOBAL g : INT; END_VAR TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE\n\
                END_CONFIGURATION\n";
    assert_eq!(check(text), vec![]);
}

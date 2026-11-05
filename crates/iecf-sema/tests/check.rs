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
    VAR coil AT %Q* : BOOL; END_VAR\n\
    VAR_OUTPUT open AT %I* : BOOL; END_VAR\n\
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
            ("E0022", 10),
            ("E0022", 14),
            ("E0022", 15),
            ("E0022", 17),
            ("E0022", 17)
        ]
    );
}

#[test]
fn a_partly_address_in_a_body_is_an_error() {
    let text = "PROGRAM p\nVAR x : BOOL; END_VAR\nx := %I*;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let ids: Vec<_> = parsed.diagnostics.iter().map(|d| d.code.id()).collect();
    assert_eq!(ids, vec!["E0022"]);
}

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
                PROGRAM p\nVAR fi : F; x : INT; b : BOOL; END_VAR\nfi();\nb := fi.q;\nx := fi.hidden;\nEND_PROGRAM\n";
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
            "DINT does not convert to INT implicitly: use DINT_TO_INT (CODESYS and TwinCAT convert it, with a \
             warning: --extension implicit-narrowing)",
            "DINT does not convert to REAL implicitly: use DINT_TO_REAL (CODESYS and TwinCAT convert it, with a \
             warning: --extension implicit-narrowing)",
            "WORD does not convert to BYTE implicitly: use WORD_TO_BYTE (CODESYS and TwinCAT convert it, with a \
             warning: --extension implicit-narrowing)",
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
            "'ton' is reserved: it names a standard function block (Ed.3 6.1.3); choose another name (CODESYS and \
             TwinCAT let a variable hide it in its POU: --extension shadowing)",
            "'lt' is reserved: it names a standard function (Ed.3 6.1.3); choose another name (CODESYS and TwinCAT \
             let a variable hide it in its POU: --extension shadowing)",
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

/// Ed.3 7.3.3.4.2: the control variable, initial value and final value
/// "shall not be altered by any of the repeated statements".
#[test]
fn e0015_a_for_loop_does_not_change_its_variable_or_bounds() {
    let found = check(&prog(
        "FOR i := 1 TO 3 DO i := 5; END_FOR;\n\
         FOR i := us TO 3 DO us := 1; END_FOR;\n\
         FOR i := 1 TO arr[2] DO arr[1] := 0; arr[2] := 0; arr[i] := 0; END_FOR;\n\
         FOR i := 1 TO 3 DO FOR i := 1 TO 2 DO ; END_FOR; END_FOR;\n\
         FOR i := 1 TO WORD_TO_INT(w) DO w.3 := TRUE; END_FOR;\n\
         FOR q := 1 TO UPPER_BOUND(arr, 1) BY si DO arr[q] := 0; si := 2; END_FOR;\n\
         FOR i := 1 TO 3 DO IF b THEN WHILE b DO i := 0; END_WHILE; END_IF; END_FOR;\n",
    ));
    // Not seen: an element whose index is neither a literal nor the
    // bound's own name (arr[i]), a call's argument (w), the step (si).
    assert_eq!(
        found.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(),
        vec![("E0015", 10), ("E0015", 11), ("E0015", 12), ("E0015", 13), ("E0015", 16)]
    );
    assert_eq!(found[0].2, "the loop's statements may not change the FOR's control variable (Ed.3 7.3.3.4.2)");
    assert_eq!(found[1].2, "the loop's statements may not change what the FOR's initial value reads (Ed.3 7.3.3.4.2)");
    assert_eq!(found[2].2, "the loop's statements may not change what the FOR's final value reads (Ed.3 7.3.3.4.2)");
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
fn auto_addresses_place_what_var_config_leaves() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE s : STRUCT a AT %I* : INT; q AT %Q* : INT; END_STRUCT;\nEND_TYPE\n\
                PROGRAM main\nVAR d : s; END_VAR\nVAR p AT %M* : INT; END_VAR\nVAR_EXTERNAL g : s; END_VAR\nEND_PROGRAM\n\
                FUNCTION_BLOCK b\nVAR_INPUT i : s; END_VAR\nEND_FUNCTION_BLOCK\n\
                CONFIGURATION c\n\
                VAR_GLOBAL one AT %I* : BOOL; END_VAR\n\
                VAR_GLOBAL g : s; many : ARRAY[0..1] OF s; END_VAR\n\
                RESOURCE r ON x\nTASK t(INTERVAL := T#10ms, PRIORITY := 1);\nPROGRAM m WITH t : main;\nEND_RESOURCE\n\
                VAR_CONFIG r.m.d.q AT %QW0 : INT; END_VAR\n\
                END_CONFIGURATION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without it: the globals' `*` addresses, and the instance's two
    // VAR_CONFIG leaves out (d.a, p); an input's, an array's always.
    let mut off = codes(&Options::default());
    off.sort();
    assert_eq!(off, vec![("E0022", 9), ("E0022", 12), ("E0022", 13), ("E0022", 13), ("E0023", 16), ("E0023", 16)]);
    // With it: one W0015 for each global and for the instance; the input
    // and the array stay refused.
    let mut on = codes(&Options::default().with(Extension::AutoAddresses));
    on.sort();
    assert_eq!(on, vec![("E0022", 9), ("E0022", 13), ("W0015", 12), ("W0015", 13), ("W0015", 16)]);
}

#[test]
fn a_partly_type_s_in_out_binds_the_caller_s() {
    // An in-out holds no data of its own: the caller's variable, placed
    // where it is declared, is bound. An input is a copy: refused.
    let text = "TYPE s : STRUCT a AT %I* : INT; END_STRUCT;\nEND_TYPE\n\
                FUNCTION_BLOCK b\nVAR z AT %Q* : BOOL; END_VAR\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK c\nVAR_IN_OUT io : s; ib : b; END_VAR\nEND_FUNCTION_BLOCK\n\
                FUNCTION f : INT\nVAR_IN_OUT io : s; END_VAR\nf := io.a;\nEND_FUNCTION\n\
                FUNCTION_BLOCK d\nVAR_INPUT i : s; END_VAR\nEND_FUNCTION_BLOCK\n";
    assert_eq!(codes(text), vec![("E0022", 14)]);
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
fn standard_functions_inputs_left_out() {
    // Ed.3 6.6.1.4.2: a call by name may leave inputs out, each then its
    // type's default value; an extensible function has as many inputs as
    // the highest name given (6.6.2.5.1), at least as many as in order.
    let body = "i := LIMIT(IN := i, MX := 5);\ni := SEL(G := b, IN1 := i);\ni := ADD(IN3 := i);\n\
                s := MID(IN := s, L := 2);\nb := GT(IN2 := i);\ni := DINT_TO_INT();\n\
                i := MUL(IN1 := i);\ni := ABS();\nr := TO_REAL();\ni := MUX(K := 1);\ni := ADD(IN1025 := i);\n\
                i := ADD(IN1024 := i);";
    let found = check(&prog(body));
    let got: Vec<_> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(got, vec![("E0016", 16), ("E0016", 17), ("E0016", 18), ("E0016", 19), ("E0016", 20)], "{found:#?}");
    assert_eq!(
        found[0].2,
        "a call by name has as many inputs as the highest it names (Ed.3 6.6.2.5.1), and MUL takes at least IN1 to IN2"
    );
    assert_eq!(found[1].2, "ABS takes the type of IN from what it is given, and this call gives it nothing: give IN");
    assert_eq!(
        found[3].2,
        "a call by name has as many inputs as the highest it names (Ed.3 6.6.2.5.1), and MUX takes at least IN0"
    );
    assert_eq!(found[4].2, "ADD takes at most 1024 inputs, not IN1025 (docs/dependencies.md)");
    // A default no literal writes: not compiled yet.
    let text = "PROGRAM p\nVAR e : (red, green); a : ARRAY[1..2] OF INT; END_VAR\n\
                e := MAX(IN2 := e);\na := SEL(G := TRUE, IN1 := a);\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0009", 3), ("E0009", 4)]);
}

#[test]
fn a_part_is_taken_of_a_variable() {
    // Ed.3 6.6.1.3; Annex A, Variable. Of a field of a result it stays.
    let text = "TYPE pw : STRUCT a : WORD; END_STRUCT; END_TYPE\n\
                FUNCTION fw : WORD\nVAR_INPUT i : INT; END_VAR\nfw := 1;\nEND_FUNCTION\n\
                FUNCTION fs : pw\nVAR_INPUT i : INT; END_VAR\nfs.a := 1;\nEND_FUNCTION\n\
                PROGRAM p\nVAR b : BOOL; w : WORD; y : BYTE; END_VAR\n\
                b := fw(1).3;\ny := fw(1).%B0;\nb := SEL(b, w, w).3;\nb := (w AND w).3;\n\
                b := (w).3;\nb := fs(1).a.3;\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("E0014", 12), ("E0014", 13), ("E0014", 14), ("E0014", 15)]);
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
        "Ed.3 Figure 11 defines no conversion from TIME to DINT (CODESYS, TwinCAT and Siemens have it, in milliseconds: --extension time-number-conversions)"
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
fn time_number_conversions_extension() {
    use iecf_sema::{Extension, Options};
    let text = prog(
        "d := TIME_TO_DINT(t);\nt := DINT_TO_TIME(d);\nt := TO_TIME(u);\nw := TIME_TO_WORD(t);\n\
         r := TIME_TO_REAL(t);\nlr := TO_LREAL(t);\nt := LREAL_TO_TIME(lr);\nt := TO_TIME(r);\nb := TIME_TO_BOOL(t);",
    );
    let parsed = iecf_syntax::parse(FileId(0), &text);
    let found = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o);
    let codes = |o: &Options| found(o).iter().map(|d| d.code.id()).collect::<Vec<_>>();
    assert_eq!(codes(&Options::default()), ["E0013"; 9]);
    assert_eq!(
        found(&Options::default())[4].message,
        "Ed.3 Figure 11 defines no conversion from TIME to REAL (CODESYS has it, in milliseconds: --extension time-number-conversions)"
    );
    let on = Options::default().with(Extension::TimeNumberConversions);
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
    assert_eq!(at, vec![("E0027", 3), ("W0006", 7), ("W0006", 7), ("W0006", 7), ("E0027", 8)]);
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
        vec![("E0027", 2), ("E0027", 6), ("E0027", 7), ("E0027", 8), ("W0006", 9), ("E0013", 10)]
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
                k := 1 + note(2, k);\n\
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
VAR c : col; x : INT; w : WORD; m : mode; bi : B; mi : (low, top) := top; END_VAR
c := 27;
c := red + 1;
x := c + col#blue;
w := lv#hi OR lo;
bi(io := c);
bi(io := x);
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
                FUNCTION_BLOCK S2\nVAR hh : H; END_VAR\nEND_FUNCTION_BLOCK\n\
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
/// name theirs alike, and as their configuration's, hiding it; a POU is checked in
/// each resource that runs it. A FUNCTION reaches one global wherever it
/// runs, and CONSTANT globals of one name hold one value.
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
    assert_eq!(codes, vec![("E0017", 2), ("E0017", 9), ("E0012", 26)], "{found:?}");
    assert!(
        found[0].2.contains("'g' is a different global in resource R1 of CONFIGURATION PLANT and in resource R2"),
        "{found:?}"
    );
    assert!(found[1].2.contains("in resource R2 of CONFIGURATION PLANT, which runs ONLY1"), "{found:?}");
    assert!(
        found[2].2.contains("CONSTANT 'lim' of resource R2 of CONFIGURATION PLANT is not the integer value"),
        "{found:?}"
    );
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

/// An instance reaches the globals seen where it is declared, wherever
/// it is called from: r2, which has no `g`, calls r1's through a
/// reference. And a name is CONSTANT in every scope or in none.
#[test]
fn an_instance_belongs_where_it_is_declared() {
    let text = "FUNCTION_BLOCK counter\n\
                VAR_EXTERNAL g : INT; END_VAR\n\
                g := g + 1;\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM owner\n\
                VAR_EXTERNAL shared : REF_TO counter; END_VAR\n\
                VAR mine : counter; END_VAR\n\
                shared := REF(mine);\n\
                END_PROGRAM\n\
                PROGRAM user\n\
                VAR_EXTERNAL shared : REF_TO counter; END_VAR\n\
                IF shared <> NULL THEN shared^(); END_IF;\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                VAR_GLOBAL shared : REF_TO counter; END_VAR\n\
                RESOURCE r1 ON PLC VAR_GLOBAL g : INT; END_VAR TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : owner; END_RESOURCE\n\
                RESOURCE r2 ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : user; END_RESOURCE\n\
                END_CONFIGURATION\n";
    assert_eq!(check(text), vec![]);
    let text = "CONFIGURATION plant\n\
                RESOURCE r1 ON PLC VAR_GLOBAL CONSTANT k : INT := 1; END_VAR END_RESOURCE\n\
                RESOURCE r2 ON PLC VAR_GLOBAL k : INT := 1; END_VAR END_RESOURCE\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0012", 3)], "{found:?}");
}

/// A PROGRAM's own VAR_GLOBAL (Ed.3 Table 47, 8a and 8b): seen by what
/// each instance of it holds and calls, beside what its resource sees.
#[test]
fn a_program_declares_globals() {
    let text = "FUNCTION_BLOCK counter\n\
                VAR_EXTERNAL z : INT; END_VAR\n\
                z := z + 1;\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR_GLOBAL z : INT := 5; END_VAR\n\
                VAR c : counter; END_VAR\n\
                c();\n\
                z := z + 10;\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                RESOURCE r1 ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM a WITH t : main; PROGRAM b WITH t : main; END_RESOURCE\n\
                END_CONFIGURATION\n";
    assert_eq!(check(text), vec![]);
    let text = "FUNCTION readz : INT\n\
                VAR_EXTERNAL z : INT; END_VAR\n\
                readz := z;\n\
                END_FUNCTION\n\
                FUNCTION_BLOCK other\n\
                VAR_EXTERNAL z : INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR_GLOBAL z : INT; rg : INT; END_VAR\n\
                VAR_GLOBAL CONSTANT k : INT := 1; END_VAR\n\
                VAR x : INT; END_VAR\n\
                x := readz();\n\
                END_PROGRAM\n\
                PROGRAM second\n\
                VAR_GLOBAL CONSTANT k : INT := 2; END_VAR\n\
                VAR o : other; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                RESOURCE r1 ON PLC\n\
                VAR_GLOBAL rg : INT; END_VAR\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM a WITH t : main;\n\
                PROGRAM b WITH t : main;\n\
                PROGRAM c WITH t : second;\n\
                END_RESOURCE\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0017", 2), ("E0017", 6), ("E0012", 15)], "{found:?}");
    assert!(found[0].2.contains("in program A of resource R1 of CONFIGURATION PLANT and in program B"), "{found:?}");
    assert!(found[1].2.contains("in program C of resource R1"), "{found:?}");
    assert!(found[2].2.contains("CONSTANT 'k' of PROGRAM SECOND"), "{found:?}");
}

/// `R.g` in a connection and a TASK names the global `g` of the
/// resource it is written in (Ed.3 6.8.2, Global_Var_Access); another
/// resource's is out of scope (6.8.1), and a type still has to fit.
#[test]
fn a_global_named_through_its_resource() {
    let text = "TYPE Pt : STRUCT x : INT; END_STRUCT; END_TYPE\n\
                PROGRAM main\n\
                VAR_INPUT a : INT; END_VAR\n\
                VAR_OUTPUT o : INT; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                RESOURCE r1 ON PLC\n\
                VAR_GLOBAL g : INT; p : Pt; iv : TIME; b : BOOL; END_VAR\n\
                TASK t(INTERVAL := r1.iv, PRIORITY := 1);\n\
                PROGRAM m WITH t : main(a := r1.g, o => r1.p.x);\n\
                PROGRAM n WITH t : main(a := r1.b);\n\
                END_RESOURCE\n\
                RESOURCE r2 ON PLC\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM m WITH t : main(a := r1.g);\n\
                END_RESOURCE\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0013", 11), ("E0016", 15)], "{found:?}");
    assert!(found[1].2.contains("'r1' is another resource"), "{found:?}");
}

/// A program instance declared RETAIN keeps its PROGRAM's variables of
/// sections neither RETAIN nor NON_RETAIN: a reference among them is
/// warned of.
#[test]
fn a_retain_program_instance_keeps_no_reference() {
    let text = "PROGRAM main\n\
                VAR r : REF_TO INT; x : INT; END_VAR\n\
                END_PROGRAM\n\
                PROGRAM fine\n\
                VAR NON_RETAIN r : REF_TO INT; END_VAR\n\
                VAR x : INT; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                RESOURCE cpu ON PLC\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM RETAIN a WITH t : main;\n\
                PROGRAM b WITH t : main;\n\
                PROGRAM RETAIN c WITH t : fine;\n\
                PROGRAM NON_RETAIN d WITH t : main;\n\
                END_RESOURCE\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("W0006", 11)], "{found:?}");
}

/// What the grammar of a POU has no place for: VAR_GLOBAL in a
/// FUNCTION_BLOCK, a FUNCTION or a METHOD; an address on a block's
/// input or output; SFC in a FUNCTION.
#[test]
fn sections_where_the_grammar_has_none() {
    let text = "FUNCTION_BLOCK fb\n\
                VAR_GLOBAL g : INT; END_VAR\n\
                VAR_INPUT i AT %IX0.0 : BOOL; END_VAR\n\
                VAR_OUTPUT o AT %QX0.0 : BOOL; END_VAR\n\
                METHOD PUBLIC m\n\
                VAR_GLOBAL g3 : INT; END_VAR\n\
                END_METHOD\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION f : INT\n\
                VAR_GLOBAL g2 : INT; END_VAR\n\
                f := 1;\n\
                END_FUNCTION\n\
                FUNCTION h : INT\n\
                INITIAL_STEP s0 : END_STEP\n\
                TRANSITION FROM s0 TO s0 := TRUE; END_TRANSITION\n\
                END_FUNCTION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        codes,
        vec![("E0005", 2), ("E0022", 3), ("E0022", 4), ("E0005", 6), ("E0005", 10), ("E0005", 13)],
        "{found:?}"
    );
}

/// A global named as one further out hides it (as in CODESYS): a
/// resource's its configuration's, a PROGRAM's its resource's - of
/// another type too, the VAR_EXTERNAL checked against the inner one.
#[test]
fn an_inner_global_hides_an_outer_one() {
    let text = "FUNCTION_BLOCK fb\n\
                VAR_EXTERNAL g : INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR_EXTERNAL g : REAL; END_VAR\n\
                END_PROGRAM\n\
                PROGRAM own\n\
                VAR_GLOBAL g : INT; END_VAR\n\
                VAR x : fb; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION plant\n\
                VAR_GLOBAL g : DINT; END_VAR\n\
                RESOURCE r1 ON PLC\n\
                VAR_GLOBAL g : REAL; END_VAR\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM a WITH t : main;\n\
                PROGRAM b WITH t : own;\n\
                END_RESOURCE\n\
                END_CONFIGURATION\n";
    assert_eq!(check(text), vec![]);
}

/// CASE selects on any elementary type (Ed.3 7.3.3.3.3), its labels
/// constants of that type.
#[test]
fn case_on_any_elementary_type() {
    let text = "TYPE Pt : STRUCT x : INT; END_STRUCT; END_TYPE\n\
                PROGRAM main\n\
                VAR p : Pt; s : STRING; c : CHAR; v : CHAR; r : REAL; b : BOOL; t : TIME; d : DATE; q : INT; END_VAR\n\
                VAR CONSTANT k : CHAR := 'k'; END_VAR\n\
                CASE p OF 1: q := 1; END_CASE;\n\
                CASE s OF 1: q := 1; END_CASE;\n\
                CASE c OF v: q := 1; END_CASE;\n\
                CASE r OF 'a': q := 1; END_CASE;\n\
                CASE b OF TRUE: q := 1; FALSE: q := 2; END_CASE;\n\
                CASE c OF 'a', k..'z': q := 1; END_CASE;\n\
                CASE s OF 'ab': q := 1; 'x'..'z': q := 2; END_CASE;\n\
                CASE r OF 1.0: q := 1; 2.0..3.0: q := 2; END_CASE;\n\
                CASE t OF T#1s: q := 1; T#2s..T#5s: q := 2; END_CASE;\n\
                CASE d OF D#2020-01-01: q := 1; D#2020-02-01..D#2020-12-31: q := 2; END_CASE;\n\
                END_PROGRAM\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0014", 5), ("E0013", 6), ("E0032", 7), ("E0013", 8)], "{found:?}");
}

/// A function block's call may give its arguments in order (Ed.3
/// 6.6.3.4.1): inputs and in-outs, then outputs.
#[test]
fn a_block_called_with_arguments_in_order() {
    let text = "FUNCTION_BLOCK adder\n\
                VAR_INPUT a : INT; b : INT; END_VAR\n\
                VAR_OUTPUT o : INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR f : adder; t : TON; r : INT; q : BOOL; e : TIME; END_VAR\n\
                f(1, 2);\n\
                f(1, 2, r);\n\
                t(TRUE, T#1s, q, e);\n\
                f(1);\n\
                f(1, 2, r, r);\n\
                f(1, b := 2);\n\
                f(TRUE, 2, r);\n\
                f(1, 2, 3);\n\
                t(TRUE);\n\
                END_PROGRAM\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        codes,
        vec![
            ("E0016", 7),
            ("E0016", 10),
            ("E0016", 11),
            ("E0016", 12),
            ("E0013", 13),
            ("E0015", 14),
            ("E0013", 14),
            ("E0016", 15)
        ],
        "{found:?}"
    );
    // Every parameter, outputs too, where declared (Ed.3 6.6.1.4.2).
    assert!(found[0].2.contains("ADDER takes 3 argument(s) in order, not 2"), "{found:?}");
    assert!(found[0].2.contains("(A, B, O;"), "{found:?}");
}

/// Every in-out is given a variable at each call of a function block
/// (Ed.3 6.6.1.4 rule 2): an earlier call's is not kept.
#[test]
fn a_block_call_gives_every_in_out() {
    let text = "FUNCTION_BLOCK acc\n\
                VAR_IN_OUT n : INT; END_VAR\n\
                VAR_INPUT a : INT; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR f : acc; x : INT; END_VAR\n\
                f(n := x, a := 1);\n\
                f(a := 2);\n\
                f();\n\
                END_PROGRAM\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0016", 8), ("E0016", 9)], "{found:?}");
    assert!(found[0].2.contains("in-out 'N' of ACC must be passed at each call"), "{found:?}");
}

/// `fb.act()` names an ACTION: not a call IEC has (Ed.3 6.7.4).
#[test]
fn an_action_is_not_called_from_outside() {
    let text = "FUNCTION_BLOCK fb\n\
                VAR_OUTPUT n : INT; END_VAR\n\
                INITIAL_STEP s0 : bump(N); END_STEP\n\
                TRANSITION FROM s0 TO s0 := FALSE; END_TRANSITION\n\
                ACTION bump : n := n + 1; END_ACTION\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR f : fb; END_VAR\n\
                f();\n\
                f.bump();\n\
                f.nothing();\n\
                END_PROGRAM\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0016", 10), ("E0010", 11)], "{found:?}");
    assert!(found[0].2.contains("'bump' is an ACTION of FB"), "{found:?}");
}

/// An action's time is a duration literal or a variable (Ed.3 Annex A,
/// Action_Time), not an expression.
#[test]
fn an_action_time_is_a_literal_or_a_variable() {
    let text = "PROGRAM main\n\
                VAR d : TIME := T#20ms; k : INT := 2; n : INT; END_VAR\n\
                INITIAL_STEP s0 :\n\
                a(D, d);\n\
                b(L, T#30ms);\n\
                c(D, d * k);\n\
                e(L, d + T#10ms);\n\
                END_STEP\n\
                TRANSITION FROM s0 TO s0 := FALSE; END_TRANSITION\n\
                ACTION a : n := n + 1; END_ACTION\n\
                ACTION b : n := n + 1; END_ACTION\n\
                ACTION c : n := n + 1; END_ACTION\n\
                ACTION e : n := n + 1; END_ACTION\n\
                END_PROGRAM\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0005", 6), ("E0005", 7)], "{found:?}");
}

/// `fb1 WITH t2` in a program configuration (Ed.3 6.8.2): read, and
/// refused with its own message.
#[test]
fn a_block_on_its_own_task_is_refused() {
    let text = "FUNCTION_BLOCK blk\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR_INPUT i : INT; END_VAR\n\
                VAR fb1 : blk; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION c\n\
                VAR_GLOBAL g : INT; END_VAR\n\
                RESOURCE r ON PLC\n\
                TASK t1(INTERVAL := T#10ms, PRIORITY := 1);\n\
                TASK t2(INTERVAL := T#20ms, PRIORITY := 2);\n\
                PROGRAM p WITH t1 : main(i := g, fb1 WITH t2);\n\
                END_RESOURCE\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0009", 12)], "{found:?}");
    assert!(found[0].2.contains("'fb1 WITH t2' gives a function block instance a task of its own"), "{found:?}");
}

/// VAR_ACCESS (Ed.3 6.8.1): what a path may lead to, its type and its
/// direction, in a CONFIGURATION and in a PROGRAM.
#[test]
fn access_paths() {
    let text = "FUNCTION_BLOCK blk\n\
                VAR_OUTPUT d1 : BOOL; END_VAR\n\
                VAR_IN_OUT io : INT; END_VAR\n\
                VAR_ACCESS BAD : d1 : BOOL; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM main\n\
                VAR count : INT; fb1 : blk; arr : ARRAY [1..3] OF WORD; END_VAR\n\
                VAR CONSTANT lim5 : INT := 5; END_VAR\n\
                VAR_TEMP t : INT; END_VAR\n\
                VAR_EXTERNAL w : UINT; END_VAR\n\
                VAR_ACCESS LOCAL1 : count : INT READ_WRITE; LOCAL2 : fb1.d1 : BOOL; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION cell\n\
                VAR_GLOBAL w : UINT; END_VAR\n\
                RESOURCE st1 ON PLC\n\
                VAR_GLOBAL z2 : BOOL; END_VAR\n\
                TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n\
                PROGRAM p1 WITH fast : main;\n\
                END_RESOURCE\n\
                VAR_ACCESS\n\
                DOG : w : UINT READ_ONLY;\n\
                GAMMA : st1.z2 : BOOL READ_WRITE;\n\
                THETA : st1.p1.fb1.d1 : BOOL READ_WRITE;\n\
                BIT : st1.p1.arr[2].%X3 : BOOL;\n\
                IN1 : st1.%IX1.0 : BOOL;\n\
                E1 : st1.p1.t : INT;\n\
                E2 : st1.p1.lim5 : INT READ_WRITE;\n\
                E3 : st1.p1 : INT;\n\
                E4 : st1.p1.count : DINT;\n\
                E5 : st1.p1.nothing : INT;\n\
                E6 : st1.p1.fb1.io : INT;\n\
                E7 : st1.p1.w : UINT;\n\
                E8 : st1.p1.arr[9] : WORD;\n\
                DOG : w : UINT;\n\
                E10 : st1.p1.count + 1 : INT;\n\
                END_VAR\n\
                END_CONFIGURATION\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        codes,
        vec![
            ("E0005", 4),
            ("E0035", 26),
            ("E0035", 27),
            ("E0035", 28),
            ("E0013", 29),
            ("E0010", 30),
            ("E0035", 31),
            ("E0035", 32),
            ("E0020", 33),
            ("E0012", 34),
            ("E0035", 35)
        ],
        "{found:?}"
    );
}

/// An SFC indicator variable (Ed.3 6.7.4.2, deprecated): a BOOL the POU
/// sees, warned of.
#[test]
fn an_indicator_variable_is_warned_of() {
    let text = "PROGRAM main\n\
                VAR done : BOOL; n : INT; END_VAR\n\
                INITIAL_STEP s0 : a(N, done); b(N, n); c(N, nothing); END_STEP\n\
                TRANSITION FROM s0 TO s0 := done; END_TRANSITION\n\
                ACTION a : done := TRUE; END_ACTION\n\
                ACTION b : n := 1; END_ACTION\n\
                ACTION c : n := 2; END_ACTION\n\
                END_PROGRAM\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("W0007", 3), ("E0013", 3), ("E0010", 3)], "{found:?}");
}

/// Audit 2's wrong messages, now proper errors: a constant index out of
/// bounds (Ed.3 6.4.4.5.1), a variable as a CASE label, an array of an
/// enumeration declared in place, a STRING too long, a bit past 7; and
/// a constant CASE selector, which is no error.
#[test]
fn audit_two_errors() {
    let text = "TYPE Col : (red, green); END_TYPE\n\
                PROGRAM main\n\
                VAR p : ARRAY [1..4] OF INT; k : INT; c : Col; END_VAR\n\
                VAR a : ARRAY [1..2] OF (x1, x2); END_VAR\n\
                VAR s : STRING[70000]; END_VAR\n\
                VAR q AT %QX0.8 : BOOL; END_VAR\n\
                VAR CONSTANT ka : ARRAY [1..3] OF INT := [1, 2, 3]; END_VAR\n\
                VAR y : INT := ka[4]; END_VAR\n\
                CASE 3 OF 3: k := 1; END_CASE;\n\
                CASE 1 + 2 OF 3: k := 2; END_CASE;\n\
                CASE c OF c: k := 1; END_CASE;\n\
                p[7] := 1;\n\
                END_PROGRAM\n";
    let found = check(text);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(
        codes,
        vec![("E0005", 4), ("E0020", 5), ("E0022", 6), ("E0020", 8), ("E0032", 11), ("E0020", 12)],
        "{found:?}"
    );
}

/// Several SFC networks in a POU, each with exactly one INITIAL_STEP
/// (Ed.3 Annex A, `SFC : Sfc_Network +`).
#[test]
fn a_case_label_earlier_labels_cover_is_warned_of() {
    // The first arm that matches runs (Ed.3 7.3.3.3.3): a label all of
    // whose values earlier labels take selects nothing.
    let text = "TYPE COLOR : (red, green, blue); END_TYPE\n\
                PROGRAM p\nVAR i : INT; w : WORD; c : COLOR; a : INT; END_VAR\n\
                VAR CONSTANT K : INT := 5; END_VAR\n\
                CASE i OF\n\
                1..10: a := 1;\n\
                K: a := 2;\n\
                8..12: a := 3;\n\
                3, 3: a := 4;\n\
                11, 12, 13: a := 5;\n\
                -5..0, 1..13: a := 6;\n\
                END_CASE;\n\
                CASE w OF 16#FF: a := 1; 255: a := 2; 0..254: a := 3; END_CASE;\n\
                CASE c OF red, green: a := 1; COLOR#red: a := 2; blue: a := 3; END_CASE;\n\
                END_PROGRAM\n";
    assert_eq!(
        codes(text),
        vec![
            ("W0009", 7),
            ("W0009", 9),
            ("W0009", 9),
            ("W0009", 10),
            ("W0009", 10),
            ("W0009", 11),
            ("W0009", 13),
            ("W0009", 14)
        ]
    );
}

#[test]
fn an_array_initial_value_of_another_length_is_warned_of() {
    // Ed.3 6.4.4.5.2: "In either case, the user shall be warned".
    let text = "PROGRAM p\nVAR\n\
                a : ARRAY[0..5] OF INT := [1, 2];\n\
                b : ARRAY[0..1] OF INT := [1, 2, 3];\n\
                c : ARRAY[0..5] OF INT := [2(1, 2, 3)];\n\
                d : ARRAY[0..5] OF INT := [1, 2, 4()];\n\
                e : ARRAY[0..1, 0..2] OF INT := [6(0)];\n\
                f : ARRAY[0..1] OF ARRAY[0..1] OF INT := [[1, 2], [3]];\n\
                r : REF_TO INT; i : INT;\n\
                END_VAR\ni := REF(i)^;\nEND_PROGRAM\n";
    assert_eq!(codes(text), vec![("W0008", 3), ("W0008", 4), ("W0008", 8), ("E0014", 11)]);
}

#[test]
fn an_sfc_in_an_action() {
    // Ed.3 6.7.4.1: names are the POU's; a transition joins steps of its
    // own chart; each chart's networks have one INITIAL_STEP; a BOOL
    // variable is an action of one chart.
    let head = "PROGRAM main\nVAR go, lamp : BOOL; n : INT; END_VAR\n\
                INITIAL_STEP o0 : sub(N); lamp(N); END_STEP\n\
                TRANSITION FROM o0 TO o0 := go; END_TRANSITION\n\
                ACTION sub :\n";
    let tail = "END_ACTION\nACTION mark : n := n + 1; END_ACTION\nEND_PROGRAM\n";
    let ok = format!(
        "{head}INITIAL_STEP i0 : cnt(N); mark(N); END_STEP\n\
         TRANSITION FROM i0 TO i0 := o0.X AND i0.T > T#1s AND sub.Q; END_TRANSITION\n\
         ACTION cnt : n := n + 1; END_ACTION\n{tail}"
    );
    assert_eq!(check(&ok), vec![]);
    // Not named as a variable of the POU.
    let clash = "PROGRAM main\nVAR s0 : INT; act : BOOL; END_VAR\nINITIAL_STEP s0 : act(N); END_STEP\n\
                 TRANSITION FROM s0 TO s0 := FALSE; END_TRANSITION\nACTION act : s0 := 1; END_ACTION\nEND_PROGRAM\n";
    assert_eq!(codes(clash), vec![("E0012", 3), ("E0012", 5)]);
    let bad = format!(
        "{head}STEP i0 : lamp(N); END_STEP\n\
         TRANSITION FROM i0 TO o0 := go; END_TRANSITION\n\
         ACTION mark : n := 0; END_ACTION\n{tail}"
    );
    assert_eq!(
        codes(&bad),
        vec![("E0016", 5), ("E0016", 6), ("E0010", 7), ("E0012", 10)],
        "sub's network without an INITIAL_STEP; lamp of two charts; o0 of another chart; mark twice"
    );
}

#[test]
fn sfc_networks() {
    let ok = "PROGRAM main\n\
              VAR a : INT; END_VAR\n\
              INITIAL_STEP s0 : END_STEP\n\
              TRANSITION FROM s0 TO s1 := a > 1; END_TRANSITION\n\
              STEP s1 : END_STEP\n\
              TRANSITION FROM s1 TO s0 := TRUE; END_TRANSITION\n\
              INITIAL_STEP t0 : END_STEP\n\
              TRANSITION FROM t0 TO t0 := FALSE; END_TRANSITION\n\
              END_PROGRAM\n";
    assert_eq!(check(ok), vec![]);
    let bad = "PROGRAM main\n\
               VAR a : INT; END_VAR\n\
               INITIAL_STEP s0 : END_STEP\n\
               TRANSITION FROM s0 TO s1 := a > 1; END_TRANSITION\n\
               INITIAL_STEP s1 : END_STEP\n\
               TRANSITION FROM s1 TO s0 := TRUE; END_TRANSITION\n\
               STEP t0 : END_STEP\n\
               TRANSITION FROM t0 TO t0 := FALSE; END_TRANSITION\n\
               END_PROGRAM\n";
    let found = check(bad);
    let codes: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(codes, vec![("E0016", 3), ("E0016", 7)], "{found:?}");
    assert!(found[0].2.contains("the network of s0, s1 has 2"), "{found:?}");
    assert!(found[1].2.contains("the network of t0 has 0"), "{found:?}");
}

#[test]
fn validate_bcd_and_endianness_take_their_types() {
    // Ed.3 Table 39; Table 22 3a-4b, footnote d: BCD is BYTE, WORD, DWORD
    // or LWORD; Table 37: a value of a data type.
    let body = "b := IS_VALID(r);\nb := IS_VALID(lr);\nb := IS_VALID_BCD(w);\nb := IS_VALID_BCD(bt);\n\
                u := WORD_BCD_TO_UINT(w);\ni := BCD_TO_INT(w);\nw := INT_TO_BCD_WORD(i);\nbt := TO_BCD_BYTE(us);\n\
                w := TO_BIG_ENDIAN(w);\nr := FROM_LITTLE_ENDIAN(r);\narr := TO_BIG_ENDIAN(arr);\n\
                s := TO_LITTLE_ENDIAN(s);";
    assert_eq!(codes(&prog(body)), vec![]);
    let bad = "b := IS_VALID(i);\nb := IS_VALID_BCD(b);\ni := BOOL_BCD_TO_INT(b);\nw := INT_TO_BCD_BOOL(i);\n\
               b := TO_BIG_ENDIAN(tm).Q;\nb := IS_VALID_BCD(r);";
    let found = check(&prog(bad));
    let at: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(&at[..4], &[("E0014", 10), ("E0014", 11), ("E0010", 12), ("E0010", 13)], "{found:#?}");
    assert_eq!(found[0].2, "IN of IS_VALID takes ANY_REAL, not INT");
    assert_eq!(found[1].2, "IN of IS_VALID_BCD takes BYTE, WORD, DWORD or LWORD, not BOOL");
    assert!(
        found.iter().any(|(c, l, m)| *c == "E0014"
            && *l == 14
            && m == "IN of TO_BIG_ENDIAN takes a value of a data type (ANY), not TON"),
        "{found:#?}"
    );
    assert!(found.iter().any(|(c, l, _)| *c == "E0014" && *l == 15), "{found:#?}");
    // Ed.2's form, which names no bit string: Ed.3's named.
    let found = check(&prog("w := INT_TO_BCD(i);"));
    assert_eq!(
        found[0].2,
        "'INT_TO_BCD' is not declared: it is Ed.2's; Ed.3 names the bit string it gives, INT_TO_BCD_WORD \
         (or _BYTE, _DWORD, _LWORD; Table 22)"
    );
}

/// Ed.3 Annex A and 6.5.6.2: CONSTANT on VAR, VAR_EXTERNAL and
/// VAR_GLOBAL; RETAIN and NON_RETAIN on a static VAR, VAR_INPUT,
/// VAR_OUTPUT and VAR_GLOBAL, "but not in VAR_IN_OUT section"; one of
/// them per section.
#[test]
fn e0005_section_qualifiers_as_ed3_has_them() {
    let text = "VAR_GLOBAL g : INT; END_VAR\n\
                FUNCTION_BLOCK fb\n\
                VAR_INPUT CONSTANT a : INT; END_VAR\n\
                VAR_IN_OUT CONSTANT b : INT; END_VAR\n\
                VAR_TEMP RETAIN c : INT; END_VAR\n\
                VAR_EXTERNAL RETAIN g : INT; END_VAR\n\
                VAR CONSTANT RETAIN d : INT := 1; END_VAR\n\
                VAR_INPUT RETAIN e : INT; END_VAR\n\
                VAR_OUTPUT NON_RETAIN f : INT; END_VAR\n\
                VAR RETAIN h : INT; END_VAR\n\
                ;\nEND_FUNCTION_BLOCK\n\
                FUNCTION fn : INT\nVAR RETAIN k : INT; END_VAR\nfn := k;\nEND_FUNCTION\n\
                CONFIGURATION c\nVAR_GLOBAL NON_RETAIN n : INT; END_VAR\nVAR_GLOBAL CONSTANT RETAIN m : INT := 1; END_VAR\n\
                RESOURCE r ON PLC END_RESOURCE\nEND_CONFIGURATION\n";
    assert_eq!(
        codes(text),
        vec![("E0005", 3), ("E0005", 4), ("E0005", 5), ("E0005", 6), ("E0005", 7), ("E0005", 14), ("E0005", 19)]
    );
    let m = check(text);
    assert_eq!(
        m[0].2,
        "CONSTANT is for VAR, VAR_EXTERNAL and VAR_GLOBAL, not VAR_INPUT (Ed.3 Annex A): CODESYS's VAR_INPUT \
         CONSTANT, a plain input outside its CFC editor, is --extension input-constant"
    );
    assert!(m[1].2.starts_with("VAR_IN_OUT CONSTANT is CODESYS's, not Ed.3's"), "{}", m[1].2);
    assert_eq!(
        m[5].2,
        "RETAIN is for a static VAR, VAR_INPUT, VAR_OUTPUT or VAR_GLOBAL (Ed.3 6.5.6.2): this one lives only in its call"
    );
}

/// Two extensions: STRING(n), and an instance in VAR_TEMP or in a
/// FUNCTION's or METHOD's VAR (Ed.3 6.6.3.2 rule 17).
#[test]
fn string_parentheses_and_temp_instances_extensions() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : BOOL\nVAR c : CTU; END_VAR\nf := c.Q;\nEND_FUNCTION\n\
                FUNCTION_BLOCK fb\nVAR_TEMP t : TON; a : ARRAY[0..1] OF TP; r : REF_TO TON; END_VAR\n\
                METHOD PUBLIC m : BOOL\nVAR x : R_TRIG; END_VAR\nm := x.Q;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR s : STRING(5); w : WSTRING(4); k : STRING[5]; END_VAR\n;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let at = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        at(&Options::default()),
        vec![("E0005", 2), ("E0005", 6), ("E0005", 6), ("E0005", 8), ("E0005", 13), ("E0005", 13)]
    );
    let on = Options::default().with(Extension::StringParentheses).with(Extension::TempInstances);
    assert_eq!(at(&on), vec![]);
}

/// The in-out-constant extension: a VAR_IN_OUT CONSTANT is given any
/// value of its type, a STRING of any length; it is not written, nor
/// given on to an in-out that may write it.
#[test]
fn in_out_constant_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION g : INT\nVAR_IN_OUT CONSTANT x : INT; s : STRING[5]; END_VAR\nVAR_IN_OUT y : INT; END_VAR\n\
                g := x;\nEND_FUNCTION\n\
                FUNCTION h : INT\nVAR_IN_OUT CONSTANT x : INT; s : STRING[5]; END_VAR\nVAR v : INT; END_VAR\n\
                x := 1;\nh := g(x := x, s := s, y := x);\nh := g(x := x, s := s, y := v);\nEND_FUNCTION\n\
                PROGRAM p\nVAR r : INT; l : STRING[40]; END_VAR\nVAR CONSTANT k : INT := 2; END_VAR\n\
                r := g(x := 3, s := 'abcdefgh', y := r);\nr := g(k, l, r);\n\
                r := g(x := r * 2, s := CONCAT(l, 'z'), y := r);\nr := g(x := 'a', s := l, y := r);\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let at = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let wrong = [("E0015", 9), ("E0015", 10), ("E0013", 19)];
    assert_eq!(at(&Options::default()), [&[("E0005", 2), ("E0005", 7)][..], &wrong].concat());
    assert_eq!(at(&Options::default().with(Extension::InOutConstant)), wrong);
}

#[test]
fn an_initial_value_reads_only_what_is_set_up_before_it() {
    // initial-expressions (docs/extensions.md): a variable's initial
    // value names the variables set up before it, in the order declared;
    // an instance's VAR_TEMPs and in-outs are not there as it is set up
    // (E0036). Without the extension each is E0032, the checker's, as the
    // generator's.
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION F : INT\n\
                VAR_INPUT x : INT; END_VAR\n\
                VAR a : INT := x + b; b : INT := x; END_VAR\n\
                F := a;\n\
                END_FUNCTION\n\
                FUNCTION_BLOCK Fb\n\
                VAR_IN_OUT io : INT; END_VAR\n\
                VAR_TEMP t : INT := v + 1; u : INT := w; w : INT; END_VAR\n\
                VAR v : INT := io; k : INT := t; n : INT := F(v) + KC; END_VAR\n\
                VAR CONSTANT KC : INT := 2; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM p\n\
                VAR c : INT := c + 1; d : INT := THIS_IS_e; e : INT; END_VAR\n\
                END_PROGRAM\n\
                CONFIGURATION cf\n\
                VAR_GLOBAL g : INT := h; END_VAR\n\
                VAR_GLOBAL h : INT := 1; END_VAR\n\
                RESOURCE r ON x\n\
                VAR_GLOBAL rg : INT := g + h; END_VAR\n\
                PROGRAM i WITH t : p;\n\
                TASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
                END_RESOURCE\n\
                END_CONFIGURATION\n"
        .replace("THIS_IS_e", "e");
    let parsed = iecf_syntax::parse(FileId(0), &text);
    let at = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let e0032 = [3, 3, 8, 8, 9, 9, 9, 13, 13].map(|l| ("E0032", l));
    assert_eq!(at(&Options::default()), [&e0032[..], &[("E0010", 16), ("E0010", 19), ("E0010", 19)]].concat());
    assert_eq!(
        at(&Options::default().with(Extension::InitialExpressions)),
        [("E0036", 3), ("E0036", 8), ("E0036", 9), ("E0036", 9), ("E0036", 13), ("E0036", 13), ("E0036", 16)]
    );
}

#[test]
fn a_value_two_types_share_is_not_unique_but_in_an_initial_value() {
    // Ed.3 6.4.4.2 and 6.4.4.3's examples: Red, in two types - 1 in both,
    // of INT, in the second - is "not unique" in a comparison; the
    // declared type says which in an initial value. One declared in
    // place has no type's name to write: its values take their own.
    let text = "TYPE\n\
                Traffic_light: (Red, Amber, Green);\n\
                Painting_colors: (Red, Yellow, Green, Blue) := Blue;\n\
                Light: INT (Red := 1, Orange := 2, Green := 3) := Green;\n\
                Paint: INT (Red := 1, Yellow := 2, Green := 3, Blue := 4) := Blue;\n\
                END_TYPE\n\
                PROGRAM p\n\
                VAR t : Traffic_light := Red; l : Light; b : BOOL; END_VAR\n\
                VAR x : INT (idle := 0, run := 1); y : INT (idle := 0, stop := 2); END_VAR\n\
                b := t = Traffic_light#Amber;\n\
                b := t = Amber;\n\
                b := t = Red;\n\
                b := l = Red;\n\
                b := l = Light#Red;\n\
                x := idle;\n\
                END_PROGRAM\n";
    assert_eq!(codes(text), [("E0010", 12), ("E0010", 13), ("E0010", 15)]);
}

#[test]
fn codesys_enumerations_need_their_extension() {
    // codesys-enums (docs/extensions.md): CODESYS's base type after the
    // values and its COLOR.yellow; Ed.3 writes DWORD (...) and COLOR#yellow.
    use iecf_sema::{Extension, Options};
    let text = "TYPE COLOR : (white := 16#FFFFFF00, black := 16#88000000) DWORD := black; END_TYPE\n\
                TYPE Pt : STRUCT white : INT; END_STRUCT; END_TYPE\n\
                PROGRAM p\n\
                VAR c : COLOR := COLOR.white; s : Pt; b : BOOL; i : INT; END_VAR\n\
                b := c = COLOR.black;\n\
                i := s.white;\n\
                END_PROGRAM\n";
    // As the driver does: the namespaces' pass, then the check.
    let at = |o: &Options| {
        let mut units = [iecf_syntax::parse(FileId(0), text).unit];
        let mut ds = iecf_sema::namespaces::flatten(&mut units, o);
        ds.extend(iecf_sema::check_with(&[&units[0]], o));
        ds.iter().map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1)).collect::<Vec<_>>()
    };
    assert_eq!(at(&Options::default()), [("E0010", 4), ("E0010", 5), ("E0005", 1)]);
    assert_eq!(at(&Options::default().with(Extension::CodesysEnums)), []);
}

#[test]
fn a_literal_converted_to_time_needs_its_extension_too() {
    // time-number-conversions (docs/extensions.md): a number to or from
    // TIME is E0013 without it, the number a literal as well.
    use iecf_sema::{Extension, Options};
    let text = "PROGRAM p\nVAR x : TIME; d : DINT; END_VAR\n\
                x := DINT_TO_TIME(1500);\n\
                x := TO_TIME(1500);\n\
                x := LREAL_TO_TIME(1.5);\n\
                d := INT_TO_DINT(5);\n\
                END_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let at = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1, d.message.clone()))
            .collect::<Vec<_>>()
    };
    let off = at(&Options::default());
    assert_eq!(off.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(), [("E0013", 3), ("E0013", 4), ("E0013", 5)]);
    assert!(off[0].2.contains("from DINT to TIME") && off[1].2.contains("from an integer to TIME"), "{off:?}");
    assert_eq!(at(&Options::default().with(Extension::TimeNumberConversions)), []);
}

#[test]
fn a_pou_names_a_global_constant_in_a_bound_through_its_var_external() {
    // Ed.3 6.5.2.2: a POU sees a global through VAR_EXTERNAL; a TYPE's
    // bound sees the global constants directly; implicit-externals
    // lets a POU name one without (until 2026-11-19 it slipped through).
    use iecf_sema::{Extension, Options};
    let text = "TYPE Arr : ARRAY[1..GK] OF INT; END_TYPE\n\
                PROGRAM p\nVAR a : ARRAY[1..GK] OF INT; t : Arr; END_VAR\nEND_PROGRAM\n\
                PROGRAM q\nVAR_EXTERNAL CONSTANT GK : INT; END_VAR\nVAR a : ARRAY[1..GK] OF INT; END_VAR\nEND_PROGRAM\n\
                CONFIGURATION c\nVAR_GLOBAL CONSTANT GK : INT := 2; END_VAR\n\
                RESOURCE r ON x\nTASK t(INTERVAL := T#10ms, PRIORITY := 1);\nPROGRAM i1 WITH t : p;\nPROGRAM i2 WITH t : q;\n\
                END_RESOURCE\nEND_CONFIGURATION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let at = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1, d.message.clone()))
            .collect::<Vec<_>>()
    };
    let off = at(&Options::default());
    assert_eq!(off.iter().map(|(c, l, _)| (*c, *l)).collect::<Vec<_>>(), [("E0032", 3)]);
    assert!(off[0].2.starts_with("'GK' is a global CONSTANT, which a POU names through its VAR_EXTERNAL CONSTANT"));
    assert_eq!(at(&Options::default().with(Extension::ImplicitExternals)), []);
}

#[test]
fn a_reference_to_input_is_bound_by_a_call_in_order_too() {
    // reference-to (docs/extensions.md): given by name or in order, the
    // input is bound to the variable; a literal is no variable to bind.
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION F : INT\nVAR_INPUT r : REFERENCE TO INT; END_VAR\nF := r;\nEND_FUNCTION\n\
                PROGRAM p\nVAR a : INT; END_VAR\na := F(a);\na := F(r := a);\na := F(5);\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let codes: Vec<_> = iecf_sema::check_with(&[&parsed.unit], &Options::default().with(Extension::ReferenceTo))
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect();
    assert_eq!(codes, [("E0013", 9)]);
}

#[test]
fn an_ltime_conversion_is_said_to_count_nanoseconds() {
    let text =
        "PROGRAM p\nVAR x : LTIME; t : TIME; END_VAR\nx := LINT_TO_LTIME(5);\nt := DINT_TO_TIME(5);\nEND_PROGRAM\n";
    let m: Vec<String> = check(text).into_iter().map(|(_, _, m)| m).collect();
    assert!(m[0].contains("in nanoseconds") && m[1].contains("in milliseconds"), "{m:?}");
}

/// Three CODESYS ways of writing (docs/extensions.md): END_STRUCT with
/// no `;` (struct-semicolon), `1E37` (real-without-point), an array's
/// values without brackets (bare-array-init). Each E0005 without its
/// extension; FUNCTIONBLOCK is refused either way, once, and the block
/// read as the FUNCTION_BLOCK meant.
#[test]
fn codesys_syntax_extensions() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE S : STRUCT a : INT; END_STRUCT\nT : STRUCT b : INT; END_STRUCT\nEND_TYPE\n\
                FUNCTION f : REAL\nVAR a : ARRAY[1..3] OF INT := 1, 2, 3; b : ARRAY[1..4] OF INT := 4(0); END_VAR\n\
                f := 1E37 + 2.0E-3 + 2E-3;\nEND_FUNCTION\n\
                FUNCTIONBLOCK fb\nVAR_INPUT x : INT; END_VAR\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    assert_eq!(parsed.diagnostics.iter().map(line).collect::<Vec<_>>(), vec![("E0005", 8)]);
    assert_eq!(
        at(&Options::default()),
        vec![("E0005", 2), ("E0005", 3), ("E0005", 5), ("E0005", 5), ("E0005", 6), ("E0005", 6)]
    );
    let on = Options::default()
        .with(Extension::StructSemicolon)
        .with(Extension::RealWithoutPoint)
        .with(Extension::BareArrayInit);
    assert_eq!(at(&on), vec![]);
}

/// input-constant: VAR_INPUT CONSTANT a plain input, written too;
/// keyword-names: STEP, ON, R_EDGE, F_EDGE as names, said once per name
/// without the extension, and still keywords where the grammar has them
/// (an edge input, a step, a resource).
#[test]
fn input_constant_and_keyword_names_extensions() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK fb\nVAR_INPUT CONSTANT k : INT := 3; END_VAR\nVAR_INPUT x : BOOL R_EDGE; END_VAR\n\
                VAR step, on : INT; END_VAR\nstep := k; on := step;\nk := k + 1;\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK s\nVAR step : INT; END_VAR\nINITIAL_STEP s0 : a(N); END_STEP\n\
                ACTION a : step := step + 1; END_ACTION\nEND_FUNCTION_BLOCK\n\
                FUNCTION g : INT\nVAR_INPUT CONSTANT RETAIN n : INT; END_VAR\ng := n;\nEND_FUNCTION\n\
                CONFIGURATION c RESOURCE r ON PLC END_RESOURCE END_CONFIGURATION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    assert_eq!(at(&Options::default()), vec![("E0005", 2), ("E0005", 4), ("E0005", 4), ("E0005", 14)]);
    let on = Options::default().with(Extension::InputConstant).with(Extension::KeywordNames);
    // CONSTANT RETAIN is two qualifiers either way.
    assert_eq!(at(&on), vec![("E0005", 14)]);
}

/// time-function: `TIME()` is a TIME, the clock; without the extension
/// E0010 saying so. `TIME(x)` is no such call either way.
#[test]
fn time_function_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : TIME\nVAR t : TIME; d : DINT; END_VAR\nt := TIME();\nf := TIME() - t;\nd := TIME();\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    assert_eq!(at(&Options::default()), vec![("E0010", 3), ("E0010", 4), ("E0010", 5), ("E0013", 5)]);
    // A TIME is no DINT, the clock's as any.
    assert_eq!(at(&Options::default().with(Extension::TimeFunction)), vec![("E0013", 5)]);
}

/// bit-string-arithmetic: a bit string counts as the unsigned integer of
/// its width - in + - * / MOD, as an index, where a number is wanted, and
/// to and from a real where Figure 11 has no conversion. Unary minus and
/// a type both sides do not convert to stay refused.
#[test]
fn bit_string_arithmetic_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : INT\nVAR d : DWORD; w : WORD; b : BYTE; i : INT; r : REAL; a : ARRAY[0..9] OF INT; END_VAR\n\
                d := d + 1;\nw := w MOD 7 + b;\ni := i + b;\nr := r * b + BYTE_TO_REAL(b);\nf := a[b];\n\
                i := b;\nb := REAL_TO_BYTE(r);\nr := DWORD_TO_REAL(d);\n\
                i := -b;\ni := d + i;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    let off = at(&Options::default());
    for l in [3, 4, 5, 6, 7, 8, 9, 11, 12] {
        assert!(off.iter().any(|(_, n)| *n == l), "line {l} accepted without the extension: {off:?}");
    }
    assert!(!off.iter().any(|(_, n)| *n == 10), "DWORD_TO_REAL is Ed.3's: {off:?}");
    // On: unary minus on a bit string stays refused; DWORD + INT is a LINT
    // (UDINT and INT both widen to it), which an INT does not take.
    assert_eq!(at(&Options::default().with(Extension::BitStringArithmetic)), vec![("E0014", 11), ("E0013", 12)]);
}

/// integer-shifts, integer-bit-access, integer-expt: each refused without
/// its extension, its message naming it; with it, the result types.
#[test]
fn integer_shift_bit_access_and_expt_extensions() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : INT\nVAR i : INT; d : DINT; n : BYTE; b : BOOL; r : REAL; l : LREAL; END_VAR\n\
                f := SHR(i, n);\nb := d.31;\nr := EXPT(i, 2);\nl := EXPT(d, 2);\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let diags = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o);
    let off = diags(&Options::default());
    let lines: Vec<usize> = off.iter().map(|d| text[..d.span.start as usize].matches('\n').count() + 1).collect();
    assert_eq!(lines, vec![3, 3, 4, 5, 6]);
    for (d, ext) in
        off.iter().zip(["integer-shifts", "integer-shifts", "integer-bit-access", "integer-expt", "integer-expt"])
    {
        assert!(d.message.contains(ext), "{}", d.message);
    }
    let on = Options::default()
        .with(Extension::IntegerShifts)
        .with(Extension::IntegerBitAccess)
        .with(Extension::IntegerExpt);
    // EXPT of a DINT is an LREAL, which a REAL would not take: r := EXPT(d, 2) is E0013.
    assert!(diags(&on).is_empty(), "{:?}", diags(&on));
}

/// implicit-narrowing: integers and bit strings where another is
/// wanted, an integer or LREAL where a REAL is, W0010 instead of E0013;
/// a real to an integer stays E0013.
#[test]
fn implicit_narrowing_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : SINT\nVAR i : INT; d : DINT; w : WORD; r : REAL; l : LREAL; END_VAR\n\
                i := d;\nw := i;\nf := i;\ni := r;\nd := i;\nr := d;\nr := l;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    let e = |l| ("E0013", l);
    let w = |l| ("W0010", l);
    assert_eq!(at(&Options::default()), vec![e(3), e(4), e(5), e(6), e(8), e(9)]);
    assert_eq!(at(&Options::default().with(Extension::ImplicitNarrowing)), vec![w(3), w(4), w(5), e(6), w(8), w(9)]);
}

/// implicit-narrowing: a ULINT and a signed integer, which no type holds
/// both of, compared by value, W0010 instead of E0014.
#[test]
fn implicit_narrowing_compares_ulint_and_signed() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : BOOL\nVAR u : ULINT; i : INT; l : LINT; d : UDINT; END_VAR\n\
                f := u >= i + 7;\nf := l < u;\nf := d < i;\nf := u = 1.5;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    // UDINT and INT always compared, in DINT... LINT; a real is no integer.
    assert_eq!(at(&Options::default()), vec![("E0014", 3), ("E0014", 4), ("E0014", 6)]);
    assert_eq!(
        at(&Options::default().with(Extension::ImplicitNarrowing)),
        vec![("W0010", 3), ("W0010", 4), ("E0014", 6)]
    );
}

/// implicit-narrowing: an integer literal outside the integer or bit
/// string it goes into, its low bits, W0010 instead of E0020.
#[test]
fn implicit_narrowing_literal_low_bits() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : BOOL\nVAR u : UINT; l : ULINT; s : SINT; b : BOOL; r : REAL; END_VAR\n\
                u := -1;\nl := -1;\ns := 200;\nb := 2;\nu := 5;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    assert_eq!(at(&Options::default()), vec![("E0020", 3), ("E0020", 4), ("E0020", 5), ("E0020", 6)]);
    // BOOL is no integer: 2 stays E0020.
    assert_eq!(
        at(&Options::default().with(Extension::ImplicitNarrowing)),
        vec![("W0010", 3), ("W0010", 4), ("W0010", 5), ("E0020", 6)]
    );
}

/// date-number-conversions: dates and times of day to and from numbers,
/// E0013 naming the extension without it.
#[test]
fn date_number_conversions_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : DWORD\nVAR d : DATE; t : TOD; x : DT; u : UDINT; r : REAL; END_VAR\n\
                f := DT_TO_DWORD(x);\nu := DATE_TO_UDINT(d) + TOD_TO_UDINT(t);\nd := UDINT_TO_DATE(u);\n\
                r := DATE_TO_REAL(d);\nt := TO_TOD(u);\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let diags = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o);
    let off = diags(&Options::default());
    assert_eq!(off.len(), 6, "{off:?}");
    assert!(off.iter().all(|d| d.code.id() == "E0013" && d.message.contains("date-number-conversions")), "{off:?}");
    assert!(diags(&Options::default().with(Extension::DateNumberConversions)).is_empty());
}

/// shadowing: a POU's variable named as a block, a type, a standard
/// block or function; a data type's name and a global stay refused.
#[test]
fn shadowing_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK Length\nEND_FUNCTION_BLOCK\nTYPE Spd : STRUCT v : INT; END_STRUCT; END_TYPE\n\
                VAR_GLOBAL ton : INT; END_VAR\n\
                FUNCTION f : INT\nVAR length : INT; spd : Spd; tp : BOOL; max : INT; dt : INT; END_VAR\n\
                f := length + spd.v + max;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    assert_eq!(
        at(&Options::default()),
        vec![("E0018", 4), ("E0012", 6), ("E0012", 6), ("E0018", 6), ("E0018", 6), ("E0018", 6)]
    );
    assert_eq!(at(&Options::default().with(Extension::Shadowing)), vec![("E0018", 4), ("E0018", 6)]);
}

/// An untyped literal beside an INT takes the INT's type: one outside it
/// is E0020 from the checker too, not only when a CONFIGURATION makes the
/// generator run (found in OSCAT BASIC's CALENDAR_CALC, `offset * 60000`).
#[test]
fn e0020_an_untyped_literal_outside_its_operands_type() {
    let text = "FUNCTION f : DINT\nVAR i : INT; b : BOOL; w : WORD; END_VAR\n\
                f := i * 60000;\nb := i < 60000;\nf := MAX(i, 60000);\nw := w AND 16#1FFFF;\n\
                f := i * 30000;\nf := INT_TO_DINT(i) * 60000;\nEND_FUNCTION\n";
    assert_eq!(codes(text), vec![("E0020", 3), ("E0020", 4), ("E0020", 5), ("E0020", 6)]);
}

/// codesys-real-conversions: the four binary transfers by value, W0011 at
/// each; without it they are Ed.3's, silently.
#[test]
fn codesys_real_conversions_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : REAL\nVAR d : DWORD; l : LWORD; x : LREAL; END_VAR\n\
                f := DWORD_TO_REAL(d);\nd := REAL_TO_DWORD(f);\nx := LWORD_TO_LREAL(l);\nl := LREAL_TO_LWORD(x);\n\
                f := TO_REAL(d);\nf := UDINT_TO_REAL(DWORD_TO_UDINT(d));\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    assert_eq!(at(&Options::default()), vec![]);
    assert_eq!(
        at(&Options::default().with(Extension::CodesysRealConversions)),
        vec![("W0011", 3), ("W0011", 4), ("W0011", 5), ("W0011", 6), ("W0011", 7)]
    );
}

/// pointers: POINTER TO, ADR, SIZEOF refused without the extension; with
/// it, a pointer to data only, no SIZEOF of a pointer, no number for an
/// address, and no pointer that may reach a temporary kept where it
/// outlives the call - while one kept in a temporary, or to a variable
/// that lives on, is fine.
#[test]
fn pointers_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK Fb\nVAR_INPUT p : POINTER TO INT; END_VAR\nVAR q : POINTER TO INT; s : INT; END_VAR\n\
                VAR_TEMP t : INT; END_VAR\n\
                q := ADR(t);\nq := ADR(s);\nq := q + 2;\nEND_FUNCTION_BLOCK\n\
                FUNCTION F : POINTER TO INT\nVAR_IN_OUT io : POINTER TO INT; END_VAR\nVAR x : INT; l : POINTER TO INT; END_VAR\n\
                l := ADR(x);\nl := l + 1;\nio := ADR(x);\nF := l;\n\
                x := SIZEOF(l);\nx := SIZEOF(x);\nl := 16#1000;\nx := l + 1.5;\nEND_FUNCTION\n\
                FUNCTION G : INT\nVAR pf : POINTER TO Fb; END_VAR\nG := 0;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    let off = at(&Options::default());
    assert!(off.iter().filter(|(c, _)| *c == "E0005").count() >= 5, "{off:?}");
    assert!(off.contains(&("E0010", 5)) && off.contains(&("E0010", 16)), "{off:?}");
    assert!(!off.iter().any(|(c, _)| *c == "E0027"), "{off:?}");
    assert_eq!(
        at(&Options::default().with(Extension::Pointers)),
        vec![
            ("E0027", 5),  // q, static, := ADR(t), a VAR_TEMP
            ("E0027", 14), // io, an in-out, := ADR(x)
            ("E0027", 15), // F, the result, := l, a temporary pointer
            ("E0016", 16), // SIZEOF of a pointer
            ("E0013", 18), // a number for an address
            ("E0014", 19), // a pointer plus a real
            ("E0027", 22), // POINTER TO a block
        ]
    );
}

/// REF() of what a pointer points to is E0026: a reference would keep no
/// check of where it is.
#[test]
fn pointers_no_ref_through_a_pointer() {
    use iecf_sema::{Extension, Options};
    let text = "PROGRAM p\nVAR a : ARRAY[0..3] OF INT; pt : POINTER TO ARRAY[0..3] OF INT; r : REF_TO INT; END_VAR\n\
                pt := ADR(a);\nr := REF(pt^[1]);\nr := REF(a[1]);\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let found: Vec<_> = iecf_sema::check_with(&[&parsed.unit], &Options::default().with(Extension::Pointers))
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect();
    assert_eq!(found, vec![("E0026", 4)]);
}

/// pointers: `p[i]` is `(p + i * SIZEOF(T))^`, of T, one index of an
/// integer; it writes what `p` points to, not `p` - an input pointer's
/// target too, a CONSTANT pointer's; REF() of it is E0026, as of `p^`;
/// ADR() of it is where `p` points, kept as `p` may be.
#[test]
fn pointers_indexed() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK Fb\nVAR_INPUT p : POINTER TO INT; END_VAR\n\
                VAR q : POINTER TO INT; r : REF_TO INT; x : INT; b : BOOL; END_VAR\n\
                VAR_TEMP t : POINTER TO INT; END_VAR\n\
                p[0] := 1;\nx := p[x] + p[-1];\nx := p[1, 2];\nr := REF(p[0]);\nx := p[1.5];\n\
                b := p[0];\nq := ADR(p[2]);\nq := ADR(t[2]);\nt := ADR(t[1]);\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let found: Vec<_> = iecf_sema::check_with(&[&parsed.unit], &Options::default().with(Extension::Pointers))
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect();
    assert_eq!(
        found,
        vec![
            ("E0014", 7),  // two indices
            ("E0026", 8),  // REF() through a pointer
            ("E0014", 9),  // a REAL index
            ("E0013", 10), // an INT is no BOOL
            ("E0027", 12), // q, static, := where a temporary pointer points
        ]
    );
}

/// dynamic-memory: `__NEW(T, n)`, a POINTER TO T, assigned whole to a
/// pointer, of data only, its pool's size given (`--dynamic-memory`);
/// `__DELETE(p)` a statement of a pointer variable. In a project that
/// calls `__NEW`, a RETAIN pointer is E0027, not W0006. Without the
/// extension both are E0010.
#[test]
fn dynamic_memory_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK Fb\nVAR p : POINTER TO INT; x : INT; b : BOOL; END_VAR\n\
                VAR RETAIN r : POINTER TO INT; END_VAR\n\
                p := __NEW(INT, x + 1);\n__DELETE(p);\nb := __NEW(INT) = 0;\np := __NEW(Fb);\n\
                p := __NEW(5);\np := __NEW(INT, 1.5);\nb := __DELETE(p);\n__DELETE(x);\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    let off = at(&Options::default().with(Extension::Pointers));
    assert!(off.contains(&("E0010", 4)) && off.contains(&("E0010", 5)), "{off:?}");
    assert!(!off.iter().any(|(c, _)| *c == "E0005"), "{off:?}");
    let on = Options::default().with(Extension::Pointers).with(Extension::DynamicMemory);
    assert!(at(&on).contains(&("E0016", 4)), "no pool: {:?}", at(&on));
    assert_eq!(
        at(&on.with_dynamic_memory(4096)),
        vec![
            ("E0027", 3),  // RETAIN, in a project that calls __NEW
            ("E0016", 6),  // __NEW inside an expression
            ("E0027", 7),  // __NEW of a block
            ("E0016", 8),  // __NEW of no type
            ("E0014", 9),  // a REAL count
            ("E0016", 10), // __DELETE gives no value
            ("E0016", 10), // ... and is a statement
            ("E0014", 11), // __DELETE of an INT
        ]
    );
}

/// program-instances: a PROGRAM read, written and called by its name,
/// its METHOD called, from a block - not called from a FUNCTION; one with
/// a VAR_GLOBAL of its own is not made an instance; a configuration
/// declares one instance of such a program. Without the extension each
/// is refused, naming it.
#[test]
fn program_instances_extension() {
    use iecf_sema::{Extension, Options};
    let text = "PROGRAM P\nVAR x : INT; END_VAR\nMETHOD PUBLIC M : INT\nM := x;\nEND_METHOD\nEND_PROGRAM\n\
                PROGRAM G\nVAR_GLOBAL gl : INT; END_VAR\nVAR y : INT; END_VAR\nEND_PROGRAM\n\
                FUNCTION_BLOCK F\nVAR v : INT; END_VAR\nv := P.x;\nP.x := 2;\nP();\nv := P.M();\nv := G.y;\n\
                END_FUNCTION_BLOCK\nFUNCTION Fn : INT\nP();\nFn := 0;\nEND_FUNCTION\n\
                CONFIGURATION c\nRESOURCE r ON x\nTASK t(INTERVAL := T#1s, PRIORITY := 1);\n\
                PROGRAM a WITH t : P;\nPROGRAM b WITH t : P;\nEND_RESOURCE\nEND_CONFIGURATION\n";
    let mut units = vec![iecf_syntax::parse(FileId(0), text).unit];
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options, units: &mut Vec<iecf_syntax::ast::SourceUnit>| {
        let mut u = units.clone();
        iecf_sema::analyze_project(&mut u, o).diagnostics.iter().map(line).collect::<Vec<_>>()
    };
    let off = at(&Options::default(), &mut units);
    assert!(off.contains(&("E0005", 3)), "a METHOD in a PROGRAM: {off:?}");
    assert!(off.contains(&("E0010", 13)) && off.contains(&("E0016", 15)), "{off:?}");
    assert_eq!(
        at(&Options::default().with(Extension::ProgramInstances), &mut units),
        vec![
            ("E0010", 17), // G has a VAR_GLOBAL of its own: no instance
            ("E0016", 20), // called from a FUNCTION
            ("E0016", 27), // a second instance of P
        ]
    );
}

/// query-pointer: a POINTER TO a block is a reference to an instance -
/// compared with 0, assigned ADR(inst) but not of a temporary, no
/// arithmetic, no number; `__QUERYPOINTER(itf, p)` takes an interface
/// and such a pointer. Without the extension the pointer is E0027 and
/// the call E0010.
#[test]
fn query_pointer_extension() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE I\nEND_INTERFACE\nFUNCTION_BLOCK T IMPLEMENTS I\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK U\nVAR p : POINTER TO T; x : I; a : T; b : BOOL; n : INT; id : ULINT; END_VAR\n\
                VAR_TEMP tmp : T; END_VAR\n\
                b := __QUERYPOINTER(x, p) AND p <> 0;\nb := __QUERYPOINTER(a, p);\nb := __QUERYPOINTER(x, n);\n\
                p := ADR(a);\np := ADR(tmp);\np := p + 1;\nid := ADR(a);\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let line =
        |d: &iecf_diagnostics::Diagnostic| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1);
    let at = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).iter().map(line).collect::<Vec<_>>();
    let off = at(&Options::default().with(Extension::Pointers));
    assert!(off.contains(&("E0027", 6)) && off.contains(&("E0010", 8)), "{off:?}");
    assert_eq!(
        at(&Options::default().with(Extension::Pointers).with(Extension::QueryPointer).with(Extension::TempInstances)),
        vec![
            ("E0016", 9),  // an instance first
            ("E0014", 10), // into an INT
            ("E0026", 12), // ADR of a VAR_TEMP instance
            ("E0014", 13), // arithmetic
            ("E0013", 14), // an address is no number
        ]
    );
}

/// fb-init: a derived type's FB_init has its base's inputs first, of
/// their types in their order - renamed or not - and may add some; one
/// with fewer, of another type or in another order is E0031.
#[test]
fn fb_init_derived_inputs() {
    use iecf_sema::{Extension, Options};
    let head = "METHOD FB_init : BOOL\nVAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; ";
    let text = format!(
        "FUNCTION_BLOCK Base0\n{head}a : INT; b : BOOL; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK Adds EXTENDS Base0\n{head}a : INT; b : BOOL; c : REAL; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK Renames EXTENDS Base0\n{head}x : INT; y : BOOL; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK Fewer EXTENDS Base0\n{head}a : INT; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK Retyped EXTENDS Base0\n{head}a : DINT; b : BOOL; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK Swapped EXTENDS Base0\n{head}b : BOOL; a : INT; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n"
    );
    let parsed = iecf_syntax::parse(FileId(0), &text);
    let found: Vec<_> =
        iecf_sema::check_with(&[&parsed.unit], &Options::default().with(Extension::FbInit).with(Extension::CodesysOo))
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect();
    assert_eq!(found, vec![("E0031", 17), ("E0031", 22), ("E0031", 27)]);
}

/// fb-init: an in-out of a type with an FB_init is bound, not set up
/// (no E0009), and takes no arguments (E0016); an array of instances is
/// each element's FB_init, the same arguments given each, checked once
/// (E0013 once for the array), and set up only as a PROGRAM's or
/// FUNCTION_BLOCK's VAR, one name to a declaration (E0009).
#[test]
fn fb_init_in_outs_and_arrays() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK T\nMETHOD FB_init : BOOL\n\
                VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; k : INT; END_VAR\nEND_METHOD\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK W\nVAR_IN_OUT io : T; END_VAR\nVAR_IN_OUT jo : T(1); END_VAR\n\
                VAR a : ARRAY[0..1] OF T(2); b : ARRAY[1..2, 0..1] OF T(3); END_VAR\n\
                VAR c : ARRAY[0..2] OF T('x'); END_VAR\n\
                VAR f, g : ARRAY[0..1] OF T(4); END_VAR\n\
                METHOD M\nVAR_IN_OUT x : T; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let found: Vec<_> =
        iecf_sema::check_with(&[&parsed.unit], &Options::default().with(Extension::FbInit).with(Extension::CodesysOo))
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect();
    assert_eq!(found, vec![("E0016", 8), ("E0013", 10), ("E0009", 11)]);
}

/// struct-blocks: a structure member's initial value may give an
/// interface field another member that is an instance, `(Parent :=
/// Self)`; another member's data, or an instance where no interface
/// takes it, stays E0010.
#[test]
fn struct_member_initial_value_names_a_sibling() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE IObj\nEND_INTERFACE\nFUNCTION_BLOCK M IMPLEMENTS IObj\nEND_FUNCTION_BLOCK\n\
                TYPE In : STRUCT Parent : IObj; n : INT; END_STRUCT; END_TYPE\n\
                TYPE S : STRUCT\nSelf : M;\nk : INT := 3;\na : In := (Parent := Self);\nb : In := (n := k);\n\
                END_STRUCT; END_TYPE\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let found: Vec<_> = iecf_sema::check_with(
        &[&parsed.unit],
        &Options::default().with(Extension::FbInit).with(Extension::StructBlocks),
    )
    .iter()
    .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
    .collect();
    assert_eq!(found, vec![("E0010", 10)]);
}

/// A static pointer whose initial value is ADR() of a temporary would
/// dangle from the start: E0027; of a static, or in a FUNCTION, fine.
#[test]
fn pointers_initial_values() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK Fb\nVAR_TEMP t : INT; END_VAR\nVAR s : INT; p : POINTER TO INT := ADR(t); \
                q : POINTER TO INT := ADR(s); END_VAR\nEND_FUNCTION_BLOCK\n\
                FUNCTION F : INT\nVAR x : INT; r : POINTER TO INT := ADR(x); END_VAR\nF := r^;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let found: Vec<_> = iecf_sema::check_with(&[&parsed.unit], &Options::default().with(Extension::Pointers))
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect();
    assert_eq!(found, vec![("E0027", 3)]);
}

/// shadowing: a call reaches the function past a local of its name that
/// cannot be called; without the extension the local is E0018.
#[test]
fn shadowing_a_call_reaches_the_function() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : INT\nVAR max : INT; day_of_week : STRING[10]; END_VAR\n\
                max := MAX(3, 7);\nf := max + DAY_OF_WEEK(D#2026-11-19);\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let found = |o: &Options| iecf_sema::check_with(&[&parsed.unit], o).len();
    assert_eq!(found(&Options::default().with(Extension::Shadowing)), 0);
    assert!(found(&Options::default()) > 0);
}

/// Three forms from OSCAT NETWORK and BUILDING: a bit string compared
/// with a number (bit-string-arithmetic), TOD_TO_TIME and TIME_TO_TOD
/// (date-number-conversions), DATE + TIME (date-time-arithmetic).
#[test]
fn bit_compare_tod_time_and_date_plus_time() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : BOOL\nVAR u : UDINT; w : DWORD; t : TOD; d : DATE; x : TIME; END_VAR\n\
                f := u > w;\nx := TOD_TO_TIME(t);\nt := TIME_TO_TOD(x);\nd := d + T#1d;\nd := d - x;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let lines = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| text[..d.span.start as usize].matches('\n').count() + 1)
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(&Options::default()), vec![3, 4, 5, 6, 7]);
    let on = Options::default()
        .with(Extension::BitStringArithmetic)
        .with(Extension::DateNumberConversions)
        .with(Extension::DateTimeArithmetic);
    assert_eq!(lines(&on), Vec::<usize>::new());
}

#[test]
fn mixed_arguments_in_order_then_by_name() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : INT\nVAR_INPUT a : INT; b : INT; END_VAR\nf := a + b;\nEND_FUNCTION\n\
                FUNCTION_BLOCK acc\nVAR_INPUT x : INT; y : INT; END_VAR\n\
                METHOD PUBLIC m : INT\nVAR_INPUT u : INT; v : INT; END_VAR\nm := u - v;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                PROGRAM main\nVAR s : acc; n : INT; END_VAR\n\
                n := f(1, b := 2);\ns(5, y := 6);\nn := s.m(3, v := 1);\nn := LIMIT(0, MX := 9, IN := 4);\n\
                n := f(b := 2, 1);\nn := f(1, a := 2);\nn := f(1, 2, 3, b := 1);\nn := LIMIT(0, 1, 2, 3, MX := 2);\n\
                n := ADD(1, 2, IN2 := 3);\nn := ADD(1, 2, IN4 := 4);\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let lines = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| text[..d.span.start as usize].matches('\n').count() + 1)
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(&Options::default()), vec![14, 15, 16, 17, 18, 19, 20, 21, 22, 23]);
    assert_eq!(lines(&Options::default().with(Extension::MixedArguments)), vec![18, 19, 20, 21, 22]);
}

#[test]
fn chained_assignment_right_to_left() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION g : INT\ng := 1;\nEND_FUNCTION\n\
                FUNCTION f : INT\nVAR a : INT; b : INT; s : SINT; r : REAL; arr : ARRAY[0..3] OF INT; END_VAR\n\
                a := b := 0;\nr := a := s := 5;\na := s := 300;\na := r := 1.5;\nf := a;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let lines = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| text[..d.span.start as usize].matches('\n').count() + 1)
            .collect::<Vec<_>>()
    };
    // Without: each chain refused, and 300 outside SINT, and a REAL
    // into an INT, as on their own.
    assert_eq!(lines(&Options::default()), vec![6, 7, 8, 8, 9, 9]);
    assert_eq!(lines(&Options::default().with(Extension::ChainedAssignment)), vec![8, 9]);
    // An inner target with a call: refused by the parser.
    let bad = iecf_syntax::parse(
        FileId(0),
        "PROGRAM p\nVAR a : INT; arr : ARRAY[0..3] OF INT; END_VAR\na := arr[g()] := 1;\nEND_PROGRAM\n",
    );
    assert_eq!(bad.diagnostics.len(), 1);
}

#[test]
fn jump_labels_out_never_in() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : INT\nVAR i : INT; k : INT; END_VAR\n\
                again:\ni := i + 1;\nIF i < 5 THEN JMP again; END_IF;\n\
                FOR k := 1 TO 3 DO\ninner: ;\nIF k = 2 THEN JMP out; END_IF;\nEND_FOR;\nout:\n\
                JMP inner;\nJMP nowhere;\nagain: ;\nCASE i OF 5: i := 1; END_CASE;\nf := i;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without: each label and JMP refused.
    assert_eq!(codes(&Options::default()).iter().filter(|(c, _)| *c == "E0005").count(), 8);
    assert_eq!(
        codes(&Options::default().with(Extension::JumpLabels)),
        vec![("E0021", 11), ("E0010", 12), ("E0012", 13)]
    );
}

#[test]
fn optional_semicolons_after_the_ends() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : INT\nVAR i : INT; END_VAR\n\
                IF i > 0 THEN i := 1; END_IF\nFOR i := 1 TO 2 DO f := i; END_FOR\n\
                WHILE i < 5 DO i := i + 1; END_WHILE\nREPEAT i := i - 1; UNTIL i < 0 END_REPEAT\n\
                CASE i OF 1: i := 2; END_CASE\nf := i;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let lines = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| text[..d.span.start as usize].matches('\n').count() + 1)
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(&Options::default()), vec![3, 4, 5, 6, 7]);
    assert!(lines(&Options::default().with(Extension::OptionalSemicolons)).is_empty());
    // Any other statement keeps its `;`.
    let other = iecf_syntax::parse(FileId(0), "FUNCTION f : INT\nf := 1\nEND_FUNCTION\n");
    assert_eq!(other.diagnostics.len(), 1);
}

#[test]
fn codesys_oo_spellings() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK base\nVAR_OUTPUT n : INT; END_VAR\n\
                METHOD m : INT\nm := THIS^.k();\nEND_METHOD\n\
                METHOD k : INT\nk := 1;\nEND_METHOD\n\
                METHOD PRIVATE p : INT\np := 1;\nEND_METHOD\n\
                n := n + 1;\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK derived EXTENDS base\n\
                METHOD k : INT\nk := SUPER^.k() + 1;\nEND_METHOD\n\
                METHOD p : INT\np := 2;\nEND_METHOD\n\
                SUPER^();\nEND_FUNCTION_BLOCK\n\
                PROGRAM main\nVAR d : derived; x : INT; END_VAR\nd();\nx := d.m();\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Ed.3: THIS^ (4), no OVERRIDE (15), SUPER^ (16, 21), PROTECTED m (26).
    assert_eq!(
        codes(&Options::default()),
        vec![("E0014", 4), ("E0031", 15), ("E0031", 16), ("E0031", 21), ("E0030", 26)]
    );
    // codesys-oo: all read as Ed.3's; p, PRIVATE in base, is no override.
    assert_eq!(codes(&Options::default().with(Extension::CodesysOo)), vec![]);
}

#[test]
fn internal_reads_read_only() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK acc\nVAR count : INT; END_VAR\nVAR_TEMP t : INT; END_VAR\n\
                count := count + 1;\nEND_FUNCTION_BLOCK\n\
                FUNCTION bump : INT\nVAR_IN_OUT v : INT; END_VAR\nbump := v;\nEND_FUNCTION\n\
                PROGRAM main\nVAR a : acc; n : INT; END_VAR\n\
                n := a.count;\na.count := 1;\nn := bump(v := a.count);\nn := a.t;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    assert_eq!(codes(&Options::default()), vec![("E0010", 12), ("E0010", 13), ("E0010", 14), ("E0010", 15)]);
    // Read: warned; written, in-out, a VAR_TEMP: refused still.
    assert_eq!(
        codes(&Options::default().with(Extension::InternalReads)),
        vec![("W0012", 12), ("E0010", 13), ("E0010", 14), ("E0010", 15)]
    );
}

#[test]
fn properties_get_and_set() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK m\nVAR v : INT; END_VAR\n\
                PROPERTY PUBLIC speed : INT\nGET\nspeed := v;\nEND_GET\nSET\nv := speed;\nEND_SET\nEND_PROPERTY\n\
                PROPERTY PUBLIC only : INT\nGET\nonly := 1;\nEND_GET\nEND_PROPERTY\n\
                METHOD PUBLIC bump\nspeed := speed + 1;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                PROGRAM main\nVAR x : m; k : INT; END_VAR\n\
                x.speed := 3;\nk := x.speed + x.only;\nx.only := 2;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without: each PROPERTY refused, then its uses.
    let off = codes(&Options::default());
    assert_eq!(off.iter().filter(|(c, _)| *c == "E0005").count(), 2);
    // With: read and assigned through GET and SET; `only` has no SET.
    assert_eq!(codes(&Options::default().with(Extension::Properties)), vec![("E0015", 24)]);
}

#[test]
fn named_values_are_case_labels() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE e : (x := 1, y := 2); END_TYPE\nTYPE n : INT (p := 1, q := 2); END_TYPE\n\
                FUNCTION f : INT\nVAR_INPUT a : e; b : n; END_VAR\n\
                CASE a OF e#x: f := 1; e#y: f := 2; END_CASE;\n\
                CASE b OF n#p: f := 3; END_CASE;\n\
                CASE a OF e.x: f := 4; END_CASE;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        // As a build checks it: the namespaces pass first, which reads
        // CODESYS's `e.x` (codesys-enums).
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Ed.3's T#v is a constant label; CODESYS's T.v needs its extension.
    assert_eq!(codes(&Options::default()), vec![("E0010", 7)]);
    assert_eq!(codes(&Options::default().with(Extension::CodesysEnums)), vec![]);
}

#[test]
fn internal_reads_call_a_method_and_a_body() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK state\nVAR_INPUT i : INT; END_VAR\nVAR_OUTPUT n : INT; END_VAR\n\
                METHOD PUBLIC restore\nn := 0;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK drive\nVAR st : state; END_VAR\nEND_FUNCTION_BLOCK\n\
                PROGRAM main\nVAR d : drive; k : INT; END_VAR\n\
                d.st.restore();\nd.st(i := 1, n => k);\nd.st.i := 2;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        let mut v = iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>();
        // The assignment's E0010 is said twice, as before this test
        // (the place checked twice): once here.
        v.dedup();
        v
    };
    // A method of the internal instance, and its body with its inputs
    // (as TwinCAT compiles TcOpen's): called, warned; an input of it
    // assigned: written, refused.
    assert_eq!(
        codes(&Options::default().with(Extension::InternalReads)),
        vec![("W0012", 13), ("W0012", 14), ("E0010", 15)]
    );
    assert_eq!(codes(&Options::default()), vec![("E0010", 13), ("E0010", 14), ("E0010", 15)]);
}

#[test]
fn method_in_outs_warned_and_kept() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK axis\nVAR_IN_OUT drive : INT; END_VAR\n\
                METHOD PUBLIC stop\ndrive := 0;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                PROGRAM main\nVAR ax : axis; g : INT; END_VAR\nVAR_TEMP t : INT; END_VAR\n\
                ax(drive := g);\nax(drive := t);\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    assert_eq!(codes(&Options::default()), vec![("E0010", 4)]);
    // Used, warned; a VAR_TEMP bound to it would be gone: E0027.
    assert_eq!(codes(&Options::default().with(Extension::MethodInOuts)), vec![("W0013", 4), ("E0027", 11)]);
}

#[test]
fn fb_init_arguments_where_declared() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK obj\nVAR k : INT; END_VAR\n\
                METHOD FB_init : BOOL\nVAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; n : INT; END_VAR\n\
                k := n;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK plain\nVAR k : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
                PROGRAM main\nVAR a : obj(5); b : obj(n := 6); c : plain(1); END_VAR\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    assert_eq!(codes(&Options::default()), vec![("E0005", 12), ("E0005", 12), ("E0005", 12)]);
    // In order, by name; a type with no FB_init takes none.
    assert_eq!(codes(&Options::default().with(Extension::FbInit)), vec![("E0016", 12)]);
}

#[test]
fn codesys_oo_bare_method_calls() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK a\nMETHOD PUBLIC k : INT\nk := 1;\nEND_METHOD\n\
                METHOD PUBLIC t : INT\nVAR m : INT; END_VAR\nt := k() * 2;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK b EXTENDS a\nMETHOD PUBLIC u : INT\nu := k();\nEND_METHOD\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        iecf_sema::check_with(&[&parsed.unit], o)
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Ed.3: THIS.k(); CODESYS calls it bare, its own or a base's.
    assert_eq!(codes(&Options::default()), vec![("E0010", 7), ("E0010", 12)]);
    assert!(codes(&Options::default().with(Extension::CodesysOo)).is_empty());
}

#[test]
fn struct_extends_base_fields_first() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE\nb : STRUCT a : INT; END_STRUCT;\nx EXTENDS b : STRUCT c : INT; END_STRUCT;\n\
                y EXTENDS b : STRUCT a : INT; END_STRUCT;\nEND_TYPE\n\
                FUNCTION f : INT\nVAR v : x; END_VAR\nf := v.a + v.c;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty());
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let off = codes(&Options::default());
    assert_eq!(off.iter().filter(|(c, _)| *c == "E0005").count(), 2);
    // x has b's a, then c; y's own a is b's again.
    assert_eq!(codes(&Options::default().with(Extension::StructExtends)), vec![("E0012", 4)]);
}

#[test]
fn struct_extends_in_outs_of_a_base() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE\nb : STRUCT a : INT; END_STRUCT;\nm EXTENDS b : STRUCT c : INT; END_STRUCT;\n\
                t EXTENDS m : STRUCT d : INT; END_STRUCT;\no : STRUCT a : INT; END_STRUCT;\nEND_TYPE\n\
                FUNCTION fb : INT\nVAR_IN_OUT io : b; END_VAR\nfb := io.a;\nEND_FUNCTION\n\
                FUNCTION fm : INT\nVAR_IN_OUT io : m; END_VAR\nfm := io.c;\nEND_FUNCTION\n\
                FUNCTION fi : INT\nVAR_INPUT i : b; END_VAR\nfi := i.a;\nEND_FUNCTION\n\
                FUNCTION g : INT\nVAR vt : t; vm : m; vb : b; vo : o; END_VAR\n\
                g := fb(io := vt) + fm(io := vt) + fb(io := vm);\n\
                g := fm(io := vb);\n\
                g := fb(io := vo);\n\
                g := fi(i := vt);\n\
                vb := vt;\n\
                END_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut units = vec![parsed.unit.clone()];
    let got: Vec<_> = iecf_sema::analyze_project(&mut units, &Options::default().with(Extension::StructExtends))
        .diagnostics
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect();
    // A t or an m is a b's in-out's, at any depth; a b no m's, an o (its
    // fields alike) no b's; an input or an assignment takes a b alone.
    assert_eq!(got, vec![("E0013", 22), ("E0013", 23), ("E0013", 24), ("E0013", 25)]);
}

#[test]
fn var_stat_globals_of_the_pou() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK b\nVAR_STAT n : INT; END_VAR\nVAR v : INT; END_VAR\n\
                METHOD PUBLIC m : INT\nVAR_STAT k : INT; END_VAR\nk := k + n;\nm := k;\nEND_METHOD\n\
                n := n + 1;\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK d EXTENDS b\nv := n;\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK e\nVAR w : INT; END_VAR\nVAR_STAT w : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR VAR_STAT : INT; END_VAR\nVAR_STAT := 1;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Each VAR_STAT refused; VAR_STAT stays a name where no section is.
    assert_eq!(codes(&Options::default()), vec![("E0005", 2), ("E0005", 5), ("E0005", 16), ("E0012", 16)]);
    // The block's, the method's and the base's reached; e's w twice.
    assert_eq!(codes(&Options::default().with(Extension::VarStat)), vec![("E0012", 16)]);
}

#[test]
fn method_in_outs_tested_with_isvalidref() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE s : STRUCT d : INT; END_STRUCT; END_TYPE\n\
                FUNCTION_BLOCK b\nVAR_IN_OUT io : s; END_VAR\n\
                METHOD PUBLIC m : BOOL\n\
                IF __ISVALIDREF(io) AND __ISVALIDREF(THIS.io) THEN\nio.d := io.d + 1;\nEND_IF;\n\
                m := __ISVALIDREF(io);\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK e EXTENDS b\nVAR v : BOOL; END_VAR\nv := __ISVALIDREF(io);\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // The tests are no use; the two uses on line 6 warned once each; a
    // derived body tests its base's. Not without the extension.
    assert_eq!(codes(&Options::default().with(Extension::MethodInOuts)), vec![("W0013", 6), ("W0013", 6)]);
    assert!(codes(&Options::default()).iter().all(|(c, _)| *c != "W0013"));
}

#[test]
fn short_circuit_operators() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION f : BOOL\nVAR a, b : BOOL; i : INT; AND_THEN : INT; END_VAR\n\
                f := a AND_THEN b OR_ELSE a;\nf := i AND_THEN b;\nAND_THEN := 1;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Each refused; a name elsewhere; BOOLs only.
    assert_eq!(codes(&Options::default()), vec![("E0005", 3), ("E0005", 3), ("E0005", 4), ("E0014", 4)]);
    assert_eq!(codes(&Options::default().with(Extension::ShortCircuit)), vec![("E0014", 4)]);
}

#[test]
fn codesys_oo_interface_compared_with_zero() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE i\nEND_INTERFACE\nFUNCTION f : BOOL\nVAR r : i; END_VAR\n\
                f := r <> 0 OR 0 = r;\nf := r < 0 OR r = 1;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // = and <> with 0 only.
    assert_eq!(codes(&Options::default()), vec![("E0014", 5), ("E0014", 5), ("E0014", 6), ("E0014", 6)]);
    assert_eq!(codes(&Options::default().with(Extension::CodesysOo)), vec![("E0014", 6), ("E0014", 6)]);
}

#[test]
fn codesys_oo_interface_given_zero() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE i\nEND_INTERFACE\nFUNCTION g : i\nVAR_INPUT a : i; END_VAR\ng := 0;\nEND_FUNCTION\n\
                FUNCTION f : BOOL\nVAR r : i := 0; n : INT; END_VAR\n\
                r := 0;\nr := g(a := 0);\n\
                r := 1;\nr := INT#0;\nr := n;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // The literal 0 alone, as an initial value, assigned, given, returned.
    let refused = vec![("E0013", 11), ("E0013", 12), ("E0013", 13)];
    let off = codes(&Options::default());
    assert_eq!(off.iter().filter(|(c, _)| *c == "E0013").count(), 7, "{off:?}");
    assert_eq!(codes(&Options::default().with(Extension::CodesysOo)), refused);
}

#[test]
fn a_property_giving_a_reference_assigned_whole() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK d\nVAR _b : BOOL; _p : BOOL; END_VAR\n\
                PROPERTY r : REFERENCE TO BOOL\nGET\nr REF= _b;\nEND_GET\nEND_PROPERTY\n\
                PROPERTY t : REF_TO BOOL\nGET\nt := REF(_p);\nEND_GET\nEND_PROPERTY\n\
                PROPERTY v : BOOL\nGET\nv := _b;\nEND_GET\nEND_PROPERTY\n\
                METHOD m\nr := FALSE;\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR x : d; END_VAR\n\
                x.r := TRUE;\nx.t := NULL;\nx.v := TRUE;\nx.t^ := TRUE;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut units = vec![parsed.unit.clone()];
    let o = Options::default().with(Extension::Properties).with(Extension::ReferenceTo).with(Extension::CodesysOo);
    let got: Vec<_> = iecf_sema::analyze_project(&mut units, &o)
        .diagnostics
        .iter()
        .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
        .collect();
    // A REFERENCE TO's GET takes a whole assignment, inside its block
    // too; a REF_TO's needs its ^, and a value's a SET.
    assert_eq!(got, vec![("E0015", 25), ("E0015", 26)]);
}

#[test]
fn a_pou_named_as_a_keyword_is_the_parser_s_error_alone() {
    use iecf_sema::{Options, Pack};
    // TASK is a keyword: the parser finds no name, and the checker, which
    // registers no POU without one, does not look it up (it panicked).
    let text = "INTERFACE it\nMETHOD m : INT\nEND_METHOD\nEND_INTERFACE\n\
                FUNCTION_BLOCK Task IMPLEMENTS it\nVAR x : INT; END_VAR\n\
                METHOD m : INT\nm := x;\nEND_METHOD\nx := x + 1;\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert_eq!(parsed.diagnostics.iter().map(|d| d.code.id()).collect::<Vec<_>>(), vec!["E0005"]);
    let mut o = Options::default();
    for e in Pack::Codesys.members() {
        o = o.with(*e);
    }
    for o in [Options::default(), o] {
        let mut units = vec![parsed.unit.clone()];
        assert!(iecf_sema::analyze_project(&mut units, &o).diagnostics.is_empty());
    }
}

#[test]
fn header_semicolon_and_a_method_hiding_its_type_s_variable() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK b\nVAR x : BOOL; y : INT; END_VAR\n\
                METHOD m : INT;\nVAR_INPUT x : INT; END_VAR\nx := 1;\nTHIS^.x := TRUE;\nm := x;\nEND_METHOD\n\
                METHOD k : INT\nVAR y : INT; y : INT; END_VAR\nk := 1;\nEND_METHOD\n\
                END_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let o = Options::default().with(Extension::CodesysOo);
    // The `;` refused; each of a method's names a type's is refused, and
    // with them `x := 1` (BOOL) and m := x.
    let off = codes(&o);
    assert!(off.contains(&("E0005", 3)) && off.contains(&("E0012", 4)), "{off:?}");
    assert!(off.contains(&("E0012", 10)), "{off:?}");
    // Both on: x is the method's INT, THIS^.x the block's BOOL; a second
    // y in one method is declared twice still.
    let on = codes(&o.with(Extension::HeaderSemicolon).with(Extension::Shadowing));
    assert_eq!(on, vec![("E0012", 10)]);
}

#[test]
fn expression_statements_warned() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION g : INT\ng := 1;\nEND_FUNCTION\n\
                FUNCTION_BLOCK b\nVAR_OUTPUT x : INT; END_VAR\n\
                PROPERTY p : BOOL\nGET\np := TRUE;\nEND_GET\nEND_PROPERTY\nEND_FUNCTION_BLOCK\n\
                PROGRAM m\nVAR i : b; n : INT; END_VAR\n\
                i.p;\ng() + 1;\ni.x;\nn;\nn = 1;\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    // `n = 1;` the parser's: '=' compares.
    assert_eq!(parsed.diagnostics.iter().map(|d| d.code.id()).collect::<Vec<_>>(), vec!["E0005"]);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let o = Options::default().with(Extension::Properties).with(Extension::CodesysOo);
    // Without the extension each is E0005, naming it.
    assert_eq!(codes(&o), vec![("E0005", 14), ("E0005", 15), ("E0005", 16), ("E0005", 17)]);
    let on = codes(&o.with(Extension::ExpressionStatements));
    assert_eq!(on, vec![("W0014", 14), ("W0014", 15), ("W0014", 16), ("W0014", 17)]);
    let msgs: Vec<String> = {
        let mut units = vec![parsed.unit.clone()];
        let o = Options::default()
            .with(Extension::Properties)
            .with(Extension::CodesysOo)
            .with(Extension::ExpressionStatements);
        iecf_sema::analyze_project(&mut units, &o).diagnostics.iter().map(|d| d.message.clone()).collect()
    };
    // A GET and a FUNCTION are calls; a field and a variable are not.
    let made: Vec<bool> = msgs.iter().map(|m| m.contains("its calls are made")).collect();
    assert_eq!(made, vec![true, true, false, false]);
}

#[test]
fn type_access_internal_type() {
    use iecf_sema::{Extension, Options};
    let text = "NAMESPACE L\nTYPE INTERNAL e : (a, b); END_TYPE\nTYPE PUBLIC g : INT; END_TYPE\n\
                FUNCTION f : INT\nVAR x : e; END_VAR\nf := 1;\nEND_FUNCTION\nEND_NAMESPACE\n\
                PROGRAM p\nVAR y : L.e; z : L.g; END_VAR\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Each refused, the type public; with it, e is seen inside L only.
    assert_eq!(codes(&Options::default()), vec![("E0005", 2), ("E0005", 3)]);
    assert_eq!(codes(&Options::default().with(Extension::TypeAccess)), vec![("E0028", 10)]);
}

#[test]
fn var_inst_kept_in_the_instance() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK b\nMETHOD PUBLIC m : INT\nVAR_INST n : INT; END_VAR\nn := n + 1;\nm := n;\nEND_METHOD\n\
                METHOD PUBLIC k\nVAR v : INT; END_VAR\nVAR_INST v : INT; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION f : INT\nVAR_INST w : INT; END_VAR\nf := 1;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Each refused (k's v twice too); with it, k's v twice and a
    // FUNCTION's VAR_INST.
    assert_eq!(codes(&Options::default()), vec![("E0005", 3), ("E0005", 9), ("E0012", 9), ("E0005", 13)]);
    assert_eq!(codes(&Options::default().with(Extension::VarInst)), vec![("E0012", 9), ("E0005", 13)]);
}

#[test]
fn any_input_takes_any_value() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION k : BOOL\nVAR_INPUT d : ANY; END_VAR\n\
                k := d.TypeClass = __SYSTEM.TYPE_CLASS.TYPE_INT AND d.diSize > 0;\nEND_FUNCTION\n\
                FUNCTION g : BOOL\nVAR i : INT; s : STRING; o : ANY; END_VAR\ng := k(i) AND k(s) AND k(1.5);\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // With it, only an input is ANY: g's local is no type.
    let on = codes(&Options::default().with(Extension::AnyInput).with(Extension::Pointers));
    assert_eq!(on, vec![("E0011", 6)]);
    assert!(codes(&Options::default()).contains(&("E0011", 2)));
}

#[test]
fn type_constants_through_the_type_name() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK e\nVAR CONSTANT n : INT := 4; m : INT := n + 1; END_VAR\nVAR v : INT := 2; END_VAR\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION f : INT\nVAR s : STRING[e.m]; END_VAR\nf := e.n + e.v;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // A constant's, not a variable's: e.v stays no value.
    let off = codes(&Options::default());
    assert!(off.iter().any(|(_, l)| *l == 6) && off.iter().any(|(_, l)| *l == 7), "{off:?}");
    let on = codes(&Options::default().with(Extension::TypeConstants));
    assert!(on.iter().all(|(_, l)| *l == 7) && !on.is_empty(), "{on:?}");
}

#[test]
fn union_gives_one_member_its_value() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE\nU : UNION\n    r : REAL;\n    d : DWORD;\nEND_UNION;\nEND_TYPE\n\
                FUNCTION f : INT\nVAR\n    a : U;\n    b : U := (d := 5);\n    \
                c : U := (r := 1.0, d := 2);\nEND_VAR\nf := 0;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without the extension, UNION itself and both explicit inits are
    // refused (a plain STRUCT OVERLAP takes none).
    assert_eq!(codes(&Options::default()), vec![("E0005", 2), ("E0034", 10), ("E0034", 11)]);
    // With it: a is fine with no override, b with d's alone; c gives
    // two members, which CODESYS and TwinCAT allow at most one.
    assert_eq!(codes(&Options::default().with(Extension::Union)), vec![("E0034", 11)]);
}

#[test]
fn action_call_not_a_steps_too() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK counter\nVAR n : INT; END_VAR\n\
                INITIAL_STEP s0 : tick(N); END_STEP\n\
                TRANSITION FROM s0 TO s0 := FALSE; END_TRANSITION\n\
                ACTION tick : n := n + 1; END_ACTION\nACTION inc : n := n + 1; END_ACTION\n\
                END_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR c : counter; END_VAR\nc.inc();\nc.tick();\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without the extension, neither call is let through.
    assert_eq!(codes(&Options::default()), vec![("E0016", 10), ("E0016", 11)]);
    // With it: inc, which no step runs, is fine; tick is s0's too.
    assert_eq!(codes(&Options::default().with(Extension::ActionCall)), vec![("E0016", 11)]);
}

#[test]
fn recursion_lets_a_pou_call_itself() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION fact : DINT\nVAR_INPUT n : DINT; END_VAR\n\
                IF n <= 1 THEN fact := 1; ELSE fact := n * fact(n - 1); END_IF;\nEND_FUNCTION\n\
                FUNCTION_BLOCK tree\nVAR k : INT; END_VAR\n\
                METHOD sum : INT\nVAR_INPUT x : INT; END_VAR\n\
                IF x > 0 THEN sum := x + THIS.sum(x - 1); END_IF;\nEND_METHOD\n\
                END_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK pingpong\nVAR n : INT; END_VAR\n\
                INITIAL_STEP s0 : go(N); END_STEP\n\
                TRANSITION FROM s0 TO s0 := FALSE; END_TRANSITION\n\
                ACTION go : ping(); END_ACTION\n\
                ACTION ping : IF n > 0 THEN n := n - 1; pong(); END_IF; END_ACTION\n\
                ACTION pong : ping(); END_ACTION\n\
                END_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let check = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        let a = iecf_sema::analyze_project(&mut units, o);
        let codes = a
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>();
        let rec = ["FACT", "TREE__SUM", "PINGPONG__PING", "PINGPONG__PONG", "PINGPONG__GO", "PINGPONG"]
            .map(|k| a.types.recursive(k));
        (codes, rec)
    };
    let ac = Options::default().with(Extension::ActionCall);
    // Without the extension: fact's own name is its result, sum calls
    // itself, and ping and pong each other - two actions of one block,
    // each its own in the graph.
    assert_eq!(check(&ac).0, vec![("E0016", 3), ("E0016", 9), ("E0016", 18)]);
    // With it, each that can reach itself is counted; go and the block,
    // which only reach the cycle, are not.
    assert_eq!(check(&ac.with(Extension::Recursion)), (vec![], [true, true, true, true, false, false]));
}

#[test]
fn ref_this_binds_the_own_instance() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK acc\nVAR n : DINT; END_VAR\nMETHOD PUBLIC me : REF_TO acc\nme := REF(THIS);\n\
                END_METHOD\nMETHOD PUBLIC add : REFERENCE TO acc\nadd REF= THIS^;\nEND_METHOD\n\
                END_FUNCTION_BLOCK\nFUNCTION f : INT\nVAR r : REF_TO acc; END_VAR\nr := REF(THIS);\nf := 0;\n\
                END_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lines = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        let mut l: Vec<usize> = iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| text[..d.span.start as usize].matches('\n').count() + 1)
            .collect();
        l.dedup();
        l
    };
    // REF(THIS) in a method is the own instance's, as Ed.3's THIS is; in a
    // FUNCTION there is no THIS. CODESYS's REFERENCE TO and THIS^ only
    // with their extensions.
    assert_eq!(lines(&Options::default()), vec![6, 7, 12]);
    let on = Options::default().with(Extension::ReferenceTo).with(Extension::CodesysOo);
    assert_eq!(lines(&on), vec![12]);
}

#[test]
fn any_to_is_the_overloaded_conversion() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION ANY_TO_WORD : INT\nVAR_INPUT x : INT; END_VAR\nANY_TO_WORD := x;\nEND_FUNCTION\n\
                FUNCTION f : INT\nVAR r : REAL; s : STRING; w : INT; END_VAR\nr := ANY_TO_REAL(w + 1);\n\
                s := ANY_TO_STRING(r);\nw := ANY_TO_WORD(3);\nf := ANY_TO_INT(r);\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // The project's own ANY_TO_WORD is called as it is, its INT result
    // taken by w; the others are TO_REAL, TO_STRING, TO_INT.
    assert_eq!(codes(&Options::default()), vec![("E0010", 7), ("E0010", 8), ("E0010", 10)]);
    assert_eq!(codes(&Options::default().with(Extension::AnyTo)), vec![]);
}

#[test]
fn var_info_is_known_of_a_variable() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK f\nVAR i : __SYSTEM.VAR_INFO; n : INT; END_VAR\n\
                i := __VARINFO(n);\ni := __VARINFO(n + 1);\ni := __VARINFO(n, n);\n\
                IF i.MemoryArea = __SYSTEM.MEMORY_AREA.MEM_LOCAL THEN n := LEN(i.Symbol); END_IF;\n\
                END_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Of a variable only, one; without the extension no such name.
    let with = codes(&Options::default().with(Extension::VarInfo));
    assert_eq!(with, vec![("E0016", 4), ("E0016", 5)], "{with:?}");
    let without = codes(&Options::default());
    assert!(without.iter().filter(|(c, _)| *c == "E0010").count() >= 3, "{without:?}");
}

#[test]
fn empty_arguments_are_left_out() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK c\nVAR_INPUT i : INT; END_VAR\nVAR_OUTPUT q : BOOL; END_VAR\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK f\nVAR x : c; END_VAR\nx(i :=, q =>);\nx(i :=,\nq => );\nx(i := 1,\n);\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // And a last `,` before `)`, an empty one with no name.
    assert_eq!(codes(&Options::default()), vec![("E0005", 7), ("E0005", 7), ("E0005", 8), ("E0005", 9), ("E0005", 10)]);
    assert_eq!(codes(&Options::default().with(Extension::EmptyArguments)), vec![]);
}

#[test]
fn bit_is_placed_in_its_structure() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE S : STRUCT a : BIT; n : INT; END_STRUCT; END_TYPE\n\
                TYPE T : STRUCT a : BIT; s : S; END_STRUCT; END_TYPE\n\
                FUNCTION_BLOCK f\nVAR x : BIT; END_VAR\nx := TRUE;\nEND_FUNCTION_BLOCK\n\
                FUNCTION g : BOOL\nVAR y : BIT; END_VAR\ng := y;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without the extension, each BIT; with it, T's (beside a member of a
    // named type) and the FUNCTION's.
    assert_eq!(codes(&Options::default()), vec![("E0011", 1), ("E0011", 2), ("E0011", 4), ("E0011", 8)]);
    assert_eq!(codes(&Options::default().with(Extension::Bit)), vec![("E0011", 2), ("E0011", 8)]);
}

#[test]
fn super_calls_take_codesys_forms() {
    use iecf_sema::{Extension, Options, Pack};
    let text = "FUNCTION_BLOCK b\nVAR_INPUT i : INT; END_VAR\nVAR_IN_OUT io : INT; END_VAR\nVAR n : INT; END_VAR\n\
                PROPERTY PUBLIC p : INT\nGET\np := 1;\nEND_GET\nEND_PROPERTY\nn := i;\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK d EXTENDS b\nVAR k : INT; j : INT; END_VAR\n\
                METHOD PUBLIC m\nSUPER^();\nEND_METHOD\n\
                SUPER^(i := 1, io := io);\nSUPER^();\nFOR j := 1 TO 2 DO SUPER^(); END_FOR;\n\
                k := SUPER^.n + SUPER^.p;\nk := SUPER^.q;\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let pack: Vec<Extension> =
        Pack::Codesys.members().iter().copied().filter(|e| *e != Extension::SuperCalls).collect();
    // Without super-calls: SUPER() in a method, with arguments, in a loop
    // (the second, line 18, is the first that stands), SUPER.x thrice
    // (line 20 twice); with it, only the name no base has (line 21).
    assert_eq!(
        codes(&Options::default().with_all(&pack)),
        vec![("E0031", 15), ("E0031", 17), ("E0031", 19), ("E0031", 20), ("E0031", 20), ("E0031", 21)]
    );
    assert_eq!(codes(&Options::default().with_all(&pack).with(Extension::SuperCalls)), vec![("E0010", 21)]);
}

#[test]
fn this_deref_in_an_initial_value() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE i\nEND_INTERFACE\nTYPE s : STRUCT p : i; END_STRUCT; END_TYPE\n\
                FUNCTION_BLOCK f IMPLEMENTS i\nVAR d : s := (p := THIS^); e : ARRAY[0..1] OF i := [THIS^, THIS^]; END_VAR\n\
                END_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let base = Options::default().with(Extension::InitialExpressions);
    assert_eq!(codes(&base), vec![("E0014", 5), ("E0014", 5), ("E0014", 5)]);
    assert_eq!(codes(&base.with(Extension::CodesysOo)), vec![]);
}

#[test]
fn struct_blocks_set_up_members() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE i\nEND_INTERFACE\nFUNCTION_BLOCK m IMPLEMENTS i\nMETHOD FB_init : BOOL\n\
                VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; p : i; END_VAR\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                TYPE\nABSTRACT s : STRUCT a : m(b); b : m(zz); END_STRUCT;\nEND_TYPE\n\
                FUNCTION_BLOCK h\nVAR v : s; END_VAR\nEND_FUNCTION_BLOCK\n\
                FUNCTION f : INT\nVAR w : s; END_VAR\nf := 0;\nEND_FUNCTION\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let base = Options::default().with(Extension::FbInit);
    // Without struct-blocks: ABSTRACT, each member's arguments, and each
    // variable of the structure; with it, the name nothing declares, and
    // the FUNCTION's variable (E0031 too: a block with methods there).
    assert_eq!(
        codes(&base),
        vec![("E0005", 9), ("E0009", 9), ("E0009", 9), ("E0009", 12), ("E0009", 15), ("E0031", 15)]
    );
    assert_eq!(codes(&base.with(Extension::StructBlocks)), vec![("E0010", 9), ("E0009", 15), ("E0031", 15)]);
}

#[test]
fn query_interface_tries_an_interface() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE i EXTENDS __SYSTEM.IQueryInterface\nEND_INTERFACE\n\
                INTERFACE j EXTENDS __System.IQueryInterface\nEND_INTERFACE\n\
                FUNCTION_BLOCK b IMPLEMENTS i, j\nEND_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR x : i; y : j; n : INT; k : b; ok : BOOL; END_VAR\n\
                ok := __QUERYINTERFACE(x, y);\n\
                ok := __QUERYINTERFACE(k, y);\n\
                ok := __QUERYINTERFACE(x, n);\n\
                ok := __QUERYINTERFACE(x);\n\
                ok := __QUERYINTERFACE(n, y) AND ok;\n\
                END_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without it: each IQueryInterface, each call.
    assert_eq!(
        codes(&Options::default()),
        vec![("E0011", 1), ("E0011", 3), ("E0010", 9), ("E0010", 10), ("E0010", 11), ("E0010", 12), ("E0010", 13)]
    );
    // With it: an instance first, not an interface stored into, one
    // argument, an INT first.
    assert_eq!(
        codes(&Options::default().with(Extension::QueryInterface)),
        vec![("E0016", 10), ("E0014", 11), ("E0016", 12), ("E0014", 13)]
    );
}

#[test]
fn interface_methods_public_and_final_with_codesys_oo() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE i\nMETHOD PUBLIC a : INT\nEND_METHOD\nMETHOD PUBLIC FINAL b : INT\nEND_METHOD\n\
                METHOD PRIVATE c : INT\nEND_METHOD\nMETHOD ABSTRACT d : INT\nEND_METHOD\nEND_INTERFACE\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Each specifier without it; with it, PRIVATE and ABSTRACT still.
    assert_eq!(codes(&Options::default()), vec![("E0031", 2), ("E0031", 4), ("E0031", 6), ("E0031", 8)]);
    assert_eq!(codes(&Options::default().with(Extension::CodesysOo)), vec![("E0031", 6), ("E0031", 8)]);
}

#[test]
fn codesys_oo_an_object_copied_of_its_own_type() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK b\nMETHOD m\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK d EXTENDS b\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK u\nVAR_INPUT i : b; END_VAR\nVAR_OUTPUT o : b; END_VAR\n\
                VAR x : b; y : d; arr : ARRAY[1..2] OF b; arr2 : ARRAY[1..2] OF b; END_VAR\n\
                METHOD mm\nVAR t : d; END_VAR\nEND_METHOD\n\
                x := i;\nx := y;\narr := arr2;\nEND_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without: the input, the output, the method's VAR, each copy.
    assert_eq!(
        codes(&Options::default()),
        vec![("E0031", 8), ("E0031", 9), ("E0031", 12), ("E0031", 14), ("E0031", 15), ("E0031", 16)]
    );
    // codesys-oo and temp-instances: the input, the method's instance,
    // the copy of its own type; not an output, a derived one into a
    // base, an array holding one.
    let on = Options::default().with(Extension::CodesysOo).with(Extension::TempInstances);
    let left = codes(&on);
    assert_eq!(left.iter().map(|(_, l)| *l).collect::<Vec<_>>(), vec![9, 15, 16], "{left:?}");
}

#[test]
fn codesys_oo_abstract_without_an_abstract_method() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK ABSTRACT b\nMETHOD m\nEND_METHOD\nEND_FUNCTION_BLOCK\n\
                FUNCTION_BLOCK d EXTENDS b\nEND_FUNCTION_BLOCK\n\
                PROGRAM p\nVAR x : d; y : b; END_VAR\nEND_PROGRAM\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // Without: ABSTRACT with no abstract method; with: only the instance
    // of the abstract type, which neither allows.
    assert_eq!(codes(&Options::default()), vec![("E0031", 1), ("E0031", 8)]);
    assert_eq!(codes(&Options::default().with(Extension::CodesysOo)), vec![("E0031", 8)]);
}

#[test]
fn properties_an_interface_accessors_empty_var() {
    use iecf_sema::{Extension, Options};
    let text = "INTERFACE i\nPROPERTY p : BOOL\nGET\nVAR\nEND_VAR\nEND_GET\nEND_PROPERTY\n\
                PROPERTY q : BOOL\nGET\nVAR x : INT; END_VAR\nEND_GET\nEND_PROPERTY\n\
                METHOD m\nVAR\nEND_VAR\nEND_METHOD\nEND_INTERFACE\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .filter(|d| d.code.id() == "E0031")
            .map(|d| text[..d.span.start as usize].matches('\n').count() + 1)
            .collect::<Vec<_>>()
    };
    // The empty VAR of an accessor only; one declaring a variable, and a
    // method's, stay E0031.
    assert_eq!(codes(&Options::default().with(Extension::Properties)), vec![10, 14]);
}

#[test]
fn stray_semicolons_extension() {
    use iecf_sema::{Extension, Options};
    let text = "TYPE P : STRUCT a : INT; END_STRUCT; END_TYPE;\n\
                FUNCTION_BLOCK b\nVAR x : INT;; y : INT; END_VAR\nVAR\n;\nEND_VAR\nEND_FUNCTION_BLOCK;\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    // After END_TYPE, the doubled one, the one alone, after
    // END_FUNCTION_BLOCK; none with the extension.
    assert_eq!(codes(&Options::default()), vec![("E0005", 1), ("E0005", 3), ("E0005", 5), ("E0005", 7)]);
    assert!(codes(&Options::default().with(Extension::StraySemicolons)).is_empty());
}

#[test]
fn literal_widening_extension() {
    use iecf_sema::{Extension, Options};
    let text = "FUNCTION_BLOCK f\nVAR b : BYTE; w : WORD; d : DWORD; x : DINT; l : LINT; k : INT; t : BOOL; u : UINT; i : DINT; END_VAR\n\
                w := 256 * b;\n\
                d := w * 65536;\n\
                l := 2147483648 + x;\n\
                t := k < 70000;\n\
                i := u * -1;\n\
                w := b * -1;\n\
                l := x + 18446744073709551616;\n\
                END_FUNCTION_BLOCK\n";
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let codes = |o: &Options| {
        let mut units = vec![parsed.unit.clone()];
        iecf_sema::analyze_project(&mut units, o)
            .diagnostics
            .iter()
            .map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1))
            .collect::<Vec<_>>()
    };
    let bits = Options::default().with(Extension::BitStringArithmetic);
    let lits = |ls: &[usize]| ls.iter().map(|l| ("E0020", *l)).collect::<Vec<_>>();
    assert_eq!(codes(&bits), lits(&[3, 4, 5, 6, 7, 8, 9]));
    // A bit string has no negative values, and nothing holds 2^64.
    assert_eq!(codes(&bits.with(Extension::LiteralWidening)), lits(&[8, 9]));
}

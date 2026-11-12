//! The generator's own tests: what it refuses, and the shape of what it
//! writes. Whether the C means what the program says is the harness's
//! business (harness/README.md), run against the reference compiler's traces.

use iecf_diagnostics::{FileId, SourceMap};

type Files = Vec<(String, String)>;
type Errors = Vec<(&'static str, String)>;

fn generate(text: &str) -> Result<Files, Errors> {
    generate_with(text, &iecf_sema::Options::default())
}

fn generate_with(text: &str, options: &iecf_sema::Options) -> Result<Files, Errors> {
    let mut map = SourceMap::new();
    let id = map.add("t.st", text);
    assert_eq!(id, FileId(0));
    let parsed = iecf_syntax::parse(id, text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let units = [&parsed.unit];
    let a = iecf_sema::analyze(&units, options);
    assert!(a.diagnostics.iter().all(|d| !d.is_error()), "{:?}", a.diagnostics);
    crate::generate(&units, &a.types, &map).map_err(|ds| ds.into_iter().map(|d| (d.code.id(), d.message)).collect())
}

const CONFIG: &str = "CONFIGURATION Plant\n  RESOURCE r ON PLC\n    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n    PROGRAM p WITH fast : main;\n  END_RESOURCE\nEND_CONFIGURATION\n";

#[test]
fn a_program_and_its_configuration() {
    let text = format!(
        "PROGRAM main\nVAR x : INT := 5; END_VAR\nVAR q AT %QW2 : INT; END_VAR\nq := x + 1;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["plant.h", "plant.c", "iecf.h", "plant.symbols.json"]);
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("#define PLANT_Q_BYTES 4u"), "{h}");
    assert!(h.contains("void plant_task_R__FAST(plant_t *ctx, int64_t now_ns);"), "{h}");
    assert!(c.contains("iecf_add_INT(&ctx->flag, self->V_X, ((IEC_INT)INT64_C(1)))"), "{c}");
    assert!(c.contains("iecf_noted(&ctx->flag, 0u, 4u);") && !c.contains("iecf_stopped"), "{c}");
    assert!(c.contains("iecf_set_w(ctx->image.q, 2u, (uint16_t)("), "{c}");
    assert!(c.contains("INT64_C(10000000), INT64_C(1)"), "the task table: {c}");
}

#[test]
fn what_is_not_compiled_yet_is_e0009() {
    let errs = generate("PROGRAM main\nEND_PROGRAM\n").unwrap_err();
    assert_eq!(errs[0].0, "E0009", "no CONFIGURATION");
}

#[test]
fn a_case_label_names_a_constant() {
    // Ed.3 7.3.3.3.3: a label is a constant expression; a value the
    // selector's type cannot hold never matches.
    let text = format!(
        "PROGRAM main\nVAR CONSTANT c : INT := 3; END_VAR\nVAR x : INT; END_VAR\n\
         CASE x OF c - 1: x := 2; c..c * 2: x := 0; END_CASE;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    assert!(c.contains("== ((IEC_INT)INT64_C(2))"), "{c}");
    assert!(c.contains(">= ((IEC_INT)INT64_C(3)) &&") && c.contains("<= ((IEC_INT)INT64_C(6))"), "{c}");
}

#[test]
fn an_initial_value_is_a_constant_expression_worked_out_here() {
    // Ed.3 6.6.1.2.4, Figure 6: nothing of it is left to the run.
    let one = |decl: &str| generate(&format!("PROGRAM main\n{decl}\nx := x;\nEND_PROGRAM\n{CONFIG}"));
    let files =
        one("VAR CONSTANT k : INT := 3; k2 : INT := k + 1; END_VAR VAR x : DINT := k2 * 10000; END_VAR").unwrap();
    let c = &files[1].1;
    assert!(c.contains("((IEC_DINT)INT64_C(40000))"), "{c}");
    let code = |decl: &str| one(decl).unwrap_err()[0].0;
    // A value the variable does not hold; a real that is infinite.
    assert_eq!(code("VAR CONSTANT k : INT := 300; END_VAR VAR x : INT := k * 200; END_VAR"), "E0020");
    assert_eq!(code("VAR CONSTANT kr : REAL := 3.0E38; END_VAR VAR x : REAL := kr * 10.0; END_VAR"), "E0020");
    // No value at all: a division by zero, a constant that names itself.
    assert_eq!(code("VAR CONSTANT kr : REAL := 0.0; END_VAR VAR x : REAL := 1.0 / kr; END_VAR"), "E0032");
    assert_eq!(code("VAR CONSTANT k : INT := 0; END_VAR VAR x : INT := 5 / k; END_VAR"), "E0032");
    assert_eq!(code("VAR CONSTANT a : INT := b; b : INT := a; END_VAR VAR x : INT := a; END_VAR"), "E0032");
    assert_eq!(
        code(
            "VAR CONSTANT a : ARRAY[1..2] OF INT := b; b : ARRAY[1..2] OF INT := a; END_VAR VAR x : ARRAY[1..2] OF INT := a; END_VAR"
        ),
        "E0032"
    );
    // MOD by zero is 0, as when the program runs.
    let files = one("VAR CONSTANT k : INT := 0; END_VAR VAR x : INT := 5 MOD k; END_VAR").unwrap();
    assert!(files[1].1.contains("((IEC_INT)INT64_C(0))"), "{}", files[1].1);
}

#[test]
fn a_wstring_is_utf16_units_through_the_w_functions() {
    let text = format!(
        "PROGRAM main\nVAR s : WSTRING[4] := \"a${{1F600}}\"; c : WCHAR; n : INT; END_VAR\n\
         s := CONCAT(s, \"$263A\");\nc := s[2];\nn := LEN(s);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    assert!(c.contains("IEC_WCHAR V_S[5];") || files[0].1.contains("IEC_WCHAR V_S[5];"), "{c}");
    // A character above U+FFFF: a surrogate pair.
    assert!(c.contains("iecf_wsinit(self->V_S, 4u, ((const IEC_WCHAR[]){97u, 55357u, 56832u, 0u}));"), "{c}");
    assert!(c.contains("iecf_wconcat(&ctx->flag,") && c.contains("((const IEC_WCHAR[]){9786u, 0u})"), "{c}");
    assert!(c.contains("iecf_wschar(&ctx->flag,") && c.contains("iecf_wslen("), "{c}");
    assert!(c.contains("IECF_T_WSTRING, (uint32_t)10u"), "{c}");
    assert!(files[3].1.contains("\"kind\": \"wstring\", \"length\": 4"), "{}", files[3].1);
}

#[test]
fn sel_mux_and_max_give_one_string_every_input_evaluated() {
    let text = format!(
        "PROGRAM main\nVAR s : STRING[4]; w : WSTRING; b : BOOL; k : INT; END_VAR\n\
         s := MUX(k, 'a', CONCAT(s, 'c'));\nw := SEL(b, w, \"q\");\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    // CONCAT's buffer is filled before MUX picks: both inputs evaluated.
    assert!(
        c.contains("iecf_smux(&ctx->fault, (int64_t)(self->V_K), 2u, (const char *const[]){\"a\", iecf_concat("),
        "{c}"
    );
    assert!(c.contains("if (iecf_stopped(&ctx->fault, 0u, 3u)) return;"), "a bad K stops the statement: {c}");
    assert!(c.contains("iecf_wssel(self->V_B, self->V_W, ((const IEC_WCHAR[]){113u, 0u}))"), "{c}");
}

#[test]
fn a_block_s_in_out_is_a_variable_or_a_copy_of_one() {
    let block = "FUNCTION_BLOCK dbl\nVAR_IN_OUT v : INT; END_VAR\nv := v * 2;\nEND_FUNCTION_BLOCK\n";
    let ok = format!("{block}PROGRAM main\nVAR d : dbl; x : INT; END_VAR\nd(v := x);\nEND_PROGRAM\n{CONFIG}");
    let files = generate(&ok).unwrap();
    let c = &files[1].1;
    assert!(c.contains("self->V_D.V_V = &(self->V_X);"), "{c}");
    assert!(c.contains("(*self->V_V) = "), "{c}");
    // At an address: a copy, stored back after the call (expr.rs
    // `in_out_copy`; harness case inoutcopy).
    let located = format!(
        "{block}PROGRAM main\nVAR d : dbl; END_VAR\nVAR x AT %MW0 : INT; END_VAR\nd(v := x);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&located).unwrap()[1].1;
    let call = c.find("B_DBL(ctx, &self->V_D);").expect("the call");
    assert!(c[..call].contains("= iecf_to_i16(iecf_get_w(ctx->image.m, 0u));"), "{c}");
    assert!(c[call..].contains("iecf_set_w(ctx->image.m, 0u, (uint16_t)(q"), "{c}");
}

#[test]
fn var_temp_is_a_local_set_at_each_call() {
    // Re-initialised at each call (Ed.3 6.5.1): not a field of the instance.
    let text = format!(
        "PROGRAM main\nVAR_TEMP t : INT := 3; a : ARRAY [1..2] OF INT; END_VAR\nt := t + 1;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(!h.contains("V_T;"), "not in the instance: {h}");
    let body = &c[c.find("static void P_MAIN(").unwrap()..];
    assert!(body.contains("IEC_INT V_T = ((IEC_INT)INT64_C(3));"), "{body}");
    assert!(body.contains("IEC_INT V_A[2];") && body.contains("V_A[i1] = ((IEC_INT)INT64_C(0));"), "{body}");
    assert!(body.contains("iecf_add_INT(&ctx->flag, V_T, "), "{body}");
}

#[test]
fn a_real_that_does_not_fit_leaves_its_statement_undone() {
    let text = format!(
        "PROGRAM main\nVAR x : REAL; a : INT; c : CTU; END_VAR\na := REAL_TO_INT(x);\nc(CU := TRUE, PV := REAL_TO_INT(x));\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    assert!(c.contains("iecf_round_s(&ctx->flag, &k"), "{c}");
    assert!(c.contains(") self->V_A = t"), "the assignment, kept: {c}");
    assert!(
        c.contains(") self->V_C.PV = t") && c.contains("iecf_CTU(&self->V_C);"),
        "the input kept, the block called: {c}"
    );
    let init = format!("PROGRAM main\nVAR a : INT := REAL_TO_INT(1.0E10); END_VAR\nEND_PROGRAM\n{CONFIG}");
    assert_eq!(generate(&init).unwrap_err()[0].0, "E0020", "no statement to leave undone: worked out, and refused");
}

#[test]
fn derived_types() {
    let text = format!(
        "TYPE mode : (idle, busy) := busy; pair : STRUCT a : INT := 1; b : ARRAY [1..2] OF INT; END_STRUCT; END_TYPE\n\
         PROGRAM main\nVAR m : mode; p, p2 : pair; v : ARRAY [-1..1] OF pair; i : INT; END_VAR\n\
         v[i].b[2] := p.a;\np2 := p;\nIF m = idle THEN m := mode#busy; END_IF;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("typedef struct {\n    IEC_INT V_A;\n    IEC_INT V_B[2];\n} T_PAIR;"), "{h}");
    assert!(h.contains("IEC_INT V_M;") && h.contains("T_PAIR V_V[3];"), "{h}");
    assert!(c.contains("self->V_M = ((IEC_INT)INT64_C(1));"), "the type's initial value, busy: {c}");
    assert!(
        c.contains("uint32_t o")
            && c.contains("iecf_index(&ctx->fault, (int64_t)(self->V_I), INT64_C(-1), INT64_C(1))"),
        "{c}"
    );
    assert!(c.contains("].V_B[1u] = t"), "the index checked before the store: {c}");
    assert!(
        c.contains("iecf_bytes(&(self->V_P2), &(self->V_P), sizeof (self->V_P2));"),
        "a whole structure copied: {c}"
    );
    let out_of_bounds =
        format!("PROGRAM main\nVAR a : ARRAY [1..2] OF INT; END_VAR\na[3] := 0;\nEND_PROGRAM\n{CONFIG}");
    assert_eq!(generate(&out_of_bounds).unwrap_err()[0].0, "E0009", "a constant index out of bounds");
}

#[test]
fn the_symbol_table() {
    let text = "FUNCTION_BLOCK fb\nVAR_INPUT go : BOOL; END_VAR\nVAR_OUTPUT n : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
        PROGRAM main\nVAR x : INT; f : fb; t : TON; END_VAR\nVAR q AT %QX0.3 : BOOL; END_VAR\nVAR CONSTANT k : INT := 3; END_VAR\nEND_PROGRAM\n\
        CONFIGURATION Plant\n  VAR_GLOBAL g : DINT; END_VAR\n  RESOURCE r ON PLC\n    VAR_GLOBAL rg : BOOL; END_VAR\n    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n    PROGRAM p WITH fast : main;\n  END_RESOURCE\nEND_CONFIGURATION\n";
    let files = generate(text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("extern const iecf_symbol_t plant_symbols[];"), "{h}");
    for want in [
        "{\"G\", ",
        "(uint32_t)(offsetof(plant_t, V_G)), 0, 0u, 0u}",
        "{\"R.RG\", ",
        "(offsetof(plant_t, V_R__P) + offsetof(P_MAIN_t, V_X))",
        "{\"R.P.F\", ",
        "{\"R.P.T\", ",
        ", 0u, 'Q', 3u, 0u}",
        "{\"R.P.K\", ",
        "{\"TON\", IECF_KIND_BLOCK, 0u, (uint32_t)sizeof(iecf_TON_t)",
        "{\"IN\", ",
        "(uint32_t)offsetof(iecf_TON_t, IN), 1u, 0}",
        "(uint32_t)offsetof(B_FB_t, V_GO), 1u, 0}",
        "(uint32_t)offsetof(B_FB_t, V_N), 2u, 0}",
        "const uint32_t plant_symbol_count = 7u;",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
    assert!(c.contains("(offsetof(plant_t, V_R__P) + offsetof(P_MAIN_t, V_K)), 0, 0u, 4u}"), "CONSTANT: {c}");
}

#[test]
fn retain_variables_are_one_region() {
    let text = format!(
        "PROGRAM main\nVAR RETAIN k : INT := 4; END_VAR\nVAR x : INT; END_VAR\nk := k + x;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("typedef struct {\n    IEC_INT V_K;\n} P_MAIN_retain_t;"), "{h}");
    assert!(
        h.contains("    P_MAIN_retain_t *retain;") && h.contains("    P_MAIN_retain_t V_R__P;\n} plant_retain_t;"),
        "{h}"
    );
    assert!(c.contains("ctx->V_R__P.retain = &ctx->retain.V_R__P;"), "{c}");
    assert!(c.contains("self->retain->V_K = ((IEC_INT)INT64_C(4));"), "{c}");
    let warm = &c[c.find("void plant_warm").unwrap()..];
    assert!(!warm[..warm.find('}').unwrap()].contains("init_retain"), "a warm start keeps them: {warm}");
    let init = &c[c.find("void plant_init").unwrap()..];
    assert!(init[..init.find("\n}").unwrap()].contains("P_MAIN_init_retain(ctx, &ctx->V_R__P);"), "{init}");
    assert!(
        c.contains("(offsetof(plant_t, retain) + offsetof(plant_retain_t, V_R__P) + offsetof(P_MAIN_retain_t, V_K)), 0, 0u, 8u}"),
        "the symbol, RETAIN: {c}"
    );
}

#[test]
fn retain_inside_a_function_block() {
    // Ed.3 6.5.6: the block's own RETAIN is kept, NON_RETAIN never, the
    // rest only when the instance is RETAIN; an instance of a block that
    // keeps anything is kept whole (docs/c-output.md §7).
    let text = format!(
        "FUNCTION_BLOCK inner\nVAR RETAIN n : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK outer\nVAR i : inner; plain : INT := 3; END_VAR\nVAR RETAIN j : inner; END_VAR\n\
         VAR NON_RETAIN z : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK none\nVAR a : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR f : outer; g : none; END_VAR\nVAR RETAIN h : none; END_VAR\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    let init = |b: &str| {
        let at = c.find(&format!("static void B_{b}_init(")).unwrap();
        c[at..at + c[at..].find("\n}").unwrap()].to_string()
    };
    let inner = init("INNER");
    assert!(inner.contains("    if (keep == 0) {\n        self->V_N = "), "{inner}");
    let outer = init("OUTER");
    for want in [
        "B_INNER_init(ctx, &self->V_I, keep != 0);",
        "B_INNER_init(ctx, &self->V_J, keep != 0 ? 2 : 0);",
        "    if (keep != 2) {\n        self->V_PLAIN = ",
    ] {
        assert!(outer.contains(want), "{want}: {outer}");
    }
    assert!(!outer.contains("if (keep != 2) {\n        self->V_Z"), "NON_RETAIN at every start: {outer}");
    // The instance of a block that keeps something, and the RETAIN one, in
    // the region; the plain one not.
    assert!(h.contains("typedef struct {\n    B_OUTER_t V_F;\n    B_NONE_t V_H;\n} P_MAIN_retain_t;"), "{h}");
    let main = &c[c.find("static void P_MAIN_init(").unwrap()..];
    for want in [
        "B_OUTER_init(ctx, &self->retain->V_F, warm);",
        "B_NONE_init(ctx, &self->V_G, warm);",
        "B_NONE_init(ctx, &self->retain->V_H, warm ? 2 : 0);",
    ] {
        assert!(main.contains(want), "{want}: {main}");
    }
    assert!(c.contains("void plant_warm(plant_t *ctx) {\n    plant_start(ctx, 1);"), "{c}");
}

#[test]
fn a_program_s_in_out() {
    // A pointer in the instance, bound to the global before each run; the
    // body reads and writes through it.
    let text = "PROGRAM counter\nVAR_IN_OUT n : INT; END_VAR\nn := n + 1;\nEND_PROGRAM\n\
        CONFIGURATION Plant\n  VAR_GLOBAL g : INT; END_VAR\n  RESOURCE r ON PLC\n    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n\
        PROGRAM a WITH fast : counter(n := g);\n  END_RESOURCE\nEND_CONFIGURATION\n";
    let files = generate(text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("IEC_INT *V_N;"), "{h}");
    assert!(c.contains("(*self->V_N) = t"), "{c}");
    let task = &c[c.find("void plant_task_R__FAST").unwrap()..];
    let bind = task.find("ctx->V_R__A.V_N = &(ctx->V_G);").expect(task);
    let run = task.find("P_COUNTER(ctx, &ctx->V_R__A);").expect(task);
    assert!(bind < run, "{task}");
    // A global in the process image: no pointer into it, but a copy,
    // made before the run and stored back after it.
    let located = text.replace("VAR_GLOBAL g : INT;", "VAR_GLOBAL g AT %MW0 : INT;");
    let c = &generate(&located).unwrap()[1].1;
    let task = &c[c.find("void plant_task_R__FAST").unwrap()..];
    let (copy, run) = (task.find("ctx->V_R__A.V_N = &io").expect(task), task.find("P_COUNTER(ctx").expect(task));
    let back = task.find("iecf_set_w(ctx->image.m, 0u").expect(task);
    assert!(copy < run && run < back, "{task}");
}

#[test]
fn program_connections() {
    let text = "PROGRAM counter\nVAR_INPUT inc : INT; END_VAR\nVAR_OUTPUT total : DINT; END_VAR\ntotal := total + inc;\nEND_PROGRAM\n\
        CONFIGURATION Plant\n  VAR_GLOBAL sum : LINT; END_VAR\n  RESOURCE r ON PLC\n    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n\
        PROGRAM a WITH fast : counter(inc := 2, total => sum);\n  END_RESOURCE\nEND_CONFIGURATION\n";
    let files = generate(text).unwrap();
    let c = &files[1].1;
    let task = &c[c.find("void plant_task_R__FAST").unwrap()..];
    let set = task.find("ctx->V_R__A.V_INC = ((IEC_INT)INT64_C(2));").expect(task);
    let run = task.find("P_COUNTER(ctx, &ctx->V_R__A);").expect(task);
    let take = task.find("ctx->V_SUM = ((IEC_LINT)(ctx->V_R__A.V_TOTAL));").expect(task);
    assert!(set < run && run < take, "inputs before the run, outputs after: {task}");
    assert!(c.contains("{\"R.A.INC\", ") && c.contains("offsetof(P_COUNTER_t, V_INC)), 0, 0u, 1u}"), "an input: {c}");
}

#[test]
fn strings() {
    let text = format!(
        "PROGRAM main\nVAR s : STRING[4] := 'ab'; t : STRING; n : INT; END_VAR\n\
         t := CONCAT(s, 'caf\u{e9}');\ns := t;\nn := LEN(MID(t, 2, n));\nIF s < t THEN n := 1; END_IF;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    // A STRING[4] is 5 bytes; the initial value padded with zeros.
    assert!(c.contains("iecf_sinit(self->V_S, 4u, \"ab\");"), "{c}");
    // CONCAT's buffer holds 4 + 4 characters; Latin-1 in octal.
    assert!(c.contains("char s1[9];"), "{c}");
    assert!(c.contains("iecf_concat(&ctx->flag, s1, 8u, self->V_S, \"caf\\351\")"), "{c}");
    // The store cuts to fit and is flagged; no string operation faults.
    assert!(c.contains("iecf_scopy(&ctx->flag, self->V_S, 4u, "), "{c}");
    assert!(c.contains("iecf_mid(&ctx->flag, s"), "{c}");
    assert!(c.contains("iecf_scmp(self->V_S, self->V_T) < 0"), "{c}");
    assert!(!c.contains("iecf_stopped"), "{c}");
    let h = &files[0].1;
    assert!(h.contains("char V_S[5];") && h.contains("char V_T[81];"), "{h}");

    let long = format!("PROGRAM main\nVAR s : STRING[2] := 'abc'; END_VAR\nEND_PROGRAM\n{CONFIG}");
    let errs = generate(&long).unwrap_err();
    assert_eq!(errs[0].0, "E0020", "{errs:?}");
}

#[test]
fn conversions_with_strings() {
    let text = format!(
        "PROGRAM main\nVAR s : STRING; i : INT; w : WORD; r : REAL; t : TIME; END_VAR\n\
         s := CONCAT(INT_TO_STRING(i), WORD_TO_STRING(w), REAL_TO_STRING(r), TIME_TO_STRING(t));\n\
         i := STRING_TO_INT(s);\nr := STRING_TO_REAL(s);\nt := TO_TIME(s);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    assert!(c.contains("iecf_s_str(s"), "{c}");
    assert!(c.contains("iecf_u_str(s"), "a bit string in decimal: {c}");
    assert!(c.contains("(double)(self->V_R), 7u)"), "a REAL's 7 digits: {c}");
    assert!(c.contains("iecf_time_str(s"), "{c}");
    assert!(c.contains("((IEC_INT)iecf_str_s(&ctx->flag, self->V_S, INT64_C(-32768), INT64_C(32767)))"), "{c}");
    assert!(c.contains("iecf_str_real(&ctx->flag, self->V_S, 24u)"), "rounded once, to a REAL: {c}");
    assert!(c.contains("iecf_str_time(&ctx->flag, self->V_S, 0)"), "{c}");
    assert!(!c.contains("iecf_stopped"), "no conversion of text faults: {c}");
}

#[test]
fn dates_times_of_day() {
    // docs/dependencies.md: DATE and DT in seconds since 1970, TOD in ms.
    let text = format!(
        "PROGRAM main\nVAR a : DATE := D#2026-11-07; t : TOD := TOD#12:34:56.007; s : STRING; w : USINT;\n\
         x : DT := DT#2026-11-07-12:34:56; END_VAR\n\
         w := DAY_OF_WEEK(a);\ns := TOD_TO_STRING(t);\nt := STRING_TO_TOD(s);\nx := x + T#1500ms;\n\
         t := DT_TO_TOD(x);\na := MAX(a, D#2026-01-01);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    for want in [
        "((IEC_DATE)UINT64_C(1794009600))",
        "((IEC_TOD)UINT64_C(45296007))",
        "((IEC_DT)UINT64_C(1794054896))",
        "iecf_day_of_week(self->V_A)",
        "iecf_tod_str(",
        "iecf_str_tod(&ctx->flag, ",
        "iecf_add_DT(&ctx->flag, self->V_X, ((IEC_TIME)INT64_C(1500)))",
        "iecf_dt_tod(self->V_X)",
        "iecf_max_UDINT(self->V_A, ((IEC_DATE)UINT64_C(1767225600)))",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
}

#[test]
fn long_dates() {
    // LDATE, LTOD, LDT as LINT nanoseconds: an LDT's arithmetic wrapped and
    // flagged as a LINT's, an LTOD's at midnight; TIME and DT widened
    // implicitly; a long value made a short one through iecf.h, flagged
    // where the short type may not hold it.
    let text = format!(
        "PROGRAM main\nVAR a : LDT; b : LDATE; c : LTOD := LTOD#00:00:00.000000001; t : TIME; l : LTIME; d : DT; e : DATE; y : UINT; END_VAR\n\
         a := LDT#1969-12-31-23:59:59.999999999 + t;\nc := c - LTIME#1h;\nl := t;\nt := LTIME_TO_TIME(l);\na := d;\n\
         d := LDT_TO_DT(a);\ne := LDATE_TO_DATE(b);\nb := LDT_TO_LDATE(a);\nSPLIT_LDT(IN := a, YEAR => y);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    for want in [
        "self->V_C = ((IEC_LTOD)INT64_C(1));",
        "iecf_add_LINT(&ctx->flag, ((IEC_LDT)INT64_C(-1)), iecf_time_ltime(self->V_T))",
        "iecf_sub_LTOD(self->V_C, ((IEC_LTIME)INT64_C(3600000000000)))",
        "= iecf_time_ltime(self->V_T);",
        "iecf_ltime_time(&ctx->flag, self->V_L)",
        "= iecf_dt_ldt(self->V_D);",
        "iecf_ldt_dt(&ctx->flag, self->V_A)",
        "iecf_ldt_date(&ctx->flag, self->V_B)",
        "iecf_ldt_ldate(&ctx->flag, self->V_A)",
        "iecf_split_LDT(self->V_A, &",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
}

#[test]
fn concat_and_split_dates() {
    // Ed.3 Table 36: the parts as int64_t (a ULINT through iecf_part); a
    // SPLIT's parts written before the statement, each converted to its
    // output's type, a USINT year wrapped and flagged.
    let text = format!(
        "PROGRAM main\nVAR d : DATE; t : TOD; x : DT; m : INT; u : ULINT; y : USINT; a : ARRAY[1..3] OF UDINT; END_VAR\n\
         d := CONCAT_DATE(2026, m, u);\nt := CONCAT_TOD(1, 2, 3, 4);\nx := CONCAT_DT(2026, 11, 8, 12, 0, 0, 0);\n\
         SPLIT_DATE(d, y, a[2], a[m]);\nSPLIT_TOD(IN := t, MINUTE => y);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    for want in [
        "iecf_concat_DATE(&ctx->flag, ((int64_t)(((IEC_LINT)INT64_C(2026)))), ((int64_t)(self->V_M)), iecf_part(self->V_U))",
        "iecf_concat_TOD(&ctx->flag, ",
        "iecf_concat_DT(&ctx->flag, ",
        "iecf_split_DATE(self->V_D, &",
        "iecf_split_TOD(self->V_T, &",
        "self->V_Y = ((IEC_USINT)(iecf_u_in(&ctx->flag, (uint64_t)(p5), UINT64_C(255))));",
        "IEC_UDINT *q9 = &self->V_A[iecf_index(&ctx->fault, (int64_t)(self->V_M), INT64_C(1), INT64_C(3))];",
        "*q9 = p7;",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
    // With EN and ENO the split and the stores sit inside `if (EN)`, after
    // the flag is cleared, so a narrowed part makes ENO FALSE.
    let gated = format!(
        "PROGRAM main\nVAR d : DATE; y : USINT; c, o : BOOL; END_VAR\nSPLIT_DATE(EN := c, IN := d, YEAR => y, ENO => o);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&gated).unwrap();
    let c = &files[1].1;
    let at = |s: &str| c.find(s).unwrap_or_else(|| panic!("{s}: {c}"));
    let (en, clear, split, store, eno) = (
        at("if (en"),
        at("ctx->flag.code = IECF_FAULT_NONE;"),
        at("iecf_split_DATE("),
        at("self->V_Y = ((IEC_USINT)(iecf_u_in("),
        at("self->V_O = eno"),
    );
    assert!(en < clear && clear < split && split < store && store < eno, "{c}");
    assert!(!c.contains("IEC_UDINT r"), "a SPLIT has no result: {c}");
}

#[test]
fn durations_times_numbers() {
    // Ed.3 Table 30: by an integer exact, wrapped and flagged; by a real
    // rounded half to even as REAL_TO_DINT, out of range left undone.
    let text = format!(
        "PROGRAM main\nVAR t : TIME; l : LTIME; n : INT; f : REAL; END_VAR\n\
         t := n * T#1s;\nt := DIV_TIME(t, n);\nt := MUL_TIME(t, f);\nl := LTIME#1us * n;\nl := l / f;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    for want in [
        "((IEC_TIME)iecf_fit_DINT(&ctx->flag, iecf_mul_LINT(&ctx->flag, (int64_t)(((IEC_TIME)INT64_C(1000))), ((IEC_LINT)(self->V_N)))))",
        "iecf_fit_DINT(&ctx->flag, iecf_div_LINT(&ctx->fault, &ctx->flag, (int64_t)(self->V_T), ((IEC_LINT)(self->V_N))))",
        "iecf_round_s(&ctx->flag, &",
        "((double)(self->V_T) * (((IEC_LREAL)(self->V_F))))",
        "iecf_mul_LINT(&ctx->flag, (int64_t)(((IEC_LTIME)INT64_C(1000))), ((IEC_LINT)(self->V_N)))",
        "iecf_div_LREAL(&ctx->fault, &ctx->flag, (double)(self->V_L), ((IEC_LREAL)(self->V_F)))",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
}

#[test]
fn en_eno_not_yet() {
    // ENO of a block taken into anything but a BOOL variable: not yet.
    let into = format!(
        "PROGRAM main\nVAR t : TON; c : BOOL; a : ARRAY[1..2] OF BOOL; END_VAR\n\
         t(EN := c, IN := c, PT := T#1s, ENO => a[1]);\nEND_PROGRAM\n{CONFIG}"
    );
    assert_eq!(generate(&into).unwrap_err()[0].0, "E0009");
}

#[test]
fn en_eno_on_a_block_call() {
    // Ed.3 6.6.1.5, external (docs/dependencies.md): EN evaluated first;
    // inputs, call and outputs inside `if (EN)`; ENO from the flag the
    // call alone raised, stored whatever EN was.
    let text = format!(
        "PROGRAM main\nVAR t : TON; c, q, ok : BOOL; END_VAR\n\
         t(EN := c, IN := TRUE, PT := T#1s, Q => q, NOT ENO => ok);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    let at = |s: &str| c.find(s).unwrap_or_else(|| panic!("{s}: {c}"));
    let (eno, gate, set, clear, call, took, back, store) = (
        at("IEC_BOOL eno"),
        at("if (self->V_C) {"),
        at("self->V_T.IN = "),
        at("ctx->flag.code = IECF_FAULT_NONE;"),
        at("iecf_TON(&self->V_T, ctx->now_ns);"),
        at("(IEC_BOOL)(ctx->flag.code == IECF_FAULT_NONE);"),
        at("self->V_Q = self->V_T.Q;"),
        at("self->V_OK = ((IEC_BOOL)!eno"),
    );
    assert!(
        eno < gate && gate < set && set < clear && clear < call && call < took && took < back && back < store,
        "{c}"
    );
    // The store of ENO is after the `if`: one closing brace between.
    assert!(c[back..store].contains('}'), "{c}");
}

#[test]
fn eno_set_in_a_body() {
    // A block that sets ENO: a field, 1 as each run starts, and the
    // caller's ENO is it and no flag. A function: an `eno` pointer after
    // ctx, a local when the call takes ENO, a compound literal when not.
    let text = format!(
        "FUNCTION_BLOCK lim\nVAR_INPUT x : INT; END_VAR\nIF x > 100 THEN ENO := FALSE; END_IF;\nEND_FUNCTION_BLOCK\n\
         FUNCTION half : INT\nVAR_INPUT x : INT; END_VAR\nhalf := x / 2;\nENO := x >= 0;\nEND_FUNCTION\n\
         PROGRAM main\nVAR l : lim; ok : BOOL; n : INT; END_VAR\n\
         l(x := n, ENO => ok);\nn := half(x := n, ENO => ok);\nn := half(n);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("IEC_BOOL eno;"), "{h}");
    for want in [
        "    self->eno = 1;",
        "static IEC_INT F_HALF(plant_t *ctx, IEC_BOOL *eno, IEC_INT V_X)",
        "    *eno = 1;",
        "(IEC_BOOL)(ctx->flag.code == IECF_FAULT_NONE && self->V_L.eno);",
        "F_HALF(ctx, &e",
        "(IEC_BOOL)(ctx->flag.code == IECF_FAULT_NONE && e",
        "F_HALF(ctx, &(IEC_BOOL){1}, self->V_N)",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
}

#[test]
fn block_data_parameters() {
    // Arrays, structures and STRINGs in and out of a FUNCTION_BLOCK.
    let text = format!(
        "FUNCTION_BLOCK fb\nVAR_INPUT a : ARRAY[0..1] OF INT; s : STRING[4]; END_VAR\n\
         VAR_OUTPUT b : ARRAY[0..1] OF INT; END_VAR\nb := a;\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR f : fb; x, y : ARRAY[0..1] OF INT; i : INT; END_VAR\n\
         f(a := x, s := 'abcdef', b => y);\ni := f.b[1];\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    for want in [
        "iecf_bytes(&(self->V_F.V_A), ",
        "iecf_scopy(&ctx->flag, self->V_F.V_S, 4u, ",
        "iecf_bytes(&(self->V_Y), &(self->V_F.V_B), sizeof (self->V_Y));",
        "self->V_F.V_B[1u]",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
}

#[test]
fn sfc_in_a_function_block() {
    // The chart lives in the instance; one kept whole keeps its steps
    // (Ed.3 6.7.2): its initialisation is guarded by keep != 2.
    let text = format!(
        "FUNCTION_BLOCK seq\nVAR_INPUT go : BOOL; END_VAR\nVAR_OUTPUT lit : BOOL; END_VAR\n\
         INITIAL_STEP a: END_STEP\nSTEP b: run(N); END_STEP\n\
         TRANSITION FROM a TO b := go; END_TRANSITION\nTRANSITION FROM b TO a := NOT go; END_TRANSITION\n\
         ACTION run: lit := run.Q; END_ACTION\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR s : seq; END_VAR\nVAR RETAIN k : seq; END_VAR\ns(go := TRUE);\nk(go := TRUE);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("    iecf_step_t S_A;\n    iecf_step_t S_B;\n    iecf_action_t A_RUN;"), "{h}");
    for want in [
        "    if (keep != 2) {\n        iecf_step_init(&self->S_A, 1);",
        "iecf_action(&ctx->flag, &self->A_RUN, q, t, ctx->now_ns);",
        "if (self->A_RUN.A && self->A_RUN.Q == pass",
        "if (c",
        "iecf_step_enter(&ctx->flag, &self->S_B, ctx->now_ns);",
        "B_SEQ_init(ctx, &self->retain->V_K, warm ? 2 : 0);",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
    // Statements beside a chart: not yet.
    let both = format!("PROGRAM main\nVAR x : INT; END_VAR\nx := 1;\nINITIAL_STEP a: END_STEP\nEND_PROGRAM\n{CONFIG}");
    assert_eq!(generate(&both).map(|_| ()).unwrap_err()[0].0, "E0009");
}

#[test]
fn the_symbol_table_as_json() {
    // docs/c-output.md §6: the C tables' symbols and types, as JSON.
    let text = format!(
        "TYPE pt : STRUCT x : INT; END_STRUCT; END_TYPE\n\
         PROGRAM main\nVAR a : ARRAY[1..2] OF pt; t : TON; s : STRING[4]; END_VAR\n\
         VAR RETAIN k : DINT; END_VAR\nVAR o AT %QX1.2 : BOOL; END_VAR\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (name, j) = &files[3];
    assert_eq!(name, "plant.symbols.json");
    for want in [
        "\"configuration\": \"plant\"",
        "{\"path\": \"R.P.A\", \"type\": 0, \"flags\": []}",
        "{\"path\": \"R.P.K\", \"type\": ",
        "\"flags\": [\"RETAIN\"]",
        "{\"path\": \"R.P.O\", \"type\": ",
        "\"area\": \"Q\", \"byte\": 1, \"bit\": 2}",
        "{\"name\": \"PT\", \"kind\": \"struct\", \"members\": [{\"name\": \"X\", \"type\": 2, \"flags\": []}]}",
        "{\"name\": \"ARRAY [1..2] OF PT\", \"kind\": \"array\", \"of\": 1, \"dims\": [[1, 2]]}",
        "\"kind\": \"block\", \"members\": [{\"name\": \"IN\"",
        "{\"name\": \"STRING(4)\", \"kind\": \"string\", \"length\": 4}",
    ] {
        assert!(j.contains(want), "{want}: {j}");
    }
}

#[test]
fn time_and_dates_at_an_address() {
    // Ed.3 6.5.5.3: the address is where it starts, the type's own width
    // from there (W0004 when the prefix differs); a BOOL anywhere but a
    // bit the whole byte.
    let text = format!(
        "PROGRAM main\nVAR t AT %MD0 : TIME; l AT %ML4 : LTIME; d AT %MD12 : DATE; ld AT %ML16 : LDT;\n\
         w AT %MW24 : TIME; r AT %IB12 : REAL; x AT %QB3 : BOOL; y AT %QX0.1 : BOOL; END_VAR\n\
         t := w; l := ld - ld; d := d; ld := ld; x := r > 1.0; y := x;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c, j) = (&files[0].1, &files[1].1, &files[3].1);
    for want in [
        "iecf_set_d(ctx->image.m, 0u, (uint32_t)(",
        "iecf_to_i32(iecf_get_d(ctx->image.m, 24u))",
        "iecf_set_l(ctx->image.m, 4u, (uint64_t)(",
        "iecf_to_i64(iecf_get_l(ctx->image.m, 16u))",
        "iecf_get_d(ctx->image.m, 12u)",
        "iecf_get_real(ctx->image.i, 12u)",
        "iecf_set_b(ctx->image.q, 3u, (uint8_t)((",
        "((IEC_BOOL)(iecf_get_b(ctx->image.q, 3u) != 0u))",
        "iecf_set_x(ctx->image.q, 0u, 1u, ",
        "3u, 'Q', IECF_NO_BIT, 0u}",
        "0u, 'Q', 1u, 0u}",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
    // The extent each area needs: %ML16 is bytes 16 to 23, %MW24 a TIME
    // to 27; %IB12 a REAL to 15.
    assert!(h.contains("#define PLANT_M_BYTES 28u"), "{h}");
    assert!(h.contains("#define PLANT_I_BYTES 16u"), "{h}");
    assert!(j.contains("\"area\": \"Q\", \"byte\": 3}"), "no bit for a whole byte: {j}");
    assert!(j.contains("\"area\": \"Q\", \"byte\": 0, \"bit\": 1}"), "{j}");
}

#[test]
fn time_and_date_connections_convert() {
    // A program's TIME output to an LTIME global, a DT one to an LDT: the
    // units differ, so the conversion, not a cast.
    let text = "PROGRAM clock\nVAR_OUTPUT el : TIME; stamp : DT; END_VAR\nel := T#1500ms; stamp := DT#2026-11-08-12:00:00;\nEND_PROGRAM\n\
        CONFIGURATION Plant\n  VAR_GLOBAL g : LTIME; s AT %ML8 : LDT; END_VAR\n  RESOURCE r ON PLC\n    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n\
        PROGRAM a WITH fast : clock(el => g, stamp => s);\n  END_RESOURCE\nEND_CONFIGURATION\n";
    let c = &generate(text).unwrap()[1].1;
    assert!(c.contains("ctx->V_G = iecf_time_ltime(ctx->V_R__A.V_EL);"), "{c}");
    assert!(c.contains("iecf_set_l(ctx->image.m, 8u, (uint64_t)(iecf_dt_ldt(ctx->V_R__A.V_STAMP)));"), "{c}");
}

#[test]
fn arrays_and_structures_at_an_address() {
    // Their parts back to back from the address, each its own width, a
    // BOOL a byte (docs/dependencies.md): `pt` is 1 + 2 + 4 = 7 bytes.
    let text = format!(
        "TYPE pt : STRUCT ok : BOOL; x : INT; t : TIME; END_STRUCT; END_TYPE\n\
         PROGRAM main\nVAR a AT %QW6 : ARRAY[0..2] OF INT := [1, 2, 3]; p AT %MB0 : pt;\n\
         ps AT %MB20 : ARRAY[1..2] OF pt; s AT %MB40 : STRING[4]; inp AT %IW0 : ARRAY[0..1] OF INT := [4, 5]; END_VAR\n\
         VAR c : ARRAY[0..2] OF INT; i : INT; END_VAR\n\
         a[i] := 1; p.t := T#1s; ps[2] := p; c := a; a := c; s := 'ab';\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c, j) = (&files[0].1, &files[1].1, &files[3].1);
    for want in [
        // The initial value built in a C array, then copied in.
        "init1[2] = ((IEC_INT)INT64_C(3));",
        "iecf_set_w(ctx->image.q, (6u + (uint32_t)(k2) * 2u), (uint16_t)(init1[k2]));",
        // An element by a computed index, checked first.
        "iecf_set_w(ctx->image.q, (6u + (uint32_t)(o",
        // p.t at byte 3; ps[2] at 20 + 7, the whole of p, image to image.
        "iecf_set_d(ctx->image.m, 3u, (uint32_t)(",
        "iecf_move(ctx->image.m + 27u, ctx->image.m + 0u, 7u);",
        // Between a C array and the image, element by element.
        "self->V_C[k",
        "] = iecf_to_i16(iecf_get_w(ctx->image.q, (6u + (uint32_t)(k",
        // A STRING there as its n + 1 bytes.
        "iecf_scopy(&ctx->flag, (*(char (*)[5])(ctx->image.m + 40u)), 4u, ",
    ] {
        assert!(c.contains(want), "{want}: {c}");
    }
    // An input's initial value is the platform's.
    assert!(!c.contains("INT64_C(4)"), "{c}");
    assert!(h.contains("#define PLANT_M_BYTES 45u"), "STRING[4] at 40 is bytes 40 to 44: {h}");
    assert!(h.contains("#define PLANT_Q_BYTES 12u"), "{h}");
    assert!(
        j.contains("{\"path\": \"R.P.PS\", \"type\": ")
            && j.contains("\"flags\": [\"PACKED\"], \"area\": \"M\", \"byte\": 20}"),
        "{j}"
    );
    assert!(c.contains("'M', IECF_NO_BIT, 16u}"), "IECF_SYM_PACKED: {c}");
}

#[test]
fn what_is_not_at_an_address_yet() {
    let ty = "TYPE pt : STRUCT x : INT; END_STRUCT; END_TYPE\n";
    let retain = format!(
        "{ty}PROGRAM main\nEND_PROGRAM\nCONFIGURATION Plant\n  VAR_GLOBAL RETAIN g AT %MB0 : pt; END_VAR\n  RESOURCE r ON PLC\n    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n    PROGRAM p WITH fast : main;\n  END_RESOURCE\nEND_CONFIGURATION\n"
    );
    assert_eq!(
        generate(&retain).unwrap_err()[0],
        ("E0009", "a RETAIN variable at an address is not compiled to C yet".to_string())
    );
    // A WSTRING at an address: byte by byte, little-endian, at any byte.
    let wide = format!(
        "PROGRAM main\nVAR w AT %MW1 : WSTRING[4] := \"a\"; END_VAR\nVAR c : WCHAR; END_VAR\nw[1] := c;\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&wide).unwrap();
    let c = &files[1].1;
    assert!(c.contains("iecf_wimg_put((ctx->image.m + 1u), 4u, init"), "{c}");
    assert!(
        c.contains("{ IEC_WCHAR wimg[5]; iecf_wimg_get(wimg, 4u, (ctx->image.m + 1u)); iecf_wsput(&ctx->flag, wimg,"),
        "{c}"
    );
    // A reference to one there, or in C: the run knows which, so read
    // through iecf_wref_get, a store written back by iecf_wref_put.
    let wref = format!(
        "PROGRAM main\nVAR w AT %MW1 : WSTRING[4]; END_VAR\nVAR r : REF_TO WSTRING[4]; c : WCHAR; END_VAR\nr := REF(w);\nr^[1] := c;\nc := r^[2];\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&wref).unwrap()[1].1;
    assert!(c.contains("iecf_ref_image((uint8_t)'M', 1u, IECF_NO_BIT)"), "{c}");
    assert!(c.contains("IEC_WCHAR *wp = iecf_wref_get(wimg, 4u, &ctx->image, wr); iecf_wsput("), "{c}");
    assert!(c.contains("iecf_wref_put(&ctx->image, wr, 4u, wp);"), "{c}");
}

#[test]
fn references_in_c_and_into_the_image() {
    let text = format!(
        "TYPE node : STRUCT v : INT; next : REF_TO node; END_STRUCT; END_TYPE\n\
         PROGRAM main\n\
         VAR w AT %MW4 : INT; b AT %MX6.2 : BOOL; END_VAR\n\
         VAR a : INT; r : REF_TO INT := REF(a); rb : REF_TO BOOL; n : node; t : TON; rt : REF_TO TON; END_VAR\n\
         r := REF(w);\n\
         r^ := r^ + 1;\n\
         rb := REF(b);\n\
         n.next := REF(n);\n\
         rt := REF(t);\n\
         rt^(IN := TRUE, PT := T#1s);\n\
         END_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c, json) = (&files[0].1, &files[1].1, &files[3].1);
    // An initial value REF(a), filled in once the variables are known.
    assert!(c.contains("self->V_R = iecf_ref_c(&(self->V_A));") && !c.contains("/*@REF"), "{c}");
    assert!(c.contains("iecf_ref_image((uint8_t)'M', 4u, IECF_NO_BIT)"), "{c}");
    assert!(c.contains("iecf_ref_image((uint8_t)'M', 6u, 2u)"), "a BOOL at a bit: {c}");
    // `r^ :=` checks the reference first: NULL stops before the store.
    assert!(
        c.contains("iecf_ref_t r2 = iecf_deref(&ctx->fault, self->V_R, &ctx->nowhere);\n        if (iecf_stopped("),
        "{c}"
    );
    assert!(c.contains("iecf_rset_INT(&ctx->image, r2, "), "{c}");
    // A call through a reference: checked, then the block on what it references.
    assert!(c.contains("iecf_TON(&(*(iecf_TON_t *)r"), "{c}");
    assert!(h.contains("} nowhere;") && h.contains("iecf_TON_t t") && h.contains("IEC_INT t"), "{h}");
    assert!(json.contains("\"kind\": \"reference\""), "{json}");
}

#[test]
fn a_reference_takes_only_its_own_subrange() {
    let text = format!(
        "TYPE small : INT(0..9); END_TYPE\n\
         PROGRAM main\n\
         VAR s : small; r : REF_TO INT; END_VAR\n\
         r := REF(s);\n\
         END_PROGRAM\n{CONFIG}"
    );
    let errs = generate(&text).unwrap_err();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(errs[0].0, "E0013");
    assert!(errs[0].1.starts_with("this references a subrange 0..9, the reference no subrange"), "{}", errs[0].1);
}

#[test]
fn reference_to_is_written_as_the_ref_to_it_stands_for() {
    let text = format!(
        "FUNCTION_BLOCK addto\n\
         VAR_INPUT r : REFERENCE TO INT; x : INT; END_VAR\n\
         IF __ISVALIDREF(r) THEN r := r + x; END_IF;\n\
         END_FUNCTION_BLOCK\n\
         PROGRAM main\n\
         VAR a, b : INT; ra, rb : REFERENCE TO INT; f : addto; END_VAR\n\
         ra REF= a;\n\
         rb REF= ra;\n\
         ra := rb + 1;\n\
         f(r := b, x := 2);\n\
         ra REF= 0;\n\
         END_PROGRAM\n{CONFIG}"
    );
    let on = iecf_sema::Options::default().with(iecf_sema::Extension::ReferenceTo);
    let files = generate_with(&text, &on).unwrap();
    let (c, json) = (&files[1].1, &files[3].1);
    // REF= of a variable is REF(); of another REFERENCE TO, a copy; of 0, NULL.
    assert!(c.contains("= iecf_ref_c(&(self->V_A));") && c.contains("self->V_RA = t"), "{c}");
    assert!(c.contains("iecf_ref_t t") && c.contains("= self->V_RA;"), "{c}");
    assert!(c.contains("= iecf_ref_null();"), "{c}");
    // A use is a dereference, checked: unbound stops the statement.
    assert!(c.contains("= iecf_deref(&ctx->fault, self->V_RA, &ctx->nowhere);"), "{c}");
    assert!(c.contains("iecf_rget_INT(&ctx->image, iecf_deref(&ctx->fault, self->V_RB, &ctx->nowhere))"), "{c}");
    // A call binds the block's REFERENCE TO input.
    assert!(c.contains("self->V_F.V_R = t"), "{c}");
    assert!(c.contains("((IEC_BOOL)!iecf_ref_is_null(self->V_R))"), "{c}");
    assert!(json.contains("\"REFERENCE TO INT\""), "{json}");
}

#[test]
fn a_function_s_in_out_its_copies_and_its_order() {
    let head = "TYPE small : INT(0..9); END_TYPE\n\
                FUNCTION bump : INT\n\
                VAR_IN_OUT v : INT; END_VAR\n\
                v := v + 1; bump := v;\n\
                END_FUNCTION\n";
    let prog =
        |decl: &str, body: &str| format!("{head}PROGRAM main\nVAR {decl} END_VAR\n{body}\nEND_PROGRAM\n{CONFIG}");
    // At an address, or of a subrange: a copy, stored back after the call
    // - clamped into the subrange.
    let c = |text: String| generate(&text).unwrap()[1].1.clone();
    let located = c(prog("w AT %MW0 : INT; END_VAR VAR k : INT;", "k := bump(w);"));
    assert!(
        located.contains("F_BUMP(ctx, &q") && located.contains("iecf_set_w(ctx->image.m, 0u, (uint16_t)(q"),
        "{located}"
    );
    let sub = c(prog("s : small; k : INT;", "k := bump(s);"));
    assert!(sub.contains("iecf_clamp_s(&ctx->flag, (int64_t)(q"), "{sub}");
    // With EN, the call - with effects - is inside `if (EN)`, so EN FALSE
    // leaves the in-out as it was (enable.rs; harness case fneffen).
    let gated = generate(&prog("x : INT; k : INT; b : BOOL;", "k := bump(EN := b, v := x);")).unwrap();
    let c = &gated[1].1;
    let at = c.find("if (en").expect("an if (EN)");
    assert!(c[at..].contains("= F_BUMP(ctx, &(self->V_X));"), "{c}");
    // A pure call stays inline; one with effects is a line of its own,
    // after what is left of it is read.
    let c = generate(&prog("x : INT; k : INT;", "k := x + bump(x);")).unwrap()[1].1.clone();
    assert!(c.contains("IEC_INT s1 = self->V_X;\n        IEC_INT r2 = F_BUMP(ctx, &(self->V_X));"), "{c}");
}

#[test]
fn a_method_is_a_function_given_its_instance() {
    // OO.md step A: `M_<TYPE>__<METHOD>(ctx, self, ...)`, its type's
    // variables through `self`, THIS.x as x; a CLASS has no body
    // function; a method can fault, so its call is checked.
    let text = format!(
        "CLASS K\nVAR n : INT; END_VAR\nVAR PUBLIC d : INT := 1; END_VAR\n\
         METHOD PUBLIC Div : INT\nVAR_INPUT by_ : INT; END_VAR\nTHIS.n := n + 1;\nDiv := d / by_;\nEND_METHOD\n\
         END_CLASS\n\
         PROGRAM main\nVAR k : K; x : INT; END_VAR\nk.d := 6;\nx := k.Div(by_ := x);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert!(c.contains("static IEC_INT M_K__DIV(plant_t *ctx, B_K_t *self, IEC_INT V_BY_);"), "{c}");
    assert!(c.contains("/* METHOD K.Div */") && c.contains("/* CLASS K */"), "{c}");
    assert!(!c.contains("static void B_K(plant_t"), "a class has no body: {c}");
    assert!(c.contains("iecf_add_INT(&ctx->flag, self->V_N, ((IEC_INT)INT64_C(1)))"), "{c}");
    assert!(c.contains("self->V_K.V_D = "), "{c}");
    assert!(c.contains("IEC_INT r") && c.contains(" = M_K__DIV(ctx, &(self->V_K), "), "{c}");
    assert!(c.contains("if (iecf_stopped(&ctx->fault, 0u, 13u)) return;"), "{c}");
}

#[test]
fn a_derived_type_starts_with_its_base_and_dispatches_through_its_table() {
    // OO.md step B: the base first in the structure, the root's type
    // pointer set by each `_init`; THIS.m() of an overridden method read
    // from the table, through a thunk for the override; a call on a named
    // instance direct, its inherited method given the base part.
    let text = format!(
        "CLASS A\nVAR v : INT; END_VAR\n\
         METHOD PUBLIC f : INT\nf := 1;\nEND_METHOD\n\
         METHOD PUBLIC g : INT\ng := THIS.f() + v;\nEND_METHOD\nEND_CLASS\n\
         CLASS B EXTENDS A\nMETHOD PUBLIC OVERRIDE f : INT\nf := 2 + v;\nEND_METHOD\nEND_CLASS\n\
         PROGRAM main\nVAR b : B; x : INT; END_VAR\nx := b.g();\nEND_PROGRAM\n{CONFIG}"
    );
    let out = generate(&text).unwrap();
    let (h, c) = (&out[0].1, &out[1].1);
    assert!(h.contains("struct B_A_s {\n    const iecf_otype_t *type;\n    IEC_INT V_V;\n};"), "{h}");
    assert!(h.contains("struct B_B_s {\n    B_A_t base;\n};"), "{h}");
    assert!(
        c.contains("    IEC_INT (*m_F)(plant_t *ctx, B_A_t *self);\n    IEC_INT (*m_G)(plant_t *ctx, B_A_t *self);"),
        "{c}"
    );
    assert!(c.contains("    .base.m_F = M_B__F__vt,\n    .base.m_G = M_A__G,"), "{c}");
    assert!(c.contains("static const iecf_otype_t TY_B = {&TY_A, \"B\", &VT_B, NULL, 0u};"), "{c}");
    assert!(c.contains("    B_A_init(ctx, &self->base, keep);\n    self->base.type = &TY_B;"), "{c}");
    assert!(c.contains("((const VT_A_t *)((*self)).type->table)->m_F(ctx, &((*self)))"), "{c}");
    assert!(c.contains("return M_B__F(ctx, (B_B_t *)self);"), "{c}");
    assert!(c.contains("self->base.V_V"), "the inherited variable: {c}");
    assert!(c.contains("M_A__G(ctx, &(self->V_B).base)"), "{c}");
}

#[test]
fn an_interface_is_an_instance_and_its_type_s_table() {
    // OO.md step C: a type that implements an interface carries its type,
    // which lists its table for it; a named instance takes that table, THIS
    // finds it in the type; a call through the interface is checked for
    // NULL (D8) and reads the slot, a thunk to the method.
    let text = format!(
        "INTERFACE I\nMETHOD m : INT\nVAR_INPUT x : INT; END_VAR\nEND_METHOD\nEND_INTERFACE\n\
         INTERFACE J EXTENDS I\nEND_INTERFACE\n\
         CLASS K IMPLEMENTS J\nMETHOD PUBLIC m : INT\nVAR_INPUT x : INT; END_VAR\nm := x;\nEND_METHOD\n\
         METHOD PUBLIC me\nVAR_IN_OUT out : HOLD; END_VAR\nout(it := THIS);\nEND_METHOD\nEND_CLASS\n\
         FUNCTION_BLOCK HOLD\nVAR_INPUT it : I; END_VAR\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR k : K; i : I; j : J; n : INT; END_VAR\ni := k;\nj ?= i;\nn := i.m(3);\nEND_PROGRAM\n{CONFIG}"
    );
    let out = generate(&text).unwrap();
    let (h, c) = (&out[0].1, &out[1].1);
    assert!(h.contains("struct B_K_s {\n    const iecf_otype_t *type;\n};"), "{h}");
    assert!(h.contains("    iecf_itf_t V_IT;"), "{h}");
    assert!(
        c.contains("typedef struct {\n    IEC_INT (*m_M)(plant_t *ctx, void *self, IEC_INT V_X);\n} IT_I_t;"),
        "{c}"
    );
    assert!(c.contains("static const iecf_itype_t ITY_I = {\"I\"};"), "{c}");
    assert!(c.contains("static const iecf_itfimpl_t ITS_K[] = {{&ITY_I, &I_I__K}, {&ITY_J, &I_J__K}};"), "{c}");
    assert!(c.contains("static const iecf_otype_t TY_K = {NULL, \"K\", &VT_K, ITS_K, 2u};"), "{c}");
    assert!(c.contains("return M_K__M(ctx, (B_K_t *)self, V_X);"), "the thunk: {c}");
    assert!(c.contains("iecf_itf_make(&(self->V_K), &I_I__K)"), "a named instance: {c}");
    assert!(c.contains("iecf_itf_of(&((*self)), &ITY_I)"), "THIS: {c}");
    assert!(c.contains("self->V_J = iecf_itf_as(t"), "?=: {c}");
    assert!(c.contains("if (i") && c.contains(".self == NULL) iecf_fail(&ctx->fault, IECF_FAULT_NULL);"), "{c}");
    assert!(c.contains(".table)->m_M(ctx, i") && c.contains(".self, ((IEC_INT)INT64_C(3)))"), "{c}");
}

#[test]
fn instances_in_arrays_structures_and_globals() {
    // inst.rs: each element set up and given its initial value, a
    // structure holding an instance written after the instance's type, an
    // element's address checked before the call, a global instance set
    // at every start.
    let text = "FUNCTION_BLOCK F\nVAR_INPUT i : INT := 1; END_VAR\nVAR_OUTPUT o : INT; END_VAR\no := o + i;\nEND_FUNCTION_BLOCK\n\
                TYPE S : STRUCT f : F; t : TOF := (PT := T#1s); END_STRUCT; END_TYPE\n\
                PROGRAM main\nVAR fs : ARRAY[1..3] OF F := [(i := 5)]; s : S; n : INT; END_VAR\nVAR_EXTERNAL g : F; END_VAR\n\
                fs[n]();\ns.f();\ng();\nEND_PROGRAM\n\
                CONFIGURATION Plant\nVAR_GLOBAL g : F := (i := 3); END_VAR\n  RESOURCE r ON PLC\n    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);\n    PROGRAM p WITH fast : main;\n  END_RESOURCE\nEND_CONFIGURATION\n";
    let files = generate(text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("    B_F_t V_FS[3];"), "{h}");
    let (bf, ts) = (h.find("struct B_F_s {").unwrap(), h.find("} T_S;").unwrap());
    assert!(bf < ts, "T_S after B_F_t: {h}");
    assert!(c.contains("B_F_init(ctx, &self->V_FS[0], warm);\n    self->V_FS[0].V_I = ((IEC_INT)INT64_C(5));"), "{c}");
    assert!(c.contains("iecf_TOF_init(&self->V_S.V_T);\n    self->V_S.V_T.PT = ((IEC_TIME)INT64_C(1000));"), "{c}");
    let call = c.find("B_F_t *o").unwrap();
    let rest = &c[call..];
    let (stop, run) = (rest.find("iecf_stopped").unwrap(), rest.find("B_F(ctx, &(*o").unwrap());
    assert!(stop < run, "the index checked before the call: {rest}");
    assert!(c.contains("B_F_init(ctx, &ctx->V_G, warm);\n    ctx->V_G.V_I = ((IEC_INT)INT64_C(3));"), "{c}");
    // What holds an instance is copied whole, as an instance is.
    let copy = text.replace("g();\n", "g();\nfs := fs;\n");
    let c = &generate(&copy).unwrap()[1].1;
    assert!(c.contains("iecf_bytes(&(self->V_FS), &(self->V_FS), sizeof (self->V_FS));"), "{c}");
}

#[test]
fn string_array_and_structure_parameters_and_results() {
    // params.rs: each passed by a pointer, its result into the caller's
    // buffer; an input not given NULL, a STRING one cut and flagged where
    // it is declared; an output stored after the fault check.
    let text = format!(
        "TYPE PT : STRUCT x : INT; END_STRUCT; END_TYPE\n\
         FUNCTION L : STRING[6]\nVAR_INPUT name : STRING[3] := 'n'; END_VAR\nL := name;\nEND_FUNCTION\n\
         FUNCTION M : PT\nVAR_INPUT xs : ARRAY[1..2] OF INT; END_VAR\nVAR_OUTPUT o : PT; END_VAR\nM.x := xs[1];\nEND_FUNCTION\n\
         PROGRAM main\nVAR s : STRING[6]; xs : ARRAY[1..2] OF INT; p, q : PT; END_VAR\n\
         s := L();\np := M(xs := xs, o => q);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    assert!(c.contains("static const char * F_L(plant_t *ctx, void *R_, const void *P_NAME)"), "{c}");
    assert!(c.contains("iecf_sinit((*(char (*)[7])R_), 6u, \"\");"), "the result its initial value first: {c}");
    let cut =
        c.find("iecf_scopy(&ctx->flag, V_NAME, 3u, (const char *)P_NAME);\n        iecf_noted(&ctx->flag, ").unwrap();
    assert!(c[cut..].lines().nth(1).is_some_and(|l| l.ends_with(", 3u);")), "flagged at line 3: {c}");
    assert!(c.contains("iecf_sinit(V_NAME, 3u, \"n\");"), "not given: its initial value: {c}");
    assert!(c.contains("return (const char *)R_;"), "{c}");
    assert!(c.contains("F_L(ctx, &s"), "{c}");
    assert!(c.contains(", NULL)"), "an input not given: {c}");
    assert!(c.contains("static void F_M(plant_t *ctx, void *R_, const void *P_XS, void *O_O)"), "{c}");
    assert!(c.contains("if (O_O) iecf_bytes(O_O, &V_O, sizeof V_O);"), "{c}");
    let call = c.find("F_M(ctx, &d").unwrap();
    let rest = &c[call..];
    let (stop, store) = (rest.find("iecf_stopped").unwrap(), rest.find("iecf_bytes(&(self->V_Q)").unwrap());
    assert!(stop < store, "the output stored after the check: {rest}");
}

#[test]
fn a_variable_length_array_is_an_iecf_flex_t() {
    let f =
        "FUNCTION F : DINT\nVAR_IN_OUT a : ARRAY [*] OF INT; END_VAR\nF := UPPER_BOUND(a, 1) + a[2];\nEND_FUNCTION\n";
    let text = format!(
        "{f}PROGRAM main\nVAR CONSTANT N : INT := 4; END_VAR\nVAR x : ARRAY [0..N] OF INT; r : DINT; END_VAR\nr := F(x);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    assert!(
        c.contains("F_F(ctx, &(iecf_flex_t){(void *)&(self->V_X), (const int64_t[]){INT64_C(0), INT64_C(4)}})"),
        "the array and its bounds, N's value: {c}"
    );
    assert!(c.contains("((IEC_DINT)((iecf_flex_t *)V_A)->b[1])"), "UPPER_BOUND(a, 1): {c}");
    assert!(
        c.contains("iecf_index(&ctx->fault, INT64_C(2), ((iecf_flex_t *)V_A)->b[0], ((iecf_flex_t *)V_A)->b[1])"),
        "a constant index checked against the caller's bounds: {c}"
    );
    // A whole one assigned: its size checked as it runs, then copied.
    let text = format!(
        "FUNCTION F : DINT\nVAR_IN_OUT a : ARRAY [*] OF INT; END_VAR VAR b : ARRAY [1..2] OF INT; END_VAR\na := b; F := 0;\nEND_FUNCTION\nPROGRAM main\nVAR x : ARRAY [1..2] OF INT; r : DINT; END_VAR\nr := F(x);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert!(
        c.contains("if (iecf_flex_n(((iecf_flex_t *)V_A), 0u) != 2u) iecf_fail(&ctx->fault, IECF_FAULT_INDEX);"),
        "{c}"
    );
}

#[test]
fn an_array_star_input_and_output_are_the_callers_array() {
    // By reference, as Siemens passes an FC's structured parameters
    // (docs/dependencies.md): no copy in, none back.
    let f = "FUNCTION F : DINT\nVAR_INPUT a : ARRAY [*] OF INT; END_VAR\nVAR_OUTPUT o : ARRAY [*] OF INT; END_VAR\no[1] := a[1];\nEND_FUNCTION\n";
    let text = format!(
        "{f}PROGRAM main\nVAR x : ARRAY [1..2] OF INT; y : ARRAY [0..4] OF INT; r : DINT; END_VAR\nr := F(a := x, o => y);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let c = &files[1].1;
    assert!(c.contains("F_F(plant_t *ctx, const void *P_A, void *O_O)"), "{c}");
    assert!(
        c.contains("&(iecf_flex_t){(void *)&(self->V_X), (const int64_t[]){INT64_C(1), INT64_C(2)}}, &(iecf_flex_t){(void *)&(self->V_Y), (const int64_t[]){INT64_C(0), INT64_C(4)}})"),
        "both given as they are: {c}"
    );
    assert!(c.contains("((const iecf_flex_t *)P_A)->p"), "the input read where it is: {c}");
    assert!(
        c.contains("< iecf_flex_n(((iecf_flex_t *)O_O), 0u);"),
        "the output's elements set as the call starts: {c}"
    );
    assert!(!c.contains("iecf_bytes("), "nothing copied: {c}");
}

#[test]
fn a_block_never_called_is_named_once() {
    // docs/c-output.md §1: clean with every warning on, no attribute.
    let text = format!(
        "FUNCTION_BLOCK SPARE\nVAR_INPUT a : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
         FUNCTION_BLOCK USED\nVAR_INPUT a : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR s : SPARE; u : USED; END_VAR\nu();\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert!(c.contains("int warm) {\n    (void)B_SPARE;\n"), "{c}");
    assert!(!c.contains("(void)B_USED;") && !c.contains("(void)B_SPARE_init;"), "{c}");
}

#[test]
fn a_function_of_literals_alone_is_computed_in_the_type_wanted() {
    // docs/dependencies.md, Untyped literals.
    let text = format!(
        "PROGRAM main\nVAR v : INT; d : DINT; r : REAL; b : BOOL; END_VAR\n\
         v := MAX(10, 11);\nd := ADD(1, 2);\nr := SQRT(2.0);\nr := 1.0 / 3.0;\nr := MAX(7, 2) / 2;\n\
         r := 7 / 2;\nb := GT(MAX(1, 2), 1);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert!(c.contains("iecf_max_INT(((IEC_INT)INT64_C(10)), ((IEC_INT)INT64_C(11)))"), "{c}");
    assert!(c.contains("iecf_add_DINT(&ctx->flag, ((IEC_DINT)INT64_C(1)), ((IEC_DINT)INT64_C(2)))"), "{c}");
    assert!(c.contains("IECF_SQRT((double)("), "{c}");
    assert!(c.contains("iecf_div_REAL("), "a quotient of reals, in REAL: {c}");
    assert!(c.contains("iecf_max_LINT(") && c.contains("iecf_div_LINT("), "integers, then the real: {c}");
    assert!(c.contains(" = 3.0f;"), "7 / 2 is 3: {c}");
}

#[test]
fn an_initial_value_of_standard_functions_is_worked_out_or_checked() {
    // docs/behaviour/arrays-subranges.md, Initial values.
    let one = |decl: &str| {
        generate(&format!(
            "FUNCTION UF : INT\nVAR_INPUT a : INT; END_VAR\nUF := a;\nEND_FUNCTION\n\
             PROGRAM main\nVAR CONSTANT k : INT := -3; kr : REAL := 1.5; END_VAR\nVAR v : INT; {decl} END_VAR\n\
             v := v;\nEND_PROGRAM\n{CONFIG}"
        ))
    };
    let c = |decl: &str| one(decl).unwrap()[1].1.clone();
    let code = |decl: &str| one(decl).unwrap_err()[0].0;
    // Exact: nothing left to the run.
    assert!(c("x : INT := MAX(ABS(k), 2);").contains("self->V_X = ((IEC_INT)INT64_C(3));"));
    assert!(c("x : INT := REAL_TO_INT(kr);").contains("self->V_X = ((IEC_INT)INT64_C(2));"));
    assert!(c("x : WORD := ROL(16#8001, 1);").contains("self->V_X = ((IEC_WORD)UINT64_C(3));"));
    assert!(c("x : TIME := T#2s * 1.5;").contains("self->V_X = ((IEC_TIME)INT64_C(3000));"));
    // The target's mathematics, once the compiler has checked the result.
    assert!(c("x : REAL := kr ** 2;").contains("self->V_X = iecf_real_of(&ctx->flag, IECF_EXPT("));
    assert!(c("x : BOOL := SQRT(2.0) > 1.0;").contains("IECF_SQRT(2.0)"));
    // What the run would flag, and a result too near the type's limit.
    assert_eq!(code("x : SINT := INT_TO_SINT(300);"), "E0020");
    assert_eq!(code("x : INT := REAL_TO_INT(1.0E10);"), "E0020");
    assert_eq!(code("x : INT := ABS(-32768);"), "E0020");
    assert_eq!(code("x : REAL := SQRT(-1.0);"), "E0020");
    assert_eq!(code("x : REAL := EXP(88.0);"), "E0020");
    // What it would fault on.
    assert_eq!(code("x : INT := MUX(2, 5, 6);"), "E0032");
    assert_eq!(code("x : WORD := SHL(16#01, -1);"), "E0032");
    assert_eq!(code("x : INT := DIV(k, 0);"), "E0032");
    assert_eq!(code("x : TIME := T#1s / 0;"), "E0032");
    // No constant.
    assert_eq!(code("x : INT := v + 1;"), "E0032");
    assert_eq!(code("x : INT := UF(k);"), "E0032");
    // The last digit of the target's mathematics deciding: not yet.
    assert_eq!(code("x : INT := REAL_TO_INT(SQRT(16.0));"), "E0009");
    assert_eq!(code("x : REAL := 2.0 / SIN(0.0);"), "E0009");
}

#[test]
fn a_part_of_a_constant_array_or_structure_is_a_constant() {
    let one = |decl: &str| {
        generate(&format!(
            "TYPE PT : STRUCT x : INT := 7; a : ARRAY [0..1] OF INT := [8, 9]; END_STRUCT; END_TYPE\n\
             PROGRAM main\nVAR CONSTANT n : INT := 2; ka : ARRAY [1..3] OF INT := [4, 2(5)];\n\
             kps : ARRAY [1..2] OF PT := [(x := 11), (a := [6])]; {decl} END_VAR\n\
             VAR v : INT; END_VAR\nv := v;\nEND_PROGRAM\n{CONFIG}"
        ))
    };
    let c = |decl: &str| one(decl).unwrap()[1].1.clone();
    let code = |decl: &str| one(decl).unwrap_err()[0].0;
    assert!(c("END_VAR VAR y : INT := ka[n + 1] + kps[1].x;").contains("self->V_Y = ((IEC_INT)INT64_C(16));"));
    // The part's own, else its type's.
    assert!(c("END_VAR VAR y : INT := kps[2].x + kps[1].a[1];").contains("self->V_Y = ((IEC_INT)INT64_C(16));"));
    assert!(c("END_VAR VAR y : INT := kps[2].a[0] + kps[2].a[1];").contains("self->V_Y = ((IEC_INT)INT64_C(6));"));
    // A whole part, where its type goes.
    assert!(c("END_VAR VAR y : PT := kps[1];").contains("self->V_Y.V_X = ((IEC_INT)INT64_C(11));"));
    // Outside the array; a variable for an index; a part that names itself.
    assert_eq!(code("END_VAR VAR y : INT := ka[4];"), "E0032");
    assert_eq!(code("END_VAR VAR y : INT := ka[v];"), "E0032");
    assert_eq!(code("kz : ARRAY [1..2] OF INT := [kz[1], 1];"), "E0032");
    assert_eq!(code("kd : ARRAY [1..2] OF PT := [kd[2], kd[1]];"), "E0032");
    // One part of a constant from another of it is none.
    assert!(
        c("kc : ARRAY [1..2] OF INT := [kc[2], 3]; END_VAR VAR y : INT := kc[1];")
            .contains("self->V_Y = ((IEC_INT)INT64_C(3));")
    );
}

#[test]
fn a_standard_blocks_instance_is_an_in_out() {
    // Ed.3 6.6.3.4 rules 6 and 7: the caller's own, by its address.
    let text = format!(
        "FUNCTION F1 : BOOL\nVAR_IN_OUT p : TON; END_VAR\np(IN := TRUE, PT := T#1s); F1 := p.Q;\nEND_FUNCTION\n\
         FUNCTION_BLOCK B1\nVAR_IN_OUT p : TON; END_VAR\nVAR q : BOOL; END_VAR\nq := F1(p);\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR x : TON; xs : ARRAY [1..2] OF TON; b : B1; ok : BOOL; END_VAR\n\
         b(p := x);\nok := F1(xs[2]);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("iecf_TON_t (*V_P);") || c.contains("iecf_TON_t (*V_P);"), "{h}{c}");
    assert!(c.contains("self->V_B.V_P = &(self->V_X);"), "{c}");
    assert!(c.contains("F_F1(ctx, &(self->V_XS[1u]))"), "{c}");
}

#[test]
fn a_subrange_is_a_functions_parameter_and_result_and_starts_at_its_lower_limit() {
    let text = format!(
        "TYPE SRT : INT (2..5); END_TYPE\n\
         FUNCTION F1 : SRT\nVAR_INPUT p : SRT; END_VAR\nVAR_OUTPUT o : SRT; END_VAR\nVAR_IN_OUT io : SRT; END_VAR\n\
         o := p; io := p; F1 := p;\nEND_FUNCTION\n\
         PROGRAM main\nVAR x : SRT; v : INT; END_VAR\nv := F1(v, x, v);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    // Ed.3 6.4.4.4.2: the lower limit, the result and the output too.
    assert!(c.contains("self->V_X = ((IEC_INT)INT64_C(2));"), "{c}");
    assert!(c.contains("IEC_INT V_F1 = ((IEC_INT)INT64_C(2));"), "{c}");
    assert!(c.contains("IEC_INT V_O = ((IEC_INT)INT64_C(2));"), "{c}");
    // The input clamped as the function starts; each store in the body.
    assert!(c.contains("V_P = ((IEC_INT)iecf_clamp_s(&ctx->flag, (int64_t)(V_P), INT64_C(2), INT64_C(5)));"), "{c}");
    assert!(c.matches("iecf_clamp_s(").count() >= 4, "{c}");
}

#[test]
fn a_comparison_of_more_than_two_computes_each_input_once() {
    // Ed.3 Table 33: each neighbouring pair; 7.3.2: left to right.
    let text = format!(
        "FUNCTION Inc : INT\nVAR_IN_OUT io : INT; END_VAR\nio := io + 1; Inc := io;\nEND_FUNCTION\n\
         PROGRAM main\nVAR n : INT; b : BOOL; END_VAR\nb := GT(10, Inc(n), 0);\nb := GE(n, 2, 1);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert_eq!(c.matches("F_INC(ctx, &(self->V_N))").count(), 1, "called once: {c}");
    assert!(c.contains(" & ((IEC_BOOL)(("), "every pair computed, `&`: {c}");
    assert!(
        c.contains("((IEC_BOOL)(((IEC_BOOL)((self->V_N) >= (((IEC_INT)INT64_C(2))))) & "),
        "no temporaries without a call: {c}"
    );
}

#[test]
fn sel_mux_and_move_of_whole_values_select_where_the_input_is() {
    // Ed.3 Table 32, footnote d: a pointer to the input selected.
    let text = format!(
        "TYPE A3 : ARRAY [1..3] OF INT; END_TYPE\n\
         PROGRAM main\nVAR b : BOOL; k : INT; v : INT; a : A3; a2 : A3; r : REF_TO INT; r2 : REF_TO INT; END_VAR\n\
         a := SEL(b, a, a2);\nv := MUX(k, a, a2)[2];\nr := SEL(b, r, NULL);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert!(c.contains("IEC_BOOL g") && c.contains("IEC_INT (*p") && c.contains(" ? p"), "a pointer selected: {c}");
    assert!(!c.contains("iecf_bytes(&(self->V_A), &(self->V_A2)"), "no copy before the store: {c}");
    assert!(c.contains("== 0 ?") && c.contains("(iecf_fail(&ctx->fault, IECF_FAULT_MUX), "), "{c}");
    assert!(c.contains("iecf_ref_t v") && c.contains("iecf_ref_null()"), "{c}");
}

#[test]
fn a_reference_a_function_gives_is_computed_once_and_checked() {
    let text = "FUNCTION_BLOCK BA\nVAR_INPUT i : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
         FUNCTION PickB : REF_TO BA\nVAR_EXTERNAL gb : BA; END_VAR\nPickB := REF(gb);\nEND_FUNCTION\n\
         PROGRAM main\nVAR_EXTERNAL gb : BA; END_VAR\nVAR tm : TON; END_VAR\nPickB()^.i := 3;\ntm.PT := T#2s;\nEND_PROGRAM\n\
         CONFIGURATION plant\nVAR_GLOBAL gb : BA; END_VAR\nRESOURCE r ON x\nTASK t(INTERVAL := T#10ms, PRIORITY := 1);\n\
         PROGRAM p WITH t : main;\nEND_RESOURCE\nEND_CONFIGURATION\n";
    let c = &generate(text).unwrap()[1].1;
    assert_eq!(c.matches("F_PICKB(ctx, &d").count(), 1, "called once: {c}");
    assert!(c.contains("iecf_deref(&ctx->fault, d"), "checked before the store: {c}");
    assert!(c.contains("self->V_TM.PT = "), "a standard block's input set from outside: {c}");
}

#[test]
fn a_statement_whose_conversion_fails_faults_on_nothing_after() {
    // docs/behaviour/conversions.md: left undone, the task carries on.
    let text = format!(
        "PROGRAM main\nVAR r : REAL; v : INT; a : ARRAY [1..3] OF INT; END_VAR\n\
         v := 10 / REAL_TO_INT(r);\na[REAL_TO_INT(r)] := 1;\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert!(c.matches("iecf_undone(&ctx->fault, k").count() >= 2, "{c}");
    assert!(c.contains("if (!k"), "the store left undone: {c}");
}

#[test]
fn an_en_call_nested_runs_inside_the_outer_and_before_its_flag_is_cleared() {
    let text = format!(
        "PROGRAM main\nVAR v : INT; b : BOOL; e1 : BOOL; e2 : BOOL; END_VAR\n\
         v := ADD(EN := b, IN1 := ADD(EN := b, IN1 := 1, IN2 := 2, ENO => e2), IN2 := 3, ENO => e1);\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    let outer = c.find("if (en").unwrap();
    let inner = c[outer + 1..].find("if (en").map(|i| i + outer + 1).expect(c);
    // The outer clears the flag for its own call only after the inner ran.
    let clears: Vec<usize> = c.match_indices("ctx->flag.code = IECF_FAULT_NONE;").map(|(i, _)| i).collect();
    assert_eq!(clears.len(), 2, "{c}");
    assert!(inner < clears[0] && clears[0] < clears[1], "{c}");
    assert!(c.contains("self->V_E2 = eno") && c.contains("self->V_E1 = eno"), "{c}");
}

#[test]
fn an_edge_input_is_an_implicit_trigger_the_body_reads() {
    // Ed.3 6.6.3.2 rule 13.
    let text = format!(
        "FUNCTION_BLOCK E\nVAR_INPUT clk : BOOL R_EDGE; END_VAR\nVAR_OUTPUT q : BOOL; END_VAR\nq := clk;\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR e : E; b : BOOL; END_VAR\ne(clk := b);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let (h, c) = (&files[0].1, &files[1].1);
    assert!(h.contains("iecf_R_TRIG_t E_CLK;") || c.contains("iecf_R_TRIG_t E_CLK;"), "{h}{c}");
    let body = &c[c.find("static void B_E(").expect(c)..];
    let call = body.find("self->E_CLK.CLK = self->V_CLK;").expect(body);
    let read = body.find("= self->E_CLK.Q;").expect(body);
    assert!(call < read, "{body}");
}

#[test]
fn a_part_of_a_bit_string_is_read_by_a_shift_and_written_into_its_whole() {
    // Ed.3 6.6.1.3, Table 17.
    let text = format!(
        "PROGRAM main\nVAR w : WORD; b : BYTE; x : BOOL; END_VAR\nb := w.%B1;\nw.%B0 := b;\nx := w.3;\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    assert!(c.contains("((IEC_BYTE)(((uint64_t)(self->V_W) >> 8u) & UINT64_C(255)))"), "{c}");
    assert!(c.contains("self->V_W = ((IEC_WORD)(((uint64_t)(self->V_W) & ~(UINT64_C(255) << 0u))"), "{c}");
    assert!(c.contains("((IEC_BOOL)(((uint64_t)(self->V_W) >> 3u) & UINT64_C(1)))"), "{c}");
}

#[test]
fn a_for_variable_at_an_address_counts_in_a_copy() {
    let text = format!(
        "PROGRAM main\nVAR i AT %MW0 : INT; END_VAR\nVAR n : INT; END_VAR\nFOR i := 1 TO 3 DO n := n + i; END_FOR;\nEND_PROGRAM\n{CONFIG}"
    );
    let c = &generate(&text).unwrap()[1].1;
    let head = c.find("for (fv").expect(c);
    assert!(c[head..].contains("iecf_set_w(ctx->image.m, 0u"), "written as each pass starts: {c}");
    assert!(c.contains("if (d") && c.contains(") iecf_set_w(ctx->image.m, 0u, (uint16_t)(fv"), "after the loop: {c}");
}

#[test]
fn a_tasks_interval_may_be_a_global_constant() {
    let text = "PROGRAM main\nVAR v : INT; END_VAR\nv := 1;\nEND_PROGRAM\n\
        CONFIGURATION Plant\n  VAR_GLOBAL CONSTANT base : TIME := T#20ms; slow : TIME := base * 2; END_VAR\n\
        RESOURCE r ON PLC\n    TASK t(INTERVAL := slow, PRIORITY := 1);\n    PROGRAM a WITH t : main;\n  END_RESOURCE\nEND_CONFIGURATION\n";
    let c = &generate(text).unwrap()[1].1;
    assert!(c.contains("{\"R__T\", plant_task_R__T, INT64_C(40000000), INT64_C(1)}"), "{c}");
}

#[test]
fn a_blocks_in_out_may_be_of_its_own_type() {
    // A pointer: the type is declared ahead of every structure.
    let text = format!(
        "FUNCTION_BLOCK S\nVAR_IN_OUT x : S; END_VAR\nVAR_OUTPUT n : INT; END_VAR\nn := n + 1;\nEND_FUNCTION_BLOCK\n\
         PROGRAM main\nVAR q : S; q2 : S; END_VAR\nq(x := q2);\nEND_PROGRAM\n{CONFIG}"
    );
    let files = generate(&text).unwrap();
    let h = &files[0].1;
    let fwd = h.find("typedef struct B_S_s B_S_t;").expect(h);
    let def = h.find("struct B_S_s {").expect(h);
    assert!(fwd < def && h.contains("B_S_t *V_X;"), "{h}");
}

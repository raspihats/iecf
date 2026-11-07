//! The generator's own tests: what it refuses, and the shape of what it
//! writes. Whether the C means what the program says is the harness's
//! business (harness/README.md), run against matiec's traces.

use iecf_diagnostics::{FileId, SourceMap};

type Files = Vec<(String, String)>;
type Errors = Vec<(&'static str, String)>;

fn generate(text: &str) -> Result<Files, Errors> {
    let mut map = SourceMap::new();
    let id = map.add("t.st", text);
    assert_eq!(id, FileId(0));
    let parsed = iecf_syntax::parse(id, text);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let units = [&parsed.unit];
    let a = iecf_sema::analyze(&units, &iecf_sema::Options::default());
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
    assert_eq!(names, ["plant.h", "plant.c", "iecf.h"]);
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
    let text = format!("PROGRAM main\nVAR s : WSTRING; END_VAR\ns := \"a\";\nEND_PROGRAM\n{CONFIG}");
    let errs = generate(&text).unwrap_err();
    assert_eq!(errs, vec![("E0009", "a variable of type WSTRING is not compiled to C yet".to_string())]);
    let errs = generate("PROGRAM main\nEND_PROGRAM\n").unwrap_err();
    assert_eq!(errs[0].0, "E0009", "no CONFIGURATION");
}

#[test]
fn a_block_s_in_out_must_be_a_variable_with_an_address_in_c() {
    let block = "FUNCTION_BLOCK dbl\nVAR_IN_OUT v : INT; END_VAR\nv := v * 2;\nEND_FUNCTION_BLOCK\n";
    let ok = format!("{block}PROGRAM main\nVAR d : dbl; x : INT; END_VAR\nd(v := x);\nEND_PROGRAM\n{CONFIG}");
    let files = generate(&ok).unwrap();
    let c = &files[1].1;
    assert!(c.contains("self->V_D.V_V = &(self->V_X);"), "{c}");
    assert!(c.contains("(*self->V_V) = "), "{c}");
    let located = format!(
        "{block}PROGRAM main\nVAR d : dbl; END_VAR\nVAR x AT %MW0 : INT; END_VAR\nd(v := x);\nEND_PROGRAM\n{CONFIG}"
    );
    assert_eq!(generate(&located).unwrap_err()[0].0, "E0009");
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
    assert_eq!(generate(&init).unwrap_err()[0].0, "E0009", "no statement to leave undone");
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
    assert!(h.contains("IEC_DINT V_M;") && h.contains("T_PAIR V_V[3];"), "{h}");
    assert!(c.contains("self->V_M = ((IEC_DINT)INT64_C(1));"), "the type's initial value, busy: {c}");
    assert!(
        c.contains("uint32_t o")
            && c.contains("iecf_index(&ctx->fault, (int64_t)(self->V_I), INT64_C(-1), INT64_C(1))"),
        "{c}"
    );
    assert!(c.contains("].V_B[1u] = t"), "the index checked before the store: {c}");
    assert!(c.contains("iecf_bytes(&(self->V_P2), s"), "a whole structure copied: {c}");
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

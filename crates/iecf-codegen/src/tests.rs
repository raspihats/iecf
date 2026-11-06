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
    let text = format!("PROGRAM main\nVAR s : STRING; END_VAR\ns := 'a';\nEND_PROGRAM\n{CONFIG}");
    let errs = generate(&text).unwrap_err();
    assert_eq!(errs, vec![("E0009", "a variable of type STRING is not compiled to C yet".to_string())]);
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
    let in_block = format!(
        "FUNCTION_BLOCK fb\nVAR RETAIN n : INT; END_VAR\nEND_FUNCTION_BLOCK\nPROGRAM main\nVAR f : fb; END_VAR\nEND_PROGRAM\n{CONFIG}"
    );
    assert_eq!(generate(&in_block).unwrap_err()[0].0, "E0009", "RETAIN inside a FUNCTION_BLOCK: not yet");
}

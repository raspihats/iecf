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
fn var_temp_waits_for_its_slice() {
    // Re-initialised at each call (Ed.3 6.5.1): not a field of the instance.
    let text = format!("PROGRAM main\nVAR_TEMP t : INT; END_VAR\nt := 1;\nEND_PROGRAM\n{CONFIG}");
    assert_eq!(
        generate(&text).unwrap_err(),
        vec![("E0009", "VAR_TEMP in a PROGRAM is not compiled to C yet".to_string())]
    );
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

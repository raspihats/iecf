//! Array bounds as constant expressions (Ed.3 6.4.4.5.1, 6.6.1.2.4) and
//! variable-length arrays (Ed.3 6.5.3).
use iecf_diagnostics::FileId;

/// `(code, line)` of each diagnostic of the project `text`.
fn codes(text: &str) -> Vec<(&'static str, usize)> {
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(!parsed.diagnostics.iter().any(|d| d.is_error()), "parse: {:#?}", parsed.diagnostics);
    let mut units = vec![parsed.unit];
    let a = iecf_sema::analyze_project(&mut units, &iecf_sema::Options::default());
    a.diagnostics.into_iter().map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1)).collect()
}

#[test]
fn bounds_name_constants() {
    let text = "CONFIGURATION c
VAR_GLOBAL CONSTANT W : INT := 3; WW : DINT := W * 2; END_VAR
END_CONFIGURATION
TYPE ROW : ARRAY[0..WW] OF INT; END_TYPE
FUNCTION_BLOCK B
VAR_EXTERNAL CONSTANT W : INT; END_VAR
VAR a : ARRAY[1..N] OF INT; s : STRING[N + W]; r : INT (0..N); END_VAR
VAR CONSTANT N : INT := 4; END_VAR
END_FUNCTION_BLOCK
";
    assert_eq!(codes(text), vec![]);
}

#[test]
fn e0032_a_bound_that_is_not_constant() {
    let text = "FUNCTION_BLOCK B
VAR n : INT := 3; x : REAL := 2.0; END_VAR
VAR CONSTANT R : REAL := 3.0; END_VAR
VAR a : ARRAY[1..n] OF INT; END_VAR
VAR b : ARRAY[1..R] OF INT; END_VAR
VAR s : STRING[n]; END_VAR
VAR lim : INT (0..n); END_VAR
END_FUNCTION_BLOCK
";
    assert_eq!(codes(text), vec![("E0032", 4), ("E0032", 5), ("E0032", 6), ("E0032", 7)]);
}

#[test]
fn a_local_name_hides_a_global_constant() {
    // N in the block is its variable, not the global constant.
    let text = "CONFIGURATION c
VAR_GLOBAL CONSTANT N : INT := 3; END_VAR
END_CONFIGURATION
FUNCTION_BLOCK B
VAR N : INT; a : ARRAY[1..N] OF INT; END_VAR
END_FUNCTION_BLOCK
";
    assert_eq!(codes(text), vec![("E0032", 5)]);
}

#[test]
fn bounds_in_order_and_within_dint() {
    let text = "FUNCTION_BLOCK B
VAR a : ARRAY[5..1] OF INT; END_VAR
VAR b : ARRAY[0..3000000000] OF INT; END_VAR
END_FUNCTION_BLOCK
";
    assert_eq!(codes(text), vec![("E0014", 2), ("E0014", 3)]);
}

#[test]
fn e0033_array_star_only_as_a_parameter() {
    let text = "TYPE T : ARRAY[*] OF INT; END_TYPE
FUNCTION F : INT
VAR_INPUT i : ARRAY[*] OF INT; END_VAR
VAR_IN_OUT io : ARRAY[*, *] OF INT; END_VAR
VAR_OUTPUT o : ARRAY[*] OF INT; END_VAR
VAR v : ARRAY[*] OF INT; END_VAR
F := 0;
END_FUNCTION
FUNCTION_BLOCK B
VAR_IN_OUT io : ARRAY[*] OF INT; END_VAR
VAR_INPUT i : ARRAY[*] OF INT; END_VAR
VAR_IN_OUT m : ARRAY[*, 1..3] OF INT; END_VAR
END_FUNCTION_BLOCK
";
    assert_eq!(codes(text), vec![("E0033", 1), ("E0033", 6), ("E0033", 11), ("E0033", 12)]);
}

#[test]
fn an_array_star_takes_any_bounds_and_gives_none() {
    let text = "FUNCTION F : INT
VAR_IN_OUT a : ARRAY[*] OF INT; END_VAR
F := G(a);
END_FUNCTION
FUNCTION G : INT
VAR_IN_OUT a : ARRAY[1..3] OF INT; END_VAR
G := DINT_TO_INT(LOWER_BOUND(a, 1));
END_FUNCTION
FUNCTION_BLOCK B
VAR x : ARRAY[0..9] OF INT; m : ARRAY[1..2, 1..2] OF INT; d : ARRAY[0..9] OF DINT; END_VAR
F(x);
F(a := x);
F(m);
F(d);
END_FUNCTION_BLOCK
";
    // F gives its ARRAY[*] on to a fixed in-out: E0013; m has two
    // dimensions and d other elements.
    assert_eq!(codes(text), vec![("E0013", 3), ("E0013", 13), ("E0013", 14)]);
}

#[test]
fn lower_and_upper_bound() {
    let text = "FUNCTION_BLOCK B
VAR a : ARRAY[1..20, -2..2] OF INT; k : INT; i : DINT; r : REAL; END_VAR
i := LOWER_BOUND(a, 2) + UPPER_BOUND(ARR := a, DIM := 1);
i := LOWER_BOUND(a, k);
i := LOWER_BOUND(a, 0);
i := UPPER_BOUND(a, 3);
i := UPPER_BOUND(k, 1);
i := UPPER_BOUND(a, r);
i := UPPER_BOUND(a);
END_FUNCTION_BLOCK
";
    assert_eq!(codes(text), vec![("E0014", 5), ("E0014", 6), ("E0014", 7), ("E0014", 8), ("E0016", 9)]);
}

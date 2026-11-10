//! Classes and methods (Ed.3 6.6.5, 6.6.7; OO.md step A).
use iecf_diagnostics::FileId;

/// `(code, line, message)` of each diagnostic of a project of these
/// files, its namespaces taken out first, as the build does.
fn check_files(files: &[&str]) -> Vec<(&'static str, usize, String)> {
    let mut units = Vec::new();
    for (i, t) in files.iter().enumerate() {
        let parsed = iecf_syntax::parse(FileId(i as u32), t);
        assert!(!parsed.diagnostics.iter().any(|d| d.is_error()), "parse: {:#?}", parsed.diagnostics);
        units.push(parsed.unit);
    }
    let a = iecf_sema::analyze_project(&mut units, &iecf_sema::Options::default());
    a.diagnostics
        .into_iter()
        .map(|d| {
            let t = files[d.span.file.0 as usize];
            (d.code.id(), t[..d.span.start as usize].matches('\n').count() + 1, d.message)
        })
        .collect()
}

fn check(text: &str) -> Vec<(&'static str, usize, String)> {
    check_files(&[text])
}

fn codes(text: &str) -> Vec<(&'static str, usize)> {
    check(text).into_iter().map(|(c, l, _)| (c, l)).collect()
}

/// Ed.3 6.6.5.4.3 Example 2, its counter's limit renamed (MAX is a
/// standard function's name), with a PUBLIC variable and a method
/// without a result. 23 lines.
const COUNTER: &str = "CLASS COUNTER
VAR CV : UINT; Top : UINT := 1000; END_VAR
VAR PUBLIC hits : INT; END_VAR
VAR PRIVATE secret : INT; END_VAR
METHOD PUBLIC UP : UINT
VAR_INPUT INC : UINT; END_VAR
VAR_OUTPUT QU : BOOL; END_VAR
IF CV <= Top - INC THEN CV := CV + INC; QU := FALSE; ELSE QU := TRUE; END_IF;
UP := CV;
THIS.hits := hits + 1;
END_METHOD
METHOD PUBLIC UP5 : UINT
VAR_OUTPUT QU : BOOL; END_VAR
UP5 := THIS.UP(INC := 5, QU => QU);
END_METHOD
METHOD Clear
THIS.CV := 0;
END_METHOD
METHOD PRIVATE Hide
secret := 1;
THIS.Clear();
END_METHOD
END_CLASS
";

fn with_counter(body: &str) -> String {
    format!("{COUNTER}PROGRAM p\nVAR c : COUNTER; v : UINT; b : BOOL; n : INT; END_VAR\n{body}\nEND_PROGRAM\n")
}

#[test]
fn a_class_and_its_methods_are_silent() {
    let body = "v := c.UP(INC := 5, QU => b);\nv := c.UP(5, b);\nv := c.UP5();\nc.UP(INC := 1);\n\
                n := c.hits;\nc.hits := 3;\nv := c.UP(EN := b, INC := 1, ENO => b);";
    assert_eq!(check(&with_counter(body)), vec![]);
}

#[test]
fn e0030_access_specifiers() {
    // The body starts on line 26. A method without a specifier is
    // PROTECTED (OO.md D1), a VAR section without one too.
    let found = check(&with_counter("c.Clear();\nc.Hide();\nn := c.secret;\nv := c.CV;"));
    let got: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(got, vec![("E0030", 26), ("E0030", 27), ("E0030", 28), ("E0010", 29)]);
    assert_eq!(
        found[0].2,
        "COUNTER.Clear is PROTECTED (none written: Ed.3 6.6.5.9): only COUNTER and the types derived from it \
         call it; write METHOD PUBLIC Clear to call it from here"
    );
    assert!(found[1].2.contains("PRIVATE: only COUNTER itself calls it"), "{}", found[1].2);
    assert_eq!(
        found[3].2,
        "'CV' is PROTECTED in the CLASS COUNTER (no specifier: Ed.3 6.6.5.10): declare it in VAR PUBLIC to reach it from outside"
    );
}

#[test]
fn e0030_internal_is_its_namespace_s() {
    let lib = "NAMESPACE Lib\nCLASS K\nVAR INTERNAL n : INT; END_VAR\n\
               METHOD INTERNAL Bump\nn := n + 1;\nEND_METHOD\nEND_CLASS\n\
               FUNCTION_BLOCK User\nVAR k : K; x : INT; END_VAR\nk.Bump();\nx := k.n;\nEND_FUNCTION_BLOCK\n\
               END_NAMESPACE\n";
    let app = "PROGRAM p\nVAR k : Lib.K; x : INT; END_VAR\nk.Bump();\nx := k.n;\nEND_PROGRAM\n";
    let found = check_files(&[lib, app]);
    let got: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(got, vec![("E0030", 3), ("E0030", 4)], "{found:?}");
}

#[test]
fn e0031_where_the_parts_go() {
    let text = "CLASS A\nVAR_INPUT i : INT; END_VAR\nVAR_OUTPUT PUBLIC o : INT; END_VAR\n\
                METHOD OVERRIDE m\nVAR PUBLIC t : INT; END_VAR\nEND_METHOD\nEND_CLASS\n\
                FUNCTION_BLOCK FINAL F\nVAR_EXTERNAL PROTECTED g : INT; END_VAR\nEND_FUNCTION_BLOCK\n\
                VAR_GLOBAL g : INT; END_VAR\n\
                PROGRAM p\nVAR PUBLIC x : INT; END_VAR\nx := THIS.x;\nEND_PROGRAM\n";
    assert_eq!(
        codes(text),
        vec![
            ("E0031", 2),
            ("E0031", 3),
            ("E0031", 3),
            ("E0031", 4),
            ("E0031", 5),
            ("E0031", 9),
            ("E0031", 13),
            ("E0031", 14)
        ]
    );
}

#[test]
fn e0031_an_instance_is_not_copied_nor_an_input() {
    // OO.md D11: not copied, not an input or output, not a FUNCTION's or
    // method's variable; an in-out it may be.
    let text = format!(
        "{COUNTER}FUNCTION_BLOCK F\nVAR_INPUT a : COUNTER; END_VAR\nVAR_IN_OUT b : COUNTER; END_VAR\nEND_FUNCTION_BLOCK\n\
         FUNCTION g : INT\nVAR c : COUNTER; END_VAR\nVAR_IN_OUT d : COUNTER; END_VAR\ng := 0;\nEND_FUNCTION\n\
         PROGRAM p\nVAR c, e : COUNTER; END_VAR\nc := e;\nc();\nEND_PROGRAM\n"
    );
    assert_eq!(codes(&text), vec![("E0031", 25), ("E0031", 29), ("E0031", 35), ("E0016", 36)]);
}

#[test]
fn a_method_s_calls_and_names() {
    // Its output and result are only its call's (6.6.5.4.7); its names
    // apart from its type's (rule 10, 6.6.5.5.5 error 2); no overloading;
    // a method without a result gives no value; a CLASS is no function.
    let text = format!(
        "{COUNTER}CLASS B\nVAR x : INT; END_VAR\n\
         METHOD m\nVAR x : INT; END_VAR\nEND_METHOD\n\
         METHOD x\nEND_METHOD\n\
         METHOD n\nEND_METHOD\nMETHOD N\nEND_METHOD\nEND_CLASS\n\
         PROGRAM p\nVAR c : COUNTER; v : UINT; b : BOOL; END_VAR\n\
         b := c.QU;\nv := c.UP;\nc.INC := 1;\nv := c.Clear();\nv := COUNTER();\nEND_PROGRAM\n"
    );
    let got = codes(&text);
    assert_eq!(
        got,
        vec![
            ("E0012", 27),
            ("E0012", 29),
            ("E0012", 33),
            ("E0010", 38),
            ("E0016", 39),
            ("E0010", 40),
            ("E0016", 41),
            ("E0030", 41),
            ("E0016", 42),
        ]
    );
}

#[test]
fn a_method_does_not_reach_its_block_s_temps_and_in_outs() {
    // Ed.3 6.6.7.2.3 rule 4.
    let text = "FUNCTION_BLOCK F\nVAR_INPUT i : INT; END_VAR\nVAR_IN_OUT io : INT; END_VAR\n\
                VAR_TEMP t : INT; END_VAR\nVAR v : INT; END_VAR\n\
                METHOD PUBLIC m : INT\nm := i + v + io;\nm := THIS.t;\nEND_METHOD\nv := THIS.v + t + io;\n\
                END_FUNCTION_BLOCK\n";
    let found = check(text);
    let got: Vec<(&str, usize)> = found.iter().map(|(c, l, _)| (*c, *l)).collect();
    assert_eq!(got, vec![("E0010", 7), ("E0010", 8)]);
    assert_eq!(found[0].2, "'io' is a VAR_IN_OUT of the method's type: a method cannot reach it (Ed.3 6.6.7.2.3)");
    assert_eq!(found[1].2, "'t' is a VAR_TEMP of F: a method cannot reach it (Ed.3 6.6.7.2.3)");
}

#[test]
fn this_and_super_are_reserved() {
    assert_eq!(
        codes("PROGRAM p\nVAR this : INT; super : BOOL; END_VAR\nEND_PROGRAM\n"),
        vec![("E0018", 2), ("E0018", 2)]
    );
}

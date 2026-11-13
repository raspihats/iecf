//! Structures with relative locations and OVERLAP (Ed.3 6.4.4.7).
use iecf_diagnostics::FileId;

/// `(code, line)` of each diagnostic of the project `text`.
fn codes(text: &str) -> Vec<(&'static str, usize)> {
    let parsed = iecf_syntax::parse(FileId(0), text);
    assert!(!parsed.diagnostics.iter().any(|d| d.is_error()), "parse: {:#?}", parsed.diagnostics);
    let mut units = vec![parsed.unit];
    let a = iecf_sema::analyze_project(&mut units, &iecf_sema::Options::default());
    a.diagnostics.into_iter().map(|d| (d.code.id(), text[..d.span.start as usize].matches('\n').count() + 1)).collect()
}

/// The standard's own example, and a variable of each type in C memory
/// and at an address.
#[test]
fn the_standards_example() {
    let text = "TYPE
  Com1_data : STRUCT
    head AT %B0 : INT;
    length AT %B2 : USINT := 26;
    flag1 AT %X3.0 : BOOL;
    tail AT %B25 : BYTE;
  END_STRUCT;
  Com2_data : STRUCT OVERLAP
    head AT %B0 : INT;
    length AT %B2 : USINT;
    flag2 AT %X3.3 : BOOL;
    data1 AT %B5 : BYTE;
    data2 AT %B5 : REAL;
    tail AT %B19 : BYTE;
  END_STRUCT;
  Com_data : STRUCT OVERLAP
    C1 AT %B0 : Com1_data;
    C2 AT %B0 : Com2_data;
  END_STRUCT;
END_TYPE
PROGRAM main
  VAR c : Com_data; c1 : Com1_data := (head := 3); END_VAR
  VAR m AT %MB0 : Com_data; END_VAR
  c.C1.head := c.C2.head + 1;
END_PROGRAM";
    assert_eq!(codes(text), vec![]);
}

#[test]
fn e0034_each_component_placed_or_none() {
    let text = "TYPE
  Mixed : STRUCT a AT %B0 : INT; b : INT; END_STRUCT;
  Partly : STRUCT a AT %B0 : INT; b AT %I* : BOOL; END_STRUCT;
  Plain : STRUCT a : INT; b : INT; END_STRUCT;
END_TYPE";
    assert_eq!(codes(text), vec![("E0034", 2), ("E0034", 3)]);
}

/// Bits apart are apart; a byte, a word over it, are not.
#[test]
fn e0034_components_share_nothing_without_overlap() {
    let text = "TYPE
  Clash : STRUCT w AT %B0 : WORD; b AT %B1 : BYTE; END_STRUCT;
  Bits : STRUCT
    x AT %X0.0 : BOOL;
    y AT %X0.1 : BOOL;
    z AT %X0.1 : BOOL;
  END_STRUCT;
  Apart : STRUCT x AT %X0.0 : BOOL; y AT %X0.7 : BOOL; w AT %B1 : WORD; END_STRUCT;
  OverBit : STRUCT x AT %X2.0 : BOOL; w AT %B1 : WORD; END_STRUCT;
  Shared : STRUCT OVERLAP w AT %B0 : WORD; b AT %B1 : BYTE; END_STRUCT;
END_TYPE";
    assert_eq!(codes(text), vec![("E0034", 2), ("E0034", 6), ("E0034", 9)]);
}

/// "Overlapped structures cannot be initialized explicitly" (Ed.3
/// 6.4.4.7.2): not in the type, not a variable, not a field of a
/// structure holding one.
#[test]
fn e0034_an_overlap_has_no_initial_value() {
    let text = "TYPE
  O : STRUCT OVERLAP a AT %B0 : INT := 1; b AT %B0 : BYTE; END_STRUCT;
  P : STRUCT OVERLAP a AT %B0 : INT; END_STRUCT;
  Q : STRUCT p : P := (a := 2); END_STRUCT;
  R : P := (a := 3);
END_TYPE
PROGRAM main
  VAR v : P := (a := 4); w : P; END_VAR
END_PROGRAM";
    assert_eq!(codes(text), vec![("E0034", 2), ("E0034", 4), ("E0034", 5), ("E0034", 8)]);
}

/// OVERLAP without relative locations says nothing of which share.
#[test]
fn e0034_an_overlap_places_its_components() {
    assert_eq!(codes("TYPE U : STRUCT OVERLAP a : INT; b : BYTE; END_STRUCT; END_TYPE"), vec![("E0034", 1)]);
}

#[test]
fn e0022_what_a_relative_location_is() {
    let text = "TYPE
  S : STRUCT
    i AT %X0.1 : INT;
    t AT %B9.1 : BYTE;
    u AT %X9 : BOOL;
    v AT %10.2 : BOOL;
    f AT %B12 : TON;
    r AT %B16 : REF_TO INT;
    full AT %IX0.0 : BOOL;
  END_STRUCT;
END_TYPE";
    assert_eq!(codes(text), vec![("E0022", 3), ("E0022", 4), ("E0022", 5), ("E0022", 7), ("E0022", 8), ("E0022", 9)]);
}

/// %B is a byte location for any width, as the standard's example has
/// it; %W, %D and %L name a width, warned when it is not the type's.
#[test]
fn w0004_a_size_that_is_not_the_types() {
    let text = "TYPE
  S : STRUCT
    a AT %B0 : REAL;
    b AT %D4 : REAL;
    c AT %W8 : REAL;
    d AT %B12 : BOOL;
  END_STRUCT;
END_TYPE";
    assert_eq!(codes(text), vec![("W0004", 5)]);
}

/// A relative location is a component's place: no variable has one, nor
/// does VAR_CONFIG give one.
#[test]
fn e0022_a_relative_location_outside_a_structure() {
    let text = "PROGRAM main
  VAR x AT %B0 : INT; END_VAR
END_PROGRAM
VAR_GLOBAL g AT %X1.0 : BOOL; END_VAR";
    assert_eq!(codes(text), vec![("E0022", 2), ("E0022", 4)]);
}

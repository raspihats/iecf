(* Three CODESYS ways of writing, each an extension (docs/extensions.md).
   Worked by hand:
   struct-semicolon: Pair ends END_STRUCT with no ';', the next type
   follows; Trio's END_STRUCT stands before END_TYPE.
   bare-array-init: a : ARRAY[1..5] OF SINT := 1, 2, 2(7), 3; is
   [1, 2, 7, 7, 3], sum 20 (%QB0); Pair's v := 8, 9 in a STRUCT, 8 + 9 =
   17 (%QB1); the type Row := 4, 5, 6, 4 + 5 + 6 = 15 (%QB2).
   real-without-point: r := 1E3 + 25E-1, 1002.5; REAL_TO_SINT(r / 100.0)
   is 10.025 rounded, 10 (%QB3); Trio's z := 2E1, 20.0, as SINT 20
   (%QB4).
   reference-skip: none of the three is IEC 61131-3. *)
TYPE
  Pair : STRUCT
    v : ARRAY[1..2] OF SINT := 8, 9;
  END_STRUCT
  Row : ARRAY[0..2] OF SINT := 4, 5, 6;
  Trio : STRUCT
    z : REAL := 2E1;
  END_STRUCT
END_TYPE

PROGRAM main
  VAR
    a : ARRAY[1..5] OF SINT := 1, 2, 2(7), 3;
    p : Pair;
    w : Row;
    t : Trio;
    r : REAL;
    i : INT;
    s : SINT;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
  END_VAR
  s := 0;
  FOR i := 1 TO 5 DO
    s := s + a[i];
  END_FOR;
  q0 := s;
  q1 := p.v[1] + p.v[2];
  q2 := w[0] + w[1] + w[2];
  r := 1E3 + 25E-1;
  q3 := REAL_TO_SINT(r / 100.0);
  q4 := REAL_TO_SINT(t.z);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

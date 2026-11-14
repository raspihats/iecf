(* CASE on a selector of any elementary type (Ed.3 7.3.3.3.3: "a
   variable of elementary data type", its labels "literals, enumerated
   values, or subranges" it can be compared with): BOOL, the bit
   strings, CHAR and WCHAR, STRING and WSTRING, REAL, TIME, DATE and
   TOD; labels that are literals, constants and subranges; a selector
   that is a FUNCTION with effects, called once. q[i] is the arm taken,
   9 for ELSE. Worked by hand:

   tick 0, n 1: b TRUE 1; w 16#2 1; bb 1 1; lw 1 ELSE 9; 'a' 1; "a" 1;
   'ab' 1; "ab" 1; 1.0 1; T#1s 1; the date 1; noon 1. calls 1 (Q12).
   tick 1, n 2: b FALSE 2; w 16#4, in 4..8, 2; bb 2 ELSE 9; lw 2 1; 'c'
   in 'b'..'d' 2; "z" in "y".."z" 2; 'cd', the constant, 2; "zz" 2; 2.5
   in 2.0..3.0 2; T#3s in the constant 2s..5s 2; June in February to
   December 2; 13:30 2. calls 2.
   tick 2, n 3: b TRUE 1; w 16#8 2; bb 3 1; lw 3 9; 'x' 9; "q" 9; 'zz'
   in 'x'..'zzzz' 3; "q" 9; -1.0 9; T#9s 9; 2021 9; 01:00 9. calls 3.
   reference-skip: CASE on anything but an integer or an enumeration:
   refused. *)
FUNCTION bump : STRING[4]
  VAR_INPUT s : STRING[4]; END_VAR
  VAR_EXTERNAL calls : USINT; END_VAR
  calls := calls + 1;
  bump := s;
END_FUNCTION
PROGRAM main
  VAR CONSTANT kc : CHAR := 'b'; ks : STRING[4] := 'cd'; kt : TIME := T#2s; END_VAR
  VAR_EXTERNAL calls : USINT; END_VAR
  VAR b : BOOL; w : WORD; bb : BYTE; c : CHAR; s : STRING[4]; r : REAL; t : TIME; d : DATE; wc : WCHAR; lw : LWORD; ws : WSTRING[4]; tod1 : TOD; n : USINT; END_VAR
  VAR q AT %QB0 : ARRAY [0..11] OF USINT; qc AT %QB12 : USINT; END_VAR
  n := n + 1;
  b := (n MOD 2) = 1;
  w := SHL(WORD#1, n);
  bb := USINT_TO_BYTE(n);
  lw := USINT_TO_LWORD(n);
  CASE n OF 1: c := 'a'; s := 'ab'; wc := "a"; ws := "ab"; r := 1.0; t := T#1s; d := D#2020-01-01; tod1 := TOD#12:00:00;
            2: c := 'c'; s := 'cd'; wc := "z"; ws := "zz"; r := 2.5; t := T#3s; d := D#2020-06-15; tod1 := TOD#13:30:00;
  ELSE c := 'x'; s := 'zz'; wc := "q"; ws := "q"; r := -1.0; t := T#9s; d := D#2021-01-01; tod1 := TOD#01:00:00;
  END_CASE;
  CASE b OF TRUE: q[0] := 1; FALSE: q[0] := 2; END_CASE;
  CASE w OF 16#0002: q[1] := 1; 16#0004..16#0008: q[1] := 2; ELSE q[1] := 3; END_CASE;
  CASE bb OF 16#01, 16#03: q[2] := 1; ELSE q[2] := 9; END_CASE;
  CASE lw OF 16#2: q[3] := 1; ELSE q[3] := 9; END_CASE;
  CASE c OF 'a': q[4] := 1; kc..'d': q[4] := 2; ELSE q[4] := 9; END_CASE;
  CASE wc OF "a": q[5] := 1; "y".."z": q[5] := 2; ELSE q[5] := 9; END_CASE;
  CASE bump(s) OF 'ab': q[6] := 1; ks: q[6] := 2; 'x'..'zzzz': q[6] := 3; ELSE q[6] := 9; END_CASE;
  CASE ws OF "ab": q[7] := 1; "zz": q[7] := 2; ELSE q[7] := 9; END_CASE;
  CASE r OF 1.0: q[8] := 1; 2.0..3.0: q[8] := 2; ELSE q[8] := 9; END_CASE;
  CASE t OF T#1s: q[9] := 1; kt..T#5s: q[9] := 2; ELSE q[9] := 9; END_CASE;
  CASE d OF D#2020-01-01: q[10] := 1; D#2020-02-01..D#2020-12-31: q[10] := 2; ELSE q[10] := 9; END_CASE;
  CASE tod1 OF TOD#12:00:00: q[11] := 1; TOD#13:00:00..TOD#14:00:00: q[11] := 2; ELSE q[11] := 9; END_CASE;
  qc := calls;
END_PROGRAM
CONFIGURATION c VAR_GLOBAL calls : USINT; END_VAR RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

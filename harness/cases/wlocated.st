(* WSTRING at an address (docs/dependencies.md): its n + 1 UTF-16 units
   back to back from its byte, each little-endian, at an odd byte too; in
   a structure and an array there, packed. Its initial value written at
   the start (not into %I); a store writes the whole string back, so
   units after its 0 stay as they were.
   Worked by hand, the input win at %IW0:
   init: qw "ab" at Q1: 61 00 62 00 00 00 00 00; qp.n 0, qp.w "z" at
   Q11: 7a 00 00 00 00 00.
   Tick 0, win 16#263A: qw := CONCAT(qw, win) "ab$263A", then qw[1] :=
   win: "$263Ab$263A" - Q1 3a 26 62 00 3a 26 00 00; qp.n := LEN(qw) 3 at
   Q9; qp.w := qw cut to "$263Ab" - Q11 3a 26 62 00 00 00; qa[1] := qp.w
   cut to "$263A" at M4: 3a 26 00 00 (qa[0] at M0 empty); q2 := qw[2] 'b'
   at Q17.
   Tick 1, win 16#0041 ('A'): CONCAT gives "$263Ab$263AA", cut to 3:
   "$263Ab$263A", qw[1] := 'A': "Ab$263A"; qp.n 3; qp.w "Ab"; qa[1] "A".
   The cuts are flagged STRING_LENGTH: first qp.w := qw in tick 0, line
   32, a flag that stays (tick 1's CONCAT cut, line 29, behind it).
   reference-skip: WCHAR and s[i] are Ed.3's. *)
TYPE pair : STRUCT n : INT; w : WSTRING[2] := "z"; END_STRUCT; END_TYPE

PROGRAM main
VAR
  win AT %IW0 : WCHAR;
  qw AT %QW1 : WSTRING[3] := "ab";
  qp AT %QW9 : pair;
  q2 AT %QW17 : WCHAR;
  qa AT %MW0 : ARRAY[0..1] OF WSTRING[1];
END_VAR
qw := CONCAT(qw, win);
qw[1] := win;
qp.n := LEN(qw);
qp.w := qw;
qa[1] := qp.w;
q2 := qw[2];
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

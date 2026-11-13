(* A part of a block's elementary input or output read from outside
   (Ed.3 Table 17): `f.q.%X4`, `f.q.%B1`, of an element of an array of
   instances, in a condition. Worked by hand: f.iw 16#0123, so f.q is
   ROL(16#0123, 4), 16#1230: its byte 1 is 16#12 (Q0); its bit 4 is 1,
   iw's bit 1 is 1, fa[2].q is 16#0040 and its bit 6 is 1, f.q's bit 5
   is 1: the four bits 2#1111 (Q1 0f).
   reference-skip: partial access, as `partaccess`. *)
FUNCTION_BLOCK fb
  VAR_INPUT iw : WORD; END_VAR
  VAR_OUTPUT q : WORD; END_VAR
  q := ROL(iw, 4);
END_FUNCTION_BLOCK
PROGRAM main
  VAR f : fb; fa : ARRAY [1..2] OF fb; i : INT := 2; b1, b2, b3, b4 : BOOL; y : BYTE; END_VAR
  VAR q AT %QB0 : BYTE; q1 AT %QB1 : BYTE; END_VAR
  f(iw := 16#0123);
  fa[2](iw := 16#0004);
  b1 := f.q.%X4;
  y := f.q.%B1;
  b2 := f.iw.%X1;
  b3 := fa[i].q.%X6;
  IF f.q.%X5 THEN b4 := TRUE; END_IF;
  q := y;
  q1 := BOOL_TO_BYTE(b1) OR SHL(BOOL_TO_BYTE(b2), 1) OR SHL(BOOL_TO_BYTE(b3), 2) OR SHL(BOOL_TO_BYTE(b4), 3);
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

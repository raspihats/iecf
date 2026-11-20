(* A method testing its block's VAR_IN_OUT with __ISVALIDREF
   (method-in-outs), as TcOpen's tasks do before using one: FALSE until
   a call of the block binds it, and a use the test guards is no fault.
   Worked by hand, s.d from 3:
   cycle 0: i.M() before any call of i: unbound, -1 (the ELSIF that
     reads io not tested); i(io := s): d 6; i.M(): d 7, 7.
   cycle 1: i.M(): bound by the last call, d 8, 8; i(): 16; i.M(): 17.
   A derived block's body tests the in-out it inherits, bound by its
   call: %QW4 1, and its d (from 5) 5 + 10 = 15 at %QW6, 25 next.
   reference-skip: not IEC 61131-3. *)
TYPE St : STRUCT d : INT; END_STRUCT; END_TYPE

FUNCTION_BLOCK B
VAR_IN_OUT io : St; END_VAR
METHOD PUBLIC M : INT
IF NOT __ISVALIDREF(io) THEN
  M := -1;
ELSIF io.d > 1000 THEN
  M := 0;
ELSE
  THIS^.io.d := io.d + 1;
  M := io.d;
END_IF
END_METHOD
io.d := io.d * 2;
END_FUNCTION_BLOCK

FUNCTION_BLOCK D EXTENDS B
VAR_OUTPUT ok : BOOL; END_VAR
ok := __ISVALIDREF(io);
IF __ISVALIDREF(THIS^.io) THEN
  io.d := io.d + 10;
END_IF
END_FUNCTION_BLOCK

PROGRAM main
  VAR i : B; s : St := (d := 3); j : D; t : St := (d := 5); END_VAR
  VAR a AT %QW0 : INT; b AT %QW2 : INT; c AT %QW4 : INT; e AT %QW6 : INT; END_VAR
  a := i.M();
  i(io := s);
  b := i.M();
  j(io := t);
  c := BOOL_TO_INT(j.ok);
  e := t.d;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

(* An element of an in-out array passed to a FUNCTION that writes a
   global: the call is sequenced (its operands each on a line, as it may
   change what they read), and the element's index is found once. Until
   2026-11-19 it was found twice, the first line left unused, which C's
   -Wall -Werror refused (found in the OSCAT BASIC library's HOLIDAY).
   Worked by hand: hs[1].d is 5; G(y, hs[i].d) with y 0 is 5, gx 1
   (%QW2); 5 = y fails on the first tick: y := y + hs[i].d, 5 (%QW0);
   the second tick G(5, 5) = 10, gx 2; 10 = 5 fails, y := 10. *)
TYPE H : STRUCT d : INT; END_STRUCT; END_TYPE

FUNCTION G : INT
  VAR_INPUT a, b : INT; END_VAR
  VAR_EXTERNAL gx : INT; END_VAR
  gx := gx + 1;
  G := a + b;
END_FUNCTION

FUNCTION_BLOCK FB
  VAR_IN_OUT hs : ARRAY[0..3] OF H; END_VAR
  VAR_OUTPUT y : INT; END_VAR
  VAR i : INT := 1; END_VAR
  IF G(y, hs[i].d) = y THEN
    y := -1;
  ELSE
    y := y + hs[i].d;
  END_IF;
END_FUNCTION_BLOCK

PROGRAM main
  VAR a : ARRAY[0..3] OF H := [(d := 0), (d := 5)]; f : FB; END_VAR
  VAR_EXTERNAL gx : INT; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; END_VAR
  f(hs := a);
  q0 := f.y;
  q1 := gx;
END_PROGRAM

CONFIGURATION cf
  VAR_GLOBAL gx : INT; END_VAR
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

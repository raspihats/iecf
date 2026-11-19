(* Found in OSCAT NETWORK and BUILDING: a call's arguments in order
   first, then by name (mixed-arguments). Worked by hand:
   %QW0: F(1, c := 3, b := 2): a 1, b 2, c 3: 123.
   %QW2: F(1, 2, c := 4, o => s): 124, and s := 1 + 2 + 4 = 7 (%QW4).
   %QW6: sum(5, y := 6), then sum.s: 5 + 6 = 11.
   %QW8: LIMIT(0, MX := 10, IN := 15): MN 0, the rest by name: 10.
   %QW10: sum.m(9, v := 2), a method: 9 - 2 = 7.
   reference-skip: not IEC 61131-3. *)
FUNCTION F : INT
  VAR_INPUT a : INT; b : INT; c : INT; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  F := a * 100 + b * 10 + c;
  o := a + b + c;
END_FUNCTION

FUNCTION_BLOCK ACC
  VAR_INPUT x : INT; y : INT; END_VAR
  VAR_OUTPUT s : INT; END_VAR
  METHOD PUBLIC m : INT
    VAR_INPUT u : INT; v : INT; END_VAR
    m := u - v;
  END_METHOD
  s := x + y;
END_FUNCTION_BLOCK

PROGRAM main
  VAR sum : ACC; END_VAR
  VAR
    q0 AT %QW0 : INT; q1 AT %QW2 : INT; s AT %QW4 : INT; q3 AT %QW6 : INT;
    q4 AT %QW8 : INT; q5 AT %QW10 : INT;
  END_VAR
  q0 := F(1, c := 3, b := 2);
  q1 := F(1, 2, c := 4, o => s);
  sum(5, y := 6);
  q3 := sum.s;
  q4 := LIMIT(0, MX := 10, IN := 15);
  q5 := sum.m(9, v := 2);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

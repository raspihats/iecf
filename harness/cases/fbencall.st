(* A function block's EN given a call (Ed.3 6.6.1.5): of a FUNCTION
   with an in-out, of a method, of a method through an interface. EN is
   computed first, once, and only then the block is called or not, ENO
   telling which. Worked by hand, tick by tick:
   Advance adds 1 to n and gives n > 2: n 1, 2, 3, 4; c1 runs at ticks
   2 and 3, total (%QB0) 0, 0, 1, 2; n (%QB1); ENO e1 (%QX4.0) FALSE,
   FALSE, TRUE, TRUE.
   v.Ready gives its count < 2, then counts: TRUE, TRUE, FALSE, FALSE;
   c2 adds 10: 10, 20, 20, 20 (%QB2); e2 (%QX4.1).
   ig.Open counts and gives whether its count is even: FALSE, TRUE,
   FALSE, TRUE; c3 adds 5: 0, 5, 5, 10 (%QB3); e3 (%QX4.2).
   reference-skip: METHOD and INTERFACE are Ed.3's; the reference
   compiler (Ed.2) has neither. *)
FUNCTION Advance : BOOL
  VAR_IN_OUT cnt : INT; END_VAR
  cnt := cnt + 1;
  Advance := cnt > 2;
END_FUNCTION

FUNCTION_BLOCK Valve
  VAR n : INT; END_VAR
  METHOD PUBLIC Ready : BOOL
    Ready := n < 2;
    n := n + 1;
  END_METHOD
END_FUNCTION_BLOCK

INTERFACE IGate
  METHOD Open : BOOL
  END_METHOD
END_INTERFACE

FUNCTION_BLOCK Gate IMPLEMENTS IGate
  VAR k : INT; END_VAR
  METHOD PUBLIC Open : BOOL
    k := k + 1;
    Open := k MOD 2 = 0;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Count
  VAR_INPUT inc : INT; END_VAR
  VAR_OUTPUT total : INT; END_VAR
  total := total + inc;
END_FUNCTION_BLOCK

PROGRAM main
  VAR
    n : INT;
    v : Valve;
    g : Gate;
    ig : IGate;
    c1, c2, c3 : Count;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    e1 AT %QX4.0 : BOOL;
    e2 AT %QX4.1 : BOOL;
    e3 AT %QX4.2 : BOOL;
  END_VAR
  ig := g;
  c1(EN := Advance(n), inc := 1, ENO => e1);
  c2(EN := v.Ready(), inc := 10, ENO => e2);
  c3(EN := ig.Open(), inc := 5, ENO => e3);
  q0 := INT_TO_SINT(c1.total);
  q1 := INT_TO_SINT(n);
  q2 := INT_TO_SINT(c2.total);
  q3 := INT_TO_SINT(c3.total);
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

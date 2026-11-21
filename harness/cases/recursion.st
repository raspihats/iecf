(* A FUNCTION, a method and a block's body calling themselves
   (recursion), each call counted. Worked by hand:
   %QD0: fact(10), 3628800 (16#375F00), 00 5f 37 00.
   %QW4: t's o, sum(4) = 4 + 3 + 2 + 1, 10, 0a 00.
   %QW6: c's o, one for each of its three calls of itself, 03 00.
   %QD8: runaway(0) calls itself without an end: the call past
   IECF_DEPTH_MAX (64) is a fault, 14 (IECF_FAULT_DEPTH), at runaway's
   line 19, and %QD8 stays 0.
   reference-skip: the reference compiler knows a POU only after its
   declaration, so it can express no recursion. *)
FUNCTION fact : DINT
  VAR_INPUT n : DINT; END_VAR
  IF n <= 1 THEN fact := 1; RETURN; END_IF;
  fact := n * fact(n - 1);
END_FUNCTION

FUNCTION runaway : DINT
  VAR_INPUT n : DINT; END_VAR
  runaway := runaway(n + 1);
END_FUNCTION

FUNCTION_BLOCK Tree
  VAR_INPUT n : INT; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  METHOD sum : INT
    VAR_INPUT k : INT; END_VAR
    IF k = 0 THEN sum := 0; RETURN; END_IF;
    sum := k + THIS.sum(k - 1);
  END_METHOD
  o := THIS.sum(n);
END_FUNCTION_BLOCK

FUNCTION_BLOCK Count
  VAR_INPUT n : INT; END_VAR
  VAR_IN_OUT me : Count; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  IF n > 0 THEN
    o := o + 1;
    me(n := n - 1, me := me);
  END_IF;
END_FUNCTION_BLOCK

PROGRAM main
  VAR t : Tree; c : Count; END_VAR
  VAR q AT %QD0 : DINT; w AT %QW4 : INT; v AT %QW6 : INT; r AT %QD8 : DINT; END_VAR
  q := fact(10);
  t(n := 4);
  w := t.o;
  c(n := 3, me := c);
  v := c.o;
  r := runaway(0);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

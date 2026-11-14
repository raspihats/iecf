(* R_EDGE and F_EDGE inputs of a FUNCTION_BLOCK and a PROGRAM (Ed.3
   6.6.3.2 rule 13, Table 40 6a): an implicit R_TRIG or F_TRIG in the
   instance; the body sees its Q. The input %IX0.0 is FALSE at tick 0,
   TRUE at 1 and 2, FALSE at 3, TRUE at 4. Worked by hand, ticks 0..4:
   ET counts clk's rising edges in n: 0 1 1 1 2 (%QB0); fall's falling
   ones in m: an F_TRIG first called with CLK FALSE gives Q TRUE once
   (Table 44: M starts FALSE), so 1 1 1 2 2 (%QB1).
   The PROGRAM's own x, connected to %IX0.0, counted in p: 0 1 1 1 2
   (%QB2). Read from outside, e.clk is what was given, not the edge:
   0 1 1 0 1 (%QX3.0).
   reference-skip: R_EDGE and F_EDGE: the reference compiler does not
   support them. *)
FUNCTION_BLOCK ET
  VAR_INPUT clk : BOOL R_EDGE; fall : BOOL F_EDGE; END_VAR
  VAR_OUTPUT n : INT; m : INT; END_VAR
  IF clk THEN
    n := n + 1;
  END_IF;
  IF fall THEN
    m := m + 1;
  END_IF;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_INPUT x : BOOL R_EDGE; END_VAR
  VAR
    e : ET;
    p : INT;
  END_VAR
  VAR
    i AT %IX0.0 : BOOL;
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QX3.0 : BOOL;
  END_VAR
  e(clk := i, fall := i);
  q0 := INT_TO_SINT(e.n);
  q1 := INT_TO_SINT(e.m);
  IF x THEN
    p := p + 1;
  END_IF;
  q2 := INT_TO_SINT(p);
  q3 := e.clk;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM pr WITH t : main(x := %IX0.0);
  END_RESOURCE
END_CONFIGURATION

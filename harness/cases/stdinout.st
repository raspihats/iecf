(* A standard function block's instance as a VAR_IN_OUT: the caller's
   own instance, by its address, in a FUNCTION, in one that passes it
   on, and in a FUNCTION_BLOCK; given a variable, an array's element
   and what a reference refers to.
   tick is TRUE at ticks 0, 2, 4. Count calls c(CU := up, PV := 3).
   Count(c1, tick): c1 counts the three rising edges: 1 1 2 2 3 3
   (%QB0), and c1.Q, CV >= 3, is TRUE at ticks 4 and 5 (%QX4.0).
   Pass(cs[2], tick) gives it to Count: the same, 1 1 2 2 3 3 (%QB1),
   and it is cs[2] that counted (%QB5).
   Count(r^, TRUE), r referring to cs[1]: one rising edge, 1 at every
   tick (%QB2).
   Hold runs the TON it is given, PT 20 ms, the interval 10 ms: IN TRUE
   from tick 0, Q at tick 2 (%QX3.0), and it is t1 that ran (%QX6.0).
   reference-skip: untried on the reference compiler. *)
FUNCTION Count : INT
  VAR_IN_OUT c : CTU; END_VAR
  VAR_INPUT up : BOOL; END_VAR
  c(CU := up, PV := 3);
  Count := c.CV;
END_FUNCTION
FUNCTION Pass : INT
  VAR_IN_OUT c : CTU; END_VAR
  VAR_INPUT up : BOOL; END_VAR
  Pass := Count(c, up);
END_FUNCTION
FUNCTION_BLOCK Hold
  VAR_IN_OUT tm : TON; END_VAR
  VAR_INPUT go : BOOL; END_VAR
  VAR_OUTPUT q : BOOL; END_VAR
  tm(IN := go, PT := T#20ms);
  q := tm.Q;
END_FUNCTION_BLOCK
PROGRAM main
  VAR
    c1 : CTU;
    cs : ARRAY [1..2] OF CTU;
    t1 : TON;
    h : Hold;
    r : REF_TO CTU;
    tick : BOOL;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QX3.0 : BOOL;
    q4 AT %QX4.0 : BOOL;
    q5 AT %QB5 : SINT;
    q6 AT %QX6.0 : BOOL;
  END_VAR
  tick := NOT tick;
  q0 := INT_TO_SINT(Count(c1, tick));
  q1 := INT_TO_SINT(Pass(cs[2], tick));
  r := REF(cs[1]);
  q2 := INT_TO_SINT(Count(r^, TRUE));
  h(tm := t1, go := TRUE);
  q3 := h.q;
  q4 := c1.Q;
  q5 := INT_TO_SINT(cs[2].CV);
  q6 := t1.Q;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

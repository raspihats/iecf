(* SFC indicator variables (Ed.3 6.7.4.2, Table 58 field "c",
   deprecated; W0007): after the standard's own example, Figure 23's
   S22 to S27, in small. An indicator does nothing of its own: the
   action sets it, and a transition reads it. Actions run before the
   transitions, and a step's action once more as it is left (its final
   scan, as `sfcdemo`). Worked by hand (q0 the step whose action ran
   last, q1 the count of breaker, q2 closed):

   tick 0: S22: breaker, n 1, not closed (Q 16 01 00).
   tick 1: n 2, closed TRUE (Q 16 02 01); S22 to S23.
   tick 2: breaker's final scan, n 3, then opener resets closed (Q 17
   03 00); NOT closed: S23 to S22.
   tick 3: opener's final scan, then breaker, n 4, closed (Q 16 04 01).
   sfcind.iecf.trace: the reference compiler takes the indicator too, and
   has no final scan (as `sfcdemo`): n one less from tick 2. *)
PROGRAM main
  VAR closed : BOOL; n : USINT; st : USINT := 22; END_VAR
  VAR q AT %QB0 : USINT; q1 AT %QB1 : USINT; q2 AT %QB2 : USINT; END_VAR
  INITIAL_STEP s22 : breaker(N, closed); END_STEP
  TRANSITION FROM s22 TO s23 := closed; END_TRANSITION
  STEP s23 : opener(N); END_STEP
  TRANSITION FROM s23 TO s22 := NOT closed; END_TRANSITION
  ACTION breaker :
    st := 22;
    n := n + 1;
    closed := n >= 2;
    q := st; q1 := n; q2 := BOOL_TO_USINT(closed);
  END_ACTION
  ACTION opener :
    st := 23;
    closed := FALSE;
    q := st; q1 := n; q2 := BOOL_TO_USINT(closed);
  END_ACTION
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

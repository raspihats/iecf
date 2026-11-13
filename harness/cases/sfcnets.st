(* Two SFC networks in one POU (Ed.3 Annex A, SFC : Sfc_Network +),
   each with its own INITIAL_STEP, evolving side by side in each scan.
   Network A goes s0 to s1 once ca reaches 2 and straight back; network
   B goes t0 to t1 once cb reaches 3 and stays. A third, w0 alone, runs
   watch every scan: Q0 is 1 when s1 is active and 2 when t1 is (as the
   steps stand when watch runs, actions before transitions). Worked by
   hand, cnt_a and cnt_b each counting their step's scans:

   tick 0: ca 1, cb 1; s0 and t0: Q0 00.
   tick 1: ca 2, cb 2: s0's transition holds, s1 next.
   tick 2: s1 active (Q0 01); cnt_a's final scan, ca 3; cb 3: t1 next.
   tick 3: s0 again, t1 (Q0 02); cnt_b's final scan, cb 4; ca 4: s1.
   tick 4: s1 and t1 (Q0 03); cnt_a's final scan, ca 5.
   tick 5: s0 and t1 (Q0 02).
   reference-skip: the reference compiler takes one network. *)
PROGRAM main
  VAR ca, cb : INT; END_VAR
  VAR q AT %QB0 : USINT; END_VAR
  INITIAL_STEP s0 : cnt_a(N); END_STEP
  TRANSITION FROM s0 TO s1 := ca >= 2; END_TRANSITION
  STEP s1 : END_STEP
  TRANSITION FROM s1 TO s0 := TRUE; END_TRANSITION
  INITIAL_STEP t0 : cnt_b(N); END_STEP
  TRANSITION FROM t0 TO t1 := cb >= 3; END_TRANSITION
  STEP t1 : END_STEP
  TRANSITION FROM t1 TO t1 := FALSE; END_TRANSITION
  INITIAL_STEP w0 : watch(N); END_STEP
  TRANSITION FROM w0 TO w0 := FALSE; END_TRANSITION
  ACTION cnt_a : ca := ca + 1; END_ACTION
  ACTION cnt_b : cb := cb + 1; END_ACTION
  ACTION watch : q := BOOL_TO_USINT(s1.X) + 2 * BOOL_TO_USINT(t1.X); END_ACTION
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

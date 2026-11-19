(* Two CODESYS ways, each an extension (docs/extensions.md). Worked by
   hand:
   input-constant: Ramp's VAR_INPUT CONSTANT k is a plain input, given 3
   by the call and written in the body (k := k + 1, 4): a CONSTANT that
   means something only to CODESYS's CFC editor.
   keyword-names: step, on, r_edge and f_edge are names. step := k, 4;
   on := step + 1, 5; FOR i := 1 TO 10 BY step runs for 1, 5, 9: r_edge
   := r_edge + on three times, 15 (%QB0). f_edge := MAX(step, on), 5
   (%QB1). The SFC beside them keeps STEP as its keyword: Count's chart
   goes s0 -> s1 -> s0, its action adding 1 to n in s1, every second
   tick: after two ticks n is 1 (%QB2).
   x : BOOL R_EDGE keeps R_EDGE as its keyword: TRUE on the first tick
   only, so seen counts 1 (%QB3).
   reference-skip: neither is IEC 61131-3. *)
FUNCTION_BLOCK Ramp
  VAR_INPUT CONSTANT k : INT := 1; END_VAR
  VAR_INPUT x : BOOL R_EDGE; END_VAR
  VAR_OUTPUT r_edge, f_edge, seen : INT; END_VAR
  VAR step, on, i : INT; END_VAR
  k := k + 1;
  step := k;
  on := step + 1;
  r_edge := 0;
  FOR i := 1 TO 10 BY step DO
    r_edge := r_edge + on;
  END_FOR;
  f_edge := MAX(IN1 := step, IN2 := on);
  IF x THEN
    seen := seen + 1;
  END_IF;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Count
  VAR_OUTPUT n : INT; END_VAR
  INITIAL_STEP s0 : END_STEP
  TRANSITION FROM s0 TO s1 := TRUE; END_TRANSITION
  STEP s1 : bump(P); END_STEP
  TRANSITION FROM s1 TO s0 := TRUE; END_TRANSITION
  ACTION bump : n := n + 1; END_ACTION
END_FUNCTION_BLOCK

PROGRAM main
  VAR r : Ramp; c : Count; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
  END_VAR
  r(k := 3, x := TRUE);
  c();
  q0 := INT_TO_SINT(r.r_edge);
  q1 := INT_TO_SINT(r.f_edge);
  q2 := INT_TO_SINT(c.n);
  q3 := INT_TO_SINT(r.seen);
END_PROGRAM

CONFIGURATION cf
  RESOURCE res ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

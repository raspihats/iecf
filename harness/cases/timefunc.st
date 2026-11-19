(* TIME() (the time-function extension): the running task's clock, in
   milliseconds. The task runs every 10 ms, its clock 0, 10, 20, 30 ms on
   ticks 0..3. Worked by hand:
   %QB0: TIME() in the PROGRAM, as ms / 10: 0, 1, 2, 3.
   %QB1: Now10, a FUNCTION reading it too: the same.
   %QB2: Since, a block that keeps its first reading, OSCAT's way of
   measuring (T_PLC_MS): TIME() - start, 0, 10, 20, 30 ms, / 10.
   %QX3.0: a TON of PT T#25ms beside it, started on tick 0: Q from
   tick 3 (30 ms) on, as Since passes 25 ms.
   reference-skip: TIME() is CODESYS's, not IEC 61131-3. *)
FUNCTION Now10 : SINT
  Now10 := DINT_TO_SINT(TIME_TO_DINT(TIME()) / 10);
END_FUNCTION

FUNCTION_BLOCK Since
  VAR_OUTPUT elapsed : TIME; END_VAR
  VAR start : TIME; first : BOOL := TRUE; END_VAR
  IF first THEN
    start := TIME();
    first := FALSE;
  END_IF;
  elapsed := TIME() - start;
END_FUNCTION_BLOCK

PROGRAM main
  VAR s : Since; t : TON; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QX3.0 : BOOL;
  END_VAR
  q0 := DINT_TO_SINT(TIME_TO_DINT(TIME()) / 10);
  q1 := Now10();
  s();
  q2 := DINT_TO_SINT(TIME_TO_DINT(s.elapsed) / 10);
  t(IN := TRUE, PT := T#25ms);
  q3 := t.Q;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

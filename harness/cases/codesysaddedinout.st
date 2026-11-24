(* A derived block that adds an in-out, standing where its base is
   wanted (codesys-oo), as TcOpen's TcoCylinderMoveTask, which adds
   MoveToPositionSignal to TcoTask and is given as `refInverseTask :
   REFERENCE TO TcoTask`. A call through the base binds only the
   base's; the added in-out keeps what the last call bound, and one
   never bound is a NULL fault where the body uses it.
   Worked by hand: tick 1, m1(sig := a) sets a TRUE (from FALSE),
   then r REF= m1, r() through Job: sig still a, set FALSE; runs 2.
   Tick 2: the same, a TRUE then FALSE, runs 4; then go := TRUE.
   Tick 3: as tick 2, runs 6, then r REF= m2, never called: r.Count()
   is a method, 1; r() reaches Mover's body, `sig := NOT sig` on an
   in-out never bound: fault NULL, main line 39.
   reference-skip: not IEC 61131-3 (Ed.3 6.6.8.4 refuses it). *)
FUNCTION_BLOCK Job
  VAR_OUTPUT runs : INT; END_VAR
  METHOD Count : INT
    Count := 1;
  END_METHOD
  runs := runs + 1;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Mover EXTENDS Job
  VAR_IN_OUT sig : BOOL; END_VAR
  SUPER^();
  sig := NOT sig;
END_FUNCTION_BLOCK

PROGRAM main
  VAR m1 : Mover; m2 : Mover; r : REFERENCE TO Job; a : BOOL; go : BOOL; n : INT; END_VAR
  VAR q0 AT %QX0.0 : BOOL; q2 AT %QW2 : INT; q4 AT %QW4 : INT; END_VAR
  m1(sig := a);
  r REF= m1;
  r();
  q0 := a;
  q2 := m1.runs;
  IF go THEN
    r REF= m2;
    q4 := r.Count();
    r();
  END_IF
  go := m1.runs >= 4;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

(* shadowing: a call reaches the function past a local of its name that
   cannot be called, as CODESYS resolves it (found in OSCAT NETWORK's
   DLOG_CRON_TAB). Worked by hand: Twice(21) + LEN(twice) with twice the
   STRING 'x', 42 + 1 = 43 (%QW0); max := MAX(3, 7), 7 (%QW2);
   DAY_OF_WEEK(D#2026-11-19), a Thursday, 4 (%QB4).
   reference-skip: shadowing is CODESYS's, not IEC 61131-3. *)
FUNCTION Twice : INT
VAR_INPUT x : INT; END_VAR
Twice := 2 * x;
END_FUNCTION
PROGRAM main
VAR twice : STRING[10]; max : INT; d : DATE := D#2026-11-19; END_VAR
VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QB4 : USINT; END_VAR
twice := 'x';
max := MAX(3, 7);
q0 := Twice(21) + LEN(twice);
q1 := max;
q2 := DAY_OF_WEEK(d);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

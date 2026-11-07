(* SFC (Ed.3 6.7; docs/behaviour/sfc.md): Prokop's demo chart. Each scan
   runs the actions, those ending first, then the transitions; an action
   runs once more after its step is left (the final scan), so the P
   action EntryCount runs twice per entry, as the standard's
   ACTION_CONTROL and CODESYS have it - P1 would run once. matiec tests
   the transitions first and has no final scan: iecf's trace is
   sfcdemo.iecf.trace. *)
PROGRAM DemoSfc
VAR
    start AT %IX0.0 : BOOL;
    running AT %QX0.0 : BOOL;
    entries AT %QW1 : INT;
END_VAR

INITIAL_STEP Idle:
    IdleMode(N);
END_STEP

STEP Active:
    ActiveMode(N);
    EntryCount(P);
END_STEP

TRANSITION FROM Idle TO Active
    := start;
END_TRANSITION

TRANSITION FROM Active TO Idle
    := NOT start;
END_TRANSITION

ACTION IdleMode:
    running := FALSE;
END_ACTION

ACTION ActiveMode:
    running := TRUE;
END_ACTION

ACTION EntryCount:
    entries := entries + 1;
END_ACTION

END_PROGRAM

CONFIGURATION Config0
    RESOURCE Res0 ON PLC
        TASK task0(INTERVAL := T#10ms, PRIORITY := 0);
        PROGRAM instance0 WITH task0 : DemoSfc;
    END_RESOURCE
END_CONFIGURATION

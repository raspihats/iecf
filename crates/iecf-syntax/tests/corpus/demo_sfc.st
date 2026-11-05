(* demo_sfc.st - textual SFC through the full pipeline, exactly as the
 * graphical path produces it (spec/graphical-languages.md §3): the POU
 * below is byte-for-byte what the Workbench's SFC generator emits for
 * the demo chart in workbench/src/lib/graphical/sfc-generate.test.ts
 * ("demo chart parity" test guards the match - edit either side and
 * that test fails), and the CONFIGURATION mirrors what the server
 * generates from prokop.json. matiec compiles SFC natively; the step
 * semantics exercised by test_pipeline.py are matiec's own.
 *)
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

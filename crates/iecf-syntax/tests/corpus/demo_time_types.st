(* demo_time_types.st - TIME, DATE, TOD and DT as matiec stores them (int64
   seconds + int32 nanoseconds, spec/abi.md §6), arithmetic on them (which
   failed to build until matiec_overrides.h named matiec's own __time_add),
   and RETAIN of a TIME, a STRING and a DATE.

   No variable is called `dt` or `tod`: matiec reserves the type names
   (toolchain/MATIEC.md). *)
PROGRAM main
VAR
    elapsed : TIME := T#1500ms;
    shift_start : TOD := TOD#14:30:15.25;
    stamp : DT := DT#2026-10-04-14:30:15;
    today : DATE := D#2026-10-04;
    half : TIME;
    gap : TIME;
END_VAR
VAR RETAIN
    hold : TIME := T#5s;
    note : STRING := 'kept';
    since : DATE := D#2000-01-01;
    ticks : DINT;
END_VAR
elapsed := elapsed + T#10ms;
shift_start := shift_start + T#1s;
stamp := stamp + T#1s;
half := elapsed / 2;
gap := elapsed - T#500ms;
ticks := ticks + 1;
END_PROGRAM

CONFIGURATION Config0
    RESOURCE Res0 ON PLC
        TASK task0(INTERVAL := T#10ms, PRIORITY := 0);
        PROGRAM instance0 WITH task0 : main;
    END_RESOURCE
END_CONFIGURATION

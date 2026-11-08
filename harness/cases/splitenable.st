(* EN and ENO on a SPLIT (Ed.3 6.6.1.5, Table 36; docs/dependencies.md).
   EN FALSE: IN is not evaluated - i is out of bounds at tick 0 and
   nothing faults - and every output keeps its value, ENO FALSE. EN TRUE:
   the parts are stored, ENO TRUE. A part that does not fit its output
   (1999 into a USINT, at tick 2) is the call's own flag: wrapped, flagged
   and ENO FALSE. iecf only (reference-skip: the reference compiler has no
   SPLIT). *)
PROGRAM main
VAR
  r_year AT %MW0 : UINT := 7;
  r_month AT %MB2 : USINT := 7;
  r_day AT %MB3 : USINT := 7;
  r_ok AT %MX4.0 : BOOL;
  r_wide AT %MX4.1 : BOOL;
  r_small AT %MB5 : USINT := 7;
END_VAR
VAR
  days : ARRAY[1..2] OF DATE := [D#2026-11-08, D#1999-12-31];
  i : INT := 3;
  run : BOOL;
  narrow : BOOL;
END_VAR
SPLIT_DATE(EN := run, IN := days[i], YEAR => r_year, MONTH => r_month, DAY => r_day, ENO => r_ok);
SPLIT_DATE(EN := narrow, IN := days[2], YEAR => r_small, ENO => r_wide);
narrow := run;
run := TRUE;
i := 1;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

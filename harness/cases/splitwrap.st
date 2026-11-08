(* A SPLIT's part converted to an output's type that cannot hold it
   (Ed.3 Table 36; docs/dependencies.md): the year 2026 into a USINT wraps
   to 16#EA and is flagged, as any integer conversion; the month and day
   fit. iecf only (reference-skip: the reference compiler has no SPLIT). *)
PROGRAM main
VAR
  r_year AT %MB0 : USINT;
  r_month AT %MB1 : USINT;
  r_day AT %MB2 : USINT;
END_VAR
SPLIT_DATE(D#2026-11-08, r_year, r_month, r_day);
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

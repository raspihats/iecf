(* A SPLIT's part stored into a subrange (Ed.3 Table 36, 6.4.4.3): the
   month 11 into a USINT (1..6) is clamped to 6 and flagged, as any store
   into a subrange (docs/dependencies.md); the day 8 into an element of an
   array of the subrange, clamped too. iecf only (matiec-skip: matiec has
   no SPLIT). *)
TYPE half : USINT (1..6); END_TYPE
PROGRAM main
VAR
  r_month AT %MB0 : USINT;
  r_day AT %MB1 : USINT;
  r_hour AT %MB2 : USINT;
END_VAR
VAR
  m : half := 1;
  a : ARRAY[1..2] OF half;
  h : USINT (0..23);
END_VAR
SPLIT_DT(IN := DT#2026-11-08-05:00:00, MONTH => m, DAY => a[2], HOUR => h);
r_month := m;
r_day := a[2];
r_hour := h;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

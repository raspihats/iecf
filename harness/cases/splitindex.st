(* A SPLIT whose output is an array element out of bounds (Ed.3 Table 36,
   6.5.2.4): the task faults and nothing of the statement is stored - not
   the year before the bad element in the outputs' order, nor the day
   after it. At tick 0 the index is 2 and all three are stored; at tick 1
   it is 4 and the date another, and the bytes stay as tick 0 left them.
   iecf only (matiec-skip: matiec has no SPLIT). *)
PROGRAM main
VAR
  r_year AT %MW0 : UINT;
  r_day AT %MB2 : USINT;
  r_month AT %MB3 : USINT;
END_VAR
VAR
  i : INT := 1;
  d : DATE := D#2026-11-08;
  a : ARRAY[1..3] OF USINT;
END_VAR
i := i + 1;
SPLIT_DATE(IN := d, YEAR => r_year, MONTH => a[i], DAY => r_day);
r_month := a[2];
i := i + 1;
d := D#2027-12-09;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

(* WHILE loops whose guarded variable the body only steps toward the
   bound (docs/c-output.md §2, crates/iecf-codegen ranges.rs while_facts):
   their indexes proven in range and taken unchecked, the results those of
   the checked C. An insertion sort, j from i - 1 down while j >= 1; a
   search up from 1 while j <= 8, left by EXIT; a sum down from 8 while
   j > 0 by steps of 2. Eight INTs filled from %IB0 each tick:
   a[i] = (seed * (37 i + 11)) MOD 101. Worked by hand, tick 0 (seed 3):
   a = 43 53 63 73 83 93 2 12, sorted 2 12 43 53 63 73 83 93; %QW0
   a[1] * 100 + a[8] = 293 (25 01); %QW2 the first j with a[j] >= 50, 4
   (04 00); %QW4 a[8] + a[6] + a[4] + a[2] = 93 + 73 + 53 + 12 = 231
   (e7 00); %QW6 a[4], 53 (35 00). *)
PROGRAM main
VAR
  a : ARRAY[1..8] OF INT;
  i : INT; j : INT; k : INT; s : INT;
END_VAR
VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; seed AT %IB0 : BYTE; END_VAR
FOR i := 1 TO 8 DO
  a[i] := (BYTE_TO_INT(seed) * (i * 37 + 11)) MOD 101;
END_FOR;
FOR i := 2 TO 8 DO
  k := a[i];
  j := i - 1;
  WHILE j >= 1 DO
    IF a[j] <= k THEN EXIT; END_IF;
    a[j + 1] := a[j];
    j := j - 1;
  END_WHILE;
  a[j + 1] := k;
END_FOR;
q0 := a[1] * 100 + a[8];
j := 1;
WHILE j <= 8 DO
  IF a[j] >= 50 THEN EXIT; END_IF;
  j := j + 1;
END_WHILE;
q1 := j;
s := 0;
j := 8;
WHILE j > 0 DO
  s := s + a[j];
  j := j - 2;
END_WHILE;
q2 := s;
q3 := a[4];
END_PROGRAM
CONFIGURATION plant
  RESOURCE cpu ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

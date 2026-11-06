(* A REAL converted to an integer it does not fit leaves its statement
   undone and is flagged (docs/c-output.md §5, decided 2026-11-06,
   docs/dependencies.md). At tick 1, x is 1.0E10,
   beyond INT: the assignment keeps a's value, the IF and the CASE run no
   branch, the FOR does not run, the WHILE ends, the REPEAT ends after
   its first pass. At tick 2 x is back in range. The first is flagged,
   code 4 at line 29. matiec's C converts out of range, which C leaves
   undefined; its trace is what x86 gives. iecf's is
   unconverted.iecf.trace. *)
PROGRAM main
VAR
  k AT %IW0 : INT;
  a AT %MW0 : INT;
  b AT %MW2 : INT;
  c AT %MW4 : INT;
  n AT %MW6 : INT;
  w AT %MW8 : INT;
  r AT %MW10 : INT;
  ticks AT %MW12 : INT;
END_VAR
VAR
  x : REAL;
  i : INT;
END_VAR
x := 100.4;
IF k >= 1 THEN
  x := 1.0E10;
END_IF;
a := REAL_TO_INT(x);
IF REAL_TO_INT(x) > 50 THEN
  b := b + 1;
ELSE
  b := b + 100;
END_IF;
CASE REAL_TO_INT(x) OF
  100: c := c + 1;
ELSE
  c := c + 100;
END_CASE;
FOR i := 1 TO REAL_TO_INT(x) / 50 DO
  n := n + 1;
END_FOR;
i := 0;
WHILE i < REAL_TO_INT(x) / 50 DO
  i := i + 1;
  w := w + 1;
END_WHILE;
i := 0;
REPEAT
  i := i + 1;
  r := r + 1;
UNTIL i >= REAL_TO_INT(x) / 50
END_REPEAT;
ticks := ticks + 1;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

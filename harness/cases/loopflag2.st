(* As loopflag, the statement never overflowing: x 0, plus 100 on each of
   eight passes, 800 (MD2 20 03 00 00). The FOR's step past 32767 is the
   only flag, at the FOR's line (13): i -32768 (MW0 00 80), n -32768.
   reference-skip: the reference compiler's FOR to INT's last value does
   not end (its counter wraps). *)
PROGRAM main
VAR
  i AT %MW0 : INT;
  x AT %MD2 : DINT;
  n AT %MW6 : INT;
END_VAR
x := 0;
FOR i := 32760 TO 32767 DO
  x := x + 100;
END_FOR;
n := i;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE R ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM P WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

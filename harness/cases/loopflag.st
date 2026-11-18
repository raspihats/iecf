(* One statement's overflow carried across a FOR loop (runtime/iecf.h
   iecf_*_or, crates/iecf-codegen stmt.rs loop_flags): the loop's only
   statement that can flag ORs its overflow into a local, taken after the
   loop with its line; the FOR's own step is taken after that. The order
   is the run's. Worked by hand:
   tick 0: x 2147483000, plus 100 on each of i = 32760..32767, eight
   passes: the seventh leaves DINT (2147483700): wrapped, flagged at
   x := x + 100 (line 23), the first flag. x ends 2147483800 - 2^32 =
   -2147483496 (MD2 98 00 00 80). The FOR's step past 32767 leaves INT
   too, flagged after (not the first): i -32768 (MW0 00 80), n := i
   (MW6 00 80).
   tick 1: the same; the first flag stays.
   reference-skip: the reference compiler's FOR to INT's last value does
   not end (its counter wraps). *)
PROGRAM main
VAR
  i AT %MW0 : INT;
  x AT %MD2 : DINT;
  n AT %MW6 : INT;
END_VAR
x := 2147483000;
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

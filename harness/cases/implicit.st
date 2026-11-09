(* The implicit-externals extension (docs/extensions.md): a PROGRAM, a
   FUNCTION_BLOCK and a FUNCTION use the configuration's globals without
   declaring them VAR_EXTERNAL, a CONSTANT one too; a FUNCTION that
   writes one has effects, so x := total + bump(1) reads total first
   (Ed.3 7.3.2).
   Worked by hand: tick 0, vin 2: qx 0 + 3 = 3 (total read as 0, then
   bump makes it 1 and gives 1 * 3), total 1 + 2 = 3, qs 3, qt 3.
   Tick 1, vin 5: qx 3 + 12 = 15 (total 3, bump makes it 4, 4 * 3),
   total 4 + 5 = 9, qs 9, qt 9.
   reference-skip: an extension, which the reference compiler has not. *)
FUNCTION bump : INT
VAR_INPUT n : INT; END_VAR
total := total + n;
bump := total * k3;
END_FUNCTION

FUNCTION_BLOCK counter
VAR_OUTPUT seen : INT; END_VAR
seen := total;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  vin AT %IW0 : INT;
  qx AT %QW0 : INT;
  qs AT %QW2 : INT;
  qt AT %QW4 : INT;
END_VAR
VAR
  c : counter;
END_VAR
qx := total + bump(1);
total := total + vin;
c();
qs := c.seen;
qt := total;
END_PROGRAM

CONFIGURATION Plant
  VAR_GLOBAL
    total : INT;
  END_VAR
  VAR_GLOBAL CONSTANT
    k3 : INT := 3;
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

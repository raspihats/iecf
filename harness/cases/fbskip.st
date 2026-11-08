(* A function block call with EN FALSE is not made at all (Ed.3 6.6.1.5,
   Example 2; docs/dependencies.md): its inputs are not evaluated - x
   divides by zero and nothing faults - nor set, the block does not run,
   its output is not taken, and NOT ENO gives TRUE. With EN TRUE at tick 1
   (zero is 1 by then) it runs once. The reference compiler cannot compile
   `NOT ENO =>` (PROPOSAL.md §1b): iecf only. *)
FUNCTION_BLOCK acc
VAR_INPUT x : INT; END_VAR
VAR_OUTPUT total : INT; END_VAR
total := total + x;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  r_n AT %MW0 : INT;
  r_total AT %MW2 : INT;
  r_off AT %MX4.0 : BOOL;
END_VAR
VAR
  a : acc;
  run : BOOL;
  zero : INT;
END_VAR
r_n := -1;
a(EN := run, x := 100 / zero, total => r_n, NOT ENO => r_off);
r_total := a.total;
run := TRUE;
zero := 1;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

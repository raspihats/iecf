(* Where iecf differs from matiec on purpose (PROPOSAL.md §6.3): an
   integer overflow is a fault. At tick 2, a + b leaves INT. matiec wraps
   to -32768 and goes on; iecf stops the task at the statement - r_sum
   keeps its value, `after` is not counted - and runs nothing more until
   the next initialisation. Its trace is overflow.iecf.trace.

   MOD by zero is 0 in both: Ed.3 Table 29 e defines it so. *)
PROGRAM main
VAR
  a AT %IW0 : INT;
  b AT %IW2 : INT;
  r_mod AT %MW0 : INT;
  r_sum AT %MW2 : INT;
  after AT %MW4 : INT;
END_VAR
r_mod := a MOD b;
r_sum := a + b;
after := after + 1;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

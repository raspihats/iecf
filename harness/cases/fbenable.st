(* EN and ENO on a function block call (Ed.3 6.6.1.5; docs/dependencies.md):
   an accumulator adds 10000 at each call it is enabled for, %IX0.0 its EN.
   Each tick r_n is set to -1 before the call: with EN FALSE (tick 1) the
   block is not called, its total stays and is not taken into r_n, and ENO
   is FALSE. Its fourth enabled call (tick 4) overflows inside the block:
   the total wraps, the flag is raised in the block's body, and ENO is
   FALSE. r_bits: bit 0 ENO. The reference compiler takes the output into
   r_n with EN FALSE and has no flag to clear ENO: iecf's trace is
   fbenable.iecf.trace. NOT ENO, which the reference compiler cannot
   compile, is fbskip's. *)
FUNCTION_BLOCK acc
VAR_INPUT x : INT; END_VAR
VAR_OUTPUT total : INT; END_VAR
total := total + x;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  c AT %IX0.0 : BOOL;
  r_n AT %MW0 : INT;
  r_total AT %MW2 : INT;
  r_bits AT %MB4 : BYTE;
END_VAR
VAR
  a : acc;
  ok : BOOL;
END_VAR
r_n := -1;
a(EN := c, x := 10000, total => r_n, ENO => ok);
r_total := a.total;
r_bits := 0;
IF ok THEN r_bits := r_bits OR 16#01; END_IF;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

(* Where iecf differs from the reference compiler on purpose (PROPOSAL.md §6.3): an
   integer overflow is flagged. At tick 2, a + b leaves INT. Both wrap to
   -1536 and go on, as CODESYS, TwinCAT, Siemens and Rockwell do; iecf
   also records the first overflow, its POU and line, in ctx->flag, which
   stays until the platform clears it. Its trace is overflow.iecf.trace:
   the reference compiler's bytes, with "flag 2 MAIN 18" from tick 2.

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

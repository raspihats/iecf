(* A narrowing conversion out of the target's range keeps the low bits,
   as CODESYS and TwinCAT have it (INT_TO_SINT(4223) = 127), and the reference compiler's
   C cast does; iecf also flags it (PROPOSAL.md §6.3). At tick 1, 4223
   leaves SINT: both give 127, and iecf records "flag 4 MAIN 18" in
   ctx->flag. At tick 2, -1 leaves UINT: 65535. Its trace is
   narrow.iecf.trace: the reference compiler's bytes, with the flag from tick 1. *)
PROGRAM main
VAR
  a AT %IW0 : INT;
  s AT %MB0 : SINT;
  u AT %MW2 : UINT;
END_VAR
VAR
  d : DINT;
END_VAR
d := INT_TO_DINT(a);
u := DINT_TO_UINT(d);
s := INT_TO_SINT(a);
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

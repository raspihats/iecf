(* VAR_TEMP (Ed.3 6.5.2.2): set to its initial value at each call of its
   PROGRAM or FUNCTION_BLOCK, where VAR keeps its value. matiec reads
   Ed.2, where VAR_TEMP takes no initial value: each starts at 0. An
   array in VAR_TEMP is left out: matiec's C for it does not compile
   (PROPOSAL.md §1b). *)
FUNCTION_BLOCK acc
VAR_INPUT x : INT; END_VAR
VAR_OUTPUT kept, fresh : INT; END_VAR
VAR total : INT; END_VAR
VAR_TEMP part : INT; END_VAR
total := total + x;
part := part + x;
kept := total;
fresh := part;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  k AT %IW0 : INT;
  r_kept AT %MW0 : INT;
  r_fresh AT %MW2 : INT;
  r_tmp AT %MW4 : INT;
END_VAR
VAR
  a : acc;
END_VAR
VAR_TEMP
  tmp : INT;
  flag : BOOL;
END_VAR
a(x := k);
tmp := tmp + k;
flag := NOT flag;
r_kept := a.kept;
r_fresh := a.fresh;
IF flag THEN r_tmp := tmp; END_IF;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

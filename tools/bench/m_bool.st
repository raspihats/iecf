(* iecf's benchmark (docs/testing/performance.md): BOOL logic and IF.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)

PROGRAM main
  VAR i : DINT; b : BOOL; c : BOOL; n : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    b := (i MOD 3 = 0) XOR c; IF b AND NOT c THEN n := n + 1; END_IF; c := b OR (i MOD 5 = 1);
  END_FOR;
  o1 := n;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

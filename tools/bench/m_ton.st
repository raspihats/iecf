(* iecf's benchmark (docs/testing/performance.md): a TON called.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)

PROGRAM main
  VAR i : DINT; t : TON; n : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    t(IN := i MOD 2 = 0, PT := T#5ms); IF t.Q THEN n := n + 1; END_IF;
  END_FOR;
  o1 := n;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

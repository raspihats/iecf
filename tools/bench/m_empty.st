(* iecf's benchmark (docs/testing/performance.md): one statement: the cost of a cycle itself.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
PROGRAM main
  VAR n : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  n := n + 1;
  o1 := n;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

(* iecf's benchmark (docs/testing/performance.md): a DINT sum.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)

PROGRAM main
  VAR i : DINT; acc : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    acc := acc + i MOD 7;
  END_FOR;
  o1 := acc;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

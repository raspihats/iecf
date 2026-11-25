(* iecf's benchmark (docs/testing/performance.md): an LREAL sum.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)

PROGRAM main
  VAR i : DINT; x : LREAL; s : LREAL; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    x := DINT_TO_LREAL(i); s := s + x;
  END_FOR;
  o1 := LREAL_TO_DINT(s / 1000.0);
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

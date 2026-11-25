(* iecf's benchmark (docs/testing/performance.md): a FUNCTION call.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
FUNCTION Add3 : DINT
  VAR_INPUT a : DINT; b : DINT; c : DINT; END_VAR
  Add3 := a + b + c;
END_FUNCTION
PROGRAM main
  VAR i : DINT; n : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    n := Add3(a := n, b := i, c := 1) MOD 100000;
  END_FOR;
  o1 := n;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

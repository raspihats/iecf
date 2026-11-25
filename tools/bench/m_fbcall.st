(* iecf's benchmark (docs/testing/performance.md): a FUNCTION_BLOCK call.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
FUNCTION_BLOCK Acc
  VAR_INPUT x : DINT; k : DINT; END_VAR
  VAR_OUTPUT y : DINT; END_VAR
  y := (y + x * k) MOD 100000;
END_FUNCTION_BLOCK
PROGRAM main
  VAR i : DINT; f : Acc; n : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    f(x := i MOD 9, k := 2); n := f.y;
  END_FOR;
  o1 := n;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

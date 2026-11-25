(* iecf's benchmark (docs/testing/performance.md): a DINT array element updated.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)

PROGRAM main
  VAR i : DINT; a : ARRAY[0..63] OF DINT; k : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    k := i MOD 64; a[k] := a[k] + i MOD 5;
  END_FOR;
  o1 := a[3];
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

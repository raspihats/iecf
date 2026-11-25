(* iecf's benchmark (docs/testing/performance.md): one statement each: an LREAL product of a DINT, an LREAL sum.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
PROGRAM main
  VAR i : DINT; k : INT; x : LREAL; s : LREAL; r : REAL; arr : ARRAY[1..64] OF REAL; END_VAR
  VAR o1 AT %QD0 : DINT; o2 AT %QD4 : DINT; o3 AT %QD8 : DINT; END_VAR
  FOR i := 1 TO 5000 DO
    x := DINT_TO_LREAL(i) * 0.001;
    s := s + x;
  END_FOR;
  IF s > 1.0E9 THEN s := 0.0; END_IF;
  o1 := LREAL_TO_DINT(s);
  o2 := REAL_TO_DINT(r);
  o3 := REAL_TO_DINT(arr[7] * 100.0);
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

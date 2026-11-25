(* iecf's benchmark (docs/testing/performance.md): integer and LREAL arithmetic in a loop: DINT products and MOD, ROL and XOR of a DWORD, SQRT.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
PROGRAM main
  VAR i : DINT; acc : DINT; w : DWORD := 16#1234567; x : LREAL; END_VAR
  VAR o1 AT %QD0 : DINT; o2 AT %QD4 : DINT; o3 AT %QD8 : DWORD; END_VAR
  FOR i := 1 TO 20000 DO
    acc := (acc * 31 + i) MOD 1000003;
    w := ROL(w, 3) XOR DINT_TO_DWORD(i);
    x := x + SQRT(DINT_TO_LREAL(i)) * 0.5;
  END_FOR;
  IF x > 1.0E9 THEN x := 0.0; END_IF;
  o1 := acc;
  o2 := LREAL_TO_DINT(x);
  o3 := w;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

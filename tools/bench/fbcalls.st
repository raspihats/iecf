(* iecf's benchmark (docs/testing/performance.md): 10 instances of a FUNCTION_BLOCK and a FUNCTION called 50 times a cycle, a CTU.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
FUNCTION_BLOCK Filter
  VAR_INPUT x : DINT; k : DINT; END_VAR
  VAR_OUTPUT y : DINT; END_VAR
  VAR state : DINT; n : DINT; END_VAR
  state := state + (x - state) / k;
  n := n + 1;
  y := state + n MOD 7;
END_FUNCTION_BLOCK
FUNCTION Mix : DINT
  VAR_INPUT a : DINT; b : DINT; END_VAR
  Mix := (a * 3 + b) MOD 100003;
END_FUNCTION
PROGRAM main
  VAR f1 : Filter; f2 : Filter; f3 : Filter; f4 : Filter; f5 : Filter; f6 : Filter; f7 : Filter; f8 : Filter; f9 : Filter; f10 : Filter; c : CTU; i : INT; rep : INT; acc : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; o2 AT %QW4 : INT; END_VAR
  FOR rep := 1 TO 50 DO
    FOR i := 1 TO 10 DO
      f1(x := acc + 1, k := 4);
      acc := Mix(a := acc, b := f1.y);
      f2(x := acc + 2, k := 4);
      acc := Mix(a := acc, b := f2.y);
      f3(x := acc + 3, k := 4);
      acc := Mix(a := acc, b := f3.y);
      f4(x := acc + 4, k := 4);
      acc := Mix(a := acc, b := f4.y);
      f5(x := acc + 5, k := 4);
      acc := Mix(a := acc, b := f5.y);
      f6(x := acc + 6, k := 4);
      acc := Mix(a := acc, b := f6.y);
      f7(x := acc + 7, k := 4);
      acc := Mix(a := acc, b := f7.y);
      f8(x := acc + 8, k := 4);
      acc := Mix(a := acc, b := f8.y);
      f9(x := acc + 9, k := 4);
      acc := Mix(a := acc, b := f9.y);
      f10(x := acc + 10, k := 4);
      acc := Mix(a := acc, b := f10.y);
    END_FOR;
    c(CU := NOT c.CU, PV := 30000);
  END_FOR;
  IF c.Q THEN c(R := TRUE); c(R := FALSE); END_IF;
  o1 := acc;
  o2 := c.CV;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

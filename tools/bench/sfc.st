(* iecf's benchmark (docs/testing/performance.md): 40 instances of a FUNCTION_BLOCK whose body is a four-step SFC chart of N actions.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
FUNCTION_BLOCK Machine
  VAR_INPUT seed : DINT; END_VAR
  VAR_OUTPUT work : DINT; cycles : DINT; END_VAR
  VAR n : DINT; acc : DINT; END_VAR
  INITIAL_STEP idle : start(N); END_STEP
  TRANSITION FROM idle TO fill := n >= 2; END_TRANSITION
  STEP fill : filling(N); END_STEP
  TRANSITION FROM fill TO heat := n >= 5; END_TRANSITION
  STEP heat : heating(N); END_STEP
  TRANSITION FROM heat TO drain := n >= 9; END_TRANSITION
  STEP drain : draining(N); END_STEP
  TRANSITION FROM drain TO idle := n >= 12; END_TRANSITION
  ACTION start :
    n := n + 1;
    acc := (acc * 7 + seed) MOD 100003;
    work := acc;
  END_ACTION
  ACTION filling :
    n := n + 1;
    acc := (acc + seed * 3) MOD 100003;
    work := acc;
  END_ACTION
  ACTION heating :
    n := n + 1;
    acc := (acc * 3 + 11) MOD 100003;
    work := acc;
  END_ACTION
  ACTION draining :
    n := n + 1;
    IF n >= 12 THEN n := 0; cycles := cycles + 1; END_IF;
    acc := (acc + 5) MOD 100003;
    work := acc;
  END_ACTION
END_FUNCTION_BLOCK
PROGRAM main
  VAR m1 : Machine; m2 : Machine; m3 : Machine; m4 : Machine; m5 : Machine; m6 : Machine; m7 : Machine; m8 : Machine; m9 : Machine; m10 : Machine; m11 : Machine; m12 : Machine; m13 : Machine; m14 : Machine; m15 : Machine; m16 : Machine; m17 : Machine; m18 : Machine; m19 : Machine; m20 : Machine; m21 : Machine; m22 : Machine; m23 : Machine; m24 : Machine; m25 : Machine; m26 : Machine; m27 : Machine; m28 : Machine; m29 : Machine; m30 : Machine; m31 : Machine; m32 : Machine; m33 : Machine; m34 : Machine; m35 : Machine; m36 : Machine; m37 : Machine; m38 : Machine; m39 : Machine; m40 : Machine; total : DINT; c : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; o2 AT %QD4 : DINT; END_VAR
  total := 0; c := 0;
  m1(seed := 1); total := (total + m1.work) MOD 1000003; c := c + m1.cycles;
  m2(seed := 2); total := (total + m2.work) MOD 1000003; c := c + m2.cycles;
  m3(seed := 3); total := (total + m3.work) MOD 1000003; c := c + m3.cycles;
  m4(seed := 4); total := (total + m4.work) MOD 1000003; c := c + m4.cycles;
  m5(seed := 5); total := (total + m5.work) MOD 1000003; c := c + m5.cycles;
  m6(seed := 6); total := (total + m6.work) MOD 1000003; c := c + m6.cycles;
  m7(seed := 7); total := (total + m7.work) MOD 1000003; c := c + m7.cycles;
  m8(seed := 8); total := (total + m8.work) MOD 1000003; c := c + m8.cycles;
  m9(seed := 9); total := (total + m9.work) MOD 1000003; c := c + m9.cycles;
  m10(seed := 10); total := (total + m10.work) MOD 1000003; c := c + m10.cycles;
  m11(seed := 11); total := (total + m11.work) MOD 1000003; c := c + m11.cycles;
  m12(seed := 12); total := (total + m12.work) MOD 1000003; c := c + m12.cycles;
  m13(seed := 13); total := (total + m13.work) MOD 1000003; c := c + m13.cycles;
  m14(seed := 14); total := (total + m14.work) MOD 1000003; c := c + m14.cycles;
  m15(seed := 15); total := (total + m15.work) MOD 1000003; c := c + m15.cycles;
  m16(seed := 16); total := (total + m16.work) MOD 1000003; c := c + m16.cycles;
  m17(seed := 17); total := (total + m17.work) MOD 1000003; c := c + m17.cycles;
  m18(seed := 18); total := (total + m18.work) MOD 1000003; c := c + m18.cycles;
  m19(seed := 19); total := (total + m19.work) MOD 1000003; c := c + m19.cycles;
  m20(seed := 20); total := (total + m20.work) MOD 1000003; c := c + m20.cycles;
  m21(seed := 21); total := (total + m21.work) MOD 1000003; c := c + m21.cycles;
  m22(seed := 22); total := (total + m22.work) MOD 1000003; c := c + m22.cycles;
  m23(seed := 23); total := (total + m23.work) MOD 1000003; c := c + m23.cycles;
  m24(seed := 24); total := (total + m24.work) MOD 1000003; c := c + m24.cycles;
  m25(seed := 25); total := (total + m25.work) MOD 1000003; c := c + m25.cycles;
  m26(seed := 26); total := (total + m26.work) MOD 1000003; c := c + m26.cycles;
  m27(seed := 27); total := (total + m27.work) MOD 1000003; c := c + m27.cycles;
  m28(seed := 28); total := (total + m28.work) MOD 1000003; c := c + m28.cycles;
  m29(seed := 29); total := (total + m29.work) MOD 1000003; c := c + m29.cycles;
  m30(seed := 30); total := (total + m30.work) MOD 1000003; c := c + m30.cycles;
  m31(seed := 31); total := (total + m31.work) MOD 1000003; c := c + m31.cycles;
  m32(seed := 32); total := (total + m32.work) MOD 1000003; c := c + m32.cycles;
  m33(seed := 33); total := (total + m33.work) MOD 1000003; c := c + m33.cycles;
  m34(seed := 34); total := (total + m34.work) MOD 1000003; c := c + m34.cycles;
  m35(seed := 35); total := (total + m35.work) MOD 1000003; c := c + m35.cycles;
  m36(seed := 36); total := (total + m36.work) MOD 1000003; c := c + m36.cycles;
  m37(seed := 37); total := (total + m37.work) MOD 1000003; c := c + m37.cycles;
  m38(seed := 38); total := (total + m38.work) MOD 1000003; c := c + m38.cycles;
  m39(seed := 39); total := (total + m39.work) MOD 1000003; c := c + m39.cycles;
  m40(seed := 40); total := (total + m40.work) MOD 1000003; c := c + m40.cycles;
  o1 := total;
  o2 := c;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

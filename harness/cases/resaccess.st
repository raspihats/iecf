(* A global named through its resource (Ed.3 6.8.2, Global_Var_Access):
   `R.g` and `R.g.x` in a program's connections - an input, an output,
   an in-out - and in a TASK's INTERVAL, beside the plain names. r1's
   task runs every 20 ms, on ticks 0 and 2; r2's every tick, after it.
   Worked by hand:

   tick 0, r1.m: a 10, b p.y 4: o 14 into res (Q0 0e 00), o2 6 into
   p.x, cnt 1. r1.w: a p.x 6, b cnt 1: o 7 (Q2 07 00); its in-out is the
   configuration's g, 2. r2.m: a its own g1 20, b g 2: o 22 (Q4 16 00),
   o2 18 (Q6 12 00).
   tick 1: r2.m alone, the same.
   tick 2, r1.m: 14, 6, cnt 2. r1.w: 6 + 2 = 8 (08 00), g 3. r2.m: 20 +
   3 = 23 (17 00), 17 (11 00).
   reference-skip: program connections, as `connections`. *)
TYPE Pt : STRUCT x : INT; y : INT; END_STRUCT; END_TYPE
PROGRAM main
  VAR_INPUT a : INT; b : INT; END_VAR
  VAR_OUTPUT o : INT; o2 : INT; END_VAR
  VAR_IN_OUT n : INT; END_VAR
  o := a + b;
  o2 := a - b;
  n := n + 1;
END_PROGRAM
CONFIGURATION plant
  VAR_GLOBAL g : INT := 1; END_VAR
  RESOURCE r1 ON PLC
    VAR_GLOBAL g1 : INT := 10; p : Pt := (x := 3, y := 4); cnt : INT; iv : TIME := T#20ms; END_VAR
    VAR_GLOBAL res AT %QW0 : INT; res2 AT %QW2 : INT; END_VAR
    TASK t(INTERVAL := r1.iv, PRIORITY := 1);
    PROGRAM m WITH t : main(a := r1.g1, b := r1.p.y, o => r1.res, o2 => r1.p.x, n := r1.cnt);
    PROGRAM w WITH t : main(a := r1.p.x, b := r1.cnt, o => res2, n := g);
  END_RESOURCE
  RESOURCE r2 ON PLC
    VAR_GLOBAL g1 : INT := 20; cnt : INT; END_VAR
    VAR_GLOBAL res AT %QW4 : INT; out2 AT %QW6 : INT; END_VAR
    TASK t(INTERVAL := T#10ms, PRIORITY := 2);
    PROGRAM m WITH t : main(a := r2.g1, b := g, o => r2.res, o2 => out2, n := r2.cnt);
  END_RESOURCE
END_CONFIGURATION

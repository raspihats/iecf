(* Initial values that are standard functions of constants (Ed.3
   6.6.1.2.4): worked out by the compiler where the result is exact,
   and the mathematics of Table 28 and ** computed as the variable is
   set up, by the target's own, once the compiler has worked out that
   the result is one the type holds - in a program, a TYPE, a
   structure's field, an array, a global, a block's input and a
   FUNCTION's input and own variable.
   k -3, kr 1.5, kw 16#8001, kt T#2s, ks SQRT(2.0). Worked by hand, the
   same at every tick:
   ABS(k): 3 (%QB0). MAX(k, 2): 2 (%QB1). LIMIT(0, k, 10): 0 (%QB2).
   SEL(k < 0, 1, 2): 2 (%QB3). MUX(1, 5, 6): 6 (%QB4).
   REAL_TO_INT(kr), 1.5 to the even one: 2 (%QB5); REAL_TO_INT(2.5): 2
   (%QB6); TRUNC_INT(kr): 1 (%QB7).
   SHL(kw, 1): 16#0002, 2 (%QB8). ROL(kw, 1): 16#0003, 3 (%QB9).
   ROR(kw, 17), by 1: 16#C000; shifted right by 12, 12 (%QB10).
   kt * 1.5 is T#3s: 1 (%QB11). kt / 3 is T#666ms: 1 (%QB12).
   kr ** 2, 2.25, times 4.0: 9 (%QB13). SQRT(2.0) times 10.0: 14
   (%QB14). 1.0 / 3.0 times 9.0: 3 (%QB15). ks * ks, 1.9999999, times
   50.0: 100 (%QB16). SQRT(2.0) > 1.0: TRUE (%QX17.0). SEL(ks > 1.0, 1,
   2): 2 (%QB18). lr : LREAL := EXP(LN(2.0)), times 50.0: 100 (%QB19).
   arr := [SQRT(4.0), SQRT(9.0)]: arr[1] + arr[2] is 5 (%QB20).
   The structure PT: x := ABS(-4), 4 (%QB21); y := SQRT(16.0), 4
   (%QB22). The TYPE RT : REAL := SQRT(9.0): 3 (%QB23).
   The block F: inp := fk ** 2, fk 3.0: 9 (%QB24). The FUNCTION G: a :=
   SQRT(4.0) and loc := EXPT(2.0, 3): G() is 10 (%QB25), G(a := 1.0) 9
   (%QB26). The global gx := SQRT(INT_TO_REAL(g + 9)), g 7: 4 (%QB27).
   cv : DINT := INT_TO_DINT(k) * 1000, -3000, over 100: -30 (%QB28).
   u : USINT := INT_TO_USINT(ABS(k)): 3 (%QB29).
   reference-skip: the reference compiler takes no constant in an
   initial value (as constinit). *)
TYPE
  PT : STRUCT x : INT := ABS(-4); y : REAL := SQRT(16.0); END_STRUCT;
  RT : REAL := SQRT(9.0);
END_TYPE
FUNCTION G : INT
  VAR_INPUT a : REAL := SQRT(4.0); END_VAR
  VAR loc : REAL := EXPT(2.0, 3); END_VAR
  G := REAL_TO_INT(a + loc);
END_FUNCTION
FUNCTION_BLOCK F
  VAR CONSTANT fk : REAL := 3.0; END_VAR
  VAR_INPUT inp : REAL := fk ** 2; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  o := REAL_TO_INT(inp);
END_FUNCTION_BLOCK
CONFIGURATION c
  VAR_GLOBAL CONSTANT g : INT := 7; END_VAR
  VAR_GLOBAL gx : REAL := SQRT(INT_TO_REAL(g + 9)); END_VAR
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR CONSTANT
    k : INT := -3;
    kr : REAL := 1.5;
    kw : WORD := 16#8001;
    kt : TIME := T#2s;
    ks : REAL := SQRT(2.0);
  END_VAR
  VAR_EXTERNAL gx : REAL; END_VAR
  VAR
    a : INT := ABS(k);
    b : INT := MAX(k, 2);
    c : INT := LIMIT(0, k, 10);
    d : INT := SEL(k < 0, 1, 2);
    e : INT := MUX(1, 5, 6);
    f1 : INT := REAL_TO_INT(kr);
    f2 : INT := REAL_TO_INT(2.5);
    f3 : INT := TRUNC_INT(kr);
    w1 : WORD := SHL(kw, 1);
    w2 : WORD := ROL(kw, 1);
    w3 : WORD := ROR(kw, 17);
    t1 : TIME := kt * 1.5;
    t2 : TIME := kt / 3;
    r1 : REAL := kr ** 2;
    r2 : REAL := SQRT(2.0);
    r3 : REAL := 1.0 / 3.0;
    r4 : REAL := ks * ks;
    bb : BOOL := SQRT(2.0) > 1.0;
    s1 : INT := SEL(ks > 1.0, 1, 2);
    lr : LREAL := EXP(LN(2.0));
    arr : ARRAY [1..2] OF REAL := [SQRT(4.0), SQRT(9.0)];
    p : PT;
    rt : RT;
    fi : F;
    cv : DINT := INT_TO_DINT(k) * 1000;
    u : USINT := INT_TO_USINT(ABS(k));
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    q6 AT %QB6 : SINT;
    q7 AT %QB7 : SINT;
    q8 AT %QB8 : SINT;
    q9 AT %QB9 : SINT;
    q10 AT %QB10 : SINT;
    q11 AT %QB11 : SINT;
    q12 AT %QB12 : SINT;
    q13 AT %QB13 : SINT;
    q14 AT %QB14 : SINT;
    q15 AT %QB15 : SINT;
    q16 AT %QB16 : SINT;
    q17 AT %QX17.0 : BOOL;
    q18 AT %QB18 : SINT;
    q19 AT %QB19 : SINT;
    q20 AT %QB20 : SINT;
    q21 AT %QB21 : SINT;
    q22 AT %QB22 : SINT;
    q23 AT %QB23 : SINT;
    q24 AT %QB24 : SINT;
    q25 AT %QB25 : SINT;
    q26 AT %QB26 : SINT;
    q27 AT %QB27 : SINT;
    q28 AT %QB28 : SINT;
    q29 AT %QB29 : SINT;
  END_VAR
  q0 := INT_TO_SINT(a);
  q1 := INT_TO_SINT(b);
  q2 := INT_TO_SINT(c);
  q3 := INT_TO_SINT(d);
  q4 := INT_TO_SINT(e);
  q5 := INT_TO_SINT(f1);
  q6 := INT_TO_SINT(f2);
  q7 := INT_TO_SINT(f3);
  q8 := WORD_TO_SINT(w1);
  q9 := WORD_TO_SINT(w2);
  q10 := WORD_TO_SINT(SHR(w3, 12));
  IF t1 = T#3s THEN
    q11 := 1;
  END_IF;
  IF t2 = T#666ms THEN
    q12 := 1;
  END_IF;
  q13 := REAL_TO_SINT(r1 * 4.0);
  q14 := REAL_TO_SINT(r2 * 10.0);
  q15 := REAL_TO_SINT(r3 * 9.0);
  q16 := REAL_TO_SINT(r4 * 50.0);
  q17 := bb;
  q18 := INT_TO_SINT(s1);
  q19 := LREAL_TO_SINT(lr * 50.0);
  q20 := REAL_TO_SINT(arr[1] + arr[2]);
  q21 := INT_TO_SINT(p.x);
  q22 := REAL_TO_SINT(p.y);
  q23 := REAL_TO_SINT(rt);
  fi();
  q24 := INT_TO_SINT(fi.o);
  q25 := INT_TO_SINT(G());
  q26 := INT_TO_SINT(G(a := 1.0));
  q27 := REAL_TO_SINT(gx);
  q28 := DINT_TO_SINT(cv / 100);
  q29 := USINT_TO_SINT(u);
END_PROGRAM

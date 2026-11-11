(* Initial values that are constant expressions (Ed.3 6.6.1.2.4,
   Figure 6): literals, CONSTANT variables and operators, worked out by
   the compiler - an integer one exactly, as the checker does it, a REAL
   one in single precision - for a variable, an array, a structure, a
   string, a block's and a FUNCTION's inputs and own variables.
   k 3, kr 1.5, kt T#2s, ks 'ab', kb TRUE; k2 := k + 1, 4; k3 := k2 * k,
   12; g, a global constant, 7. Worked by hand, the same at every tick:
   a := k * 2 + 1: 7 (%QB0). r := kr * 2.0: 3.0 (%QB1). r2 := kr / 4.0,
   0.375, times 8.0: 3 (%QB2). t := kt + T#500ms is T#2s500ms: 1 (%QB3).
   s := ks: LEN 2 (%QB4). b := NOT kb OR k > 2: TRUE (%QX5.0).
   arr : ARRAY [1..k] := [k, 2(k3)]: arr[3] 12 (%QB6). p := (x := k2, y
   := kr): p.x 4 (%QB7).
   The block F: inp := fk + g, its constant 10 and the global 7: 17
   (%QB8). The FUNCTION G: a := c * 2 and loc := c + 1, c 5: G() 10 + 6,
   16 (%QB9); G(a := 1) 7 (%QB10).
   lvl : INT (0..10) := k: 3 (%QB11). m : MODE := run (%QB12 1).
   n := -k: -3 (%QB13). lr : LREAL := kr, times 2.0: 3 (%QB14).
   d : DINT := k3 * 10000, exactly 120000 though k3 is an INT: d / 10000
   is 12 (%QB15).
   A TYPE and a VAR_GLOBAL see the global constants directly: U := g +
   1, 8 (%QB16), though the block H has a constant g of its own, 1
   (%QB17); gx := g * 2: 14 (%QB18). A block's constant in its method:
   get's input a := mc, 4, and its x := mc + 1, 5: 9 (%QB19); in a
   block derived from it, v := mc * 3: 12 (%QB20).
   A constant of an enumeration, m2 := km, km run: 1 (%QB21). Bit
   strings: bb := 16#0F AND 16#3C, 16#0C: 12 (%QB22); bx := kx XOR
   16#05, kx 16#0F: 10 (%QB23). A named value, nv := hi + 1: 6 (%QB24).
   reference-skip: the reference compiler takes no constant in an
   initial value or a bound (as flexarrays). *)
TYPE
  PT : STRUCT x : INT; y : REAL; END_STRUCT;
  MODE : (idle, run);
  U : INT := g + 1;
  LV : INT (lo := 1, hi := 5);
END_TYPE
FUNCTION_BLOCK H
  VAR CONSTANT g : INT := 1; END_VAR
  VAR_OUTPUT u : U; own : INT := g; END_VAR
END_FUNCTION_BLOCK
FUNCTION_BLOCK M
  VAR CONSTANT mc : INT := 4; END_VAR
  METHOD PUBLIC get : INT
    VAR_INPUT a : INT := mc; END_VAR
    VAR x : INT := mc + 1; END_VAR
    get := a + x;
  END_METHOD
END_FUNCTION_BLOCK
FUNCTION_BLOCK D EXTENDS M
  VAR_OUTPUT v : INT := mc * 3; END_VAR
END_FUNCTION_BLOCK
FUNCTION_BLOCK F
  VAR CONSTANT fk : INT := 10; END_VAR
  VAR_EXTERNAL CONSTANT g : INT; END_VAR
  VAR_INPUT inp : INT := fk + g; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  o := inp;
END_FUNCTION_BLOCK
FUNCTION G : INT
  VAR CONSTANT c : INT := 5; END_VAR
  VAR_INPUT a : INT := c * 2; END_VAR
  VAR loc : INT := c + 1; END_VAR
  G := a + loc;
END_FUNCTION
CONFIGURATION c
  VAR_GLOBAL CONSTANT g : INT := 7; END_VAR
  VAR_GLOBAL gx : INT := g * 2; END_VAR
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR CONSTANT
    k : INT := 3;
    kr : REAL := 1.5;
    kt : TIME := T#2s;
    ks : STRING := 'ab';
    kb : BOOL := TRUE;
    k2 : INT := k + 1;
    k3 : INT := k2 * k;
    km : MODE := run;
    kx : BYTE := 16#0F;
  END_VAR
  VAR_EXTERNAL gx : INT; END_VAR
  VAR
    a : INT := k * 2 + 1;
    r : REAL := kr * 2.0;
    r2 : REAL := kr / 4.0;
    t : TIME := kt + T#500ms;
    s : STRING := ks;
    b : BOOL := NOT kb OR k > 2;
    arr : ARRAY [1..k] OF INT := [k, 2(k3)];
    p : PT := (x := k2, y := kr);
    f : F;
    lvl : INT (0..10) := k;
    m : MODE := run;
    n : INT := -k;
    lr : LREAL := kr;
    d : DINT := k3 * 10000;
    h : H;
    mi : M;
    di : D;
    m2 : MODE := km;
    bb : BYTE := 16#0F AND 16#3C;
    bx : BYTE := kx XOR 16#05;
    nv : INT := hi + 1;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QX5.0 : BOOL;
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
    q17 AT %QB17 : SINT;
    q18 AT %QB18 : SINT;
    q19 AT %QB19 : SINT;
    q20 AT %QB20 : SINT;
    q21 AT %QB21 : SINT;
    q22 AT %QB22 : SINT;
    q23 AT %QB23 : SINT;
    q24 AT %QB24 : SINT;
  END_VAR
  q0 := INT_TO_SINT(a);
  q1 := REAL_TO_SINT(r);
  q2 := REAL_TO_SINT(r2 * 8.0);
  IF t = T#2s500ms THEN
    q3 := 1;
  END_IF;
  q4 := INT_TO_SINT(LEN(s));
  q5 := b;
  q6 := INT_TO_SINT(arr[3]);
  q7 := INT_TO_SINT(p.x);
  f();
  q8 := INT_TO_SINT(f.o);
  q9 := INT_TO_SINT(G());
  q10 := INT_TO_SINT(G(a := 1));
  q11 := INT_TO_SINT(lvl);
  IF m = run THEN
    q12 := 1;
  END_IF;
  q13 := INT_TO_SINT(n);
  q14 := LREAL_TO_SINT(lr * 2.0);
  q15 := DINT_TO_SINT(d / 10000);
  q16 := INT_TO_SINT(h.u);
  q17 := INT_TO_SINT(h.own);
  q18 := INT_TO_SINT(gx);
  q19 := INT_TO_SINT(mi.get());
  q20 := INT_TO_SINT(di.v);
  IF m2 = run THEN
    q21 := 1;
  END_IF;
  q22 := BYTE_TO_SINT(bb);
  q23 := BYTE_TO_SINT(bx);
  q24 := INT_TO_SINT(nv);
END_PROGRAM

(* A function block's in-out given, and its output taken into, any
   variable of the type (Ed.3 6.6.3.4): an element by a constant and by
   a computed index, a field, a variable through a reference, an
   element at an address. And an enumeration declared in place with
   named values (Ed.3 6.4.4.3), used as its base type.
   ADD1 adds 1 to its in-out v; twice := 2 * v; neg := v < 0.
   Worked by hand, k %IB0 1, 2, 3; arr and out start [0, 0, 0]:
   b(v := arr[2], twice => out[1]); i := k; b(v := arr[i], twice =>
   out[i]); and, last in the scan, b(v := arr[k + 1]):
     tick 0: arr[2] 1, out[1] 2; i 1: arr[1] 1, out[1] 2; arr [1, 1,
     0], out [2, 0, 0]; then arr[2] 2.
     tick 1: arr[2] 3, out[1] 6; i 2: arr[2] 4, out[2] 8; arr [1, 4,
     0], out [6, 8, 0]; then arr[3] 1.
     tick 2: arr[2] 5, out[1] 10; i 3: arr[3] 2, out[3] 4; arr [1, 5,
     2], out [10, 8, 4]; then arr[4], outside 1..3: a fault (index),
     before the block runs.
     %QB0..2 arr, %QB3..5 out.
   b(v := p.y, twice => po.x): p.y 1, 2, 3 (%QB6); po.x 2, 4, 6 (%QB7).
   b(v := r^, twice => ro^), r to tgt, ro to d2: tgt + 1, d2 twice it.
   b(v := tgt, twice => d), d a DINT, the output widened: tgt + 1
   again: tgt 2, 4, 6 (%QB8); d2 2, 6, 10 (%QB9); d 4, 8, 12 (%QB10).
   b(v := p.x, NOT neg => nb): nb TRUE (%QX11.0).
   b(v := lm[2], twice => lm[1]), lm at %MW0: lm[2] 1, 2, 3 and lm[1]
   2, 4, 6 in %MW2 and %MW0.
   x : INT (lo := 1, hi := 5) := hi, in place: x + lo 6, then x := x + 1:
   6, 7, 8 (%QB12). y : (p3 := 3, q4): 3, then y := q4: 3, 4, 4 (%QB13).
   SPARE is a block with an instance that is never called: the C still
   builds with every warning an error, as the harness builds it.
   reference-skip: named values are Ed.3's; the reference compiler
   (Ed.2) has none. *)
TYPE PT : STRUCT x, y : INT; END_STRUCT; END_TYPE
FUNCTION_BLOCK ADD1
  VAR_IN_OUT v : INT; END_VAR
  VAR_OUTPUT twice : INT; neg : BOOL; END_VAR
  v := v + 1;
  twice := 2 * v;
  neg := v < 0;
END_FUNCTION_BLOCK
FUNCTION_BLOCK SPARE
  VAR_INPUT a : INT; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  o := a;
END_FUNCTION_BLOCK
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR
    arr, out : ARRAY [1..3] OF INT;
    p, po : PT;
    r, ro : REF_TO INT;
    tgt, d2, i : INT;
    d : DINT;
    nb : BOOL;
    b : ADD1;
    idle : SPARE;
    x : INT (lo := 1, hi := 5) := hi;
    y : (p3 := 3, q4);
  END_VAR
  VAR lm AT %MW0 : ARRAY [1..2] OF INT; END_VAR
  VAR k AT %IB0 : SINT; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4b AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    q6 AT %QB6 : SINT;
    q7 AT %QB7 : SINT;
    q8 AT %QB8 : SINT;
    q9 AT %QB9 : SINT;
    q10 AT %QB10 : SINT;
    q11 AT %QX11.0 : BOOL;
    q12 AT %QB12 : SINT;
    q13 AT %QB13 : SINT;
  END_VAR
  b(v := arr[2], twice => out[1]);
  i := k;
  b(v := arr[i], twice => out[i]);
  b(v := p.y, twice => po.x);
  r := REF(tgt);
  ro := REF(d2);
  b(v := r^, twice => ro^);
  b(v := tgt, twice => d);
  b(v := p.x, NOT neg => nb);
  b(v := lm[2], twice => lm[1]);
  q0 := INT_TO_SINT(arr[1]);
  q1 := INT_TO_SINT(arr[2]);
  q2 := INT_TO_SINT(arr[3]);
  q3 := INT_TO_SINT(out[1]);
  q4b := INT_TO_SINT(out[2]);
  q5 := INT_TO_SINT(out[3]);
  q6 := INT_TO_SINT(p.y);
  q7 := INT_TO_SINT(po.x);
  q8 := INT_TO_SINT(tgt);
  q9 := INT_TO_SINT(d2);
  q10 := DINT_TO_SINT(d);
  q11 := nb;
  q12 := INT_TO_SINT(x + lo);
  x := x + 1;
  q13 := INT_TO_SINT(y);
  y := q4;
  b(v := arr[k + 1]);
END_PROGRAM

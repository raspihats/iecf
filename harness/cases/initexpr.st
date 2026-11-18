(* A variable's initial value that names variables and calls functions
   (the initial-expressions extension, docs/extensions.md), worked out as
   the variable is set up, in the order declared. Worked by hand:
   globals first: base 5, scaled base * Twice(3) = 5 * 6 = 30, RETAIN
   kept base + 1 = 6. Twice(x): its own y := x + 1, then (y - 1) * 2.
   main: a base * 2 = 10; b a + Twice(4) = 18; arr [a, b, 7], arr[1]
   18; p (x := a * 2), x 20, y its type's 9; c (d := b - a): Cnt sets
   up its own first, d 3 and q d * 10 = 30, then d is given 8; tm (PT :=
   pt0), T#2s; rg a * 20 = 200 into INT(0..100), clamped to 100 and
   flagged CONVERSION at its line (56), the first flag; s 'v10', LEN 3;
   RETAIN r kept * 2 = 12; m r + 1 = 13; the VAR_TEMP t b + 1 = 19 at
   each run. Each tick: c() runs its VAR_TEMP t := q + d, then q := t
   (38, then 46); n := acc.Add(t), the method's k := total + n (20,
   then 39); r := r + 1 (13, 14).
   A warm restart before tick 2: r keeps 14, kept 6; m is r + 1 = 15;
   c, acc set up again (q 38, n 20 at tick 2), r 15.
   %QB0 a 10, %QB1 b 18, %QB2 arr[1] 18, %QB3 p.x 20, %QB4 p.y 9, %QB5
   c.q, %QB6 rg 100, %QB7 LEN(s) 3, %QB8 scaled 30, %QB9 kept 6, %QB10
   r, %QB11 m, %QB12 n, %QX13.0 tm.PT = T#2s.
   reference-skip: a variable in an initial value is not IEC 61131-3. *)
TYPE Pt : STRUCT x : INT; y : INT := 9; END_STRUCT; END_TYPE

FUNCTION Twice : INT
  VAR_INPUT x : INT; END_VAR
  VAR y : INT := x + 1; END_VAR
  Twice := (y - 1) * 2;
END_FUNCTION

FUNCTION_BLOCK Cnt
  VAR_INPUT d : INT := 3; END_VAR
  VAR_OUTPUT q : INT := d * 10; END_VAR
  VAR_TEMP t : INT := q + d; END_VAR
  q := t;
END_FUNCTION_BLOCK

CLASS Accum
  VAR total : INT := 1; END_VAR
  METHOD PUBLIC Add : INT
    VAR_INPUT n : INT; END_VAR
    VAR k : INT := total + n; END_VAR
    total := k;
    Add := k;
  END_METHOD
END_CLASS

PROGRAM main
  VAR_EXTERNAL base : INT; scaled : INT; kept : INT; END_VAR
  VAR
    a : INT := base * 2;
    b : INT := a + Twice(4);
    arr : ARRAY[0..2] OF INT := [a, b, 7];
    p : Pt := (x := a * 2);
    c : Cnt := (d := b - a);
    pt0 : TIME := T#2s;
    tm : TON := (PT := pt0);
    rg : INT(0..100) := a * 20;
    s : STRING[8] := CONCAT('v', INT_TO_STRING(a));
    acc : Accum;
    n : INT;
  END_VAR
  VAR RETAIN r : INT := kept * 2; END_VAR
  VAR m : INT := r + 1; END_VAR
  VAR_TEMP t : INT := b + 1; END_VAR
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
    b0 AT %QX13.0 : BOOL;
  END_VAR
  c();
  n := acc.Add(t);
  r := r + 1;
  q0 := INT_TO_SINT(a);
  q1 := INT_TO_SINT(b);
  q2 := INT_TO_SINT(arr[1]);
  q3 := INT_TO_SINT(p.x);
  q4 := INT_TO_SINT(p.y);
  q5 := INT_TO_SINT(c.q);
  q6 := INT_TO_SINT(rg);
  q7 := INT_TO_SINT(LEN(s));
  q8 := INT_TO_SINT(scaled);
  q9 := INT_TO_SINT(kept);
  q10 := INT_TO_SINT(r);
  q11 := INT_TO_SINT(m);
  q12 := INT_TO_SINT(n);
  b0 := tm.PT = T#2s;
END_PROGRAM

CONFIGURATION cf
  VAR_GLOBAL base : INT := 5; scaled : INT := base * Twice(3); END_VAR
  VAR_GLOBAL RETAIN kept : INT := base + 1; END_VAR
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

(* SEL, MUX and MOVE of arrays, structures, instances, interfaces and
   references (Ed.3 Table 32, footnote d: allowed for user-defined
   types): the input selected, every input computed first, in order.
   a [1, 2, 3], a2 [4, 5, 6], a3 [7, 8, 9]; b TRUE. Worked by hand:
   SEL(b, a, a2)[1]: 4 (%QB0). SEL(NOT b, a, a2)[2]: 2 (%QB1).
   p.x 7, p2.x 9: SEL(b, p, p2).x 9 (%QB2). MUX(1, a, a2, a3)[3]: 6
   (%QB3); MUX(2, a, a2, a3)[1]: 7 (%QB4). MOVE(a)[2]: 2 (%QB5).
   Sum3(SEL(b, a, a2)), a FUNCTION's input: 4 + 5 + 6, 15 (%QB6).
   f(q := SEL(NOT b, a, a2)), a block's input: f.o[3] 3 (%QB7).
   la AT %MW0, [10, 11, 12]: SEL(b, a, la)[1] 10 (%QB8). Through a
   reference to a2: SEL(b, a, ra^)[2] 5 (%QB9).
   An instance: t2's PT is T#3s; tc := SEL(b, t1, t2) is a copy of t2:
   its PT T#3s (%QB10 1).
   Interfaces: o1 gives 1, o2 2. i := SEL(b, o1, o2): i.g() 2
   (%QB11). j := MUX(0, o1, NULL, i): j.g() 1 (%QB12). k := SEL(b, j,
   NULL) is NULL (%QB13 1).
   References: x1 21, x2 22. r := SEL(b, REF(x1), REF(x2)): r^ 22
   (%QB14). r2 := REF(x1); SEL(b, r, r2)^ := 30 writes x1: 30 (%QB15).
   A FUNCTION's result among them: SEL(b, a, Same(a3))[1] 7 (%QB16).
   Last, MUX(5, a, a2)[1]: K outside 0..1 faults, and the statement
   stores nothing (%QB17 stays 0).
   reference-skip: SEL, MUX, MOVE of arrays and structures, methods:
   the reference compiler refuses them. *)
TYPE
  A3 : ARRAY [1..3] OF INT;
  PT : STRUCT x : INT; y : REAL; END_STRUCT;
END_TYPE
INTERFACE IG
  METHOD g : INT END_METHOD
END_INTERFACE
FUNCTION_BLOCK BG IMPLEMENTS IG
  VAR_INPUT n : INT; END_VAR
  METHOD PUBLIC g : INT
    g := n;
  END_METHOD
END_FUNCTION_BLOCK
FUNCTION Sum3 : INT
  VAR_INPUT q : A3; END_VAR
  Sum3 := q[1] + q[2] + q[3];
END_FUNCTION
FUNCTION Same : A3
  VAR_INPUT q : A3; END_VAR
  Same := q;
END_FUNCTION
FUNCTION_BLOCK FB
  VAR_INPUT q : A3; END_VAR
  VAR_OUTPUT o : A3; END_VAR
  o := q;
END_FUNCTION_BLOCK
PROGRAM main
  VAR
    b : BOOL := TRUE;
    a : A3 := [1, 2, 3];
    a2 : A3 := [4, 5, 6];
    a3 : A3 := [7, 8, 9];
    p : PT := (x := 7);
    p2 : PT := (x := 9);
    f : FB;
    ra : REF_TO A3;
    t1 : TON;
    t2 : TON := (PT := T#3s);
    tc : TON;
    o1 : BG := (n := 1);
    o2 : BG := (n := 2);
    i : IG;
    j : IG;
    k : IG;
    x1 : INT := 21;
    x2 : INT := 22;
    r : REF_TO INT;
    r2 : REF_TO INT;
    v5 : INT := 5;
  END_VAR
  VAR
    la AT %MW0 : A3 := [10, 11, 12];
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
    q17 AT %QB17 : SINT;
  END_VAR
  q0 := INT_TO_SINT(SEL(b, a, a2)[1]);
  q1 := INT_TO_SINT(SEL(NOT b, a, a2)[2]);
  q2 := INT_TO_SINT(SEL(b, p, p2).x);
  q3 := INT_TO_SINT(MUX(1, a, a2, a3)[3]);
  q4 := INT_TO_SINT(MUX(2, a, a2, a3)[1]);
  q5 := INT_TO_SINT(MOVE(a)[2]);
  q6 := INT_TO_SINT(Sum3(SEL(b, a, a2)));
  f(q := SEL(NOT b, a, a2));
  q7 := INT_TO_SINT(f.o[3]);
  q8 := INT_TO_SINT(SEL(b, a, la)[1]);
  ra := REF(a2);
  q9 := INT_TO_SINT(SEL(b, a, ra^)[2]);
  tc := SEL(b, t1, t2);
  IF tc.PT = T#3s THEN
    q10 := 1;
  END_IF;
  i := SEL(b, o1, o2);
  q11 := INT_TO_SINT(i.g());
  j := MUX(0, o1, NULL, i);
  q12 := INT_TO_SINT(j.g());
  k := SEL(b, j, NULL);
  IF k = NULL THEN
    q13 := 1;
  END_IF;
  r := SEL(b, REF(x1), REF(x2));
  q14 := INT_TO_SINT(r^);
  r2 := REF(x1);
  SEL(b, r, r2)^ := 30;
  q15 := INT_TO_SINT(x1);
  q16 := INT_TO_SINT(SEL(b, a, Same(a3))[1]);
  q17 := INT_TO_SINT(MUX(v5, a, a2)[1]);
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

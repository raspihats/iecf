(* References a FUNCTION gives (Ed.3 6.4.4.10), stored through and
   called through; a block's input and a whole instance set through a
   reference; a standard block's input set from outside (Ed.3 6.6.7.2.2,
   Figure 20 a); REF() of an interface variable.
   g1 1, g2 2. Pick(w) gives REF(g2) if w, else REF(g1). Worked by hand:
   r := Pick(TRUE): r^ 2 (%QB0). Pick(FALSE)^ := 5 writes g1: 5 (%QB1).
   Nul() sets no result: NULL (%QB2 1).
   PickP()^.x := 7 writes gp.x: 7 (%QB3); PickP()^.x + 1: 8 (%QB4).
   PickB()^.i := 3; PickB()^() adds i to n: gb.n 3 (%QB5).
   PickB()^(i := 4); PickB()^.n: 7 (%QB6).
   rb := REF(gb); rb^ := b2, a copy of b2, whose i is 10 and n 0; then
   gb() adds 10: gb.n 10 (%QB7).
   tm.PT := T#2s, set from outside: T#2s (%QB8 1). rt := REF(gt1);
   rt^.PT := T#3s: gt1.PT T#3s (%QB9 1). PickT()^.PT := T#4s: T#4s
   (%QB10 1). t3.PT := T#9s; rt^ := t3 copies t3 into gt1: T#9s (%QB11
   1).
   o1 gives 3, o2 6. pi := REF(i); pi^ := o1: i.g() 3 (%QB12).
   pi := REF(is[2]); pi^ := o2: is[2].g() 6 (%QB13), and pi^ is not
   NULL (%QB14 1).
   Last, Nul()^ read: NULL dereferenced faults NULL, nothing stored
   (%QB15 stays 0).
   reference-skip: untried on the reference compiler. *)
TYPE
  PT : STRUCT x : INT; END_STRUCT;
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
FUNCTION_BLOCK BA
  VAR_INPUT i : INT; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + i;
END_FUNCTION_BLOCK
FUNCTION Pick : REF_TO INT
  VAR_INPUT w : BOOL; END_VAR
  VAR_EXTERNAL g1 : INT; g2 : INT; END_VAR
  IF w THEN
    Pick := REF(g2);
  ELSE
    Pick := REF(g1);
  END_IF;
END_FUNCTION
FUNCTION PickP : REF_TO PT
  VAR_EXTERNAL gp : PT; END_VAR
  PickP := REF(gp);
END_FUNCTION
FUNCTION PickB : REF_TO BA
  VAR_EXTERNAL gb : BA; END_VAR
  PickB := REF(gb);
END_FUNCTION
FUNCTION PickT : REF_TO TON
  VAR_EXTERNAL gt1 : TON; END_VAR
  PickT := REF(gt1);
END_FUNCTION
FUNCTION Nul : REF_TO INT
END_FUNCTION
PROGRAM main
  VAR_EXTERNAL g1 : INT; g2 : INT; gp : PT; gb : BA; gt1 : TON; END_VAR
  VAR
    r : REF_TO INT;
    rb : REF_TO BA;
    b2 : BA := (i := 10);
    tm : TON;
    rt : REF_TO TON;
    t3 : TON;
    o1 : BG := (n := 3);
    o2 : BG := (n := 6);
    i : IG;
    is : ARRAY [1..2] OF IG;
    pi : REF_TO IG;
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
  END_VAR
  r := Pick(TRUE);
  q0 := INT_TO_SINT(r^);
  Pick(FALSE)^ := 5;
  q1 := INT_TO_SINT(g1);
  IF Nul() = NULL THEN
    q2 := 1;
  END_IF;
  PickP()^.x := 7;
  q3 := INT_TO_SINT(gp.x);
  q4 := INT_TO_SINT(PickP()^.x + 1);
  PickB()^.i := 3;
  PickB()^();
  q5 := INT_TO_SINT(gb.n);
  PickB()^(i := 4);
  q6 := INT_TO_SINT(PickB()^.n);
  rb := REF(gb);
  rb^ := b2;
  gb();
  q7 := INT_TO_SINT(gb.n);
  tm.PT := T#2s;
  IF tm.PT = T#2s THEN
    q8 := 1;
  END_IF;
  rt := REF(gt1);
  rt^.PT := T#3s;
  IF gt1.PT = T#3s THEN
    q9 := 1;
  END_IF;
  PickT()^.PT := T#4s;
  IF gt1.PT = T#4s THEN
    q10 := 1;
  END_IF;
  t3.PT := T#9s;
  rt^ := t3;
  IF gt1.PT = T#9s THEN
    q11 := 1;
  END_IF;
  pi := REF(i);
  pi^ := o1;
  q12 := INT_TO_SINT(i.g());
  pi := REF(is[2]);
  pi^ := o2;
  q13 := INT_TO_SINT(is[2].g());
  IF pi^ <> NULL THEN
    q14 := 1;
  END_IF;
  q15 := INT_TO_SINT(Nul()^);
END_PROGRAM

CONFIGURATION c
  VAR_GLOBAL
    g1 : INT := 1;
    g2 : INT := 2;
    gp : PT;
    gb : BA;
    gt1 : TON;
  END_VAR
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

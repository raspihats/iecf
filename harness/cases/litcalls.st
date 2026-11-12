(* A standard function given literals alone, and a quotient of real
   literals: no operand gives the expression a type, so it is computed
   in the type wanted of it - the variable assigned, the other operand,
   the conversion's own. Worked by hand, the same at every tick:
   v := MAX(10, 11): 11 (%QB0). r := SQRT(2.0), 1.4142135, times 10.0:
   14 (%QB1). SEL(TRUE, 1, 2): 2 (%QB2). w := SHL(1, 3): 8 (%QB3).
   ABS(-3): 3 (%QB4). r := 1.0 / 3.0, times 9.0: 3 (%QB5).
   LIMIT(1, 5, 3): 3 (%QB6). MUX(1, 5, 6), the second of its inputs: 6
   (%QB7). ADD(1, 2, 3): 6 (%QB8). r := MAX(1.5, 2), times 2.0: 4
   (%QB9). v := 10; v := v + MAX(1, 2): 12 (%QB10).
   Integers stay integers where a real is wanted: r := MAX(7, 2) / 2 is
   7 / 2, 3, as r := 7 / 2 is (%QB16): times 2.0, 6 (%QB11); r := 1.5 +
   7 / 2 is 4.5: times 2.0, 9 (%QB12).
   -MAX(1, 2): -2 (%QB13). GT(MAX(1, 2), 1): TRUE (%QX14.0).
   REAL_TO_INT(SQRT(16.0)): 4 (%QB15). d := ADD(30000, 30000) in DINT:
   60000, over 1000 is 60 (%QB17). lr := SQRT(2.0) in LREAL: lr * lr *
   50.0 is 100.00000000000001, 100 (%QB18). COS(0): 1 (%QB19).
   EXPT(2.0, 3): 8 (%QB20). (MAX(1, 2)) + 1: 3 (%QB21).
   An initial value: i : INT := MAX(1, 2), 2 (%QB23).
   Last, the same ADD(30000, 30000) wanted as an INT is computed in
   INT: it wraps to -5536 and is flagged OVERFLOW; over 100 is -55
   (%QB22).
   reference-skip: untried on the reference compiler. *)
PROGRAM main
  VAR
    v : INT;
    r : REAL;
    lr : LREAL;
    b : BOOL;
    w : WORD;
    d : DINT;
    i : INT := MAX(1, 2);
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
    q14 AT %QX14.0 : BOOL;
    q15 AT %QB15 : SINT;
    q16 AT %QB16 : SINT;
    q17 AT %QB17 : SINT;
    q18 AT %QB18 : SINT;
    q19 AT %QB19 : SINT;
    q20 AT %QB20 : SINT;
    q21 AT %QB21 : SINT;
    q22 AT %QB22 : SINT;
    q23 AT %QB23 : SINT;
  END_VAR
  v := MAX(10, 11);
  q0 := INT_TO_SINT(v);
  r := SQRT(2.0);
  q1 := REAL_TO_SINT(r * 10.0);
  v := SEL(TRUE, 1, 2);
  q2 := INT_TO_SINT(v);
  w := SHL(1, 3);
  q3 := WORD_TO_SINT(w);
  v := ABS(-3);
  q4 := INT_TO_SINT(v);
  r := 1.0 / 3.0;
  q5 := REAL_TO_SINT(r * 9.0);
  v := LIMIT(1, 5, 3);
  q6 := INT_TO_SINT(v);
  v := MUX(1, 5, 6);
  q7 := INT_TO_SINT(v);
  v := ADD(1, 2, 3);
  q8 := INT_TO_SINT(v);
  r := MAX(1.5, 2);
  q9 := REAL_TO_SINT(r * 2.0);
  v := 10;
  v := v + MAX(1, 2);
  q10 := INT_TO_SINT(v);
  r := MAX(7, 2) / 2;
  q11 := REAL_TO_SINT(r * 2.0);
  r := 1.5 + 7 / 2;
  q12 := REAL_TO_SINT(r * 2.0);
  v := -MAX(1, 2);
  q13 := INT_TO_SINT(v);
  q14 := GT(MAX(1, 2), 1);
  v := REAL_TO_INT(SQRT(16.0));
  q15 := INT_TO_SINT(v);
  r := 7 / 2;
  q16 := REAL_TO_SINT(r);
  d := ADD(30000, 30000);
  q17 := DINT_TO_SINT(d / 1000);
  lr := SQRT(2.0);
  q18 := LREAL_TO_SINT(lr * lr * 50.0);
  r := COS(0);
  q19 := REAL_TO_SINT(r);
  r := EXPT(2.0, 3);
  q20 := REAL_TO_SINT(r);
  v := (MAX(1, 2)) + 1;
  q21 := INT_TO_SINT(v);
  q23 := INT_TO_SINT(i);
  v := ADD(30000, 30000);
  q22 := INT_TO_SINT(v / 100);
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

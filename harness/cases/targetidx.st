(* An assignment's target indexed by a FUNCTION with effects or by a
   real converted to an integer; a statement whose conversion fails is
   left undone, though its placeholder would fault.
   Inc adds 1 to its in-out and gives it. n 0. Worked by hand:
   a[Inc(n)] := 5: Inc called once, n 1, a[1] 5 (%QB0, n %QB1).
   aa[Inc(n)][Inc(n)] := 9: n 2, then 3; aa[2][3] 9 (%QB2, n %QB3).
   ps[Inc(n) - 2].x := 4: n 4, ps[2].x 4 (%QB4).
   r 2.0: a[REAL_TO_INT(r)] := 6: a[2] 6 (%QB5).
   The target first, then the value (Ed.3 7.3.2): b[Inc(n)] := n makes
   n 5, then reads it: b[5] 5 (%QB6).
   Then r is 1.0E10, beyond INT, and each statement it is converted in
   is left undone and flagged CONVERSION, first at the line of
   a[REAL_TO_INT(r)] := 7 - though the placeholder it would give, 0,
   is outside a and divides by zero: a[1] is still 5 (%QB7); v := 10 /
   REAL_TO_INT(r) leaves v 7 (%QB8); v := a[REAL_TO_INT(r)] leaves it 7
   (%QB9); IF a[REAL_TO_INT(r)] > 0 runs no branch (%QB10 stays 0).
   Last, a[i] := 1 with i 4: a fault (INDEX), nothing stored.
   reference-skip: untried on the reference compiler. *)
TYPE
  PT : STRUCT x : INT; END_STRUCT;
END_TYPE
FUNCTION Inc : INT
  VAR_IN_OUT io : INT; END_VAR
  io := io + 1;
  Inc := io;
END_FUNCTION
PROGRAM main
  VAR
    n : INT;
    r : REAL := 2.0;
    v : INT := 7;
    i : INT := 4;
    a : ARRAY [1..3] OF INT;
    aa : ARRAY [1..3] OF ARRAY [1..3] OF INT;
    ps : ARRAY [1..3] OF PT;
    b : ARRAY [1..9] OF INT;
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
  END_VAR
  a[Inc(n)] := 5;
  q0 := INT_TO_SINT(a[1]);
  q1 := INT_TO_SINT(n);
  aa[Inc(n)][Inc(n)] := 9;
  q2 := INT_TO_SINT(aa[2][3]);
  q3 := INT_TO_SINT(n);
  ps[Inc(n) - 2].x := 4;
  q4 := INT_TO_SINT(ps[2].x);
  a[REAL_TO_INT(r)] := 6;
  q5 := INT_TO_SINT(a[2]);
  b[Inc(n)] := n;
  q6 := INT_TO_SINT(b[5]);
  r := 1.0E10;
  a[REAL_TO_INT(r)] := 7;
  q7 := INT_TO_SINT(a[1]);
  v := 10 / REAL_TO_INT(r);
  q8 := INT_TO_SINT(v);
  v := a[REAL_TO_INT(r)];
  q9 := INT_TO_SINT(v);
  IF a[REAL_TO_INT(r)] > 0 THEN
    q10 := 1;
  END_IF;
  a[i] := 1;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

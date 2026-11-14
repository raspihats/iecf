(* Outputs of a structure, an array and a STRING type taken into an
   element by a computed index, and through a reference (Ed.3 6.6.1.4:
   Param_Assign, `Variable_Name '=>' Variable`): a FUNCTION's, found
   after it returns, and a function block's, found before it runs - the
   index checked there, as an elementary output's target is. Worked by
   hand: tick 0, i 1 (%IB0): Make(a := 5) into ps[1], ss[1], vs[1]:
   ps[1].x 5 (%QB0), ps[1].y 10 (%QB1), vs[1][1] 7 (%QB2), ss[1] 'ab'
   (%QX7.0); g(a := 7) into ps[2], ss[2]: ps[2].x 7 (%QB3), ps[2].y -7
   (%QB4), ss[2] 'cd' (%QX7.1); Make(a := 3) through rp^ and rst^ into pnt
   and st: pnt.x 3 (%QB5), pnt.y 6 (%QB6), st 'ab' (%QX7.2). Last, a
   whole output of an instance that is an element read, pnt :=
   gs[i - 1].p after gs[0](a := 4): 4 (%QB8), -4 (%QB9).
   Tick 1, i 2: Make into ps[2] (5, 10), then g's target ps[3] is
   outside 0..2: the task stops there, fault INDEX, line 70, before g
   runs.
   reference-skip: the reference compiler refuses REF_TO STRING[8], and
   its C for an output into an element does not compile (as fnoutx). *)
TYPE POINT : STRUCT x, y : INT; END_STRUCT; END_TYPE

FUNCTION Make : INT
  VAR_INPUT a : INT; END_VAR
  VAR_OUTPUT p : POINT; s : STRING[8]; v : ARRAY[0..1] OF INT; END_VAR
  p.x := a;
  p.y := a * 2;
  s := 'ab';
  v[0] := a + 1;
  v[1] := a + 2;
  Make := a;
END_FUNCTION

FUNCTION_BLOCK Gen
  VAR_INPUT a : INT; END_VAR
  VAR_OUTPUT p : POINT; s : STRING[8]; END_VAR
  p.x := a;
  p.y := -a;
  s := 'cd';
END_FUNCTION_BLOCK

PROGRAM main
  VAR
    i AT %IB0 : SINT;
  END_VAR
  VAR
    ps : ARRAY[0..2] OF POINT;
    ss : ARRAY[0..2] OF STRING[8];
    vs : ARRAY[0..2] OF ARRAY[0..1] OF INT;
    g : Gen;
    gs : ARRAY[0..1] OF Gen;
    pnt : POINT;
    st : STRING[8];
    rp : REF_TO POINT;
    rst : REF_TO STRING[8];
    n : INT;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    q6 AT %QB6 : SINT;
    c0 AT %QX7.0 : BOOL;
    c1 AT %QX7.1 : BOOL;
    c2 AT %QX7.2 : BOOL;
    q8 AT %QB8 : SINT;
    q9 AT %QB9 : SINT;
  END_VAR
  n := Make(a := 5, p => ps[i], s => ss[i], v => vs[i]);
  g(a := 7, p => ps[i + 1], s => ss[i + 1]);
  rp := REF(pnt);
  rst := REF(st);
  n := Make(a := 3, p => rp^, s => rst^);
  q0 := INT_TO_SINT(ps[1].x);
  q1 := INT_TO_SINT(ps[1].y);
  q2 := INT_TO_SINT(vs[1][1]);
  q3 := INT_TO_SINT(ps[2].x);
  q4 := INT_TO_SINT(ps[2].y);
  q5 := INT_TO_SINT(pnt.x);
  q6 := INT_TO_SINT(pnt.y);
  c0 := ss[1] = 'ab';
  c1 := ss[2] = 'cd';
  c2 := st = 'ab';
  gs[i - 1](a := 4);
  pnt := gs[i - 1].p;
  q8 := INT_TO_SINT(pnt.x);
  q9 := INT_TO_SINT(pnt.y);
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

(* GT, GE, EQ, LE and LT with more than two inputs (Ed.3 Table 33:
   each neighbouring pair holds), every input computed once and in
   order; the negation of a TIME and an LTIME; the shifts and rotations
   of a BOOL. Worked by hand, the same at every tick:
   v 5: GT(v, 2, 1) TRUE (%QX0.0); GT(v, 6, 1) FALSE (%QX0.1);
   GE(v, 5, 5, 1) TRUE (%QX0.2); LT(1, 2, 2) FALSE (%QX0.3);
   LE(1, 2, 2) TRUE (%QX0.4). s 'ab': EQ('ab', s, 'ab') TRUE (%QX0.5).
   t T#2s: LT(T#1s, t, T#3s) TRUE (%QX0.6).
   Inc adds 1 to its in-out and gives it. n := 0; GT(10, Inc(n), 0):
   Inc is called once, n is 1 (%QB1), and 10 > 1 > 0 TRUE (%QX0.7).
   GT(n, Inc(n), 0): n is read first, 1, then Inc makes it 2: 1 > 2 is
   FALSE (%QX2.0), and n is 2 (%QB3).
   -T#1s500ms is T#-1s500ms (%QX2.1); -LTIME#2ns is LTIME#-2ns
   (%QX2.2). A BOOL is one bit: SHL(TRUE, 0) TRUE (%QX2.3), SHL(TRUE,
   1) FALSE (%QX2.4), ROL(TRUE, 5) TRUE (%QX2.5), SHR(TRUE, 1) FALSE
   (%QX2.6), ROR(TRUE, 2) TRUE (%QX2.7).
   Last, the TIME DINT's lowest, T#-24d20h31m23s648ms, negated: it
   wraps to itself and is flagged OVERFLOW (%QB4 1).
   reference-skip: no LTIME; -t on a TIME and SHL on a BOOL: its C
   does not build. *)
FUNCTION Inc : INT
  VAR_IN_OUT io : INT; END_VAR
  io := io + 1;
  Inc := io;
END_FUNCTION
PROGRAM main
  VAR
    v : INT := 5;
    n : INT;
    s : STRING := 'ab';
    t : TIME := T#2s;
    t2 : TIME;
    l2 : LTIME;
    bb : BOOL := TRUE;
    tm : TIME;
  END_VAR
  VAR
    q00 AT %QX0.0 : BOOL;
    q01 AT %QX0.1 : BOOL;
    q02 AT %QX0.2 : BOOL;
    q03 AT %QX0.3 : BOOL;
    q04 AT %QX0.4 : BOOL;
    q05 AT %QX0.5 : BOOL;
    q06 AT %QX0.6 : BOOL;
    q07 AT %QX0.7 : BOOL;
    q1 AT %QB1 : SINT;
    q20 AT %QX2.0 : BOOL;
    q21 AT %QX2.1 : BOOL;
    q22 AT %QX2.2 : BOOL;
    q23 AT %QX2.3 : BOOL;
    q24 AT %QX2.4 : BOOL;
    q25 AT %QX2.5 : BOOL;
    q26 AT %QX2.6 : BOOL;
    q27 AT %QX2.7 : BOOL;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
  END_VAR
  q00 := GT(v, 2, 1);
  q01 := GT(v, 6, 1);
  q02 := GE(v, 5, 5, 1);
  q03 := LT(1, 2, 2);
  q04 := LE(1, 2, 2);
  q05 := EQ('ab', s, 'ab');
  q06 := LT(T#1s, t, T#3s);
  n := 0;
  q07 := GT(10, Inc(n), 0);
  q1 := INT_TO_SINT(n);
  q20 := GT(n, Inc(n), 0);
  q3 := INT_TO_SINT(n);
  t2 := T#1s500ms;
  t2 := -t2;
  q21 := t2 = T#-1s500ms;
  l2 := LTIME#2ns;
  l2 := -l2;
  q22 := l2 = LTIME#-2ns;
  q23 := SHL(bb, 0);
  q24 := SHL(bb, 1);
  q25 := ROL(bb, 5);
  q26 := SHR(bb, 1);
  q27 := ROR(bb, 2);
  tm := T#-24d20h31m23s648ms;
  tm := -tm;
  IF tm = T#-24d20h31m23s648ms THEN
    q4 := 1;
  END_IF;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

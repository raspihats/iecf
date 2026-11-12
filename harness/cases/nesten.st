(* Calls with EN nested in another's arguments, and EN on a FUNCTION
   giving an array or a structure (Ed.3 6.6.1.5; docs/dependencies.md:
   EN FALSE does not evaluate the inputs, calls nothing and leaves the
   statement undone). Worked by hand:
   b TRUE: ADD(EN := b, IN1 := ADD(EN := b, IN1 := 1, IN2 := 2),
   IN2 := 3): 6 (%QB0). b FALSE, v 9 before: the statement is undone, v
   9 (%QB1).
   t 32767: ADD(EN := TRUE, IN1 := ADD(EN := TRUE, IN1 := t, IN2 := 1,
   ENO => e2), IN2 := 0, ENO => e1): the inner one overflows, wraps to
   -32768 and is flagged OVERFLOW, its ENO FALSE (%QX2.0 0); the outer
   adds 0, no flag of its own: its ENO TRUE (%QX2.1 1); v / 1000 is -32
   (%QB3).
   Inc adds 1 to its in-out. n 0: ADD(EN := TRUE, IN1 := Inc(EN :=
   TRUE, io := n), IN2 := 10): 11 (%QB4), n 1 (%QB5). The outer's EN
   FALSE, v 5 before: Inc is not called, v 5 (%QB6), n still 1 (%QB7).
   FA gives [x, 2x, 0]: a := FA(EN := TRUE, x := 4), a[2] 8 (%QB8);
   a := FA(EN := FALSE, x := 7) leaves a: a[1] 4 (%QB9).
   p := FP(EN := TRUE, x := 3, ENO => e3): p.x 3 (%QB10), e3 TRUE
   (%QX11.0). SumA(FA(EN := TRUE, x := 2)), 2 + 4 + 0: 6 (%QB12).
   The first flag is the inner ADD's OVERFLOW.
   reference-skip: untried on the reference compiler. *)
TYPE
  A3 : ARRAY [1..3] OF INT;
  PT : STRUCT x : INT; END_STRUCT;
END_TYPE
FUNCTION Inc : INT
  VAR_IN_OUT io : INT; END_VAR
  io := io + 1;
  Inc := io;
END_FUNCTION
FUNCTION FA : A3
  VAR_INPUT x : INT; END_VAR
  FA[1] := x;
  FA[2] := x * 2;
END_FUNCTION
FUNCTION FP : PT
  VAR_INPUT x : INT; END_VAR
  FP.x := x;
END_FUNCTION
FUNCTION SumA : INT
  VAR_INPUT q : A3; END_VAR
  SumA := q[1] + q[2] + q[3];
END_FUNCTION
PROGRAM main
  VAR
    b : BOOL := TRUE;
    v : INT;
    t : INT := 32767;
    n : INT;
    e1 : BOOL;
    e2 : BOOL;
    e3 : BOOL;
    a : A3;
    p : PT;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q20 AT %QX2.0 : BOOL;
    q21 AT %QX2.1 : BOOL;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    q6 AT %QB6 : SINT;
    q7 AT %QB7 : SINT;
    q8 AT %QB8 : SINT;
    q9 AT %QB9 : SINT;
    q10 AT %QB10 : SINT;
    q11 AT %QX11.0 : BOOL;
    q12 AT %QB12 : SINT;
  END_VAR
  v := ADD(EN := b, IN1 := ADD(EN := b, IN1 := 1, IN2 := 2), IN2 := 3);
  q0 := INT_TO_SINT(v);
  b := FALSE;
  v := 9;
  v := ADD(EN := b, IN1 := ADD(EN := b, IN1 := 1, IN2 := 2), IN2 := 3);
  q1 := INT_TO_SINT(v);
  v := ADD(EN := TRUE, IN1 := ADD(EN := TRUE, IN1 := t, IN2 := 1, ENO => e2), IN2 := 0, ENO => e1);
  q20 := e2;
  q21 := e1;
  q3 := INT_TO_SINT(v / 1000);
  n := 0;
  v := ADD(EN := TRUE, IN1 := Inc(EN := TRUE, io := n), IN2 := 10);
  q4 := INT_TO_SINT(v);
  q5 := INT_TO_SINT(n);
  v := 5;
  v := ADD(EN := FALSE, IN1 := Inc(EN := TRUE, io := n), IN2 := 10);
  q6 := INT_TO_SINT(v);
  q7 := INT_TO_SINT(n);
  a := FA(EN := TRUE, x := 4);
  q8 := INT_TO_SINT(a[2]);
  a := FA(EN := FALSE, x := 7);
  q9 := INT_TO_SINT(a[1]);
  p := FP(EN := TRUE, x := 3, ENO => e3);
  q10 := INT_TO_SINT(p.x);
  q11 := e3;
  q12 := INT_TO_SINT(SumA(FA(EN := TRUE, x := 2)));
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

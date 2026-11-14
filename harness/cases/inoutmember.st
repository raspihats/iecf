(* An instance's input or PUBLIC variable given to an in-out (Ed.3
   6.6.1.4: an in-out takes a variable; an input may be set from outside,
   6.6.7.2.2): the field itself, as an assignment from outside sets it.
   Inc adds 1 to its in-out. Worked by hand, tick by tick:
   f.a and f.pub 1, 2: f's q = a * 10 + pub, 11 then 22 (%QB0);
   fs[i].a, i 1: 1, 2, fs[1].q 10 then 20 (%QB1); b.v (a CLASS's
   PUBLIC) 1, 2 (%QB2); tm.PT (TON's input) T#1s then T#2s: %QX3.0 TRUE
   at tick 0, %QX3.1 TRUE at tick 1. And outputs taken into them, as
   an assignment from outside sets them: g(q => f.k), Gen's q 5, f.k 5
   (%QB4); Two(o => b.w), 2 (%QB5); ENO => f.ok TRUE (%QX6.0);
   SPLIT_DATE(IN := D#2024-03-05, YEAR => f.y), 2024 (%QW8, 16#07E8).
   reference-skip: VAR PUBLIC and CLASS are Ed.3's; the reference
   compiler (Ed.2) has neither. *)
FUNCTION Inc : INT
  VAR_IN_OUT c : INT; END_VAR
  c := c + 1;
  Inc := c;
END_FUNCTION

FUNCTION IncT : INT
  VAR_IN_OUT c : TIME; END_VAR
  c := c + T#1s;
  IncT := 0;
END_FUNCTION

FUNCTION Two : INT
  VAR_OUTPUT o : INT; END_VAR
  o := 2;
  Two := 1;
END_FUNCTION

FUNCTION_BLOCK Gen
  VAR_OUTPUT q : INT; END_VAR
  q := 5;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Acc
  VAR_INPUT a : INT; k : INT; ok : BOOL; y : UINT; END_VAR
  VAR_OUTPUT q : INT; END_VAR
  VAR PUBLIC pub : INT; END_VAR
  q := a * 10 + pub;
END_FUNCTION_BLOCK

CLASS Box
  VAR PUBLIC v : INT; w : INT; END_VAR
END_CLASS

PROGRAM main
  VAR
    f : Acc;
    fs : ARRAY[0..1] OF Acc;
    b : Box;
    tm : TON;
    g : Gen;
    n : INT;
    i : INT := 1;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    t1 AT %QX3.0 : BOOL;
    t2 AT %QX3.1 : BOOL;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    e6 AT %QX6.0 : BOOL;
    q8 AT %QW8 : UINT;
  END_VAR
  n := Inc(f.a);
  n := Inc(f.pub);
  f();
  n := Inc(fs[i].a);
  fs[i]();
  n := Inc(b.v);
  n := IncT(tm.PT);
  q0 := INT_TO_SINT(f.q);
  q1 := INT_TO_SINT(fs[1].q);
  q2 := INT_TO_SINT(b.v);
  t1 := tm.PT = T#1s;
  t2 := tm.PT = T#2s;
  g(q => f.k);
  n := Two(o => b.w);
  n := Two(EN := TRUE, ENO => f.ok);
  SPLIT_DATE(IN := D#2024-03-05, YEAR => f.y);
  q4 := INT_TO_SINT(f.k);
  q5 := INT_TO_SINT(b.w);
  e6 := f.ok;
  q8 := f.y;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

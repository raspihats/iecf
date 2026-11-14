(* A list repeated in an array's initial value (Ed.3 6.4.4.5.2: "A :
   ARRAY[0..5] OF INT := [2(1, 2, 3)] is equivalent to the
   initialization sequence 1, 2, 3, 1, 2, 3"), more values than
   elements ignored (W0008), and a constant's element found in one.
   And two transitions of one chart whose conditions each convert a
   REAL to an integer: a condition left undone is FALSE.
   Worked by hand: c is 1, 2, 3, 1, 2, 3: c[4] 2 (%QB1). K is 4, 5, 6,
   4, 5, 6: K[5] 6 (%QB2). d, five elements of the same list: d[4] 2
   (%QB3). e is [9, 3(2, 7)], four elements: 9, 2, 7, 2: e[3] 2 (%QB4).
   Seq: s0 to s1 when REAL_TO_INT(r) > 2, back when it is > 5; watch
   sets o to s1.X before the transitions are tested (%QB0).
   tick 0: r 3.0: o 0; s0 to s1.         00
   tick 1: o 1; 3 > 5 is not.            01
   tick 2: r 1.0E10: flagged CONVERSION,
           the condition FALSE: s1 stays. 01
   tick 3: r 6.0: o 1; s1 to s0.         01
   tick 4: o 0; s0 to s1.                00
   tick 5: o 1.                          01
   The flag stays from tick 2 on, until the platform clears it.
   reference-skip: the reference compiler's chart tests transitions
   first (sfcdemo), and a conversion out of range is iecf's own. *)
FUNCTION_BLOCK Seq
  VAR_INPUT r : REAL; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  INITIAL_STEP s0 : END_STEP
  TRANSITION FROM s0 TO s1 := REAL_TO_INT(r) > 2; END_TRANSITION
  STEP s1 : END_STEP
  TRANSITION FROM s1 TO s0 := REAL_TO_INT(r) > 5; END_TRANSITION
  INITIAL_STEP w0 : watch(N); END_STEP
  TRANSITION FROM w0 TO w0 := FALSE; END_TRANSITION
  ACTION watch : o := BOOL_TO_INT(s1.X); END_ACTION
END_FUNCTION_BLOCK

PROGRAM main
  VAR CONSTANT K : ARRAY[0..5] OF INT := [2(4, 5, 6)]; END_VAR
  VAR
    c : ARRAY[0..5] OF INT := [2(1, 2, 3)];
    d : ARRAY[0..4] OF INT := [2(1, 2, 3)];
    e : ARRAY[0..3] OF INT := [9, 3(2, 7)];
    s : Seq;
    n : INT;
    r : REAL;
  END_VAR
  VAR
    q0 AT %QB0 : USINT;
    q1 AT %QB1 : USINT;
    q2 AT %QB2 : USINT;
    q3 AT %QB3 : USINT;
    q4 AT %QB4 : USINT;
  END_VAR
  r := 3.0;
  IF n = 2 THEN
    r := 1.0E10;
  ELSIF n > 2 THEN
    r := 6.0;
  END_IF;
  s(r := r);
  q0 := INT_TO_USINT(s.o);
  q1 := INT_TO_USINT(c[4]);
  q2 := INT_TO_USINT(K[5]);
  q3 := INT_TO_USINT(d[4]);
  q4 := INT_TO_USINT(e[3]);
  n := n + 1;
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

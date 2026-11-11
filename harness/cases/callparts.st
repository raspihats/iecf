(* A part of a FUNCTION's result read where the call is: a field, an
   element, a variable of an instance (Ed.3 6.6.2.3: a call is an operand,
   and what it gives is of the result's type).
   MKP gives a PT with x = v and y[2] = v + 1; MKA an array with [1] = v
   and [3] = 2 * v; MKC a CNT called once with inc := v, so n = v. BUMP
   adds 1 to its in-out and gives a PT with x the new value.
   Worked by hand, k %IB0 1, 2, 3; j is 2:
   MKP(k).x 1, 2, 3 (%QB0). MKP(k).y[2] 2, 3, 4 (%QB1); MKP(k).y[j], the
   index computed, the same (%QB2).
   MKA(k)[3] 2, 4, 6 (%QB3). MKA(k)[k]: [1] 1, [2] 0, [3] 6 (%QB4).
   MKC(k).n 1, 2, 3 (%QB5).
   Two calls in one expression: MKP(k).x + MKA(k)[1] 2, 4, 6 (%QB6).
   In a condition: MKP(k).x > MKC(2).n only at k 3: 0, 0, 1 (%QB7).
   A call with effects is made once though a part of it is read:
   BUMP(cnt).x 1, 2, 3 (%QB8) and cnt after it 1, 2, 3 (%QB9).
   %QB10 := MKA(k)[k + 1]: [2] 0, [3] 4; at tick 2 the index is 4,
   outside 1..3: a fault (index), the statement stopped, %QB10 kept 4.
   Every construct here but the CNT result is Ed.2's. *)
TYPE PT : STRUCT x : INT; y : ARRAY [1..3] OF INT; END_STRUCT; END_TYPE
FUNCTION_BLOCK CNT
  VAR_INPUT inc : INT := 1; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + inc;
END_FUNCTION_BLOCK
FUNCTION MKP : PT
  VAR_INPUT v : INT; END_VAR
  MKP.x := v;
  MKP.y[2] := v + 1;
END_FUNCTION
FUNCTION MKA : ARRAY [1..3] OF INT
  VAR_INPUT v : INT; END_VAR
  MKA[1] := v;
  MKA[3] := v * 2;
END_FUNCTION
FUNCTION MKC : CNT
  VAR_INPUT v : INT; END_VAR
  MKC(inc := v);
END_FUNCTION
FUNCTION BUMP : PT
  VAR_IN_OUT n : INT; END_VAR
  n := n + 1;
  BUMP.x := n;
END_FUNCTION
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR j : INT := 2; cnt : INT; END_VAR
  VAR k AT %IB0 : SINT; END_VAR
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
  q0 := INT_TO_SINT(MKP(k).x);
  q1 := INT_TO_SINT(MKP(k).y[2]);
  q2 := INT_TO_SINT(MKP(k).y[j]);
  q3 := INT_TO_SINT(MKA(k)[3]);
  q4 := INT_TO_SINT(MKA(k)[k]);
  q5 := INT_TO_SINT(MKC(k).n);
  q6 := INT_TO_SINT(MKP(k).x + MKA(k)[1]);
  IF MKP(k).x > MKC(2).n THEN
    q7 := 1;
  ELSE
    q7 := 0;
  END_IF;
  q8 := INT_TO_SINT(BUMP(cnt).x);
  q9 := INT_TO_SINT(cnt);
  q10 := INT_TO_SINT(MKA(k)[k + 1]);
END_PROGRAM

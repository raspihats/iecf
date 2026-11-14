(* A block's ENO, and outputs, taken into an element, a field and a bit,
   each found - its index checked - before the block runs. Worked by
   hand:
   t(EN := TRUE, ..., ENO => a[i]), i 2: ENO TRUE, a[2] TRUE (%QX0.0).
   t(EN := FALSE, ..., NOT ENO => w.3): ENO FALSE, its NOT TRUE: w 8,
   w.%B0 08 (%QB1).
   t(EN := TRUE, ..., ENO => s.ok): s.ok TRUE (%QX0.1).
   o(q => ws[i].3, qb => ws[i].%B1), q TRUE and qb 7: ws[2] 16#0708,
   its bytes 08 (%QB2) and 07 (%QB3).
   F(q => ws[1].2), q TRUE: ws[1] 4 (%QB4).
   Last, i 3: o(q => ws[i].0) faults (INDEX) before o runs, so n, o's
   count of its runs, stays 1 (%QB5).
   reference-skip: partial access, NOT ENO =>: refused (iec2c crashes
   on the latter). *)
TYPE
  S : STRUCT ok : BOOL; END_STRUCT;
END_TYPE
FUNCTION_BLOCK O
  VAR_OUTPUT q : BOOL; qb : BYTE; n : INT; END_VAR
  q := TRUE;
  qb := 7;
  n := n + 1;
END_FUNCTION_BLOCK
FUNCTION F : INT
  VAR_OUTPUT q : BOOL; END_VAR
  q := TRUE;
  F := 1;
END_FUNCTION
PROGRAM main
  VAR
    t : TON;
    o : O;
    a : ARRAY [1..2] OF BOOL;
    ws : ARRAY [1..2] OF WORD;
    s : S;
    w : WORD;
    i : INT := 2;
    v : INT;
  END_VAR
  VAR
    q00 AT %QX0.0 : BOOL;
    q01 AT %QX0.1 : BOOL;
    q1 AT %QB1 : BYTE;
    q2 AT %QB2 : BYTE;
    q3 AT %QB3 : BYTE;
    q4 AT %QB4 : BYTE;
    q5 AT %QB5 : SINT;
  END_VAR
  t(EN := TRUE, IN := FALSE, PT := T#1s, ENO => a[i]);
  q00 := a[2];
  t(EN := FALSE, IN := FALSE, PT := T#1s, NOT ENO => w.3);
  q1 := w.%B0;
  t(EN := TRUE, IN := FALSE, PT := T#1s, ENO => s.ok);
  q01 := s.ok;
  o(q => ws[i].3, qb => ws[i].%B1);
  q2 := ws[2].%B0;
  q3 := ws[2].%B1;
  v := F(q => ws[1].2);
  q4 := ws[1].%B0;
  q5 := INT_TO_SINT(o.n);
  i := 3;
  o(q => ws[i].0);
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

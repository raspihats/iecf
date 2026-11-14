(* A real that does not fit its integer, in the index of a place a call
   takes: the statement is left undone (docs/behaviour/conversions.md).
   A FUNCTION after the conversion is not called, a block is not called,
   and nothing is stored - not into the element the placeholder, 0,
   would find. r is 1.0E10, beyond INT; r2 3.0. Worked by hand, the
   same each tick but the counts:
   n := Two(o => a[REAL_TO_INT(r)]): an output's place is found after
   the call, so Two is called (calls 1) and neither a[0] nor n stored.
   n := Inc(a[REAL_TO_INT(r) + 1]): Inc not called, a[1] 0, n 7.
   n := Two(i := REAL_TO_INT(r), o => x): not called, x and n kept.
   n := REAL_TO_INT(r) + Inc(a[2]): not called, a[2] 0.
   Two(o => a[REAL_TO_INT(r)]), a statement: called (calls 2), not
   stored. SPLIT_DATE into a[REAL_TO_INT(r)] and a[1]: neither stored.
   The block calls, none made: an instance by the index; an in-out by
   it, alone and with EN, whose ENO then keeps bs[1] TRUE; an output,
   a STRING output and ENO into it; an array in-out; an instance
   in-out; TON's Q into bs[..]. Then f(q => a[REAL_TO_INT(r2)]) and
   f() run: f.q 2 then 4, x 2 then 4, a[3] 1 then 3.
   %QB0 a[0] 0, %QB1 a[1] 0, %QB2 a[2] 0, %QB3 n 7, %QB4 calls 2 then
   4, %QB5 f.q 2 then 4, %QB6 x 2 then 4, %QB7 fs[0].q 0, %QB8 g.n +
   u.n 0, %QB9 a[3] 1 then 3; %QX10.0 bs[0] TRUE, %QX10.1 bs[1] TRUE,
   %QX10.2 ss[0] still '', %QX10.3 tm.Q FALSE (never called).
   And the same index in a whole value read, in an instance whose
   input is set, in REF(): row := aa[REAL_TO_INT(r)] keeps row[0] 5
   (%QB11); fs[REAL_TO_INT(r)].d := 9 leaves fs[0].d 0 (%QB12);
   rf := REF(a[REAL_TO_INT(r)]) leaves rf NULL (%QX10.4).
   Flagged CONVERSION at the first, no fault.
   reference-skip: a statement left undone is iecf's own. *)
FUNCTION Inc : INT
  VAR_IN_OUT c : INT; END_VAR
  c := c + 1;
  Inc := c;
END_FUNCTION

FUNCTION Two : INT
  VAR_INPUT i : INT; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  VAR_EXTERNAL calls : INT; END_VAR
  calls := calls + 1;
  o := 2 + i;
  Two := 1;
END_FUNCTION

FUNCTION_BLOCK Cnt
  VAR_INPUT d : INT; END_VAR
  VAR_IN_OUT c : INT; END_VAR
  VAR_OUTPUT q : INT; s : STRING[4]; END_VAR
  q := q + 1;
  c := c + d;
  s := 'ab';
END_FUNCTION_BLOCK

FUNCTION_BLOCK Arr
  VAR_IN_OUT v : ARRAY[0..1] OF INT; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + 1;
  v[0] := v[0] + 1;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Use
  VAR_IN_OUT f : Cnt; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + 1;
END_FUNCTION_BLOCK

PROGRAM main
  VAR_EXTERNAL calls : INT; END_VAR
  VAR
    a : ARRAY[0..3] OF INT;
    aa : ARRAY[0..1] OF ARRAY[0..1] OF INT;
    ss : ARRAY[0..1] OF STRING[4];
    bs : ARRAY[0..3] OF BOOL := [4(TRUE)];
    r : REAL := 1.0E10;
    r2 : REAL := 3.0;
    x : INT;
    n : INT := 7;
    f : Cnt;
    fs : ARRAY[0..1] OF Cnt;
    g : Arr;
    u : Use;
    tm : TON;
    row : ARRAY[0..1] OF INT := [5, 6];
    rf : REF_TO INT;
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
    b0 AT %QX10.0 : BOOL;
    b1 AT %QX10.1 : BOOL;
    b2 AT %QX10.2 : BOOL;
    b3 AT %QX10.3 : BOOL;
    b4 AT %QX10.4 : BOOL;
    q11 AT %QB11 : SINT;
    q12 AT %QB12 : SINT;
  END_VAR
  n := Two(o => a[REAL_TO_INT(r)]);
  n := Inc(a[REAL_TO_INT(r) + 1]);
  n := Two(i := REAL_TO_INT(r), o => x);
  n := REAL_TO_INT(r) + Inc(a[2]);
  Two(o => a[REAL_TO_INT(r)]);
  SPLIT_DATE(IN := D#2024-03-05, YEAR => a[REAL_TO_INT(r)], MONTH => a[1]);
  fs[REAL_TO_INT(r)](d := 1, c := x);
  f(d := 1, c := a[REAL_TO_INT(r)]);
  f(EN := TRUE, d := 1, c := a[REAL_TO_INT(r)], ENO => bs[1]);
  f(d := 1, c := x, q => a[REAL_TO_INT(r)]);
  f(d := 1, c := x, s => ss[REAL_TO_INT(r)]);
  f(d := 1, c := x, ENO => bs[REAL_TO_INT(r)]);
  g(v := aa[REAL_TO_INT(r)]);
  u(f := fs[REAL_TO_INT(r)]);
  tm(IN := TRUE, PT := T#0s, Q => bs[REAL_TO_INT(r)]);
  f(d := 1, c := x, q => a[REAL_TO_INT(r2)]);
  f(d := 1, c := x);
  row := aa[REAL_TO_INT(r)];
  fs[REAL_TO_INT(r)].d := 9;
  rf := REF(a[REAL_TO_INT(r)]);
  q0 := INT_TO_SINT(a[0]);
  q1 := INT_TO_SINT(a[1]);
  q2 := INT_TO_SINT(a[2]);
  q3 := INT_TO_SINT(n);
  q4 := INT_TO_SINT(calls);
  q5 := INT_TO_SINT(f.q);
  q6 := INT_TO_SINT(x);
  q7 := INT_TO_SINT(fs[0].q);
  q8 := INT_TO_SINT(g.n + u.n);
  q9 := INT_TO_SINT(a[3]);
  b0 := bs[0];
  b1 := bs[1];
  b2 := ss[0] = '';
  b3 := tm.Q;
  b4 := rf = NULL;
  q11 := INT_TO_SINT(row[0]);
  q12 := INT_TO_SINT(fs[0].d);
END_PROGRAM

CONFIGURATION c
  VAR_GLOBAL calls : INT; END_VAR
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

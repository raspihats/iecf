(* A subrange as a FUNCTION's input, output, in-out and result, and as
   a FUNCTION_BLOCK's in-out; and what a subrange starts as: its lower
   limit (Ed.3 6.4.4.4.2), unless its type or declaration says.
   SRT is INT (2..5); S3 is INT (2..5) := 4. Worked by hand, the same
   at every tick:
   x : INT (1..5), y : SRT, z : S3, a : ARRAY OF SRT start as 1, 2, 4,
   2: x + y + z + a[2] is 9 (%QB0).
   Sum(p : INT (0..5), q : SRT, z : S3 := 3) adds them. Sum(), each
   input its initial value, 0 + 2 + 3: 5 (%QB1). Sum(1, 3, 4): 8 (%QB2).
   Res : SRT, set only when x > 0: Res(0) is its lower limit, 2 (%QB3);
   Res(4): 4 (%QB4).
   Out(x, o => s) sets its output o : SRT to x and gives it: with 3, 3
   (%QB5), and s is 3 (%QB6).
   From here a value is outside its subrange: clamped to the bound it
   passed and flagged CONVERSION, first where Sum's input p is
   declared, as Sum(9, 3, 4) gives it 9: 5 + 3 + 4, 12 (%QB7). Res(9): 5 (%QB8).
   Out(1, o => s): 2 (%QB9), and s is 2 (%QB10).
   An in-out is the caller's variable, a store through it clamped to
   the in-out's bounds: s := 4; Bump(s) adds 10, 5 (%QB11), and s is 5
   (%QB12). The block Hold takes 10 from its in-out: h(io := s) leaves
   s 2 (%QB13). n, an INT, given to Bump: n := 4 becomes 5 (%QB14,
   %QB15).
   reference-skip: a call with no arguments: refused; and a subrange
   starts at 0 there (Ed.3 6.4.4.4: its lower limit), not clamped into
   one. *)
TYPE
  SRT : INT (2..5);
  S3 : INT (2..5) := 4;
END_TYPE
FUNCTION Sum : INT
  VAR_INPUT p : INT (0..5); q : SRT; z : S3 := 3; END_VAR
  Sum := p + q + z;
END_FUNCTION
FUNCTION Res : SRT
  VAR_INPUT x : INT; END_VAR
  IF x > 0 THEN
    Res := x;
  END_IF;
END_FUNCTION
FUNCTION Out : INT
  VAR_INPUT x : INT; END_VAR
  VAR_OUTPUT o : SRT; END_VAR
  o := x;
  Out := o;
END_FUNCTION
FUNCTION Bump : INT
  VAR_IN_OUT io : SRT; END_VAR
  io := io + 10;
  Bump := io;
END_FUNCTION
FUNCTION_BLOCK Hold
  VAR_IN_OUT io : SRT; END_VAR
  io := io - 10;
END_FUNCTION_BLOCK
PROGRAM main
  VAR
    x : INT (1..5);
    y : SRT;
    z : S3;
    a : ARRAY [1..2] OF SRT;
    s : SRT;
    n : INT;
    h : Hold;
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
    q14 AT %QB14 : SINT;
    q15 AT %QB15 : SINT;
  END_VAR
  q0 := INT_TO_SINT(x + y + z + a[2]);
  q1 := INT_TO_SINT(Sum());
  q2 := INT_TO_SINT(Sum(1, 3, 4));
  q3 := INT_TO_SINT(Res(0));
  q4 := INT_TO_SINT(Res(4));
  n := 3;
  q5 := INT_TO_SINT(Out(x := n, o => s));
  q6 := INT_TO_SINT(s);
  q7 := INT_TO_SINT(Sum(9, 3, 4));
  q8 := INT_TO_SINT(Res(9));
  q9 := INT_TO_SINT(Out(x := 1, o => s));
  q10 := INT_TO_SINT(s);
  s := 4;
  q11 := INT_TO_SINT(Bump(s));
  q12 := INT_TO_SINT(s);
  h(io := s);
  q13 := INT_TO_SINT(s);
  n := 4;
  q14 := INT_TO_SINT(Bump(n));
  q15 := INT_TO_SINT(n);
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

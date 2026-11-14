(* FUNCTIONs with effects in the index of a variable given to an
   in-out, of a FUNCTION and of a block: each made once, in order, the
   element found before the call (an index out of bounds stops the
   statement first, docs/dependencies.md); a variable at an address or
   of a subrange given a copy, stored back after the call. Worked by
   hand, each tick from j 0:

   fadd(s[finc(j)], 20): j 1, s[1] 0 + 20 = 20 (40 at tick 1; inside
   0..50, no flag). fadd(la[finc(j)], 3): j 2, la[2] (Q1) 3, then 6.
   f(io := la[finc(j)], p := a[finc(j)]): j 3, then 4; la[3] (Q2) 1,
   then 2; a[4].y 1, then 2. From j 0 again: fp(a[finc(j)]): j 1, a[1].x
   10, then 20. finc(b[finc(j)]): j 1 again from 0, b[1] 1, then 2.
   Q4 s[1], Q5 a[4].y, Q6 a[1].x, Q7 b[1]; Q0 and Q3 untouched.
   reference-skip: VAR_IN_OUT of a FUNCTION, as `fnorder`. *)
TYPE small : SINT(0..50); END_TYPE
TYPE pt : STRUCT x : INT; y : INT; END_STRUCT; END_TYPE
FUNCTION finc : INT
  VAR_IN_OUT x : INT; END_VAR
  x := x + 1;
  finc := x;
END_FUNCTION
FUNCTION fadd : SINT
  VAR_IN_OUT x : SINT; END_VAR
  VAR_INPUT d : SINT; END_VAR
  x := x + d;
  fadd := x;
END_FUNCTION
FUNCTION fp : INT
  VAR_IN_OUT p : pt; END_VAR
  p.x := p.x + 10;
  fp := p.x;
END_FUNCTION
FUNCTION_BLOCK fb
  VAR_IN_OUT io : SINT; p : pt; END_VAR
  io := io + 1;
  p.y := p.y + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR j : INT; s : ARRAY [1..4] OF small; a : ARRAY [1..4] OF pt; b : ARRAY [1..4] OF INT; f : fb; r : INT; END_VAR
  VAR la AT %QB0 : ARRAY [1..4] OF SINT; END_VAR
  VAR q4 AT %QB4 : USINT; q5 AT %QB5 : USINT; q6 AT %QB6 : USINT; q7 AT %QB7 : USINT; END_VAR
  j := 0;
  r := SINT_TO_INT(fadd(s[finc(j)], 20));
  r := SINT_TO_INT(fadd(la[finc(j)], 3));
  f(io := la[finc(j)], p := a[finc(j)]);
  j := 0;
  r := fp(a[finc(j)]);
  j := 0;
  r := finc(b[finc(j)]);
  q4 := SINT_TO_USINT(s[1]);
  q5 := INT_TO_USINT(a[4].y);
  q6 := INT_TO_USINT(a[1].x);
  q7 := INT_TO_USINT(b[1]);
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

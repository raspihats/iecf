(* FUNCTIONs with effects in a block call: in the index of the target an
   output is taken into, and as a STRING input. Each is made once, in
   order; an output's target is found before the block runs (an index
   out of bounds stops the statement first), so a call there runs before
   the block does (docs/dependencies.md). Worked by hand, each tick from
   j 1:

   f(io := j, s := 'x', q => a[finc(j)]): finc makes j 2 and gives 2,
   the target a[2]; then the block, io j 2: q 2 + LEN('x') 1 = 3 into
   a[2] (Q0 03). f(io := j, s := fso(x := 'bb', y => j)): fso gives
   'bb' and sets j to 7; then the block, io 7: q 7 + 2 = 9 (Q1 09); j 7
   (Q2 07).
   fbeffects.iecf.trace: the reference compiler copies the in-out in
   before the other inputs are computed and back after the call, so
   fso's j is lost, and finds the output's target after the block runs
   (Q 02 04 02); iecf binds the in-out to j itself. *)
FUNCTION finc : INT
  VAR_IN_OUT x : INT; END_VAR
  x := x + 1;
  finc := x;
END_FUNCTION
FUNCTION fso : STRING
  VAR_INPUT x : STRING; END_VAR
  VAR_OUTPUT y : INT; END_VAR
  y := 7;
  fso := x;
END_FUNCTION
FUNCTION_BLOCK fb
  VAR_IN_OUT io : INT; END_VAR
  VAR_INPUT s : STRING; END_VAR
  VAR_OUTPUT q : INT; END_VAR
  q := io + LEN(s);
END_FUNCTION_BLOCK
PROGRAM main
  VAR f : fb; a : ARRAY [1..4] OF INT; j : INT; END_VAR
  VAR q AT %QB0 : USINT; q1 AT %QB1 : USINT; q2 AT %QB2 : USINT; END_VAR
  j := 1;
  f(io := j, s := 'x', q => a[finc(j)]);
  q := INT_TO_USINT(a[2]);
  f(io := j, s := fso(x := 'bb', y => j));
  q1 := INT_TO_USINT(f.q);
  q2 := INT_TO_USINT(j);
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

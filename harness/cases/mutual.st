(* Two blocks that name each other: b holds an a, and a takes a b as an
   in-out and through a reference, reads its output and calls its
   method. a's type is written before b's, which holds it; a's body
   after b's variables are known (docs/c-output.md §3). Worked by hand:

   tick 0: x(): total 0 + inner.n 0 + m 3 = 3 (Q2). y(other := x, r :=
   REF(x2)): n 0 + x.m 3 + x2.m 3 = 6 (Q0), and x2.poke(): 1 (Q4).
   tick 1: total 6; n 6 + 3 + 3 = 12 (0c 00); pokes 2.
   reference-skip: methods and REF_TO a block, as `refimage`. *)
FUNCTION_BLOCK a
  VAR_IN_OUT other : b; END_VAR
  VAR_INPUT r : REF_TO b; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + other.m;
  IF r <> NULL THEN n := n + r^.m; r^.poke(); END_IF;
END_FUNCTION_BLOCK
FUNCTION_BLOCK b
  VAR inner : a; END_VAR
  VAR_OUTPUT m : INT := 3; total : INT; pokes : INT; END_VAR
  METHOD PUBLIC poke
    pokes := pokes + 1;
  END_METHOD
  total := total + inner.n + m;
END_FUNCTION_BLOCK
PROGRAM main
  VAR x : b; x2 : b; y : a; END_VAR
  VAR q AT %QW0 : ARRAY [0..2] OF INT; END_VAR
  x();
  y(other := x, r := REF(x2));
  q[0] := y.n;
  q[1] := x.total;
  q[2] := x2.pokes;
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

(* A debug build (docs/c-output.md §10): the hook before each statement
   as it runs, with its POU, line and instance. A block's two
   instances say which is running; a FUNCTION has none; IF and FOR run
   their own line, then their statements'. Worked by hand, one tick:

   main 23 (c1()), then cnt's 17 and 18 in R.P.C1 (n 1, not > 100);
   main 24, then 17 and 18 in R.P.C2; main 25 (q := twice(c1.n)), then
   twice's 13, no instance; main 26 (the IF: c1.n is 1), then 27; main
   29 (the FOR), then its body, 30, for i 1 and for i 2. q is 2 + 1
   (Q0 03 00). *)
FUNCTION twice : INT
  VAR_INPUT x : INT; END_VAR
  twice := x * 2;
END_FUNCTION
FUNCTION_BLOCK cnt
  VAR_OUTPUT n : INT; END_VAR
  n := n + 1;
  IF n > 100 THEN n := 0; END_IF;
END_FUNCTION_BLOCK
PROGRAM main
  VAR c1, c2 : cnt; i, s2 : INT; END_VAR
  VAR q AT %QW0 : INT; END_VAR
  c1();
  c2();
  q := twice(c1.n);
  IF c1.n = 1 THEN
    q := q + 1;
  END_IF;
  FOR i := 1 TO 2 DO
    s2 := s2 + i;
  END_FOR;
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

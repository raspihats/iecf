(* A debug build (docs/c-output.md §10): the hook before each statement
   as it runs, with its POU, line and instance. A block's two
   instances say which is running; a FUNCTION has none; IF and FOR run
   their own line, then their statements'. Worked by hand, one tick:

   main 27 (c1()), then cnt's 21 and 22 in R.P.C1 (n 1, not > 100);
   main 28, then 21 and 22 in R.P.C2; main 29 (q := twice(c1.n)), then
   twice's 17, no instance; main 30 (the IF: c1.n is 1), then 31; main
   33 (the FOR), then its body, 34, for i 1 and for i 2, the FOR's 33
   before each later test (i 2, then i 3, which ends it); main 36 (the
   IF, s2 3 not > 10), 38 (its ELSIF, s2 = 3), 39; main 41 (the WHILE,
   i 3 > 2), 42, i 2, 41 again (not > 2: done); main 44 (the REPEAT),
   45, i 1, then its UNTIL 46 (1 < 2: done). q is 2 + 1 + 10 (Q0 0d
   00). *)
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
  IF s2 > 10 THEN
    q := 0;
  ELSIF s2 = 3 THEN
    q := q + 10;
  END_IF;
  WHILE i > 2 DO
    i := i - 1;
  END_WHILE;
  REPEAT
    i := i - 1;
  UNTIL i < 2
  END_REPEAT;
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

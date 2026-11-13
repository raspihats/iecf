(* REF_TO as a FUNCTION's and a method's input and output: passed as
   their value, as an interface is (docs/c-output.md §3). Worked by
   hand, i starting at 5, k at 50:

   tick 0. f(r := REF(i), o => p): f is i, 5, then i 6; d not given is
   NULL: 105 (Q0 69 00); p refers to i: 6 (Q2 06 00). b.m(REF(i)): 12
   (Q4 0c 00). f(r := p, d := NULL): i 6, then 7; d NULL: 106 (Q6 6a
   00). b.pick(REF(k), o => p2): k is 50 (Q8 32 00), and p2 refers to
   k, which g bumps through it: 51 (Q10 33 00).
   tick 1: i 7: 107, then 8 (Q0 6b 00, Q2 08 00), m 16 (10 00); 108
   (6c 00), i 9; k 51, then 52 (Q8 33 00, Q10 34 00).
   reference-skip: methods and REF_TO parameters, as `refmore`. *)
FUNCTION f : INT
  VAR_INPUT r : REF_TO INT; d : REF_TO INT; END_VAR
  VAR_OUTPUT o : REF_TO INT; END_VAR
  f := r^;
  r^ := r^ + 1;
  o := r;
  IF d = NULL THEN f := f + 100; END_IF;
END_FUNCTION
FUNCTION g : INT
  VAR_INPUT r : REF_TO INT; END_VAR
  r^ := r^ + 1;
  g := r^;
END_FUNCTION
FUNCTION_BLOCK blk
  METHOD PUBLIC m : INT
    VAR_INPUT r : REF_TO INT; END_VAR
    m := r^ * 2;
  END_METHOD
  METHOD PUBLIC pick : INT
    VAR_INPUT r : REF_TO INT; END_VAR
    VAR_OUTPUT o : REF_TO INT; END_VAR
    pick := r^;
    o := r;
  END_METHOD
END_FUNCTION_BLOCK
PROGRAM main
  VAR i : INT := 5; k : INT := 50; j : INT; p, p2 : REF_TO INT; b : blk; END_VAR
  VAR q AT %QW0 : INT; q2 AT %QW2 : INT; q3 AT %QW4 : INT; q4 AT %QW6 : INT; q5 AT %QW8 : INT; q6 AT %QW10 : INT; END_VAR
  j := f(r := REF(i), o => p);
  q := j;
  q2 := p^;
  q3 := b.m(REF(i));
  q4 := f(r := p, d := NULL);
  q5 := b.pick(r := REF(k), o => p2);
  q6 := g(p2);
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

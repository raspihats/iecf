(* A function block called with its arguments in order (Ed.3 6.6.3.4.1,
   the non-formal call): every input, output and in-out where it is
   declared, a base's first, none left off ("exactly the same number of
   parameters, in exactly the same order", 6.6.1.4.2). Worked by hand:

   tick 0. adder declares a, b, o, n, o2: f(1, 2, r, x, r2) gives a 1,
   b 2, o into r, the in-out n is x, o2 into r2: r 3 (Q0), r2 -1 (Q2 ff
   ff), x 1 (Q4). f(10, 20, r, x, r2): f.o 30 (Q6 1e 00), x 2. g, a
   wider, takes adder's a, b, o, n, o2, then its own c, p, s, o3: g(1,
   2, r, x, r2, 3, pt, 'abc', r3) gives o 3, x 3, o3 3 + 3 + 100 + 3 =
   109 (Q8 6d 00). g.twice(7, h): 14, h 3: 17 (Q10 11 00). TON's IN,
   PT, Q, ET: t(TRUE, T#20ms, done, el): not done (Q12 0); CTU's CU, R,
   PV, Q, CV: ctr(done, FALSE, 5, cq, cv): 0 (Q14).
   tick 1: x was 3: 4 after the first call (Q4 04 00); the rest as
   before.
   tick 2: x 7; t is done after 20 ms (Q12 01 00), and ctr counts its
   rising edge: 1 (Q14).
   tick 3: x 10; done stays, the count stays 1.
   reference-skip: methods, EXTENDS: refused. *)
TYPE PtT : STRUCT x : INT; y : INT; END_STRUCT; END_TYPE
FUNCTION_BLOCK adder
  VAR_INPUT a : INT; b : INT := 5; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  VAR_IN_OUT n : INT; END_VAR
  VAR_OUTPUT o2 : INT; END_VAR
  o := a + b;
  n := n + 1;
  o2 := a - b;
END_FUNCTION_BLOCK
FUNCTION_BLOCK wider EXTENDS adder
  VAR_INPUT c : INT; p : PtT; s : STRING[4]; END_VAR
  VAR_OUTPUT o3 : INT; END_VAR
  METHOD PUBLIC twice : INT
    VAR_INPUT v : INT; END_VAR
    VAR_OUTPUT half : INT; END_VAR
    twice := v * 2;
    half := v / 2;
  END_METHOD
  SUPER();
  o3 := o + c + p.x + LEN(s);
END_FUNCTION_BLOCK
PROGRAM main
  VAR f : adder; g : wider; t : TON; ctr : CTU; x : INT; r, r2, r3, m, h : INT; pt : PtT := (x := 100); el : TIME; done, cq : BOOL; cv : INT; END_VAR
  VAR q AT %QW0 : ARRAY [0..7] OF INT; END_VAR
  f(1, 2, r, x, r2);
  q[0] := r; q[1] := r2; q[2] := x;
  f(10, 20, r, x, r2);
  q[3] := f.o;
  g(1, 2, r, x, r2, 3, pt, 'abc', r3);
  q[4] := r3;
  m := g.twice(7, h);
  q[5] := m + h;
  t(TRUE, T#20ms, done, el);
  q[6] := BOOL_TO_INT(done);
  ctr(done, FALSE, 5, cq, cv);
  q[7] := ctr.CV;
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION

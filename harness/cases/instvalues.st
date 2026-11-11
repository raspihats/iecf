(* Function block instances as values (Ed.3 Table 72 1d, Table 42 9a,
   6.6.1.4 rule 4; docs/dependencies.md): an instance assigned is copied
   whole, every variable of it; one given as an input is a copy, read
   in the POU it is given to.
   CNT adds inc to n at each call. Worked by hand, k %IB0 1, 2, 3:
   a(inc := k): a.n 1, 3, 6.
   b := a; b(): b has a's n and inc, then adds: 2, 5, 9 (%QB0); a is as
   it was: 1, 3, 6 (%QB1).
   h.c := a; h2 := h, a structure holding an instance; h2.c() adds k:
   h2.c.n 2, 5, 9 (%QB2); h.c.n 1, 3, 6 (%QB3).
   arr[1] := a; arr2 := arr, an array of instances: arr2[1].n 1, 3, 6
   (%QB4); arr2[2].n 0 (%QB5).
   PEEK takes a CNT as an input and gives n * 10 + inc: PEEK(a) 11, 32,
   63 (%QB6); PEEK() has the instance's initial values, n 0, inc 1: 1
   (%QB7).
   WATCH takes a CNT as an input, c, and has one as an output, own,
   which it calls with inc := 5. w(c := a, own => d): seen := c.n, 1,
   3, 6 (%QB8); d is a copy of own: d.n 5, 10, 15 (%QB9); the copy kept
   in w, w.c.inc, is k: 1, 2, 3 (%QB10).
   A standard block: t1(IN := TRUE, PT := T#15ms), ticks 10 ms apart:
   ET 0, 10, 20 ms, so Q at tick 2; t2 := t1 has it: %QX11.0 0, 0, 1.
   An input set apart from a call: w.c := b; w.c.n 2, 5, 9 (%QB12).
   A FUNCTION's result an instance: MAKE(amt) sets its result's inc and
   calls it once; e := MAKE(k): e.n 1, 2, 3 (%QB13).
   A FUNCTION's own instance and its output one keep nothing from call
   to call: WORK calls c twice with inc 4, c.n 8, its result (%QB14),
   and o once with inc 7; WORK(o => f): f.n 7 at every tick (%QB15).
   Every construct here is Ed.2's too. *)
FUNCTION_BLOCK CNT
  VAR_INPUT inc : INT := 1; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + inc;
END_FUNCTION_BLOCK
TYPE HOLD : STRUCT c : CNT; k : INT; END_STRUCT; END_TYPE
FUNCTION PEEK : INT
  VAR_INPUT c : CNT; END_VAR
  PEEK := c.n * 10 + c.inc;
END_FUNCTION
FUNCTION MAKE : CNT
  VAR_INPUT amt : INT; END_VAR
  MAKE.inc := amt;
  MAKE();
END_FUNCTION
FUNCTION WORK : INT
  VAR c : CNT; END_VAR
  VAR_OUTPUT o : CNT; END_VAR
  c(inc := 4);
  c();
  o(inc := 7);
  WORK := c.n;
END_FUNCTION
FUNCTION_BLOCK WATCH
  VAR_INPUT c : CNT; END_VAR
  VAR_OUTPUT seen : INT; own : CNT; END_VAR
  seen := c.n;
  own(inc := 5);
END_FUNCTION_BLOCK
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR
    a, b, d, e, f : CNT;
    h, h2 : HOLD;
    arr, arr2 : ARRAY [1..2] OF CNT;
    w : WATCH;
    t1, t2 : TON;
  END_VAR
  VAR k AT %IB0 : SINT; END_VAR
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
    q11 AT %QX11.0 : BOOL;
    q12 AT %QB12 : SINT;
    q13 AT %QB13 : SINT;
    q14 AT %QB14 : SINT;
    q15 AT %QB15 : SINT;
  END_VAR
  a(inc := k);
  b := a;
  b();
  q0 := INT_TO_SINT(b.n);
  q1 := INT_TO_SINT(a.n);
  h.c := a;
  h2 := h;
  h2.c();
  q2 := INT_TO_SINT(h2.c.n);
  q3 := INT_TO_SINT(h.c.n);
  arr[1] := a;
  arr2 := arr;
  q4 := INT_TO_SINT(arr2[1].n);
  q5 := INT_TO_SINT(arr2[2].n);
  q6 := INT_TO_SINT(PEEK(a));
  q7 := INT_TO_SINT(PEEK());
  w(c := a, own => d);
  q8 := INT_TO_SINT(w.seen);
  q9 := INT_TO_SINT(d.n);
  q10 := INT_TO_SINT(w.c.inc);
  t1(IN := TRUE, PT := T#15ms);
  t2 := t1;
  q11 := t2.Q;
  w.c := b;
  q12 := INT_TO_SINT(w.c.n);
  e := MAKE(k);
  q13 := INT_TO_SINT(e.n);
  q14 := INT_TO_SINT(WORK(o => f));
  q15 := INT_TO_SINT(f.n);
END_PROGRAM

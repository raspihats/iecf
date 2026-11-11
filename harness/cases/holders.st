(* A structure or an array that holds function block instances, as a
   value everywhere an instance is one (docs/dependencies.md): a block's
   and a FUNCTION's input, output, in-out and result, an ARRAY[*] of
   instances, and a PROGRAM's connections of an array, a structure and
   an instance.
   CNT adds inc to n at each call; its inc stays as last given.
   Worked by hand, k %IB0 1, 2, 3:
   a(inc := k): a.n 1, 3, 6. h.c := a; h.k := k; arr[2] := a.
   f(h := h, cs := arr, io := h2, ios := arr2, o => h3):
     sum := h.c.n + cs[2].n + h.k: 3, 8, 15 (%QB0);
     o.c(inc := 3), o kept in f: 3, 6, 9, copied to h3 (%QB1); o.k :=
     sum, h3.k 3, 8, 15 (%QB2);
     io.c() is h2.c: at tick 0 its inc is 1, n 1; FNH below then calls
     it with inc := 10, which stays: 21 at tick 1, 41 at tick 2 (%QB3);
     ios[1](inc := 2) is arr2[1]: 2, 4, 6 (%QB4).
   r := FNH(h := h, o => o2, io := h2, fx := arr2):
     FNH := h; FNH.c() adds h.c's inc, k, to a.n: 2, 5, 9 (%QB5); FNH.k
     := h.k + 1: 2, 3, 4 (%QB6);
     o is set up at each call: o.c(inc := 4) 4 (%QB7); o.k := h.c.n 1,
     3, 6 (%QB8);
     io.c(inc := 10) on h2.c: 11, 31, 51 (%QB9);
     fx[UPPER_BOUND(fx, 1)]() is arr2[2]: 1, 2, 3 (%QB10).
   Connections, made before the program runs and after it:
     pa := ga, [10, 20, 30]; oa := pa; oa[2] := oa[2] + k; oa => gb,
     read at the next tick: gb[2] 0, 21, 22 (%QB11);
     pc := gc, and the program calls gc itself after: pc.n 0, 1, 2
     (%QB12); oc(inc := 5) => gd, at the next tick 0, 5, 10 (%QB13);
     ph := gh, the program then sets gh.k := k: ph.k 0, 1, 2 (%QB14);
     oh := ph; oh.k := ph.k + 100; oh => gi, at the next tick 0, 100,
     101 (%QB15).
   reference-skip: the reference compiler has no instance in a
   structure (tried in CI on instvalues). *)
FUNCTION_BLOCK CNT
  VAR_INPUT inc : INT := 1; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + inc;
END_FUNCTION_BLOCK
TYPE HOLD : STRUCT c : CNT; k : INT; END_STRUCT; END_TYPE
FUNCTION_BLOCK FBH
  VAR_INPUT h : HOLD; cs : ARRAY [1..2] OF CNT; END_VAR
  VAR_OUTPUT o : HOLD; sum : INT; END_VAR
  VAR_IN_OUT io : HOLD; ios : ARRAY [1..2] OF CNT; END_VAR
  sum := h.c.n + cs[2].n + h.k;
  o.c(inc := 3);
  o.k := sum;
  io.c();
  ios[1](inc := 2);
END_FUNCTION_BLOCK
FUNCTION FNH : HOLD
  VAR_INPUT h : HOLD; END_VAR
  VAR_OUTPUT o : HOLD; END_VAR
  VAR_IN_OUT io : HOLD; fx : ARRAY [*] OF CNT; END_VAR
  FNH := h;
  FNH.c();
  FNH.k := h.k + 1;
  o.c(inc := 4);
  o.k := h.c.n;
  io.c(inc := 10);
  fx[UPPER_BOUND(fx, 1)]();
END_FUNCTION
CONFIGURATION c
  VAR_GLOBAL
    ga : ARRAY [1..3] OF INT := [10, 20, 30];
    gb : ARRAY [1..3] OF INT;
    gh, gi : HOLD;
    gc, gd : CNT;
  END_VAR
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main(pa := ga, ph := gh, pc := gc, oa => gb, oh => gi, oc => gd);
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR_INPUT pa : ARRAY [1..3] OF INT; ph : HOLD; pc : CNT; END_VAR
  VAR_OUTPUT oa : ARRAY [1..3] OF INT; oh : HOLD; oc : CNT; END_VAR
  VAR_EXTERNAL
    gb : ARRAY [1..3] OF INT;
    gh, gi : HOLD;
    gc, gd : CNT;
  END_VAR
  VAR
    a : CNT;
    h, h2, h3, r, o2 : HOLD;
    arr, arr2 : ARRAY [1..2] OF CNT;
    f : FBH;
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
    q11 AT %QB11 : SINT;
    q12 AT %QB12 : SINT;
    q13 AT %QB13 : SINT;
    q14 AT %QB14 : SINT;
    q15 AT %QB15 : SINT;
  END_VAR
  a(inc := k);
  h.c := a;
  h.k := k;
  arr[2] := a;
  f(h := h, cs := arr, io := h2, ios := arr2, o => h3);
  q0 := INT_TO_SINT(f.sum);
  q1 := INT_TO_SINT(h3.c.n);
  q2 := INT_TO_SINT(h3.k);
  q3 := INT_TO_SINT(h2.c.n);
  q4 := INT_TO_SINT(arr2[1].n);
  r := FNH(h := h, o => o2, io := h2, fx := arr2);
  q5 := INT_TO_SINT(r.c.n);
  q6 := INT_TO_SINT(r.k);
  q7 := INT_TO_SINT(o2.c.n);
  q8 := INT_TO_SINT(o2.k);
  q9 := INT_TO_SINT(h2.c.n);
  q10 := INT_TO_SINT(arr2[2].n);
  q11 := INT_TO_SINT(gb[2]);
  oa := pa;
  oa[2] := oa[2] + k;
  q12 := INT_TO_SINT(pc.n);
  gc();
  q13 := INT_TO_SINT(gd.n);
  oc(inc := 5);
  q14 := INT_TO_SINT(ph.k);
  gh.k := k;
  q15 := INT_TO_SINT(gi.k);
  oh := ph;
  oh.k := ph.k + 100;
END_PROGRAM

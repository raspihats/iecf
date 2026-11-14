(* RETAIN references (W0006): kept as they are across a warm restart,
   as CODESYS and TwinCAT keep a pointer in retained memory. Worked by
   hand:

   tick 0: r, a[1] and the global gr are taken, all to x, and n.r to
   n.v; cnt 1. x 1 + 100 + 1000 = 1101 (Q0 4d 04); n.v 10. The blocks h
   and hk each take REF(gx), fresh 1, and count it: gx 2. q2 10 + 2 =
   12 (Q4 0c 00).
   tick 1: x 2202 (9a 08); n.v 20, gx 4: 24 (18 00).
   warm, before tick 2: every reference is RETAIN and still refers to
   what it did; x, n, cnt and gx are RETAIN too. h starts again but for
   its RETAIN r: its fresh is 0 and stays (Q6 00); hk is kept whole
   (Q7 01).
   tick 2: x 3303 (e7 0c), cnt still 1; n.v 30, gx 6: 36 (24 00).
   reference-skip: REF_TO in a structure (its -R); no warm restart. *)
TYPE Node : STRUCT r : REF_TO INT; v : INT; END_STRUCT; END_TYPE
FUNCTION_BLOCK holder
  VAR RETAIN r : REF_TO INT; END_VAR
  VAR_OUTPUT fresh : USINT; END_VAR
  VAR_EXTERNAL gx : INT; END_VAR
  IF r = NULL THEN r := REF(gx); fresh := fresh + 1; END_IF;
  r^ := r^ + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_EXTERNAL gx : INT; gr : REF_TO INT; END_VAR
  VAR RETAIN r : REF_TO INT; x : INT; n : Node; a : ARRAY [0..1] OF REF_TO INT; cnt : USINT; END_VAR
  VAR h : holder; END_VAR
  VAR RETAIN hk : holder; END_VAR
  VAR q AT %QW0 : INT; q1 AT %QB2 : USINT; q2 AT %QW4 : INT; q3 AT %QB6 : USINT; q4 AT %QB7 : USINT; END_VAR
  IF r = NULL THEN
    r := REF(x); n.r := REF(n.v); a[1] := REF(x); gr := REF(x);
    cnt := cnt + 1;
  END_IF;
  r^ := r^ + 1;
  n.r^ := n.r^ + 10;
  a[1]^ := a[1]^ + 100;
  gr^ := gr^ + 1000;
  h();
  hk();
  q := x;
  q1 := cnt;
  q2 := n.v + gx;
  q3 := h.fresh;
  q4 := hk.fresh;
END_PROGRAM
CONFIGURATION plant
  VAR_GLOBAL RETAIN gx : INT; gr : REF_TO INT; END_VAR
  RESOURCE cpu ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

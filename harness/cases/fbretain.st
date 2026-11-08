(* RETAIN inside a FUNCTION_BLOCK (Ed.3 6.5.6): at a warm restart the
   block's own RETAIN variable keeps its value, its NON_RETAIN one is set
   anew, and one that is neither keeps its value only in an instance
   declared RETAIN. A block in a block follows its own type and the
   section it is declared in, not the outer instance. An instance of a
   block that keeps anything is kept whole (docs/c-output.md §7).
   Each counter counts from 10, 20, 30; at tick 3 a warm restart.
   The reference compiler has no warm restart of its own; its host starts everything
   anew. iecf's trace is fbretain.iecf.trace. *)
FUNCTION_BLOCK cnt
VAR_OUTPUT
  ko : INT;
  po : INT;
  no : INT;
END_VAR
VAR RETAIN
  k : INT := 10;
END_VAR
VAR
  p : INT := 20;
END_VAR
VAR NON_RETAIN
  n : INT := 30;
END_VAR
k := k + 1;
p := p + 1;
n := n + 1;
ko := k;
po := p;
no := n;
END_FUNCTION_BLOCK

FUNCTION_BLOCK wrap
VAR_OUTPUT
  ck, cp, cn, dk, dp, dn : INT;
END_VAR
VAR
  c : cnt;
END_VAR
VAR RETAIN
  cr : cnt;
END_VAR
c();
cr();
ck := c.ko;
cp := c.po;
cn := c.no;
dk := cr.ko;
dp := cr.po;
dn := cr.no;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  a_k AT %MW0 : INT;
  a_p AT %MW2 : INT;
  a_n AT %MW4 : INT;
  b_k AT %MW6 : INT;
  b_p AT %MW8 : INT;
  b_n AT %MW10 : INT;
  c_k AT %MW12 : INT;
  c_p AT %MW14 : INT;
  c_n AT %MW16 : INT;
  d_k AT %MW18 : INT;
  d_p AT %MW20 : INT;
  d_n AT %MW22 : INT;
END_VAR
VAR
  a : cnt;
  w : wrap;
END_VAR
VAR RETAIN
  b : cnt;
END_VAR
a();
b();
w();
a_k := a.ko;
a_p := a.po;
a_n := a.no;
b_k := b.ko;
b_p := b.po;
b_n := b.no;
c_k := w.ck;
c_p := w.cp;
c_n := w.cn;
d_k := w.dk;
d_p := w.dp;
d_n := w.dn;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

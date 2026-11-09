(* EN on a call of a FUNCTION with effects (Ed.3 6.6.1.5): an in-out and
   an output (bump), a VAR_EXTERNAL written (tick), no result (addw).
   EN FALSE: the function is not called - its in-outs, outputs and
   globals are left as they were - ENO is FALSE, and the statement is
   left undone; ENO is FALSE too when the call's own operation raised a
   flag; not when storing its outputs does (an output clamped into a
   subrange).
   Worked by hand, v 0, r -1, t -1, w 32766, g 0 at the start:
   u 0, s 0:
   Tick 0, run TRUE: bump: v 2, t 4, r 2; tick: g 1, qg 1; addw: w 32767,
   qok TRUE; bump: u 2, s 4, qok2 TRUE.
   Tick 1, run FALSE: nothing called: v 2, r 2, t 4, qg 1, w 32767, u 2,
   s 4; qok, qok2 FALSE.
   Tick 2, run TRUE: v 4, t 8, r 4; g 2, qg 2; addw: w 32767 + 1 wraps to
   -32768, flag OVERFLOW in addw on line 39, qok FALSE; bump: u 4, its
   output 8 clamped to s 5 (flagged too, after the first), qok2 TRUE.
   reference-skip: the reference compiler (Ed.2) refuses VAR_EXTERNAL in
   a FUNCTION. *)
TYPE small : INT (0..5); END_TYPE

FUNCTION bump : INT
VAR_IN_OUT x : INT; END_VAR
VAR_INPUT d : INT; END_VAR
VAR_OUTPUT twice : INT; END_VAR
x := x + d;
twice := x * 2;
bump := x;
END_FUNCTION

FUNCTION tick : INT
VAR_EXTERNAL g : INT; END_VAR
g := g + 1;
tick := g;
END_FUNCTION

FUNCTION addw
VAR_IN_OUT x : INT; END_VAR
VAR_INPUT d : INT; END_VAR
x := x + d;
END_FUNCTION

PROGRAM main
VAR
  run AT %IX0.0 : BOOL;
  qv AT %QW0 : INT;
  qr AT %QW2 : INT;
  qt AT %QW4 : INT;
  qg AT %QW6 : INT;
  qok AT %QX8.0 : BOOL;
  qw AT %QW10 : INT;
  qok2 AT %QX8.1 : BOOL;
  qs AT %QW12 : INT;
END_VAR
VAR
  v : INT;
  r : INT := -1;
  t : INT := -1;
  w : INT := 32766;
  u, r2 : INT;
  s : small;
END_VAR
r := bump(EN := run, x := v, d := 2, twice => t);
qg := tick(EN := run);
addw(EN := run, x := w, d := 1, ENO => qok);
r2 := bump(EN := run, x := u, d := 2, twice => s, ENO => qok2);
qv := v;
qr := r;
qt := t;
qw := w;
qs := s;
END_PROGRAM

CONFIGURATION Plant
  VAR_GLOBAL g : INT; END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

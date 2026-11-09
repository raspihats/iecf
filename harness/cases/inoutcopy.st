(* An in-out given a variable at an address or of a subrange (Ed.3
   6.6.1.6 rule 6): given a copy, read before the call and stored back
   after it as an output is - clamped into the subrange and flagged
   CONVERSION. Siemens passes an elementary in-out so, copied in and
   back. To a FUNCTION (bump) and a FUNCTION_BLOCK (dbl), an element by
   a computed index too.
   Worked by hand, qw, qf 0, s 3, a[1] 0, t 1 at the start:
   Tick 0: qw 5, k 5; s 7, k 12; a[1] 2, k 14; qf 1; t 3.
   Tick 1: qw 10, k 10; bump gives 11, k 21, s stored back clamped to 9,
   flag CONVERSION on line 48; a[1] 4, k 25; qf 3; t 7.
   Tick 2: qw 15, k 15; s 13 clamped to 9, k 28; a[1] 6, k 34; qf 7;
   t 15 clamped to 9; the flag stays tick 1's.
   reference-skip: the reference compiler's C for an array element given
   as an in-out does not compile ("assignment to expression with array
   type"). *)
TYPE small : INT (0..9); END_TYPE

FUNCTION bump : INT
VAR_IN_OUT v : INT; END_VAR
VAR_INPUT d : INT; END_VAR
v := v + d;
bump := v;
END_FUNCTION

FUNCTION_BLOCK dbl
VAR_IN_OUT v : INT; END_VAR
v := v * 2 + 1;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  qw AT %QW0 : INT;
  qs AT %QW2 : INT;
  qa AT %QW4 : INT;
  qk AT %QW6 : INT;
  qf AT %QW8 : INT;
  qt AT %QW10 : INT;
END_VAR
VAR
  s : small := 3;
  a : ARRAY[0..2] OF small;
  i : INT := 1;
  t : small := 1;
  k : INT;
  d : dbl;
END_VAR
k := bump(v := qw, d := 5);
k := k + bump(v := s, d := 4);
k := k + bump(v := a[i], d := 2);
d(v := qf);
d(v := t);
qs := s;
qa := a[1];
qt := t;
qk := k;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

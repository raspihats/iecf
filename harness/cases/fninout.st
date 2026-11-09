(* A FUNCTION's in-outs (Ed.3 6.6.2.2, rule 6): an in-out reads and
   writes the variable the call passes, two at once in swap; an in-out
   passed on to another function's in-out, in twice. Externals are in
   fnorder: the reference compiler (Ed.2) refuses VAR_EXTERNAL in a
   FUNCTION. *)
FUNCTION swap : BOOL
VAR_IN_OUT a, b : INT; END_VAR
VAR t : INT; END_VAR
t := a;
a := b;
b := t;
swap := a > b;
END_FUNCTION

FUNCTION bump : INT
VAR_INPUT n : INT; END_VAR
VAR_IN_OUT v : INT; END_VAR
v := v + n * 3;
bump := v;
END_FUNCTION

FUNCTION twice : INT
VAR_IN_OUT w : INT; END_VAR
VAR k : INT; END_VAR
k := bump(n := 1, v := w);
twice := bump(n := 1, v := w);
END_FUNCTION

PROGRAM main
VAR
  vin AT %IW0 : INT;
  bin AT %IW2 : INT;
  qx AT %QW0 : INT;
  qy AT %QW2 : INT;
  qk AT %QW4 : INT;
  qok AT %QX6.0 : BOOL;
END_VAR
VAR
  x, y, k : INT;
  ok : BOOL;
END_VAR
x := vin;
y := bin;
ok := swap(a := x, b := y);
qok := ok;
k := bump(n := 2, v := x);
k := twice(w := y);
qx := x;
qy := y;
qk := k;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

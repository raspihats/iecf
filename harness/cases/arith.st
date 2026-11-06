(* Integer arithmetic of every width, in range: + - * / MOD, negation,
   ABS, MIN, MAX, LIMIT, and division and MOD of negative numbers. *)
PROGRAM main
VAR
  a AT %IW0 : INT;
  b AT %IW2 : INT;
  r_add AT %MW0 : INT;
  r_sub AT %MW2 : INT;
  r_mul AT %MD4 : DINT;
  r_div AT %MW8 : INT;
  r_mod AT %MW10 : INT;
  r_neg AT %MW12 : INT;
  r_abs AT %MW14 : INT;
  r_min AT %MW16 : INT;
  r_max AT %MW18 : INT;
  r_lim AT %MW20 : INT;
  s AT %MB22 : SINT;
  us AT %MB23 : USINT;
  ui AT %MW24 : UINT;
  ud AT %MD26 : UDINT;
  l AT %ML30 : LINT;
  ul AT %ML38 : ULINT;
END_VAR
r_add := a + b;
r_sub := a - b;
r_mul := INT_TO_DINT(a) * INT_TO_DINT(b);
IF b <> 0 THEN
  r_div := a / b;
  r_mod := a MOD b;
END_IF;
r_neg := -a;
r_abs := ABS(a);
r_min := MIN(a, b, 7);
r_max := MAX(a, b, -7);
r_lim := LIMIT(-100, a * 3, 100);
s := INT_TO_SINT(a) - SINT#3;
us := USINT#200 + USINT#5;
ui := UINT#60000 / UINT#7;
ud := UDINT#4000000000 - UDINT#1;
l := INT_TO_LINT(a) * LINT#1000000000000;
ul := ULINT#18000000000000000000 + INT_TO_ULINT(ABS(b));
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

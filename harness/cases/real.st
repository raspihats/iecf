(* REAL and LREAL: arithmetic, comparisons, conversions to and from the
   integers, and the functions of Ed.3 Table 28 whose results are exact in
   binary floating point. REAL_TO_INT rounds halves to even, as Ed.3
   Table 22's examples have it (2.5 to 2, -2.5 to -2).

   Truncation is the deprecated TRUNC (Ed.3 Table 22 2a; W0003 in iecf),
   the one form the reference compiler also knows, into a DINT: the reference compiler types it by its
   target, iecf gives DINT as the vendors do (docs/dependencies.md). *)
PROGRAM main
VAR
  inp AT %IW0 : INT;
  r_add AT %MD0 : REAL;
  r_mul AT %MD4 : REAL;
  r_div AT %ML8 : LREAL;
  r_round AT %MW16 : INT;
  r_trunc AT %MD38 : DINT;
  r_sqrt AT %MD20 : REAL;
  r_abs AT %ML24 : LREAL;
  r_cmp AT %MB32 : BYTE;
  r_lr AT %MD34 : REAL;
END_VAR
VAR
  x : REAL;
  y : LREAL;
END_VAR
x := INT_TO_REAL(inp) / 4.0;
y := INT_TO_LREAL(inp) * 0.5;
r_add := x + 1.5;
r_mul := x * -2.0;
r_div := y / 3.0;
r_round := REAL_TO_INT(x);
r_trunc := TRUNC(x);
r_sqrt := SQRT(ABS(x) * ABS(x));
r_abs := ABS(y - 100.0);
r_cmp := 0;
IF x > 1.0 THEN r_cmp := r_cmp OR 1; END_IF;
IF y <= -1.0 THEN r_cmp := r_cmp OR 2; END_IF;
IF x = 2.5 THEN r_cmp := r_cmp OR 4; END_IF;
r_lr := LREAL_TO_REAL(y) + 0.25;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

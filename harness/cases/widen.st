(* Implicit conversions (Ed.3 6.6.1.6, Figure 11): they widen and keep
   the value - in an assignment, to a block's input and from its output
   (=>), to a function's input, and between the operands of an operator,
   a comparison and MAX; an operation done in its operands' type and
   widened after (Example 3), so INT * INT overflows as an INT even into
   a DINT: at tick 1 30000 * 2 is -5536, flagged.
   Worked by hand: tick 0, vin -3, uin 40000, bin 16#FF, sbin -1: 39997,
   -1, 16#01FF, -1.5, -6, 40001, 200, -4, TRUE, -1. Tick 1, vin 30000,
   uin 2, bin 16#80, sbin 127: 30002, 127, 16#0180, 15000.0, -5536
   (flag), 3, 200, 30127, FALSE, 30000.
   reference-skip: the reference compiler has no implicit conversions
   (Ed.2). *)
FUNCTION_BLOCK pass
VAR_INPUT x : DINT; END_VAR
VAR_OUTPUT y : DINT; o : USINT; END_VAR
y := x + 1;
o := 200;
END_FUNCTION_BLOCK

FUNCTION addl : LINT
VAR_INPUT a : LINT; b : LREAL; END_VAR
addl := a + LREAL_TO_LINT(b);
END_FUNCTION

PROGRAM main
VAR
  vin AT %IW0 : INT;
  uin AT %IW2 : UINT;
  bin AT %IB4 : BYTE;
  sbin AT %IB5 : SINT;
  qd AT %QD0 : DINT;
  ql AT %QL4 : LINT;
  qw AT %QW12 : WORD;
  qr AT %QD14 : REAL;
  qx AT %QD18 : DINT;
  qy AT %QD22 : DINT;
  qo AT %QW26 : INT;
  qf AT %QL28 : LINT;
  qb AT %QX36.0 : BOOL;
  qm AT %QD38 : DINT;
END_VAR
VAR
  d : DINT;
  f : pass;
END_VAR
d := vin;
qd := d + uin;
ql := sbin;
qw := bin OR WORD#16#0100;
qr := vin / REAL#2.0;
qx := vin * 2;
f(x := uin, y => qy, o => qo);
qf := addl(sbin, vin);
qb := d < uin;
qm := MAX(sbin, d);
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

(* A FUNCTION's outputs as the reference compiler does not take them:
   negated (NOT neg =>), widened into a DINT and a LINT (Ed.3 6.6.1.6,
   rule 5), given in order after the inputs (6.6.1.4), into an array
   element by a computed index; and a call that faults: divmod(1, 0)
   divides by zero, the task stops, and nothing is stored - its output
   and its result are both for qs, which keeps the 5 it was just given.
   An output is stored before the statement's own target: qs := divmod(
   ..., r => qs) leaves qs the result. split10 has no result (Ed.3
   6.6.2.1), called as a statement, in order: its input and in-out, then
   its outputs.
   Worked by hand: tick 0, vin 7, bin 1: qd 2, ql 1, qp TRUE, arr[3] 2,
   qa 2000, qk 111, qs 1, qh 0, qlo 7, qn 1. Tick 1, vin -7, bin 0: qd -2, ql -1, qp FALSE,
   arr[0] 2, qa 2002, qk 111, qh 0, qlo -7, qn 2, qs 5, fault
   DIVISION_BY_ZERO in DIVMOD.
   reference-skip: the reference compiler crashes on NOT q =>, has no
   implicit conversions, its C for an output into an array element
   does not compile, and Ed.2 has no FUNCTION without a result. *)
FUNCTION divmod : INT
VAR_INPUT a, b : INT; END_VAR
VAR_OUTPUT q : INT; r : INT := -1; neg : BOOL; END_VAR
q := a / b;
r := a MOD b;
neg := q < 0;
divmod := 1;
END_FUNCTION

FUNCTION split10
VAR_INPUT v : INT; END_VAR
VAR_OUTPUT hi, lo : INT; END_VAR
VAR_IN_OUT calls : INT; END_VAR
hi := v / 10;
lo := v MOD 10;
calls := calls + 1;
END_FUNCTION

PROGRAM main
VAR
  vin AT %IW0 : INT;
  bin AT %IW2 : INT;
  qd AT %QD0 : DINT;
  ql AT %QL4 : LINT;
  qp AT %QX12.0 : BOOL;
  qa AT %QW14 : INT;
  qk AT %QW16 : INT;
  qs AT %QW18 : INT;
  qh AT %QW20 : INT;
  qlo AT %QW22 : INT;
  qn AT %QW24 : INT;
END_VAR
VAR
  i, n : INT;
  arr : ARRAY[0..3] OF INT;
END_VAR
qk := divmod(a := vin, b := 3, q => qd, NOT neg => qp);
qk := qk + divmod(vin, 4, ql) * 10;
i := vin MOD 4;
IF i < 0 THEN i := 0; END_IF;
qk := qk + divmod(a := 9, b := 4, q => arr[i]) * 100;
qa := arr[0] + arr[1] * 10 + arr[2] * 100 + arr[3] * 1000;
split10(vin, n, qh, qlo);
qn := n;
qs := 5;
qs := divmod(a := 1, b := bin, r => qs);
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

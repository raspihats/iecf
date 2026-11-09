(* A FUNCTION's externals (Ed.3 6.6.2.2, rules 4-5), and the order of
   evaluation once a call has effects (7.3.2, rule 3: "the leftmost
   operand shall be evaluated first"):
   - x + bump(1, x): x read before the call changes it;
   - bump(1, y) * 10 + y: y read after it;
   - bump(1, w) - bump(2, w): the left call first;
   - peek() + bump(1, w) + peek(): total read, written, read again;
   - x + mark(q => x): x read before the call's output is stored in it;
   - 100 / zin + mark(q => qm): the division first, so at tick 1, zin 0,
     it faults before mark runs, and qm, cleared just before, stays 0.
   An in-out given an element by a computed index, arr[i].
   Worked by hand: tick 0, vin 2, zin 1: qa 7, qb 55, qc -6, qd 15,
   qt 11, qp 23 (5 + 12 + 6), qe 101, qm 1. Tick 1, vin -3, zin 0: qa -3,
   qb 0, qc -6, qd 15, qt 22, qp 45 (16 + 12 + 17), qe 101, qm 0, fault
   DIVISION_BY_ZERO; total grows by every bump's n, 11 a tick. qf is
   vin + 1: 3, then -2.
   reference-skip: the reference compiler (Ed.2) refuses VAR_EXTERNAL in
   a FUNCTION. *)
FUNCTION bump : INT
VAR_INPUT n : INT; END_VAR
VAR_IN_OUT v : INT; END_VAR
VAR_EXTERNAL total : DINT; END_VAR
VAR_EXTERNAL CONSTANT k3 : INT; END_VAR
v := v + n * k3;
total := total + n;
bump := v;
END_FUNCTION

FUNCTION peek : DINT
VAR_EXTERNAL total : DINT; END_VAR
peek := total;
END_FUNCTION

FUNCTION mark : INT
VAR_OUTPUT q : INT; END_VAR
q := 1;
mark := 1;
END_FUNCTION

PROGRAM main
VAR
  vin AT %IW0 : INT;
  zin AT %IW2 : INT;
  qa AT %QW0 : INT;
  qb AT %QW2 : INT;
  qc AT %QW4 : INT;
  qd AT %QW6 : INT;
  qt AT %QD8 : DINT;
  qp AT %QD12 : DINT;
  qe AT %QW16 : INT;
  qm AT %QW18 : INT;
  qf AT %QW20 : INT;
END_VAR
VAR
  x, y, w, i : INT;
  arr : ARRAY[0..3] OF INT;
END_VAR
VAR_EXTERNAL total : DINT; END_VAR
x := vin;
qa := x + bump(1, x);
y := vin;
qb := bump(1, y) * 10 + y;
w := 0;
qc := bump(1, w) - bump(2, w);
qp := peek() + bump(1, w) + peek();
i := vin MOD 4;
IF i < 0 THEN i := 0; END_IF;
qd := bump(5, arr[i]);
qt := total;
x := vin;
qf := x + mark(q => x);
qm := 0;
qe := 100 / zin + mark(q => qm);
END_PROGRAM

CONFIGURATION Plant
  VAR_GLOBAL
    total : DINT;
  END_VAR
  VAR_GLOBAL CONSTANT
    k3 : INT := 3;
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

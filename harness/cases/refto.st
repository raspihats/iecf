(* CODESYS's REFERENCE TO (the reference-to extension, docs/extensions.md):
   bound with REF= to a variable, to what another one references, to an
   array element by a computed index, to a variable at an address, to a
   structure and to a function block instance; read, written and called
   through without `^`; `.field` and `.output` through it; a function
   block's REFERENCE TO input bound by a call, and kept by a call that does
   not pass it; __ISVALIDREF; REF= 0. At tick 4 %IW0 is 99 and a
   REFERENCE TO unbound by REF= 0 is written: a fault (IECF_FAULT_NULL),
   the statement stores nothing, and the task stops, as at tick 5.
   Worked by hand, a tick at a time: a = vin + 1, p.a = 2a, arr[vin MOD 4]
   (0 if negative) := 7, %QW4 its sum, b grows by vin + 1, %QW6 the sum
   of the vins, %QW0 = a + 100b + p.a: 309, 506, 1118, 1100, then 11400
   before the fault.
   reference-skip: REFERENCE TO is not IEC 61131-3, and the reference
   compiler has none. *)
TYPE pair : STRUCT a : INT; b : BYTE; END_STRUCT; END_TYPE

FUNCTION_BLOCK addto
VAR_INPUT r : REFERENCE TO INT; x : INT; END_VAR
VAR_OUTPUT ok : BOOL; END_VAR
ok := __ISVALIDREF(r);
IF ok THEN r := r + x; END_IF;
END_FUNCTION_BLOCK

FUNCTION_BLOCK acc
VAR_INPUT x : INT; END_VAR
VAR_OUTPUT total : INT; END_VAR
total := total + x;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  vin AT %IW0 : INT;
  q AT %QW0 : INT;
  qb AT %QX2.0 : BOOL;
  qok AT %QX2.1 : BOOL;
  qsum AT %QW4 : INT;
  qtotal AT %QW6 : INT;
END_VAR
VAR
  a, b, i : INT;
  arr : ARRAY[0..3] OF INT;
  p : pair;
  ra, rb, rq : REFERENCE TO INT;
  rp : REFERENCE TO pair;
  f : addto;
  c : acc;
  rc : REFERENCE TO acc;
END_VAR
ra REF= a;
ra := vin;
rb REF= ra;
rb := rb + 1;
rp REF= p;
rp.a := a * 2;
i := vin MOD 4;
IF i < 0 THEN i := 0; END_IF;
ra REF= arr[i];
ra := 7;
rq REF= qsum;
rq := arr[0] + arr[1] + arr[2] + arr[3];
f(r := b, x := vin);
f(x := 1);
qok := f.ok;
rc REF= c;
rc(x := vin);
qtotal := rc.total;
q := a + b * 100 + p.a;
qb := __ISVALIDREF(rb);
rb REF= 0;
IF vin = 99 THEN rb := 5; END_IF;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

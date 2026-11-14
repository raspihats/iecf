(* An element by an index out of bounds given to a FUNCTION's in-out
   of a structure: the statement stops before the call, which would
   have run on another element (docs/dependencies.md). fp writes g (Q1)
   when it runs: Q1 stays 00, Q0 is 07 from the line before, and the
   task faults (3, an index) at the call's line.
   reference-skip: VAR_IN_OUT of a FUNCTION, as `fnorder`. *)
TYPE pt : STRUCT x : INT; y : INT; END_STRUCT; END_TYPE
FUNCTION fp : INT
  VAR_IN_OUT p : pt; END_VAR
  VAR_EXTERNAL g : USINT; END_VAR
  g := 9;
  p.x := 1;
  fp := p.x;
END_FUNCTION
PROGRAM main
  VAR a : ARRAY [1..2] OF pt; k : INT := 3; r : INT; END_VAR
  VAR q AT %QB0 : USINT; END_VAR
  q := 7;
  r := fp(a[k]);
END_PROGRAM
CONFIGURATION c
  VAR_GLOBAL g AT %QB1 : USINT; END_VAR
  RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE
END_CONFIGURATION

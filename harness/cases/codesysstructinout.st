(* A structure extending another given to an in-out of its base's type
   (struct-extends), as TcOpen's TcoTixonFeeding gives its steps' Config,
   Control and Status to its tasks' generic in-outs. The block keeps the
   in-out and a method writes through it later (method-in-outs).
   Worked by hand, each tick: bp(io := t) adds 10 to t.a, bp.Again()
   100; F(m := t) adds 5 to t.b and gives t.a + t.b; bq(io := arr[i])
   adds 10 to arr[2].a. Tick 1: t.a 1 + 110 = 111, t.b 2 + 5 = 7,
   F 118, t.c 3, arr[2].a 11, arr[1].a 1. Tick 2: t.a 221, t.b 12,
   F 233, t.c 3, arr[2].a 21, arr[1].a 1.
   reference-skip: not IEC 61131-3. *)
TYPE
  Base : STRUCT a : INT := 1; END_STRUCT;
  Middle EXTENDS Base : STRUCT b : INT := 2; END_STRUCT;
  Top EXTENDS Middle : STRUCT c : INT := 3; END_STRUCT;
END_TYPE

FUNCTION_BLOCK Bump
  VAR_IN_OUT io : Base; END_VAR
  METHOD Again
    io.a := io.a + 100;
  END_METHOD
  io.a := io.a + 10;
END_FUNCTION_BLOCK

FUNCTION F : INT
  VAR_IN_OUT m : Middle; END_VAR
  m.b := m.b + 5;
  F := m.a + m.b;
END_FUNCTION

PROGRAM main
  VAR t : Top; arr : ARRAY[1..2] OF Top; i : INT := 2; bp : Bump; bq : Bump; x : INT; END_VAR
  VAR q0 AT %QW0 : INT; q2 AT %QW2 : INT; q4 AT %QW4 : INT; END_VAR
  VAR q6 AT %QW6 : INT; q8 AT %QW8 : INT; q10 AT %QW10 : INT; END_VAR
  bp(io := t);
  bp.Again();
  x := F(m := t);
  bq(io := arr[i]);
  q0 := t.a;
  q2 := t.b;
  q4 := x;
  q6 := t.c;
  q8 := arr[2].a;
  q10 := arr[1].a;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION

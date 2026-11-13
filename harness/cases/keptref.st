(* A reference a RETAIN program instance keeps (W0006; Ed.3 6.5.6.2):
   kept as it is across a warm restart, as CODESYS and TwinCAT keep a
   pointer in retained memory; the plain instance's is NULL again.
   Worked by hand:

   tick 0: both take r := REF(x), n 1, and count through it: x 1 (Q0
   01 00, Q4 01 00; n Q2 01, Q6 01).
   tick 1: x 2, n stays 1.
   warm, before tick 2: a keeps r, x 2 and n 1; b starts again.
   tick 2: a counts on, x 3 (03 00), n 1; b takes the reference again,
   n 1, x 1.
   reference-skip: untried; the reference host has no warm restart. *)
PROGRAM main
  VAR r : REF_TO INT; x : INT; n : USINT; END_VAR
  VAR q AT %Q* : INT; qn AT %Q* : USINT; END_VAR
  IF r = NULL THEN
    r := REF(x);
    n := n + 1;
  END_IF;
  r^ := r^ + 1;
  q := x;
  qn := n;
END_PROGRAM
CONFIGURATION plant
  RESOURCE cpu ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM RETAIN a WITH t : main;
    PROGRAM b WITH t : main;
  END_RESOURCE
  VAR_CONFIG
    cpu.a.q AT %QW0 : INT;
    cpu.a.qn AT %QB2 : USINT;
    cpu.b.q AT %QW4 : INT;
    cpu.b.qn AT %QB6 : USINT;
  END_VAR
END_CONFIGURATION

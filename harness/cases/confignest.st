(* VAR_CONFIG's initial value for a variable of a block held in a
   RETAIN block (Ed.3 6.5.6.2, 6.8.2): the inner block is in a section
   that says neither RETAIN nor NON_RETAIN, so it keeps what its own
   type says, nothing, and starts again at a warm restart - with
   VAR_CONFIG's value, which prevails. Worked by hand:

   tick 0: b.x 50 + 1 = 51 (Q0 33 00); a.y 70 + 1 = 71 (Q2 47 00).
   tick 1: 52, 72.
   warm, before tick 2: a is RETAIN, y stays 72; b starts again, x 1,
   then VAR_CONFIG's 50.
   tick 2: 51 (33 00), 73 (49 00).
   confignest.iecf.trace: the reference compiler generates nothing for
   VAR_CONFIG (as `varconfig`), and its host's warm restart is a cold
   one: x and y from 1 each time. *)
FUNCTION_BLOCK inner
  VAR_OUTPUT x : INT := 1; END_VAR
  x := x + 1;
END_FUNCTION_BLOCK
FUNCTION_BLOCK outer
  VAR_OUTPUT y : INT := 1; END_VAR
  VAR b : inner; END_VAR
  VAR_OUTPUT bx : INT; END_VAR
  b();
  y := y + 1;
  bx := b.x;
END_FUNCTION_BLOCK
PROGRAM main
  VAR RETAIN a : outer; END_VAR
  VAR q AT %QW0 : INT; q2 AT %QW2 : INT; END_VAR
  a();
  q := a.bx;
  q2 := a.y;
END_PROGRAM
CONFIGURATION plant
  RESOURCE cpu ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
  VAR_CONFIG
    cpu.p.a.b.x : INT := 50;
    cpu.p.a.y : INT := 70;
  END_VAR
END_CONFIGURATION

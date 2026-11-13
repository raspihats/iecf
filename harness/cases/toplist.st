(* A VAR_GLOBAL list outside any CONFIGURATION, as CODESYS writes one:
   seen by every configuration, each with its own of each variable; the
   configuration's own lim hides the list's. A plain, a CONSTANT, a
   RETAIN and a located one, reached by a program and a block. Worked
   by hand:

   tick 0: the block adds the list's kk, 4, to its g: 9; q is g + the
   configuration's lim, 100: 109 (Q0 6d 00); kept 11 (Q2); out 1 (Q6).
   tick 1: g 13, 113 (71 00); kept 12; out 2.
   warm, before tick 2: g 5 again; kept, RETAIN, stays; out is in the
   image.
   tick 2: 109; kept 13; out 3.
   reference-skip: the reference compiler has no VAR_GLOBAL outside a
   CONFIGURATION. *)
VAR_GLOBAL g : INT := 5; lim : INT := 3; END_VAR
VAR_GLOBAL CONSTANT kk : INT := 4; END_VAR
VAR_GLOBAL RETAIN kept : INT := 10; out AT %QW6 : INT; END_VAR
FUNCTION_BLOCK counter
  VAR_EXTERNAL g : INT; END_VAR
  VAR_EXTERNAL CONSTANT kk : INT; END_VAR
  g := g + kk;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_EXTERNAL g : INT; lim : INT; kept : INT; out : INT; END_VAR
  VAR c : counter; END_VAR
  VAR q AT %Q* : INT; q2 AT %Q* : INT; END_VAR
  c();
  kept := kept + 1;
  q := g + lim;
  q2 := kept;
  out := out + 1;
END_PROGRAM
CONFIGURATION A
  VAR_GLOBAL lim : INT := 100; END_VAR
  RESOURCE r1 ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE
  VAR_CONFIG r1.p.q AT %QW0 : INT; r1.p.q2 AT %QW2 : INT; END_VAR
END_CONFIGURATION

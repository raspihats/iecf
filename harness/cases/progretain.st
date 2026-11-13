(* RETAIN on a program instance (Ed.3 6.5.6.2): the instance's variables
   of sections neither RETAIN nor NON_RETAIN are kept across a warm
   restart; one that is itself a block keeps what its own type says. a
   is RETAIN, b is plain, c NON_RETAIN, which is what b is. o is a + p.x
   + arr[1] + c.n + c.kept + r + nr + LEN(s). Worked by hand:

   init: ini 7 at each instance's address (Q2, Q6, Q10).
   tick 0, a (VAR_CONFIG's a 100, inc 1): 101 + 4 + 7 + 1 + 1 + 21 + 31
   + 3 = 169 (Q0 a9 00, and oa, Q20); ini 8. b (a 200, inc 2): 202 + 4 +
   7 + 1 + 1 + 21 + 31 + 3 = 270 (Q4 0e 01, ob, Q22). c (a 10): 11 + ...
   = 79 (Q8 4f 00).
   tick 1: a 102 + 5 + 8 + 2 + 2 + 22 + 32 + 4 = 177 (b1 00), ini 9; b
   204 + ... = 279 (17 01); c 87 (57 00).
   warm, before tick 2. a keeps a 102 (VAR_CONFIG's 100 is not given
   again), p.x 5, arr[1] 8, s 'abxx', inc, and ini, 9, in the image; r is
   RETAIN anyway; nr, NON_RETAIN, starts at 30; its block c starts again
   but for its own RETAIN kept, 2. b and c keep r and the block's kept
   alone: a 200 and 10 again, ini 7.
   tick 2: a 103 + 6 + 9 + 1 + 3 + 23 + 31 + 5 = 181 (b5 00), ini 10; b
   202 + 4 + 7 + 1 + 3 + 23 + 31 + 3 = 274 (12 01), ini 8; c 83 (53 00).
   tick 3: a 104 + 7 + 10 + 2 + 4 + 24 + 32 + 6 = 189 (bd 00), ini 11; b
   283 (1b 01); c 91 (5b 00).
   reference-skip: untried on the reference compiler, whose host makes
   a warm restart a cold one. *)
TYPE Pt : STRUCT x : INT := 3; y : INT; END_STRUCT; END_TYPE
FUNCTION_BLOCK counter
  VAR_OUTPUT n : INT; END_VAR
  VAR_OUTPUT RETAIN kept : INT; END_VAR
  n := n + 1;
  kept := kept + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_INPUT inc : INT := 1; END_VAR
  VAR a : INT := 10; p : Pt; arr : ARRAY [0..1] OF INT := [5, 6]; c : counter; tm : TON; s : STRING[8] := 'ab'; END_VAR
  VAR RETAIN r : INT := 20; END_VAR
  VAR NON_RETAIN nr : INT := 30; END_VAR
  VAR q AT %Q* : INT; END_VAR
  VAR ini AT %Q* : INT := 7; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  a := a + inc;
  p.x := p.x + 1;
  arr[1] := arr[1] + 1;
  c();
  r := r + 1;
  nr := nr + 1;
  ini := ini + 1;
  s := CONCAT(s, 'x');
  o := a + p.x + arr[1] + c.n + c.kept + r + nr + LEN(s);
  q := o;
END_PROGRAM
CONFIGURATION plant
  VAR_GLOBAL two : INT := 2; oa AT %QW20 : INT; ob AT %QW22 : INT; END_VAR
  RESOURCE cpu ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM RETAIN a WITH t : main(o => oa);
    PROGRAM b WITH t : main(inc := two, o => ob);
    PROGRAM NON_RETAIN c WITH t : main;
  END_RESOURCE
  VAR_CONFIG
    cpu.a.q AT %QW0 : INT;
    cpu.a.ini AT %QW2 : INT;
    cpu.b.q AT %QW4 : INT;
    cpu.b.ini AT %QW6 : INT;
    cpu.c.q AT %QW8 : INT;
    cpu.c.ini AT %QW10 : INT;
    cpu.a.a : INT := 100;
    cpu.b.a : INT := 200;
  END_VAR
END_CONFIGURATION

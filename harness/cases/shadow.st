(* A global named as one further out hides it where it is seen (as a
   local name does in CODESYS; Ed.3 gives no rule): r1's g hides the
   configuration's in r1, the PROGRAM own's g hides r1's for what own
   holds, and r2, which declares none, sees the configuration's.
   Worked by hand:

   tick 0. r1.a (main): the block counts r1's g, 100 + 1 = 101; main
   adds 10: 111 (Q0 6f 00). r1.b (own): its block counts own's g, 1000
   + 1 = 1001, own adds 100: 1101 (Q2 4d 04). r2.a (main): the
   configuration's g, 1 + 1 + 10 = 12 (Q4 0c 00). r1.a's connection i
   := g reads r1's g before a runs, 100, and o => res puts it in r1's
   res (Q6 64 00).
   tick 1: 122 (7a 00), 1202 (b2 04), 23 (17 00), res 111 (6f 00).
   reference-skip: a global named as an outer one: the reference
   compiler refuses it. *)
FUNCTION_BLOCK counter
  VAR_EXTERNAL g : INT; END_VAR
  g := g + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_INPUT i : INT; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  VAR_EXTERNAL g : INT; END_VAR
  VAR c : counter; END_VAR
  VAR q AT %Q* : INT; END_VAR
  c();
  g := g + 10;
  q := g;
  o := i;
END_PROGRAM
PROGRAM own
  VAR_GLOBAL g : INT := 1000; END_VAR
  VAR c : counter; END_VAR
  VAR q AT %QW2 : INT; END_VAR
  c();
  g := g + 100;
  q := g;
END_PROGRAM
CONFIGURATION plant
  VAR_GLOBAL g : INT := 1; END_VAR
  RESOURCE r1 ON PLC
    VAR_GLOBAL g : INT := 100; END_VAR
    VAR_GLOBAL res AT %QW6 : INT; END_VAR
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM a WITH t : main(i := g, o => res);
    PROGRAM b WITH t : own;
  END_RESOURCE
  RESOURCE r2 ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 2);
    PROGRAM a WITH t : main;
  END_RESOURCE
  VAR_CONFIG
    r1.a.q AT %QW0 : INT;
    r2.a.q AT %QW4 : INT;
  END_VAR
END_CONFIGURATION

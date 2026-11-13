(* A PROGRAM's own VAR_GLOBAL (Ed.3 Table 47, 8a and 8b): each instance
   of the program has its own, seen by the instances it holds and the
   FUNCTIONs it calls. main runs twice in r1, a and b, each with its own
   z, gi and kept; solo has a z of its own, which its FUNCTION reads;
   plain, in r2, reaches r2's z. Worked by hand, inc 2:

   tick 0, a (z 5): c 7, the VAR_TEMP t 9, h.c 11, gi, the program's
   global instance, 13, v's own VAR_TEMP 15; z + 10 = 25 (Q0 19 00); q2
   13 + 15 = 28 (Q2 1c 00); kept 51 (Q4 33). b: the same, of its own
   (Q8 19 00, Q10 1c 00, Q12 33). out is at %QB20 for both: 2. solo (z
   1000, then VAR_CONFIG's 2000): c 2002, readz 2002 (Q16 d2 07). plain (r2's z 300): 302 (Q18
   2e 01).
   tick 1, a (z 25): 27, 29, 31, 33, 35; 45 (2d 00); 33 + 35 = 68 (44
   00); kept 52. solo 2004, plain 304, out 4.
   warm, before tick 2: all starts again but kept, RETAIN, each
   instance's own (53), and out, in the image (6).
   reference-skip: untried on the reference compiler. *)
TYPE Hold : STRUCT c : counter; END_STRUCT; END_TYPE
FUNCTION_BLOCK counter
  VAR_EXTERNAL z : INT; END_VAR
  VAR_EXTERNAL CONSTANT inc : INT; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  z := z + inc;
  o := z;
END_FUNCTION_BLOCK
FUNCTION readz : INT
  VAR_EXTERNAL z : INT; END_VAR
  readz := z;
END_FUNCTION
FUNCTION_BLOCK viatemp
  VAR_OUTPUT o : INT; END_VAR
  VAR_TEMP t : counter; END_VAR
  t();
  o := t.o;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_GLOBAL z : INT := 5; gi : counter; END_VAR
  VAR_GLOBAL CONSTANT inc : INT := 2; END_VAR
  VAR_GLOBAL RETAIN kept : USINT := 50; END_VAR
  VAR_GLOBAL out AT %QB20 : USINT; END_VAR
  VAR c : counter; h : Hold; v : viatemp; END_VAR
  VAR q AT %Q* : INT; q2 AT %Q* : INT; q3 AT %Q* : USINT; END_VAR
  VAR_TEMP t : counter; END_VAR
  c();
  t();
  h.c();
  gi();
  v();
  z := z + 10;
  q := z;
  q2 := gi.o + v.o;
  kept := kept + 1;
  q3 := kept;
  out := out + 1;
END_PROGRAM
PROGRAM solo
  VAR_GLOBAL z : INT := 1000; END_VAR
  VAR_GLOBAL CONSTANT inc : INT := 2; END_VAR
  VAR c : counter; END_VAR
  VAR q AT %QW16 : INT; END_VAR
  c();
  q := readz();
END_PROGRAM
PROGRAM plain
  VAR c : counter; END_VAR
  VAR q AT %QW18 : INT; END_VAR
  c();
  q := c.o;
END_PROGRAM
CONFIGURATION plant
  RESOURCE r1 ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM a WITH t : main;
    PROGRAM b WITH t : main;
    PROGRAM s WITH t : solo;
  END_RESOURCE
  RESOURCE r2 ON PLC
    VAR_GLOBAL z : INT := 300; END_VAR
    VAR_GLOBAL CONSTANT inc : INT := 2; END_VAR
    TASK t(INTERVAL := T#10ms, PRIORITY := 2);
    PROGRAM p WITH t : plain;
  END_RESOURCE
  VAR_CONFIG
    r1.a.q AT %QW0 : INT;
    r1.a.q2 AT %QW2 : INT;
    r1.a.q3 AT %QB4 : USINT;
    r1.b.q AT %QW8 : INT;
    r1.b.q2 AT %QW10 : INT;
    r1.b.q3 AT %QB12 : USINT;
    r1.s.z : INT := 2000;
  END_VAR
END_CONFIGURATION

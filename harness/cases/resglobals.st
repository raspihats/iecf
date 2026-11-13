(* The scope of globals (Ed.3 6.5.2.2): each RESOURCE has its own, and
   the configuration's are seen by all. main runs in r1 and in r2, so
   every instance of it, and of the blocks it holds, reaches its own
   resource's g, p, s, a, io, gc, kept: through a pointer the instance
   holds (docs/c-output.md §3). k and shared are the configuration's.
   r1's task runs first (PRIORITY 1), then r2's. Worked by hand:

   tick 0, r1 (g 100): c 101, the VAR_TEMP t 102, arr[1] 103, h.c 104
   (its n 3, lim); h2 := h copies n 3, h2.c 105 (n 6); gc, r1's own
   instance, 106. q 101 + 1000 = 1101 (Q0 4d 04). d.both: bump p.x 11,
   s 'a', 1; a[1] 0 + 1 = 1; inner 107; 1 + 11 = 12 (Q2 0c 00). q3 106
   + readk 1000 = 1106 (Q4 52 04). io 1 (Q20). kept 51 (Q6 33). outp 7
   + 6 = 13 into r1's cout (Q28 0d 00). owner: shared refers to its
   mine.
   r2 (g 200): the same with 201..207: q 1201 (Q10 b1 04); p.x 21, s
   'zza', 3, a[1] 5 + 3 = 8, 29 (Q12 1d 00); q3 1206 (Q14 b6 04); io 1
   (Q21); kept 61 (Q16 3d); outp 9 + 6 = 15 (Q30 0f 00). user: shared^()
   runs r1's instance from r2: it counts r1's g, 108 (Q24 6c 00); mine
   := shared^ copies its values and keeps r2's g: mine() 208 (Q26 d0
   00).
   tick 1, r1 (g 108): c 109, q 1109 (55 04); gc 114, q3 1114 (5a 04);
   p.x 12, s 'aa', a[1] 3, 15; kept 52; h.c n 6, h2 9, outp 16. r2 (g
   208): q 1209, q3 1214, p.x 22, s 'zzaa', a[1] 12, 34; kept 62; outp
   18. user: r1's g 116 (74 00), r2's 216 (d8 00).
   warm, before tick 2: all starts again but kept, RETAIN, each
   resource's own (53, 63), and io, in the image.
   reference-skip: the reference compiler has one set of globals. *)
TYPE
  Pt : STRUCT x : INT; y : INT; END_STRUCT;
  Hold : STRUCT c : counter; n : INT; END_STRUCT;
END_TYPE
FUNCTION_BLOCK counter
  VAR_EXTERNAL g : INT; END_VAR
  VAR_EXTERNAL CONSTANT lim : INT; END_VAR
  VAR_OUTPUT o : INT; n : INT; END_VAR
  g := g + 1;
  n := n + lim;
  o := g;
END_FUNCTION_BLOCK
FUNCTION_BLOCK base
  VAR_EXTERNAL p : Pt; END_VAR
  METHOD PUBLIC bump : INT
    VAR_EXTERNAL s : STRING[8]; END_VAR
    p.x := p.x + 1;
    s := CONCAT(s, 'a');
    bump := LEN(s);
  END_METHOD
END_FUNCTION_BLOCK
FUNCTION_BLOCK derived EXTENDS base
  VAR_EXTERNAL a : ARRAY [0..1] OF INT; END_VAR
  VAR inner : counter; END_VAR
  METHOD PUBLIC both : INT
    a[1] := a[1] + SUPER.bump();
    inner();
    both := a[1] + p.x;
  END_METHOD
END_FUNCTION_BLOCK
FUNCTION readk : INT
  VAR_EXTERNAL k : INT; END_VAR
  readk := k;
END_FUNCTION
PROGRAM main
  VAR_INPUT inp : INT; END_VAR
  VAR_OUTPUT outp : INT; END_VAR
  VAR_EXTERNAL k : INT; io : USINT; gc : counter; kept : USINT; END_VAR
  VAR c : counter; d : derived; arr : ARRAY [0..1] OF counter; h, h2 : Hold; END_VAR
  VAR q AT %Q* : INT; q2 AT %Q* : INT; q3 AT %Q* : INT; q4 AT %Q* : USINT; END_VAR
  VAR_TEMP t : counter; END_VAR
  c();
  t();
  arr[1]();
  h.c();
  h2 := h;
  h2.c();
  gc();
  q := c.o + k;
  q2 := d.both();
  q3 := gc.o + readk();
  io := io + 1;
  kept := kept + 1;
  q4 := kept;
  outp := inp + h2.c.n;
END_PROGRAM
PROGRAM owner
  VAR_EXTERNAL shared : REF_TO counter; END_VAR
  VAR mine : counter; END_VAR
  shared := REF(mine);
END_PROGRAM
PROGRAM user
  VAR_EXTERNAL shared : REF_TO counter; g : INT; END_VAR
  VAR mine : counter; END_VAR
  VAR q AT %QW24 : INT; q2 AT %QW26 : INT; END_VAR
  shared^();
  q := shared^.o;
  mine := shared^;
  mine();
  q2 := mine.o;
END_PROGRAM
CONFIGURATION plant
  VAR_GLOBAL k : INT := 1000; shared : REF_TO counter; END_VAR
  RESOURCE r1 ON PLC
    VAR_GLOBAL g : INT := 100; p : Pt := (x := 10); s : STRING[8]; a : ARRAY [0..1] OF INT; END_VAR
    VAR_GLOBAL io AT %QB20 : USINT; gc : counter; cin : INT := 7; cout AT %QW28 : INT; END_VAR
    VAR_GLOBAL CONSTANT lim : INT := 3; END_VAR
    VAR_GLOBAL RETAIN kept : USINT := 50; END_VAR
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p1 WITH t : main(inp := cin, outp => cout);
    PROGRAM own WITH t : owner;
  END_RESOURCE
  RESOURCE r2 ON PLC
    VAR_GLOBAL g : INT := 200; p : Pt := (x := 20); s : STRING[8] := 'zz'; a : ARRAY [0..1] OF INT := [0, 5]; END_VAR
    VAR_GLOBAL io AT %QB21 : USINT; gc : counter; cin : INT := 9; cout AT %QW30 : INT; END_VAR
    VAR_GLOBAL CONSTANT lim : INT := 3; END_VAR
    VAR_GLOBAL RETAIN kept : USINT := 60; END_VAR
    TASK t(INTERVAL := T#10ms, PRIORITY := 2);
    PROGRAM p1 WITH t : main(inp := cin, outp => cout);
    PROGRAM use WITH t : user;
  END_RESOURCE
  VAR_CONFIG
    r1.p1.q AT %QW0 : INT;
    r1.p1.q2 AT %QW2 : INT;
    r1.p1.q3 AT %QW4 : INT;
    r1.p1.q4 AT %QB6 : USINT;
    r2.p1.q AT %QW10 : INT;
    r2.p1.q2 AT %QW12 : INT;
    r2.p1.q3 AT %QW14 : INT;
    r2.p1.q4 AT %QB16 : USINT;
  END_VAR
END_CONFIGURATION

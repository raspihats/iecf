(* Structures with relative locations (Ed.3 6.4.4.7): each component at
   its byte, a BOOL at its bit, gaps between; OVERLAP lets components
   share bytes. The layout is iecf's (docs/c-output.md §4): little-endian,
   a structure the same bytes in C memory as at an address, an array or
   structure inside it back to back. Worked by hand:

   init: m (Com_data, %MB0..%MB25) has C1's initial values where C1 and
   C2 share bytes, C1 being declared first: length 26 at byte 2 (M 1a).
   q (Frame, %QB0..%QB15): id A5, pts 0, name 'ab' (61 62 00 00), byte 13
   lit (bit 7) 80, w BEEF (ef be).

   tick 0:
   c.C2.data2 := 1.0 puts 16#3F800000 at c's bytes 5 to 8 (00 00 80 3f);
   c.C1.head 16#1234 (34 12); c.C2.flag2 bit 3 and c.C1.flag1 bit 0 of
   byte 3 (09); length still 26. m := c: M 34 12 1a 09 00 00 00 80 3f,
   then 0. m.C2.data1 := 16#7F writes byte 5, data2's low byte (M5 7f);
   x := m.C2.data2 > 1.0 is TRUE, data2 16#3F80007F: %QX16.0.
   q.pts[1].y := -2: element 1 at byte 5, y 2 further, bytes 7, 8
   (fe ff); q.pts[0].act := TRUE, byte 2 (01). Bump(q.pts[1].y) adds 1
   through an in-out: -1 (ff ff). q.hi := q.lit: byte 13 81; Flip(q.hi)
   through an in-out: 80 again.
   f := q, the 16 bytes into C memory. f.id + 1: A6. f.name := 'xyz'
   (78 79 7a 00). ri := REF(f.pts[1].y); ri^ + 10: 9 (09 00).
   rb := REF(f.lit); rb^ := FALSE: byte 13 00. q := f: Q a6 00 01 00 00
   00 00 09 00 78 79 7a 00 00 ef be.
   r := REF(c1), a C pointer, r^.tail := 16#EE: c1's byte 25 through
   the structure's bytes. r := REF(m.C1), in the image: r^.length :=
   r^.length + 1: M2 1b. g := Make(16#11): a function's result, id 11,
   copied whole into %QB17..%QB32 with h := g.w = 16#BEEF (TRUE, %QX16.1)
   and k := c1.tail = 16#EE (TRUE, %QX16.2).
   %QB17.. g: 11 00 00 00 00 00 00 00 00 61 62 00 00 80 ef be.

   tick 1: q is as tick 0 left it. y -2, then -1, as before; but
   q.lit is FALSE now, so q.hi := q.lit is FALSE and Flip makes it
   TRUE: byte 13 01. f := q brings id A6, f.id + 1 is A7: Q a7 00 01 00
   00 00 00 09 00 78 79 7a 00 01 ef be. m is c's bytes again, length
   26, then + 1: 1b again.
   reference-skip: the reference compiler (Ed.2) has no relative
   locations. *)
TYPE
  Com1_data : STRUCT
    head   AT %B0   : INT;
    length AT %B2   : USINT := 26;
    flag1  AT %X3.0 : BOOL;
    tail   AT %B25  : BYTE;
  END_STRUCT;
  Com2_data : STRUCT OVERLAP
    head   AT %B0   : INT;
    length AT %B2   : USINT;
    flag2  AT %X3.3 : BOOL;
    data1  AT %B5   : BYTE;
    data2  AT %B5   : REAL;
    tail   AT %B19  : BYTE;
  END_STRUCT;
  Com_data : STRUCT OVERLAP
    C1 AT %B0 : Com1_data;
    C2 AT %B0 : Com2_data;
  END_STRUCT;
  Pt : STRUCT x : BYTE; act : BOOL; y : INT; END_STRUCT;
  Frame : STRUCT
    id   AT %B0    : USINT := 16#A5;
    pts  AT %B1    : ARRAY [0..1] OF Pt;
    name AT %B9    : STRING(3) := 'ab';
    lit  AT %X13.7 : BOOL := TRUE;
    hi   AT %X13.0 : BOOL;
    w    AT %W14   : WORD := 16#BEEF;
  END_STRUCT;
END_TYPE
FUNCTION Bump : INT
  VAR_IN_OUT v : INT; END_VAR
  v := v + 1;
  Bump := v;
END_FUNCTION
FUNCTION Flip : BOOL
  VAR_IN_OUT b : BOOL; END_VAR
  b := NOT b;
  Flip := b;
END_FUNCTION
FUNCTION Make : Frame
  VAR_INPUT k : USINT; END_VAR
  Make.id := k;
END_FUNCTION
PROGRAM main
  VAR
    c : Com_data;
    c1 : Com1_data;
    f : Frame;
    r : REF_TO Com1_data;
    rb : REF_TO BOOL;
    ri : REF_TO INT;
    n : INT;
    b : BOOL;
  END_VAR
  VAR
    m AT %MB0 : Com_data;
    q AT %QB0 : Frame;
    x AT %QX16.0 : BOOL;
    h AT %QX16.1 : BOOL;
    k AT %QX16.2 : BOOL;
    g AT %QB17 : Frame;
  END_VAR
  c.C2.data2 := 1.0;
  c.C1.head := 16#1234;
  c.C2.flag2 := TRUE;
  c.C1.flag1 := TRUE;
  m := c;
  m.C2.data1 := 16#7F;
  x := m.C2.data2 > 1.0;
  q.pts[1].y := -2;
  q.pts[0].act := TRUE;
  n := Bump(q.pts[1].y);
  q.hi := q.lit;
  b := Flip(q.hi);
  f := q;
  f.id := f.id + 1;
  f.name := 'xyz';
  ri := REF(f.pts[1].y);
  ri^ := ri^ + 10;
  rb := REF(f.lit);
  rb^ := FALSE;
  q := f;
  r := REF(c1);
  r^.tail := 16#EE;
  r := REF(m.C1);
  r^.length := r^.length + 1;
  g := Make(16#11);
  h := g.w = 16#BEEF;
  k := c1.tail = 16#EE;
END_PROGRAM
CONFIGURATION plant
  RESOURCE r ON PLC
    TASK t (INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

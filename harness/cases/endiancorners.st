(* The endianness functions (Ed.3 6.6.2.5.13) of what holds more than
   data: a structure holding an instance, a structure with `*`
   components, and ARRAY[*]. Worked by hand, on a little-endian target:
   tick 0 (%ID0 16#01020304):
   x.c, a CTU in Box, counts once: CV 1. y := TO_BIG_ENDIAN(x): y.w
   16#0A0B0C0D reversed, MD32 0a 0b 0c 0d; y.c copied as it is, an
   instance being no data type's value: CV 1 (MW36 01 00).
   a.out := 16#CAFEBABE at %QD0 (be ba fe ca); a.k := 16#1234. b :=
   TO_BIG_ENDIAN(a): each `*` part converted in a buffer of its own, so
   a's addresses are not written: Q0 stays be ba fe ca, b.out at %QD4 is
   ca fe ba be; b.raw at %ID4 is a.raw reversed; bk := b.k, 16#3412
   (MW40 12 34). braw := b.raw reads it back: 16#04030201 (MD42 01 02
   03 04).
   s(a := p, b := q, m := g, n := h): q := TO_BIG_ENDIAN(p) stored, then
   converted where it is: MD8 11 22 33 44 55 66 77 88; h :=
   FROM_BIG_ENDIAN(g), each INT reversed: MW24 01 02 03 04 05 06 07 08;
   r := TO_LITTLE_ENDIAN(z) (z of fixed bounds into an ARRAY[*]) as it
   is: MD46 78 56 34 12.
   tick 1: the CTU's CU stays TRUE: CV 1 still; the rest as tick 0.
   reference-skip: the reference compiler (Ed.2) has neither the
   endianness functions nor ARRAY[*], and generates nothing for
   VAR_CONFIG. *)
TYPE
  Box : STRUCT c : CTU; w : DWORD; END_STRUCT;
  IoW : STRUCT raw AT %I* : DWORD; out AT %Q* : DWORD; k : WORD; END_STRUCT;
END_TYPE
FUNCTION_BLOCK Swap
  VAR_IN_OUT a, b, r : ARRAY[*] OF DWORD; m, n : ARRAY[*, *] OF INT; END_VAR
  VAR z : ARRAY[0..0] OF DWORD := [16#12345678]; END_VAR
  b := TO_BIG_ENDIAN(a);
  n := FROM_BIG_ENDIAN(m);
  r := TO_LITTLE_ENDIAN(z);
END_FUNCTION_BLOCK
PROGRAM line
  VAR x, y : Box; a, b : IoW; s : Swap; END_VAR
  VAR
    p AT %MD0 : ARRAY[0..1] OF DWORD := [16#11223344, 16#55667788];
    q AT %MD8 : ARRAY[0..1] OF DWORD;
    g AT %MW16 : ARRAY[0..1, 0..1] OF INT := [16#0102, 16#0304, 16#0506, 16#0708];
    h AT %MW24 : ARRAY[0..1, 0..1] OF INT;
    yw AT %MD32 : DWORD;
    ycv AT %MW36 : INT;
    bk AT %MW40 : WORD;
    braw AT %MD42 : DWORD;
    zr AT %MD46 : ARRAY[0..0] OF DWORD;
  END_VAR
  x.c(CU := TRUE, PV := 5);
  x.w := 16#0A0B0C0D;
  y := TO_BIG_ENDIAN(x);
  yw := y.w;
  ycv := y.c.CV;
  a.out := 16#CAFEBABE;
  a.k := 16#1234;
  b := TO_BIG_ENDIAN(a);
  bk := b.k;
  braw := b.raw;
  s(a := p, b := q, m := g, n := h, r := zr);
END_PROGRAM
CONFIGURATION plant
  RESOURCE cpu ON PLC
    TASK fast (INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM main WITH fast : line;
  END_RESOURCE
  VAR_CONFIG
    cpu.main.a.raw AT %ID0 : DWORD;
    cpu.main.a.out AT %QD0 : DWORD;
    cpu.main.b.raw AT %ID4 : DWORD;
    cpu.main.b.out AT %QD4 : DWORD;
  END_VAR
END_CONFIGURATION

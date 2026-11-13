(* Structures with relative locations (Ed.3 6.4.4.7) wherever a value
   can be, and their components of other types. Worked by hand:

   Rec, 25 bytes: m, an enumeration, at 0 (starts Run, 5: 05 00); s, a
   SINT(-5..5), at 2 (starts at its lower limit -5: fb); p, a Pt, at 3
   (x 7, y -1: 07 00 ff ff); ws, a WSTRING(2) "A", at 7 (41 00, then a
   0 unit, then 0: 41 00 00 00 00 00); l, LINT -2, at 13 (fe ff ff ff ff
   ff ff ff); t, T#1s, at 21 (1000 ms: e8 03 00 00).
   V, an OVERLAP of 2 bytes: m (Mode) and w (UINT) at 0, b at bit 7 of
   byte 1; m is declared first, so it starts at Run: 05 00.

   init: gr, a global Rec at %QB0, its initial value: Q 05 00 fb 07 00
   ff ff 41 00 00 00 00 00 fe ff ff ff ff ff ff ff e8 03 00 00. img, two
   V at %QB25: 05 00 05 00. q29 0. q30 is cv.w, cv a CONSTANT V: its
   bytes, 5, not w's own initial value 0 (05).

   tick 0: the connection gives cin g's bytes (05 00) before main runs.
   a[1].p.y := 300 (2c 01), a[1].ws := "xy" (78 00 79 00 00 00), in C
   memory. fb(i := a[1], io := g): o := i, o.l := o.l * 10 = -20 (ec ff
   ff ff ff ff ff ff), io.w + 1 makes g 6 (06 00). gr := fb.o: Q0..24 05
   00 fb 07 00 2c 01 78 00 79 00 00 00 ec ff ff ff ff ff ff ff e8 03 00
   00. img[0] := cin (05 00), img[0].b := TRUE (05 80); img[1] := g (06
   00). a[0].s := 9 is clamped to 5 and flagged (CONVERSION, 4); q29 := a[0].s
   (05).

   tick 1: cin is 06 00, g 07 00 after the call: img 06 80 07 00; the
   rest as tick 0.
   reference-skip: the reference compiler (Ed.2) has no relative
   locations. *)
TYPE
  Mode : (Run := 5, Halt);
  Pt : STRUCT x : INT := 7; y : INT := -1; END_STRUCT;
  Rec : STRUCT
    m  AT %B0  : Mode;
    s  AT %B2  : SINT(-5..5);
    p  AT %B3  : Pt;
    ws AT %B7  : WSTRING(2) := "A";
    l  AT %B13 : LINT := -2;
    t  AT %B21 : TIME := T#1s;
  END_STRUCT;
  V : STRUCT OVERLAP
    m AT %B0   : Mode;
    w AT %B0   : UINT;
    b AT %X1.7 : BOOL;
  END_STRUCT;
END_TYPE
FUNCTION_BLOCK Fb
  VAR_INPUT i : Rec; END_VAR
  VAR_OUTPUT o : Rec; END_VAR
  VAR_IN_OUT io : V; END_VAR
  VAR k : Rec; END_VAR
  o := i;
  o.l := o.l * 10;
  io.w := io.w + 1;
  k.s := k.s + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_INPUT cin : V; END_VAR
  VAR_EXTERNAL g : V; gr : Rec; END_VAR
  VAR a : ARRAY [0..1] OF Rec; fb : Fb; END_VAR
  VAR RETAIN kept : V; END_VAR
  VAR CONSTANT cv : V; END_VAR
  VAR
    img AT %QB25 : ARRAY [0..1] OF V;
    q29 AT %QB29 : SINT;
    q30 AT %QB30 : USINT;
  END_VAR
  a[1].p.y := 300;
  a[1].ws := "xy";
  fb(i := a[1], io := g);
  gr := fb.o;
  img[0] := cin;
  img[0].b := TRUE;
  img[1] := g;
  kept := g;
  q30 := UINT_TO_USINT(cv.w);
  a[0].s := 9;
  q29 := a[0].s;
END_PROGRAM
CONFIGURATION plant
  VAR_GLOBAL
    g : V;
  END_VAR
  VAR_GLOBAL
    gr AT %QB0 : Rec;
  END_VAR
  RESOURCE r ON PLC
    TASK t (INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main(cin := g);
  END_RESOURCE
END_CONFIGURATION

(* TwinCAT's automatic addressing (auto-addresses): a `*` address no
   VAR_CONFIG entry gives - a global's own, a global's `*` components,
   a program instance's - placed by iecf in its area after the last
   byte any stated address reaches, in the order of the paths, each its
   own type's width, a BOOL a whole byte.
   Stated: x AT %MW10 (M up to 12), VAR_CONFIG RS.P.D.Q AT %QW20 (Q up
   to 22). Placed, by path: I: DEV.A 0, RS.P.D.A 2. Q: DEV.Q 22, FLAG
   24. M: ARR 12 (4 bytes), DEV.M 16, G 18, RS.P.D.M 20, RS.P.P 22.
   Worked by hand, each tick: g + 1, dev.m + 2, p + 3, flag TRUE,
   dev.q := g * 10, arr[1] + 7, d.q := 5, d.m + 1, x + 100.
   Tick 1: M x 100, arr 0 7, dev.m 2, g 1, d.m 1, p 3; Q d.q 5,
   dev.q 10, flag 1. Tick 2: x 200, arr 0 14, dev.m 4, g 2, d.m 2,
   p 6; d.q 5, dev.q 20, flag 1.
   reference-skip: not IEC 61131-3. *)
TYPE Io : STRUCT
  a AT %I* : INT;
  q AT %Q* : INT;
  m AT %M* : INT;
END_STRUCT;
END_TYPE

PROGRAM main
  VAR p AT %M* : INT; END_VAR
  VAR d : Io; END_VAR
  VAR x AT %MW10 : INT; END_VAR
  VAR_EXTERNAL g : INT; flag : BOOL; dev : Io; arr : ARRAY[0..1] OF INT; END_VAR
  g := g + 1;
  dev.m := dev.m + 2;
  p := p + 3;
  flag := TRUE;
  dev.q := g * 10;
  arr[1] := arr[1] + 7;
  d.q := 5;
  d.m := d.m + 1;
  x := x + 100;
END_PROGRAM

CONFIGURATION cf
  VAR_GLOBAL
    g AT %M* : INT;
    flag AT %Q* : BOOL;
    arr AT %M* : ARRAY[0..1] OF INT;
  END_VAR
  VAR_GLOBAL
    dev : Io;
  END_VAR
  RESOURCE rs ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
  VAR_CONFIG
    rs.p.d.q AT %QW20 : INT;
  END_VAR
END_CONFIGURATION
